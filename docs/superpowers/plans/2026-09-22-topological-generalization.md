# Pure-Rust Topological Generalization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate all graph-specific hardcoded vertex IDs, hardcoded strip indices, and vertex-count branch matching (`match vertex_count`), replacing them with general topological feature classifiers (2-cut articulation separators, Voronoi super-hub bipartite clustering, and degree-2 alternating pair contractions).

**Architecture:** Refactor `macro_decomp/` into three true topological engines (`corridor.rs` -> `Corridor2CutSolver`, `portfolio_788.rs` -> `AlternatingPairSolver`, `bipartite.rs` -> `DenseBipartiteSolver`) that dynamically detect graph topology purely from edge connectivity. Refactor `solver_pipeline.rs` to dispatch solvers via topological feature probes (`find_2cut_ports`, `super_hubs_count`, `degree2_ratio`) rather than checking `vertex_count` or graph IDs.

**Tech Stack:** Rust 2021, CaDiCaL SAT solver (`rustsat-cadical`), Rayon parallelization, Tarjan articulation point discovery.

**Spec:** `docs/superpowers/specs/2026-09-22-topological-generalization-design.md`

## Global Constraints
- 100% Pure Native Rust: Zero Python subprocess calls or dependencies.
- Zero Hardcoded Vertex IDs: No `vec![1430, 3641, ...]`, no `[164, 5787, ...]`, no hardcoded strip arrays.
- Zero Vertex-Count Branching: No `match vertex_count { 4064 => ..., 4286 => ... }` in pipeline or solver modules.
- 100% Soundness: All output tours verified by mathematical bijection, DIMACS edge validity, and `TourVerifier`.
- Backward Compatibility: Maintain test pass rate across all existing unit and integration tests.

---

### Task 1: Generalize Corridor 2-Cut Solver (`corridor.rs`)

**Files:**
- Modify: `src/cegar-fix/src/macro_decomp/corridor.rs`
- Test: `src/cegar-fix/tests/test_macro_challenge_graphs.rs`

**Interfaces:**
- Produces:
  `pub fn solve_2cut_corridor(raw_g: &Graph, timeout_secs: f64) -> Option<Vec<i32>>`
  `pub fn can_solve_2cut(raw_g: &Graph) -> Option<(i32, i32)>` (delegating to `find_2cut_ports`)
- Backward compatibility:
  `pub fn solve_710(raw_g: &Graph, timeout_secs: f64) -> Option<Vec<i32>> { solve_2cut_corridor(raw_g, timeout_secs) }`

- [ ] **Step 1: Write unit test for dynamic 2-cut solver on arbitrary graph**
  In `src/cegar-fix/tests/test_macro_challenge_graphs.rs`:
  Add test `test_dynamic_2cut_corridor_general` that calls `macro_corridor::solve_2cut_corridor` directly without mentioning graph710 or vertex count 4064.

- [ ] **Step 2: Remove vertex count check from `corridor.rs`**
  Remove `if raw_g.adjacency_list.len() != 4064 { return None; }`.
  Rename `solve_710` logic to `pub fn solve_2cut_corridor(raw_g: &Graph, timeout_secs: f64) -> Option<Vec<i32>>`.
  Keep `pub fn solve_710` as a 1-line wrapper for backward compatibility.

- [ ] **Step 3: Run targeted test**
  Run `cargo test --test test_macro_challenge_graphs test_dynamic_2cut --manifest-path src/cegar-fix/Cargo.toml --release`.

- [ ] **Step 4: Commit changes**
  `git commit -m "refactor(corridor): generalize 2-cut articulation solver to arbitrary graphs"`

---

### Task 2: Generalize Alternating Pair Solver (`portfolio_788.rs`)

**Files:**
- Modify: `src/cegar-fix/src/macro_decomp/portfolio_788.rs`
- Test: `src/cegar-fix/tests/test_macro_challenge_graphs.rs`

**Interfaces:**
- Produces:
  `pub fn solve_alternating_pairs(raw_g: &Graph, timeout_secs: f64) -> Option<Vec<i32>>`
  `pub fn can_solve_alternating_pairs(raw_g: &Graph) -> bool`
- Backward compatibility:
  `pub fn solve_788(raw_g: &Graph, timeout_secs: f64) -> Option<Vec<i32>> { solve_alternating_pairs(raw_g, timeout_secs) }`

- [ ] **Step 1: Write test for generalized alternating pair solver**
  Add test verifying `solve_alternating_pairs` on `graph788.col`.

- [ ] **Step 2: Remove vertex count check from `portfolio_788.rs`**
  Remove `if raw_g.adjacency_list.len() != 4620 { return None; }`.
  Implement `pub fn can_solve_alternating_pairs(raw_g: &Graph) -> bool` checking:
  - Total degree-2 vertices $\ge |V| / 3$.
  - Contracted graph contains valid bipartite pairs that partition the endpoints.
  Rename solving function to `solve_alternating_pairs`.

- [ ] **Step 3: Run targeted test**
  Run `cargo test --test test_macro_challenge_graphs --manifest-path src/cegar-fix/Cargo.toml --release`.

- [ ] **Step 4: Commit changes**
  `git commit -m "refactor(portfolio): generalize alternating pair solver by removing vertex count checks"`

---

### Task 3: Generalize Bipartite Cluster Splicing (`bipartite.rs`)

**Files:**
- Modify: `src/cegar-fix/src/macro_decomp/bipartite.rs`
- Test: `src/cegar-fix/tests/test_macro_challenge_graphs.rs`

**Interfaces:**
- Produces:
  `pub fn solve_bipartite_macro(raw_g: &Graph, timeout_secs: f64) -> Option<Vec<i32>>`
  `pub fn detect_bipartite_super_hubs(raw_g: &Graph) -> Option<Vec<i32>>`

- [ ] **Step 1: Replace hardcoded super-hub comparisons with topological degree queries**
  In `bipartite.rs`:
  Instead of `if super_hubs != vec![1430, 3641, 3735, 3790, 3960]`, detect super-hubs dynamically by degree threshold ($d(u) \ge 500$).
  Remove hardcoded `raw_g.adjacency_list.len() != 4286` and `!= 6620`.

- [ ] **Step 2: Dynamically discover cluster strips and endpoint interfaces**
  Refactor strip grouping and endpoint detection to derive connections from adjacency rather than hardcoded strip indices `[3, 8, 9, 13, 16]`.

- [ ] **Step 3: Run targeted test**
  Run `cargo test --test test_macro_challenge_graphs --manifest-path src/cegar-fix/Cargo.toml --release`.

- [ ] **Step 4: Commit changes**
  `git commit -m "refactor(bipartite): generalize super-hub detection and cluster extraction"`

---

### Task 4: Topological Feature Dispatch in Pipeline (`solver_pipeline.rs`)

**Files:**
- Modify: `src/cegar-fix/src/pipeline/solver_pipeline.rs`
- Test: `src/cegar-fix/tests/test_unified_cli_batch.rs`

**Interfaces:**
- Eliminates:
  `match vertex_count { 4286 => ..., 4064 => ..., 4620 => ..., 6620 => ..., _ => None }`
- Introduces:
  Topological Cascade:
  1. `if let Some((u, v)) = macro_corridor::can_solve_2cut(&g) => solve_2cut_corridor`
  2. `if macro_788::can_solve_alternating_pairs(&g) => solve_alternating_pairs`
  3. `if let Some(hubs) = macro_bipartite::detect_bipartite_super_hubs(&g) => solve_bipartite_macro`
  4. Fallback: `fallback_cegar::solve_with_contraction`

- [ ] **Step 1: Replace `match vertex_count` with topological feature probes**
  Update `solver_pipeline.rs` to route graphs based purely on their structural properties.

- [ ] **Step 2: Test end-to-end with unified CLI**
  Run `cargo test --test test_unified_cli_batch --manifest-path src/cegar-fix/Cargo.toml --release`.
  Run `./run_solver_rust.sh -i FHCPCS-col/graph1.col`.
  Run `./run_solver_rust.sh -i FHCPCS-col/graph710.col`.
  Run `./run_solver_rust.sh -i FHCPCS-col/graph788.col`.

- [ ] **Step 3: Verify zero compiler warnings**
  `RUSTFLAGS="-D warnings" cargo check --manifest-path src/cegar-fix/Cargo.toml --all-targets`.

- [ ] **Step 4: Commit and push**
  `git commit -m "feat(pipeline): replace vertex count matching with dynamic topological feature cascade"`
