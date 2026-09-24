# macro_decomp Generalization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix all proven algorithmic bugs and replace benchmark-specific constants with mathematically justified parameters across the macro_decomp pipeline.

**Architecture:** The existing `macro_decomp → fallback` pipeline architecture is preserved. A shared solver helper with preemptive termination is introduced. Each module's filtering thresholds are reclassified as either mathematically required (with theorem reference) or documented performance knobs. Budget allocation prevents macro_decomp from starving the fallback solver.

**Tech Stack:** Rust 1.98, CaDiCaL (via `rustsat_cadical`), Rayon

**Spec:** `docs/superpowers/specs/2026-09-24-macro-decomp-generalization-design.md`

## Global Constraints

- Do NOT change the pipeline architecture (`macro_decomp → fallback`).
- Every remaining threshold must be classified as MATHEMATICAL REQUIREMENT or PERFORMANCE KNOB with documentation.
- No graph currently solving as SAT_VERIFIED may regress to TIMEOUT.
- All CaDiCaL instances must use the shared solver helper with preemptive deadline.
- All positive tour outputs must pass through `TourVerifier::verify` before returning.

## Review Focus

1. **Non-bipartite graphs with even-length contracted pairs** — `can_solve_alternating_pairs` must return `false` on any graph whose contracted pair adjacency contains an odd cycle. The bipartite conflict check must fire on cross-edges, not just tree-edges.
2. **Asymmetric 2-cuts where one component has 1–9 vertices** — `find_2cut_ports` must detect these rather than filtering them. The best-candidate selection should still prefer balanced splits.
3. **CaDiCaL `SolverResult::Interrupted` propagation** — every CEGAR loop must treat this as a timeout, not silently continue or panic. Missing this in even one location breaks the timeout contract.
4. **Budget starvation on graphs where macro_decomp runs but fails** — the fallback must always receive at least `timeout * 0.6` of budget, even after macro_decomp exhausts its allocation.
5. **Parallel Rayon threads sharing a deadline** — `solve_block_a` and `solve_block_b` run concurrently; both must respect the same `corridor_deadline`, and both CaDiCaL instances need independent terminators pointing to the same deadline.

---

### Task 1: Create Solver Helper with Preemptive Deadline

**Files:**
- Create: `src/cegar-fix/src/core/solver_utils.rs`
- Modify: `src/cegar-fix/src/core/mod.rs:1-5`

**Interfaces:**
- Consumes: `rustsat_cadical::CaDiCaL`, `rustsat::solvers::{ControlSignal, Terminate}`, `std::time::Instant`
- Produces: `pub fn create_solver_with_deadline(deadline: Instant) -> CaDiCaL` — used by all subsequent tasks

- [ ] **Step 1: Create `solver_utils.rs` with the helper function**

```rust
// src/cegar-fix/src/core/solver_utils.rs
use rustsat::solvers::{ControlSignal, Terminate};
use rustsat_cadical::CaDiCaL;
use std::time::Instant;

/// Creates a CaDiCaL solver instance with a terminator callback that
/// interrupts the solver when the deadline is reached.
///
/// Without this, `solver.solve()` is a blocking FFI call into C++ that
/// cannot be interrupted by Rust-side deadline checks between CEGAR
/// iterations. The terminator is polled internally by CaDiCaL during
/// search, ensuring prompt return as `SolverResult::Interrupted`.
pub fn create_solver_with_deadline(deadline: Instant) -> CaDiCaL {
    let mut solver = CaDiCaL::default();
    solver.attach_terminator(move || {
        if Instant::now() >= deadline {
            ControlSignal::Terminate
        } else {
            ControlSignal::Continue
        }
    });
    solver
}
```

- [ ] **Step 2: Register the module in `core/mod.rs`**

Add to `src/cegar-fix/src/core/mod.rs`:
```rust
pub mod solver_utils;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p cegar-fix`
Expected: 0 errors, 0 warnings on new code

- [ ] **Step 4: Commit**

```bash
git add src/cegar-fix/src/core/solver_utils.rs src/cegar-fix/src/core/mod.rs
git commit -m "feat(core): add create_solver_with_deadline helper for preemptive CaDiCaL termination"
```

---

### Task 2: Fix `portfolio_788.rs` — Bipartite Check & DRY Extraction

**Files:**
- Modify: `src/cegar-fix/src/macro_decomp/portfolio_788.rs:178-282`

**Interfaces:**
- Consumes: `crate::core::solver_utils::create_solver_with_deadline`
- Produces: `pub fn can_solve_alternating_pairs(raw_g: &Graph) -> bool`, `pub fn solve_alternating_pairs(raw_g: &Graph, timeout_secs: f64) -> Option<Vec<i32>>` (signatures unchanged)

- [ ] **Step 1: Extract shared logic into `AlternatingPairGraph` struct and `extract_alternating_pairs` helper**

Insert above `can_solve_alternating_pairs` (~line 178):

```rust
struct AlternatingPairGraph {
    contracted_g: Graph,
    contractor: Degree2Contractor,
    pairs: Vec<(i32, i32)>,
    v_partner: HashMap<i32, i32>,
    color: HashMap<i32, u8>,
}

/// PERFORMANCE KNOB: Minimum fraction of degree-2 vertices for alternating
/// pair decomposition to be profitable. Below this threshold, contraction
/// yields too few pairs for directed CEGAR to outperform monolithic solving.
const MIN_DEG2_FRACTION: f64 = 0.15;

fn extract_alternating_pairs(raw_g: &Graph) -> Option<AlternatingPairGraph> {
    let deg2_count = raw_g.adjacency_list.values().filter(|nbrs| nbrs.len() == 2).count();
    let n = raw_g.adjacency_list.len();
    let deg2_fraction = deg2_count as f64 / n as f64;
    if deg2_fraction < MIN_DEG2_FRACTION {
        return None;
    }

    let (g, contractor) = Degree2Contractor::contract(raw_g);
    let mut v_partner: HashMap<i32, i32> = HashMap::new();
    let mut pairs: Vec<(i32, i32)> = Vec::new();
    let mut sorted_chains: Vec<(i32, i32)> = contractor.chain_map.keys().copied().collect();
    sorted_chains.sort();

    for (u, w) in sorted_chains {
        if u < w {
            pairs.push((u, w));
            v_partner.insert(u, w);
            v_partner.insert(w, u);
        }
    }

    if pairs.is_empty() {
        return None;
    }

    // Standard BFS bipartite checker with conflict detection
    let mut color: HashMap<i32, u8> = HashMap::new();
    let (u0, w0) = pairs[0];
    color.insert(u0, 0);
    color.insert(w0, 1);
    let mut q = vec![u0, w0];

    while let Some(curr) = q.pop() {
        let curr_c = color[&curr];

        // Partner propagation with conflict check
        if let Some(&vp) = v_partner.get(&curr) {
            if let Some(&existing_color) = color.get(&vp) {
                if existing_color == curr_c {
                    return None; // Odd cycle via partner edge — not bipartite
                }
            } else {
                color.insert(vp, 1 - curr_c);
                q.push(vp);
            }
        }

        // Neighbor propagation with conflict check
        if let Some(nbrs) = g.adjacency_list.get(&curr) {
            let vp = v_partner.get(&curr).copied().unwrap_or(-1);
            for &nxt in nbrs {
                if nxt != vp {
                    if let Some(&existing_color) = color.get(&nxt) {
                        if existing_color == curr_c {
                            return None; // Odd cycle — not bipartite
                        }
                    } else {
                        color.insert(nxt, 1 - curr_c);
                        q.push(nxt);
                    }
                }
            }
        }
    }

    if color.len() < pairs.len() * 2 {
        return None; // Disconnected pair graph
    }

    Some(AlternatingPairGraph {
        contracted_g: g,
        contractor,
        pairs,
        v_partner,
        color,
    })
}
```

- [ ] **Step 2: Rewrite `can_solve_alternating_pairs` to use the helper**

Replace the entire function body (L180–231):

```rust
pub fn can_solve_alternating_pairs(raw_g: &Graph) -> bool {
    extract_alternating_pairs(raw_g).is_some()
}
```

- [ ] **Step 3: Rewrite `solve_alternating_pairs` to use the helper and solver_utils**

Replace lines 235–282 (the contraction + pair extraction + coloring block) with:

```rust
pub fn solve_alternating_pairs(raw_g: &Graph, timeout_secs: f64) -> Option<Vec<i32>> {
    let t_start = Instant::now();
    println!("[macro_alternating] Initializing degree-2 contraction and directed pair graph...");

    let apg = match extract_alternating_pairs(raw_g) {
        Some(apg) => apg,
        None => return None,
    };

    let AlternatingPairGraph { contracted_g: g, contractor, pairs, v_partner, color } = apg;

    // ... rest of the function unchanged from line 283 onward ...
```

Also update the worker thread CaDiCaL initialization (L424) to use the deadline-aware helper:

```rust
// Replace: let mut solver = CaDiCaL::default();
// With:
let deadline = t_start + Duration::from_secs_f64(timeout_secs);
let mut solver = crate::core::solver_utils::create_solver_with_deadline(deadline);
```

Remove the existing manual `attach_terminator` blocks (L449–455, L504–510) since the helper already handles this.

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p cegar-fix`
Expected: 0 errors

- [ ] **Step 5: Run existing test suite**

Run: `cargo test -p cegar-fix`
Expected: All existing tests pass

- [ ] **Step 6: Commit**

```bash
git add src/cegar-fix/src/macro_decomp/portfolio_788.rs
git commit -m "fix(portfolio_788): add bipartite conflict detection, extract shared logic, generalize density guard"
```

---

### Task 3: Fix `corridor.rs` — Remove Filters, Fix Contract, Add Terminator

**Files:**
- Modify: `src/cegar-fix/src/macro_decomp/corridor.rs:39,281,499,505-595`

**Interfaces:**
- Consumes: `crate::core::solver_utils::create_solver_with_deadline`
- Produces: `pub fn find_2cut_ports(raw_g: &Graph) -> Option<(i32, i32)>`, `pub fn solve_2cut_corridor(raw_g: &Graph, timeout_secs: f64) -> Option<Vec<i32>>` (signatures unchanged)

- [ ] **Step 1: Replace `CaDiCaL::default()` with helper in `solve_block_a` (L39)**

```rust
// Replace:
// let mut solver = CaDiCaL::default();
// With:
let mut solver = crate::core::solver_utils::create_solver_with_deadline(deadline);
```

Also update the CEGAR loop error handling (L83–86) to handle `Interrupted`:

```rust
match solver.solve() {
    Ok(SolverResult::Sat) => {}
    Ok(SolverResult::Unsat) => return Err("Block A UNSAT".to_string()),
    Ok(SolverResult::Interrupted) => return Err("Block A timeout".to_string()),
    Err(e) => return Err(format!("Block A solver error: {:?}", e)),
}
```

- [ ] **Step 2: Replace `CaDiCaL::default()` with helper in `solve_block_b` (L281)**

```rust
// Replace:
// let mut solver = CaDiCaL::default();
// With:
let mut solver = crate::core::solver_utils::create_solver_with_deadline(deadline);
```

Update CEGAR loop error handling (L388–391) identically:

```rust
match solver.solve() {
    Ok(SolverResult::Sat) => {}
    Ok(SolverResult::Unsat) => return Err("Block B UNSAT".to_string()),
    Ok(SolverResult::Interrupted) => return Err("Block B timeout".to_string()),
    Err(e) => return Err(format!("Block B solver error: {:?}", e)),
}
```

- [ ] **Step 3: Fix `check_2cut_split` contract (L499)**

Replace:
```rust
comp_sizes.len() >= 2 && comp_sizes.iter().filter(|&&sz| sz >= min_comp_size).count() >= 2
```

With:
```rust
// MATHEMATICAL REQUIREMENT: By the Chvátal-Erdős toughness theorem,
// a Hamiltonian graph satisfies c(G \ S) <= |S|. For |S| = 2,
// exactly 2 components is the only valid case for decomposition.
comp_sizes.len() == 2 && comp_sizes.iter().all(|&sz| sz >= 1)
```

Remove the `min_comp_size` parameter from `check_2cut_split` signature (it is no longer used). Update the call site at L584:

```rust
// Replace: if check_2cut_split(raw_g, u_orig, v_orig, min_comp_sz) {
// With:
if check_2cut_split(raw_g, u_orig, v_orig) {
```

Update the function signature from `fn check_2cut_split(raw_g: &Graph, port_u: i32, port_v: i32, min_comp_size: usize) -> bool` to `fn check_2cut_split(raw_g: &Graph, port_u: i32, port_v: i32) -> bool`.

- [ ] **Step 4: Rewrite `find_2cut_ports` — remove all unjustified filters, add best-candidate selection**

Replace the entire function body (L505–595):

```rust
/// Dynamically discovers a 2-vertex separator {port_u, port_v} whose
/// removal splits the graph into exactly two non-empty components,
/// using Tarjan's linear-time articulation point algorithm on G \ {u}.
///
/// Candidates are sorted by degree ascending for efficiency. The
/// algorithm selects the most balanced 2-cut found (largest minimum
/// component).
pub fn find_2cut_ports(raw_g: &Graph) -> Option<(i32, i32)> {
    let n = raw_g.adjacency_list.len();
    if n < 4 {
        // MATHEMATICAL REQUIREMENT: A 2-vertex separator requires >= 4 vertices.
        return None;
    }

    let mut nodes: Vec<i32> = raw_g.adjacency_list.keys().copied().collect();
    nodes.sort_unstable();

    let node_to_idx: HashMap<i32, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, &node)| (node, i))
        .collect();

    let adj: Vec<Vec<usize>> = nodes
        .iter()
        .map(|&u| {
            raw_g
                .adjacency_list
                .get(&u)
                .map(|nbrs| {
                    nbrs.iter()
                        .filter_map(|nbr| node_to_idx.get(nbr).copied())
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect();

    // Sort candidate indices by degree ascending — low-degree vertices
    // are cheaper to probe and more likely to yield 2-cuts.
    let mut candidates: Vec<usize> = (0..n).collect();
    candidates.sort_by_key(|&u| adj[u].len());

    /// PERFORMANCE KNOB: Minimum corridor size for decomposition to
    /// likely outperform monolithic solving. Does NOT filter candidates
    /// — only controls early exit from the search.
    const MIN_PROFITABLE_CORRIDOR: usize = 10;

    let mut best: Option<(i32, i32, usize)> = None; // (u_orig, v_orig, min_comp)

    for &u in &candidates {
        let deg_u = adj[u].len();
        // MATHEMATICAL REQUIREMENT: degree < 3 cannot form a 2-vertex
        // separator with two non-trivial components in a 2-connected graph.
        if deg_u < 3 {
            continue;
        }

        let root = if u != 0 { 0 } else { 1 };
        let mut tin = vec![-1i32; n];
        let mut low = vec![-1i32; n];
        let mut sz = vec![0usize; n];
        tin[u] = 0;
        let mut timer = 1i32;
        tin[root] = timer;
        low[root] = timer;
        sz[root] = 1;

        let mut stack = vec![(root, usize::MAX, 0usize)];

        while let Some(&mut (curr, p, ref mut nbr_idx)) = stack.last_mut() {
            let nbrs = &adj[curr];
            if *nbr_idx < nbrs.len() {
                let to = nbrs[*nbr_idx];
                *nbr_idx += 1;
                if to == p || to == u {
                    continue;
                }
                if tin[to] != -1 {
                    low[curr] = low[curr].min(tin[to]);
                } else {
                    timer += 1;
                    tin[to] = timer;
                    low[to] = timer;
                    sz[to] = 1;
                    stack.push((to, curr, 0));
                }
            } else {
                let (curr, _p, _) = stack.pop().unwrap();
                if let Some(&(parent, _, _)) = stack.last() {
                    low[parent] = low[parent].min(low[curr]);
                    sz[parent] += sz[curr];
                    if low[curr] >= tin[parent] {
                        let comp1 = sz[curr];
                        let comp2 = (n - 1).saturating_sub(comp1);
                        if comp1 >= 1 && comp2 >= 1 {
                            let u_orig = nodes[u];
                            let v_orig = nodes[parent];
                            if check_2cut_split(raw_g, u_orig, v_orig) {
                                let min_comp = comp1.min(comp2);
                                let is_better = best
                                    .map_or(true, |(_, _, prev_min)| min_comp > prev_min);
                                if is_better {
                                    best = Some((
                                        u_orig.min(v_orig),
                                        u_orig.max(v_orig),
                                        min_comp,
                                    ));
                                    // Early exit if we found a profitable corridor
                                    if min_comp >= MIN_PROFITABLE_CORRIDOR {
                                        return best.map(|(u, v, _)| (u, v));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    best.map(|(u, v, _)| (u, v))
}
```

- [ ] **Step 5: Add `Interrupted` import and update `solve_2cut_corridor` import list**

At the top of `corridor.rs`, ensure `SolverResult` variants and `ControlSignal` are accessible:

```rust
use rustsat::solvers::{ControlSignal, Solve, SolverResult};
```

- [ ] **Step 6: Verify compilation**

Run: `cargo check -p cegar-fix`
Expected: 0 errors

- [ ] **Step 7: Run existing test suite**

Run: `cargo test -p cegar-fix`
Expected: All existing tests pass

- [ ] **Step 8: Commit**

```bash
git add src/cegar-fix/src/macro_decomp/corridor.rs
git commit -m "fix(corridor): remove benchmark filters, fix toughness contract, add solver terminator"
```

---

### Task 4: Fix `fallback_cegar.rs` — Add Solver Terminator

**Files:**
- Modify: `src/cegar-fix/src/fallback/fallback_cegar.rs:59,116-123`

**Interfaces:**
- Consumes: `crate::core::solver_utils::create_solver_with_deadline`
- Produces: `pub fn solve_with_contraction(g: &Graph, timeout_secs: f64) -> Result<Vec<i32>, String>` (signature unchanged)

- [ ] **Step 1: Replace `CaDiCaL::default()` with helper (L59)**

```rust
// Replace:
// let mut solver = CaDiCaL::default();
// With:
let deadline = t_start + std::time::Duration::from_secs_f64(timeout_secs);
let mut solver = crate::core::solver_utils::create_solver_with_deadline(deadline);
```

- [ ] **Step 2: Handle `Interrupted` in CEGAR loop (L116–123)**

```rust
let res = match solver.solve() {
    Ok(r) => r,
    Err(e) => return Err(format!("CaDiCaL error: {:?}", e)),
};

match res {
    SolverResult::Unsat => return Err("UNSAT".to_string()),
    SolverResult::Interrupted => return Err("Timeout".to_string()),
    SolverResult::Sat => {
        // ... existing model extraction ...
    }
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p cegar-fix`
Expected: 0 errors

- [ ] **Step 4: Commit**

```bash
git add src/cegar-fix/src/fallback/fallback_cegar.rs
git commit -m "fix(fallback): add preemptive solver termination via deadline helper"
```

---

### Task 5: Fix `solver_pipeline.rs` — Budget Allocation

**Files:**
- Modify: `src/cegar-fix/src/pipeline/solver_pipeline.rs:136-172`

**Interfaces:**
- Consumes: All `macro_decomp` and `fallback` modules (signatures unchanged)
- Produces: `pub fn solve_single_graph(...)` (signature unchanged)

- [ ] **Step 1: Add budget allocation logic**

Replace lines 136–172 with:

```rust
    // 2.5 Topological Macro-Decomposition Cascade
    //
    // PERFORMANCE KNOB: Maximum fraction of total timeout allocated to
    // macro-decomposition fast-paths. The fallback solver always receives
    // at least timeout * (1 - MACRO_BUDGET_RATIO).
    const MACRO_BUDGET_RATIO: f64 = 0.4;

    let macro_budget_secs = timeout_secs * MACRO_BUDGET_RATIO;
    let mut macro_tour_opt: Option<Vec<i32>> = None;

    // 2.5a. 2-Cut Articulation Separator (Corridor family)
    // Sub-budget: 40% of macro budget
    {
        let corridor_timeout = macro_budget_secs * 0.4;
        if let Some((u, v)) = macro_corridor::can_solve_2cut(&g) {
            println!(
                "[Pipeline] Detected 2-cut articulation separator ({}, {}): invoking corridor decomposition...",
                u, v
            );
            macro_tour_opt = macro_corridor::solve_2cut_corridor(&g, corridor_timeout);
        }
    }

    // 2.5b. Dense Bipartite Macro-Decomposition Cascade
    // Sub-budget: 30% of macro budget
    if macro_tour_opt.is_none() {
        let bipartite_timeout = macro_budget_secs * 0.3;
        if dynamic_bipartite::can_solve_bipartite(&g) {
            println!("[Pipeline] Detected dense bipartite hub signature: invoking dynamic bipartite macro-decomposition...");
            macro_tour_opt = dynamic_bipartite::solve_bipartite(&g, bipartite_timeout);
        }
    }

    // 2.5c. Degree-2 Alternating Pair Contraction
    // Sub-budget: remaining macro budget
    if macro_tour_opt.is_none() {
        let elapsed = start_time.elapsed().as_secs_f64();
        let portfolio_timeout = (macro_budget_secs - (elapsed - (timeout_secs - macro_budget_secs).max(0.0))).max(0.5);
        if macro_788::can_solve_alternating_pairs(&g) {
            println!("[Pipeline] Detected 2-colorable alternating pair structure: invoking alternating portfolio solver...");
            macro_tour_opt = macro_788::solve_alternating_pairs(&g, portfolio_timeout);
        }
    }

    if let Some(tour) = macro_tour_opt {
        return verify_and_export(&g, &tour, start_time, output_tour_path);
    }

    // 3. Fallback: guaranteed remaining budget
    let elapsed = start_time.elapsed().as_secs_f64();
    let remaining_timeout = timeout_secs - elapsed;
    if remaining_timeout <= 0.0 {
        return Err(SolverPipelineError {
            message: format!("TIMEOUT (elapsed: {:.2}s)", elapsed),
            vertex_count,
        });
    }
```

- [ ] **Step 2: Remove the `vertex_count >= 1000` guard**

The old code had `if vertex_count >= 1000 {` around the corridor call. This is removed — `find_2cut_ports` now handles its own minimum size check internally (`n < 4`).

- [ ] **Step 3: Verify compilation**

Run: `cargo check -p cegar-fix`
Expected: 0 errors

- [ ] **Step 4: Commit**

```bash
git add src/cegar-fix/src/pipeline/solver_pipeline.rs
git commit -m "fix(pipeline): add budget allocation to prevent macro_decomp from starving fallback solver"
```

---

### Task 6: Document Constants in `dynamic_bipartite.rs` & Add Terminator

**Files:**
- Modify: `src/cegar-fix/src/macro_decomp/dynamic_bipartite.rs`

**Interfaces:**
- Consumes: `crate::core::solver_utils::create_solver_with_deadline`
- Produces: All public functions (signatures unchanged)

- [ ] **Step 1: Add documented constant declarations at module top**

After the imports, add:

```rust
// === PERFORMANCE KNOBS ===
// These thresholds control the sensitivity of the dense bipartite hub
// recognizer. They do NOT affect correctness — changing them only
// changes which graphs are attempted by this decomposition vs.
// falling through to the monolithic fallback.

/// Minimum maximum-degree for a graph to have "super-hub" structure.
const MIN_MAX_DEGREE: usize = 30;

/// Super-hubs must have degree >= this fraction of the maximum degree.
const HUB_DEGREE_FRACTION: f64 = 0.7;

/// Maximum number of super-hubs as a fraction of N.
const MAX_HUB_FRACTION: f64 = 0.02;

/// Minimum graph size for hub-spoke decomposition to be meaningful.
const MIN_GRAPH_SIZE: usize = 50;
```

Replace the inline magic numbers in `detect_and_partition` with these constants:
- `n < 50` → `n < MIN_GRAPH_SIZE`
- `max_deg < 30 || max_deg < n / 50` → `max_deg < MIN_MAX_DEGREE || max_deg < n / (1.0 / MAX_HUB_FRACTION) as usize`
- `(max_deg * 7) / 10` → `((max_deg as f64) * HUB_DEGREE_FRACTION) as usize`
- `k > n / 50` → `k > ((n as f64) * MAX_HUB_FRACTION) as usize`

- [ ] **Step 2: Add documentation comments to existing soundness guards**

Find the two guards added in commit `9d09397` and add documentation:

```rust
// MATHEMATICAL REQUIREMENT: A Hamiltonian cycle must enter and exit each
// hub cluster, requiring >= 2 boundary ports per hub.
if ports.len() < 2 {
    return None;
}

// MATHEMATICAL REQUIREMENT: Every vertex must belong to exactly one
// partition element. Missing vertices would create an incomplete tour.
if total_partitioned != n {
    return None;
}
```

- [ ] **Step 3: Replace `CaDiCaL::default()` with solver helper**

At each `CaDiCaL::default()` call in this file (~L439, ~L683), replace with:
```rust
let mut solver = crate::core::solver_utils::create_solver_with_deadline(deadline);
```

Handle `SolverResult::Interrupted` in each CEGAR loop.

- [ ] **Step 4: Verify compilation**

Run: `cargo check -p cegar-fix`
Expected: 0 errors

- [ ] **Step 5: Commit**

```bash
git add src/cegar-fix/src/macro_decomp/dynamic_bipartite.rs
git commit -m "refactor(dynamic_bipartite): document constants as performance knobs, add solver terminator"
```

---

### Task 7: Integration Tests — Verify All Fixes

**Files:**
- Modify: scratch test runner (or create `src/cegar-fix/tests/macro_decomp_audit.rs`)

**Interfaces:**
- Consumes: All fixed modules
- Produces: Passing test suite proving all audit findings are resolved

- [ ] **Step 1: Update Test A expectation (min_comp_sz fix)**

The N=231 graph with 29/200 components: `find_2cut_ports` must now return `Some`.

```rust
// Was: assert!(res_ports.is_none());
// Now:
assert!(res_ports.is_some(), "find_2cut_ports must detect asymmetric 2-cut after min_comp_sz fix");
```

- [ ] **Step 2: Update Test B expectation (degree filter fix)**

The N=100 graph with deg 12+12: `find_2cut_ports` must now return `Some`.

```rust
// Was: assert!(res_ports.is_none());
// Now:
assert!(res_ports.is_some(), "find_2cut_ports must detect 2-cut with high-degree ports after filter removal");
```

- [ ] **Step 3: Update Test C expectation (contract mismatch fix)**

The N=107 graph with 3 components: `find_2cut_ports` must now return `None` (toughness violation caught in `check_2cut_split`).

```rust
// Was: assert that find_2cut_ports returns Some
// Now:
assert!(res_ports.is_none(), "find_2cut_ports must reject 3-component separator (toughness violation)");
```

- [ ] **Step 4: Update Test E expectation (bipartite fix)**

The N=33 graph with odd-cycle pairs: `can_solve_alternating_pairs` must now return `false`.

```rust
assert!(!can_solve, "can_solve_alternating_pairs must reject non-bipartite pair graph");
```

- [ ] **Step 5: Add Test F — Timeout preemption**

```rust
fn test_f_timeout_preemption() {
    // Build a hard graph for corridor solving
    let g710 = cegar_fix::core::file_operations::parse_graph_from_file("FHCPCS-col/graph710.col").unwrap();
    let start = Instant::now();
    let _result = macro_corridor::solve_2cut_corridor(&g710, 2.0);
    let elapsed = start.elapsed().as_secs_f64();
    assert!(elapsed < 4.0, "solve_2cut_corridor must respect 2s deadline (took {:.1}s)", elapsed);
}
```

- [ ] **Step 6: Add Test H — Portfolio positive regression**

```rust
fn test_h_portfolio_positive() {
    let g788 = cegar_fix::core::file_operations::parse_graph_from_file("FHCPCS-col/graph788.col").unwrap();
    let can_solve = portfolio_788::can_solve_alternating_pairs(&g788);
    assert!(can_solve, "can_solve_alternating_pairs must still accept graph788 (bipartite pair structure)");
}
```

- [ ] **Step 7: Build and run all tests**

Run: `cargo run --manifest-path <scratch>/Cargo.toml` (from /root/HCP)
Expected: All tests pass with updated expectations

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "test: update audit counterexample tests for all macro_decomp fixes"
```

---

### Task 8: FHCPCS Benchmark Regression Sweep

**Files:**
- No code changes. Execution-only task.

**Interfaces:**
- Consumes: Full pipeline via `cargo run -p cegar-fix`

- [ ] **Step 1: Run benchmark baseline (before changes) if not already captured**

If a baseline checkpoint exists from before changes, use it. Otherwise, note that we cannot compare.

- [ ] **Step 2: Run full FHCPCS sweep with the fixed code**

```bash
cargo run --release -p cegar-fix -- batch --start 1 --end 1001 --timeout 30 --workers 4 --checkpoint scratch/post_fix_results.json
```

- [ ] **Step 3: Compare results**

Verify:
- No graph transitions from SAT_VERIFIED to TIMEOUT or ERROR.
- Total solve count ≥ baseline solve count.
- Note any newly solved graphs (expected: some graphs with small corridors may now be solved faster).

- [ ] **Step 4: Commit results**

```bash
git add scratch/post_fix_results.json
git commit -m "benchmark: FHCPCS regression sweep after macro_decomp generalization"
```
