# Unified SAT-CEGAR Engine & Zero-Budget Pipeline Specification

- **Date:** 2026-09-25
- **Branch:** `feat/principled-spqr-decomposition`
- **Scope:** Architectural refactoring of SAT-CEGAR solvers and elimination of timeout allocation heuristics.

---

## 1. Problem Statement & Motivation

The current HCP solver architecture suffers from two critical flaws:

1. **Duplicate, Competing CEGAR Engines:**
   - `block_solver` (Task 5): Provides deterministic sorted clause generation, preemptive CaDiCaL deadline termination, and Hamiltonian path support, but lacks 2-opt cycle merging and contraction.
   - `fallback_cegar`: Contains degree-2 contraction and 2-opt cycle merging, but features non-deterministic edge iteration, lacks Hamiltonian path support, and only checks time iteratively without preemptive CaDiCaL interruption.
   - `Degree2Contractor` vs. `spqr_series`: Two independent implementations of degree-2 chain contraction exist concurrently.

2. **Timeout Allocation Anti-Pattern (Magic Numbers):**
   - Pipeline contains arbitrary percentage splits and vertex bounds:
     - `g.len() > 100` and `g.len() > 250` for macro budget calculation
     - `timeout_secs * 0.6` (60% macro budget)
     - `(timeout_secs - elapsed - 2.0).max(1.0)` ("2 seconds buffer")
     - `cur_g.len() <= 50` check for exploratory solver trial
     - `(remaining_timeout * 0.2).min(2.0)` (20% or 2s exploratory budget)
   - These heuristic allocations starve solvers, complicate reasoning about termination, and introduce non-deterministic race conditions across graph classes.

---

## 2. Core Architectural Principles

1. **Single Monotonic Deadline:**
   - Establish `deadline = Instant::now() + Duration::from_secs_f64(timeout_secs)` at pipeline entry.
   - Every solver stage queries remaining time as `rem = (deadline - Instant::now()).as_secs_f64()`. If `rem <= 0.0`, it aborts immediately with `TIMEOUT`.
   - **Zero budget partitioning:** No percentage splits (`* 0.6`, `* 0.2`), no hardcoded second reservations (`- 2.0`).

2. **Topological Dispatch with All-Remaining Time:**
   - Dispatch filters (`can_solve_*`) run in $O(V+E)$ time (sub-millisecond).
   - If a graph satisfies a specific topological structure (e.g. bipartite hub, 2-cut corridor, alternating pairs), that solver runs with the full remaining deadline. If it succeeds, the tour is verified and returned. If it fails or does not match, execution proceeds immediately to general decomposition.

3. **Unified CEGAR Engine (`cegar_engine`):**
   - Single canonical SAT-CEGAR implementation combining:
     - **Deterministic Encoding:** Fully sorted nodes, edges, and neighbor lists (invariance under SipHash iteration order).
     - **Preemptive Termination:** `create_solver_with_deadline(deadline)` attaching CaDiCaL's internal terminator.
     - **Safe 2-Opt Cycle Merger:** Instant heuristic merging of 2 to 4 subcycles without violating cut constraints or virtual edges.
     - **DFJ Subcycle Cuts & Cycle Blocking:** Sound combinatorial cuts on all remaining subcycles.
     - **Cycle & Path Duality:** Uniform interface supporting both Hamiltonian Cycle (`solve_cycle`) and Hamiltonian Path between specified ports (`solve_path` via dummy vertex reduction).

4. **Single Source of Truth for Series Contraction:**
   - Standardize on `decomp::spqr_series::{contract_series_chains, expand_series_tour}` for degree-2 contraction across the entire codebase.

---

## 3. Detailed Component Architecture

### 3.1. Unified Engine: `crate::solver::cegar_engine` (replacing `block_solver`)

Location: `src/cegar-fix/src/solver/cegar_engine.rs` (with re-export in `crate::solver`)

#### Public Interface:
```rust
/// Solves Hamiltonian Cycle on `g` using deterministic SAT-CEGAR with 2-opt merging and DFJ cuts.
pub fn solve_cycle(g: &Graph, timeout_secs: f64) -> Result<Vec<i32>, String>;

/// Solves Hamiltonian Path between `port_u` and `port_v` on `g`.
/// Reduces the path problem to Hamiltonian cycle by adding a dummy vertex `w` connected strictly to `u` and `v`.
pub fn solve_path(g: &Graph, port_u: i32, port_v: i32, timeout_secs: f64) -> Result<Vec<i32>, String>;
```

#### Algorithm Flow (`solve_cycle`):
1. **Validation & Edge Extraction:**
   - Collect unique nodes, sort deterministically: `nodes.sort_unstable()`.
   - Collect unique undirected edges `(min(u,v), max(u,v))` and sort deterministically.
   - For each node, sort its neighbor list.
2. **SAT Formulation:**
   - Map each undirected edge to a `rustsat::types::Lit`.
   - Instantiate CaDiCaL solver with preemptive deadline: `create_solver_with_deadline(deadline)`.
   - Add exact degree-2 constraints:
     - If degree $< 2$: return `UNSAT`.
     - If degree $== 2$: add two unit clauses.
     - If degree $3 \le d \le 8$: add at-least-2 clauses + pairwise at-most-2 clauses.
     - If degree $> 8$: add sequential counter / ladder encoding (`add_at_most_2`).
3. **CEGAR Loop:**
   - Check `Instant::now() >= deadline` -> return `TIMEOUT`.
   - Run `solver.solve()`. If `Unsat` -> return `UNSAT`; if `Interrupted` -> return `TIMEOUT`.
   - Extract active edges where `sol.lit_value(lit) == TernaryVal::True`.
   - Traverse active edges to extract disjoint simple Eulerian cycles.
   - **Base Case:** Exactly 1 cycle of length $N$ -> Return `Ok(cycle)`.
   - **Heuristic Acceleration (2-opt merge):**
     - If $2 \le \text{cycles.len()} \le 4$:
       - Run `safe_2opt_merge(&cycles, &adj_set, &forbidden_edges)`.
       - If merged cycle has length $N$ and validates against graph -> Return `Ok(merged)`.
   - **Combinatorial Cuts:**
     - For each subcycle $C$ with $|C| < N$:
       - Add DFJ cut clause: $\bigvee_{e \in \delta(C)} e$.
       - Add cycle blocking clause: $\bigvee_{e \in C} \neg e$.
   - Repeat loop until resolution or deadline expiry.

---

### 3.2. Solver Pipeline Dispatch Flow (`solver_pipeline.rs`)

Location: `src/cegar-fix/src/pipeline/solver_pipeline.rs`

```
Entry: solve_single_graph(graph_path, timeout_secs, output_path)
  │
  ├─ Compute deadline = Instant::now() + Duration::from_secs_f64(timeout_secs)
  │
  ├─ Step 1: Fast Soundness Invariants (O(V+E))
  │    └─ check_fast_invariants(&g) -> Fail fast on min degree, 1-cut, parity
  │
  ├─ Step 1.5: Topological Macro Cascade (Zero Budget Splits)
  │    ├─ rem = (deadline - Instant::now()).as_secs_f64()
  │    ├─ if dynamic_bipartite::can_solve_bipartite(&g) -> solve_bipartite(&g, rem)
  │    ├─ else if macro_corridor::can_solve_2cut(&g)   -> solve_2cut_corridor(&g, rem)
  │    ├─ else if macro_788::can_solve_alternating(&g) -> solve_alternating_pairs(&g, rem)
  │    └─ If tour found -> verify_and_export -> DONE
  │
  ├─ Step 2: Series Contraction (decomp::spqr_series)
  │    └─ Contract maximal degree-2 chains into virtual edges
  │
  ├─ Step 3: Parallel 2-Cut Decomposition Loop (decomp::spqr_parallel)
  │    ├─ While 2-cut separation pairs exist:
  │    │    ├─ Isolate smaller component G_sub
  │    │    ├─ solve_path(&sub_g, u, v, remaining_deadline)
  │    │    └─ Replace G_sub with virtual edge (u, v) in skeleton
  │
  ├─ Step 4: Skeleton Solve (Single Unified Call)
  │    ├─ rem = (deadline - Instant::now()).as_secs_f64()
  │    ├─ if rem <= 0.0 -> return TIMEOUT
  │    └─ cegar_engine::solve_cycle(&skeleton_g, rem)
  │
  ├─ Step 5: Subpath & Series Tour Expansion
  │    ├─ Unroll 2-cut subpaths via stitch_subpath
  │    └─ Unroll series chains via expand_series_tour
  │
  └─ Step 6: Tour Verification & TSPLIB Export
       └─ TourVerifier::verify -> Strict Soundness Invariant Check
```

---

## 4. Invariants & Soundness Guarantees

1. **Strict Soundness Invariant:**
   - Every tour returned by `cegar_engine` or `solver_pipeline` MUST be validated by `TourVerifier::verify`. Any failure returns an immediate `Err("Tour verification failed")`. Zero false SAT.
2. **Deterministic Reproducibility:**
   - All internal collections of vertices and edges within `cegar_engine` are sorted before literal assignment and iteration. Two runs on isomorphic inputs with the same vertex numbering must produce identical SAT formulas and cut sequences.
3. **Preemptive Resource Cleanup:**
   - CaDiCaL terminator interrupts execution immediately upon reaching `deadline`, preventing thread hang or runaways during batch benchmarks.

---

## 5. Migration & Deprecation Strategy

1. **Step 1:** Create `src/cegar-fix/src/solver/cegar_engine.rs` implementing `solve_cycle` and `solve_path` with deterministic encoding, preemptive deadline, and 2-opt merge.
2. **Step 2:** Expose `cegar_engine` in `src/cegar-fix/src/solver/mod.rs` and update existing tests targeting `block_solver` to test `cegar_engine`.
3. **Step 3:** Refactor `solver_pipeline.rs`:
   - Remove `g.len() > 100`, `g.len() > 250`, `0.6`, `- 2.0` from Step 1.5.
   - Remove `cur_g.len() <= 50`, `0.2`, `min(2.0)`, and `fallback_cegar` trial fallback from Step 4. Replace with single `cegar_engine::solve_cycle(&cur_g, remaining)`.
   - Update Step 5 error fallback to call `cegar_engine::solve_path` and `cegar_engine::solve_cycle`.
4. **Step 4:** Deprecate or retire `fallback_cegar.rs`, ensuring `cycle_merge.rs` is cleanly referenced by `cegar_engine`.
5. **Step 5:** Verification Gate: Run all 18 test suites and verify small + challenge benchmark instances.
