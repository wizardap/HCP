# Design Spec: `macro_decomp` Generalization & Bug Fixes

**Date:** 2026-09-24  
**Status:** Draft  
**Scope:** `src/cegar-fix/src/macro_decomp/`, `src/cegar-fix/src/fallback/`, `src/cegar-fix/src/pipeline/`, `src/cegar-fix/src/core/`

## 1. Background & Motivation

An independent adversarial audit of `macro_decomp` identified 7 proven issues across
`corridor.rs`, `portfolio_788.rs`, `dynamic_bipartite.rs`, `fallback_cegar.rs`, and
`solver_pipeline.rs`. The issues fall into 4 categories:

| Category | Issues | Severity |
|----------|--------|----------|
| **Soundness** | `portfolio_788` BFS accepts non-bipartite graphs (false positive) | Medium (TourVerifier catches at output) |
| **Completeness** | `min_comp_sz`, `deg > 10` filter, contract mismatch `≥2` vs `==2` | High (misses valid decompositions) |
| **Timeout** | CaDiCaL blocking FFI without terminator in `corridor.rs`, `fallback_cegar.rs` | Critical (can hang pipeline) |
| **Starvation** | `macro_decomp` consumes entire timeout budget, fallback never runs | Critical (loses practical completeness) |

All issues were proven with live execution on constructed counterexamples and verified
against git history (commits `497c38a`, `9d09397`, `3be0a8d`).

### Design Principle

Every remaining threshold must belong to exactly one of two categories:

- **Mathematically required** — proven necessary for correctness (e.g., `comps == 2` by
  Chvátal-Erdős toughness theorem). Documented with the theorem reference.
- **Performance knob** — affects only which graphs are attempted by a decomposition,
  never correctness. Documented with rationale and tuning guidance.

No unexplained magic numbers may remain.

### Constraints

- Keep current pipeline architecture (`macro_decomp → fallback`).
- Accept minor performance regression on FHCPCS benchmark (≤ 10% total time increase).
- Must not timeout any graph that currently solves successfully.
- Do not modify any code outside the listed modules.

## 2. Timeout & Budget Architecture

### 2.1. Problem

CaDiCaL's `solver.solve()` is a synchronous blocking FFI call into C++. Without
`attach_terminator`, the Rust deadline check (`Instant::now() < deadline`) only runs
*between* CEGAR iterations, not *during* a single solve call. A single hard SAT instance
can block for minutes or hours.

Additionally, `solver_pipeline.rs` passes the full `timeout_secs` to `macro_decomp`.
If `macro_decomp` hangs in a blocking `solve()`, the fallback solver is completely
starved of its execution budget.

### 2.2. Solution

#### 2.2.1. Solver Helper with Preemptive Deadline

Create `src/cegar-fix/src/core/solver_utils.rs`:

```rust
use rustsat::solvers::{ControlSignal, Terminate};
use rustsat_cadical::CaDiCaL;
use std::time::Instant;

/// Creates a CaDiCaL solver instance with a terminator callback that
/// interrupts the solver when the deadline is reached. This ensures
/// solver.solve() returns SolverResult::Interrupted promptly after
/// the deadline, rather than blocking until the next CEGAR iteration.
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

All `CaDiCaL::default()` calls in the following locations must be replaced:

| File | Line | Current | Replacement |
|------|------|---------|-------------|
| `corridor.rs` `solve_block_a` | ~L58 | `CaDiCaL::default()` | `create_solver_with_deadline(deadline)` |
| `corridor.rs` `solve_block_b` | ~L281 | `CaDiCaL::default()` | `create_solver_with_deadline(deadline)` |
| `fallback_cegar.rs` | L59 | `CaDiCaL::default()` | `create_solver_with_deadline(deadline)` |
| `portfolio_788.rs` | L424 | `CaDiCaL::default()` | Already has terminator — refactor to use shared helper |

#### 2.2.2. Handling `SolverResult::Interrupted`

Every CEGAR loop must treat `Interrupted` as a timeout:

```rust
match solver.solve() {
    Ok(SolverResult::Sat) => { /* process model */ }
    Ok(SolverResult::Unsat) => return Err("UNSAT".to_string()),
    Ok(SolverResult::Interrupted) | Err(_) => return Err("Timeout".to_string()),
}
```

#### 2.2.3. Pipeline Budget Allocation

In `solver_pipeline.rs`, replace the current "pass full timeout" approach:

```rust
const MACRO_BUDGET_RATIO: f64 = 0.4;

let macro_deadline = start_time + Duration::from_secs_f64(timeout_secs * MACRO_BUDGET_RATIO);
let fallback_deadline = start_time + Duration::from_secs_f64(timeout_secs);

// Sub-budgets within macro_decomp:
let corridor_deadline = start_time + Duration::from_secs_f64(timeout_secs * MACRO_BUDGET_RATIO * 0.4);
let bipartite_deadline = start_time + Duration::from_secs_f64(timeout_secs * MACRO_BUDGET_RATIO * 0.7);
let portfolio_deadline = macro_deadline; // gets remaining macro budget
```

After all macro_decomp attempts, compute remaining time for fallback:

```rust
let elapsed = start_time.elapsed().as_secs_f64();
let remaining = timeout_secs - elapsed;
if remaining <= 0.0 {
    return Err(SolverPipelineError { message: "TIMEOUT".into(), vertex_count });
}
// remaining is guaranteed >= timeout_secs * (1 - MACRO_BUDGET_RATIO) minus overhead
fallback_cegar::solve_with_contraction(&g, remaining)
```

`MACRO_BUDGET_RATIO` is a **performance knob** — documented as:

> Controls the maximum fraction of total timeout allocated to macro-decomposition
> fast-paths. The fallback solver always receives at least `timeout * (1 - ratio)`.
> Increase if macro-decomp routinely solves graphs that the fallback cannot.
> Decrease if macro-decomp routinely wastes time on graphs it cannot solve.

### 2.3. Files Changed

- `src/cegar-fix/src/core/solver_utils.rs` — **NEW**
- `src/cegar-fix/src/core/mod.rs` — add `pub mod solver_utils`
- `src/cegar-fix/src/macro_decomp/corridor.rs` — use helper
- `src/cegar-fix/src/fallback/fallback_cegar.rs` — use helper
- `src/cegar-fix/src/macro_decomp/portfolio_788.rs` — refactor to use helper
- `src/cegar-fix/src/macro_decomp/dynamic_bipartite.rs` — use helper
- `src/cegar-fix/src/pipeline/solver_pipeline.rs` — budget allocation

## 3. `corridor.rs` Generalization

### 3.1. Remove `min_comp_sz` Filter

**Current (wrong):**
```rust
let min_comp_sz = (n / 10).max(30);
if comp1 >= min_comp_sz && comp2 >= min_comp_sz { ... }
```

**Replacement:**

Remove `min_comp_sz` as a hard filter. The mathematical requirement is only that both
components have ≥ 1 vertex. Introduce a **best-candidate selection** strategy:

```rust
/// PERFORMANCE KNOB: Minimum corridor size for decomposition to likely
/// outperform monolithic solving. Does NOT filter candidates — only
/// affects selection priority. The algorithm still returns smaller
/// corridors if no better candidate exists.
const MIN_PROFITABLE_CORRIDOR: usize = 10;
```

Instead of returning the first valid 2-cut found, collect all valid 2-cuts and select
the one with the **largest minimum component** (most balanced split):

```rust
fn find_2cut_ports(raw_g: &Graph) -> Option<(i32, i32)> {
    let mut best: Option<(i32, i32, usize)> = None; // (u, v, min_comp_size)

    for u in sorted_candidates {
        // Tarjan on G \ {u} ...
        for each articulation point parent {
            let (small, large) = exact_component_sizes(raw_g, u, parent);
            if small >= 1 {
                let dominated = best.map_or(false, |(_, _, prev)| small <= prev);
                if !dominated {
                    best = Some((u_orig, v_orig, small));
                }
            }
        }
    }

    best.map(|(u, v, _)| (u.min(v), u.max(v)))
}
```

### 3.2. Remove Degree Upper Bound

**Current (wrong):**
```rust
if deg_u < 3 || deg_u > 10 { continue; }
```

**Replacement:**
```rust
/// MATHEMATICAL REQUIREMENT: In a 2-connected graph, a vertex with
/// degree < 3 cannot be part of a 2-vertex separator that creates
/// two non-trivial components.
if deg_u < 3 { continue; }
```

To preserve performance, sort candidates by degree ascending before iterating. Low-degree
vertices are checked first (fast and likely to yield 2-cuts). High-degree vertices are
still checked but only if no good candidate was found earlier. Combined with the
best-candidate selection from 3.1, the algorithm stops early once a sufficiently balanced
2-cut is found.

### 3.3. Fix Contract Mismatch

**Current (wrong):**

`check_2cut_split` at L504:
```rust
comp_sizes.len() >= 2 && comp_sizes.iter().filter(|&&sz| sz >= min_comp_size).count() >= 2
```

**Replacement:**
```rust
/// MATHEMATICAL REQUIREMENT: By the Chvátal-Erdős toughness theorem,
/// a Hamiltonian graph satisfies c(G \ S) <= |S|. For |S| = 2,
/// exactly 2 components is the maximum allowed. If c(G \ {u,v}) >= 3,
/// the graph is provably non-Hamiltonian.
if comp_sizes.len() != 2 {
    return false;
}
comp_sizes.iter().all(|&sz| sz >= 1)
```

### 3.4. Remove `n < 100` Guard

**Current:** `if n < 100 { return None; }`

**Replacement:** Remove entirely. No mathematical basis. Tarjan runs in O(V+E) per
candidate, which is negligible for small graphs.

### 3.5. Files Changed

- `src/cegar-fix/src/macro_decomp/corridor.rs` — all changes in this section

## 4. `portfolio_788.rs` Bipartite Fix & Cleanup

### 4.1. Fix BFS Bipartite Checker

**Current (wrong):** BFS colors vertices but never checks for conflicts when visiting
an already-colored neighbor. Silently accepts non-bipartite graphs.

**Replacement:** Standard BFS bipartite detection — when a cross-edge connects two
vertices of the same color, the graph contains an odd cycle and is non-bipartite:

```rust
if let Some(nbrs) = g.adjacency_list.get(&curr) {
    let vp = v_partner.get(&curr).copied().unwrap_or(-1);
    for &nxt in nbrs {
        if nxt != vp {
            if let Some(&existing_color) = color.get(&nxt) {
                if existing_color == curr_c {
                    // Odd cycle: not bipartite
                    return false; // or return None in solve_*
                }
            } else {
                color.insert(nxt, 1 - curr_c);
                q.push(nxt);
            }
        }
    }
}
```

Same fix for the `v_partner` propagation branch:
```rust
if let Some(&vp) = v_partner.get(&curr) {
    if let Some(&existing_color) = color.get(&vp) {
        if existing_color == curr_c {
            return false;
        }
    } else {
        color.insert(vp, 1 - curr_c);
        q.push(vp);
    }
}
```

### 4.2. Generalize Density Guard

**Current:** `deg2_count < 10 || deg2_count * 3 < n`

**Replacement:**
```rust
/// PERFORMANCE KNOB: Minimum fraction of degree-2 vertices for
/// alternating pair decomposition to be profitable. Below this
/// threshold, contraction yields too few pairs for the directed
/// CEGAR to outperform monolithic solving.
const MIN_DEG2_FRACTION: f64 = 0.15;

let deg2_fraction = deg2_count as f64 / n as f64;
if deg2_fraction < MIN_DEG2_FRACTION {
    return false;
}
```

### 4.3. DRY: Extract Shared Logic

The contraction + pair extraction + BFS coloring logic is duplicated between
`can_solve_alternating_pairs` (L180–231) and `solve_alternating_pairs` (L235–282).

Extract into a private helper:

```rust
struct AlternatingPairGraph {
    contracted_g: Graph,
    contractor: Degree2Contractor,
    pairs: Vec<(i32, i32)>,
    v_partner: HashMap<i32, i32>,
    color: HashMap<i32, u8>,
}

/// Contracts degree-2 chains, extracts alternating pairs, and verifies
/// bipartite structure. Returns None if the graph doesn't qualify.
fn extract_alternating_pairs(raw_g: &Graph) -> Option<AlternatingPairGraph> {
    // 1. Density check
    // 2. Degree2Contractor::contract
    // 3. Pair extraction from chain_map
    // 4. BFS bipartite coloring with conflict detection
    // Returns None on: too few pairs, non-bipartite, disconnected pairs
}
```

Then:
- `can_solve_alternating_pairs(g)` → `extract_alternating_pairs(g).is_some()`
- `solve_alternating_pairs(g, timeout)` → calls `extract_alternating_pairs(g)`, then
  builds directed pair graph, SAT encoding, and CEGAR loop.

This eliminates the risk of fixing the BFS in one copy but not the other.

### 4.4. Files Changed

- `src/cegar-fix/src/macro_decomp/portfolio_788.rs` — all changes in this section

## 5. `dynamic_bipartite.rs` Cleanup

### 5.1. Reclassify Constants

No constants are removed. All are reclassified and documented:

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

/// Low-degree threshold for cluster membership.
const LOW_DEGREE_THRESHOLD: usize = 4;
```

### 5.2. Preserve Existing Soundness Guards

The two soundness guards added in commit `9d09397` are mathematically required and
remain unchanged, but receive documentation:

```rust
// MATHEMATICAL REQUIREMENT: A Hamiltonian cycle must enter and exit each
// hub cluster, requiring >= 2 boundary ports per hub.
if ports.len() < 2 { return None; }

// MATHEMATICAL REQUIREMENT: Every vertex must belong to exactly one
// partition element. Missing vertices would create an incomplete tour.
if total_partitioned != n { return None; }
```

### 5.3. Timeout Integration

Replace `CaDiCaL::default()` with `create_solver_with_deadline(deadline)` at:
- `MacroSat::solver` initialization (~L439)
- Cluster CEGAR solver (~L683)

### 5.4. Files Changed

- `src/cegar-fix/src/macro_decomp/dynamic_bipartite.rs` — all changes in this section

## 6. Testing Strategy

### 6.1. Regression Test Suite

Run full FHCPCS benchmark (1001 graphs) before and after changes:
- **Solved count**: must be ≥ current count.
- **Total time**: ≤ 110% of current total.
- **No regressions**: no graph may transition from SAT_VERIFIED to TIMEOUT.

### 6.2. Correctness Tests (Counterexample Suite)

Adapt the existing scratch test runner (`scratch/src/main.rs`) into proper integration
tests. Expected behavior changes after fixes:

| Test | Graph | Current Result | Expected After Fix |
|------|-------|----------------|-------------------|
| A | N=231, comp 29/200 | `find_2cut_ports → None` | `find_2cut_ports → Some` |
| B | N=100, deg 12+12 | `find_2cut_ports → None` | `find_2cut_ports → Some` |
| C | N=107, 3 components | `find_2cut_ports → Some` | `find_2cut_ports → None` |
| E | N=33, odd cycle pairs | `can_solve_alternating_pairs → true` | `can_solve_alternating_pairs → false` |

### 6.3. New Tests

| Test | Purpose | Assertion |
|------|---------|-----------|
| F | Timeout preemption | `solve_block_b` on hard graph with 2s deadline returns within ≤ 3s |
| G | Budget starvation | Pipeline on graph971 with 10s timeout: fallback solver runs |
| H | Bipartite positive | `can_solve_alternating_pairs` returns `true` on graph788.col |
| I | Corridor positive | `solve_2cut_corridor` still solves graph710.col |

### 6.4. Files Changed

- `scratch/src/main.rs` — update test expectations
- New integration test file or `#[cfg(test)]` modules in changed files

## 7. Change Summary

| File | Type | Changes |
|------|------|---------|
| `core/solver_utils.rs` | NEW | `create_solver_with_deadline` helper |
| `core/mod.rs` | EDIT | Add `pub mod solver_utils` |
| `corridor.rs` | EDIT | Remove `min_comp_sz`, `deg > 10`, `n < 100`; fix contract; best-candidate selection; use solver helper |
| `portfolio_788.rs` | EDIT | Fix BFS bipartite check; DRY extraction; generalize density guard; use solver helper |
| `dynamic_bipartite.rs` | EDIT | Document constants; use solver helper |
| `fallback_cegar.rs` | EDIT | Use solver helper |
| `solver_pipeline.rs` | EDIT | Budget allocation with `MACRO_BUDGET_RATIO` |
| `scratch/src/main.rs` | EDIT | Update test expectations, add tests F/G/H/I |

Total: 1 new file, 7 edited files.
