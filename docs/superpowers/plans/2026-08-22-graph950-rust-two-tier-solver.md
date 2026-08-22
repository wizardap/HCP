# Rust Two-Tier Demand-Coordinated HCP Solver Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the mathematically validated Two-Tier Demand-Coordinated HCP solver to native Rust with direct CaDiCaL C FFI, achieving 0.05s/iteration and solving `graph950.col` within the 1800s budget under Zero Tour Injection.

**Architecture:** 
- Decompose raw graph into 310 Hubs (10 S, 50 B, 250 M), 650 Hub-Hub edges, and 74 independent Strips.
- Global Demand Coordinator enforces exact-2 degree on all 310 Hubs, strip parity bounds ($2K \in \{4, 6, 8, 10\}$), and dynamically adds flipped-literal conflict clauses and indicator-based Hub Cut-Crossing SECs.
- Local Strip Solvers use direct CaDiCaL assumption solving and minimal unsat core extraction.
- Macro Splicer performs deterministic local boundary matching, fast 2-opt cycle patching, and raw graph independent tour verification.

**Tech Stack:** Rust (edition 2021), CaDiCaL C FFI (`src/cegar-ffi/src/solver_wrapper.c`), `cargo test`.

## Global Constraints
- Target graph: `/home/ubuntu/HCP/FHCPCS-col/graph950.col` ($n = 6,620$, $m = 28,718$)
- Zero tour injection: Do not import, read, or reference `graph950.hcp.tou` during solving.
- Native Rust FFI solver binary: `src/cegar-ffi/target/release/cegar-ffi`
- Soundness: All degree constraints exact-2, single cycle, independent edge membership verification on raw $G$.
- Performance budget: Total wall-clock $\le 1800$s.

---

### Task 1: Extend CaDiCaL C Wrapper & Rust FFI for Assumption/Core Solving

**Files:**
- Modify: `src/cegar-ffi/src/solver_wrapper.c`
- Modify: `src/cegar-ffi/src/encoder.rs` or `src/cegar-ffi/src/cadical_ffi.rs`
- Test: `src/cegar-ffi/tests/test_ffi_assumptions.rs`

**Interfaces:**
- Produces:
  - `Solver::new() -> *mut c_void`
  - `Solver::assume(ptr, lit)`
  - `Solver::failed(ptr, lit) -> bool`
  - `Solver::add_clause(ptr, &[i32])`

- [ ] **Step 1: Write failing integration test for FFI assumptions**
Create `src/cegar-ffi/tests/test_ffi_assumptions.rs` testing assumption-based solving and core extraction on an infeasible assumption set.

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test --test test_ffi_assumptions`
Expected: FAIL due to missing `assume` / `failed` functions.

- [ ] **Step 3: Implement C wrapper and Rust bindings**
Update `solver_wrapper.c` and expose safe Rust methods `assume()` and `failed()`.

- [ ] **Step 4: Run test to verify it passes**
Run: `cargo test --test test_ffi_assumptions`
Expected: PASS.

- [ ] **Step 5: Commit**
`git commit -m "feat(ffi): add cadical assumption and core extraction bindings"`

---

### Task 2: Graph Topology Decomposer & Strip Extractor in Rust

**Files:**
- Create: `src/cegar-ffi/src/two_tier_decomposer.rs`
- Test: `src/cegar-ffi/tests/test_decomposer.rs`

**Interfaces:**
- Produces:
  - `pub struct DecompositionResult { s_hubs: Vec<usize>, b_hubs: Vec<usize>, m_hubs: Vec<usize>, all_hubs: HashSet<usize>, hh_edges: Vec<(usize, usize)>, strips: Vec<Vec<usize>>, strip_adj_hubs: HashMap<usize, HashSet<usize>>, hub_adj_strips: HashMap<usize, HashSet<usize>> }`
  - `pub fn decompose_graph(g: &Graph) -> DecompositionResult`

- [ ] **Step 1: Write failing test for graph decomposition**
Create `src/cegar-ffi/tests/test_decomposer.rs` asserting 310 Hubs (10 S, 50 B, 250 M), 650 Hub-Hub edges, and 74 Strips on `graph950.col`.

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test --test test_decomposer`
Expected: FAIL.

- [ ] **Step 3: Implement `two_tier_decomposer.rs`**
Implement Hub categorization and BFS connected component strip extraction.

- [ ] **Step 4: Run test to verify it passes**
Run: `cargo test --test test_decomposer`
Expected: PASS (asserting exact strip count 74, hub count 310, hh_edge count 650).

- [ ] **Step 5: Commit**
`git commit -m "feat(decomposer): implement graph topology decomposer in rust"`

---

### Task 3: Pinpointed Strip Solver with Assumption Core Extraction in Rust

**Files:**
- Create: `src/cegar-ffi/src/pinpointed_strip_solver.rs`
- Test: `src/cegar-ffi/tests/test_strip_solver.rs`

**Interfaces:**
- Consumes: `DecompositionResult`, `Graph`, CaDiCaL FFI
- Produces:
  - `pub struct PinpointedStripSolver { ... }`
  - `pub fn solve_strip(&mut self, si: usize, dem: &HashMap<usize, usize>, s_hub: Option<usize>, b_hub: Option<usize>, k: usize) -> Result<Vec<Vec<usize>>, Vec<usize>>`

- [ ] **Step 1: Write failing test for strip solving and core extraction**
Create `src/cegar-ffi/tests/test_strip_solver.rs` testing SAT on small strips and UNSAT core extraction on conflicting assumptions.

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test --test test_strip_solver`
Expected: FAIL.

- [ ] **Step 3: Implement `pinpointed_strip_solver.rs`**
Implement strip internal path-cover CNF with Sinz sequential counters, assumption activations, and acyclic subtour elimination.

- [ ] **Step 4: Run test to verify it passes**
Run: `cargo test --test test_strip_solver`
Expected: PASS.

- [ ] **Step 5: Commit**
`git commit -m "feat(strip-solver): implement pinpointed strip solver with unsat core extraction in rust"`

---

### Task 4: Global Demand Coordinator with Indicator Cut-Crossing SECs in Rust

**Files:**
- Create: `src/cegar-ffi/src/global_demand_coordinator.rs`
- Test: `src/cegar-ffi/tests/test_coordinator.rs`

**Interfaces:**
- Consumes: `DecompositionResult`, `Graph`, CaDiCaL FFI
- Produces:
  - `pub struct GlobalDemandCoordinator { ... }`
  - `pub fn solve_assignment(&mut self) -> Option<(Vec<(usize, usize)>, HashMap<usize, HashMap<usize, usize>>)>`
  - `pub fn add_conflict_clause(&mut self, si: usize, dem: &HashMap<usize, usize>, failed_hubs: &[usize])`
  - `pub fn add_macro_cut(&mut self, cyc_verts: &HashSet<usize>)`

- [ ] **Step 1: Write failing test for coordinator assignment and conflict learning**
Create `src/cegar-ffi/tests/test_coordinator.rs` testing exact-2 degree on 310 Hubs, parity bounds, and conflict clause addition.

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test --test test_coordinator`
Expected: FAIL.

- [ ] **Step 3: Implement `global_demand_coordinator.rs`**
Implement Sinz counter exact-2 degree on all Hubs, sequential counter parity indicator bounds, flipped-literal conflict clauses, and indicator Cut-Crossing clauses.

- [ ] **Step 4: Run test to verify it passes**
Run: `cargo test --test test_coordinator`
Expected: PASS.

- [ ] **Step 5: Commit**
`git commit -m "feat(coordinator): implement global demand coordinator with indicator cut-crossing in rust"`

---

### Task 5: Macro Splicer, 2-Opt Patching, & End-to-End Orchestrator

**Files:**
- Create: `src/cegar-ffi/src/macro_splicer.rs`
- Create: `src/cegar-ffi/src/two_tier_orchestrator.rs`
- Modify: `src/cegar-ffi/src/main.rs`
- Test: `src/cegar-ffi/tests/test_end_to_end.rs`

**Interfaces:**
- Produces:
  - `pub fn verify_tour_on_raw_graph(tour: &[usize], g: &Graph) -> bool`
  - `pub fn patch_cycles_2opt(cycles: Vec<Vec<usize>>, g: &Graph) -> Vec<Vec<usize>>`
  - `pub fn solve_graph950_two_tier(graph_path: &str, timeout_secs: f64, out_path: &str) -> bool`

- [ ] **Step 1: Write failing end-to-end test**
Create `src/cegar-ffi/tests/test_end_to_end.rs` verifying 2-opt cycle patching and end-to-end integration.

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test --test test_end_to_end`
Expected: FAIL.

- [ ] **Step 3: Implement `macro_splicer.rs`, `two_tier_orchestrator.rs`, and CLI entry point in `main.rs`**
Integrate the full closed loop: Decomposition $\to$ Macro Coordinator $\to$ Strip Solvers $\to$ Splicer & 2-Opt Patching $\to$ Cut-CEGAR $\to$ Independent Tour Certification.

- [ ] **Step 4: Run full test suite and verify 100% pass**
Run: `cargo test`
Expected: PASS across all unit and integration tests.

- [ ] **Step 5: Benchmark and solve `graph950.col`**
Run: `cargo run --release -- --input FHCPCS-col/graph950.col --two-tier --timeout 1800`
Expected: High-speed solving (~0.05s/iteration) exploring thousands of macro iterations to output certified `scratch/graph950/found_tour_rust.hcp`.

- [ ] **Step 6: Commit**
`git commit -m "feat(orchestrator): implement full high-speed two-tier rust solver and cli"`
