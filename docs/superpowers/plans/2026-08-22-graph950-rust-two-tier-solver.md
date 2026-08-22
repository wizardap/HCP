# Rust Two-Tier Demand-Coordinated HCP Solver Implementation Plan (`cegar-fix`)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the mathematically validated Two-Tier Demand-Coordinated HCP solver directly in `src/cegar-fix` using `rustsat` and `rustsat-cadical`, achieving sub-second iterations (~0.05s/iteration) and solving `graph950.col` within the 1800s budget under Zero Tour Injection.

**Architecture:** 
- Decompose raw graph into 310 Hubs (10 S, 50 B, 250 M), 650 Hub-Hub edges, and 74 independent Strips.
- Global Demand Coordinator enforces exact-2 degree on all 310 Hubs, strip parity bounds ($2K \in \{4, 6, 8, 10\}$), and dynamically adds flipped-literal conflict clauses and indicator-based Hub Cut-Crossing SECs.
- Local Strip Solvers use direct `rustsat-cadical` assumption solving and minimal unsat core extraction.
- Macro Splicer performs deterministic local boundary matching, fast 2-opt cycle patching, and raw graph independent tour verification.

**Tech Stack:** Rust (edition 2021), `rustsat`, `rustsat-cadical` (CaDiCaL 1.9.4), `src/cegar-fix`.

## Global Constraints
- Target graph: `/home/ubuntu/HCP/FHCPCS-col/graph950.col` ($n = 6,620$, $m = 28,718$)
- Target directory: `src/cegar-fix`
- Zero tour injection: Do not import, read, or reference `graph950.hcp.tou` during solving.
- Native Rust solver binary: `src/cegar-fix/target/release/cegar-fix`
- Soundness: All degree constraints exact-2, single cycle, independent edge membership verification on raw $G$.
- Performance budget: Total wall-clock $\le 1800$s.

---

### Task 1: Graph Topology Decomposer & Strip Extractor in `src/cegar-fix`

**Files:**
- Create: `src/cegar-fix/src/two_tier_decomposer.rs`
- Test: `src/cegar-fix/tests/test_decomposer.rs`

**Interfaces:**
- Produces:
  - `pub struct DecompositionResult { pub s_hubs: Vec<i32>, pub b_hubs: Vec<i32>, pub m_hubs: Vec<i32>, pub all_hubs: HashSet<i32>, pub hh_edges: Vec<(i32, i32)>, pub strips: Vec<Vec<i32>>, pub strip_adj_hubs: HashMap<usize, HashSet<i32>>, pub hub_adj_strips: HashMap<i32, HashSet<usize>> }`
  - `pub fn decompose_graph(g: &Graph) -> DecompositionResult`

- [ ] **Step 1: Write failing test for graph decomposition**
Create `src/cegar-fix/tests/test_decomposer.rs` asserting 310 Hubs (10 S, 50 B, 250 M), 650 Hub-Hub edges, and 74 Strips on `graph950.col`.

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test --test test_decomposer` (in `src/cegar-fix`)
Expected: FAIL.

- [ ] **Step 3: Implement `two_tier_decomposer.rs`**
Implement Hub categorization and BFS connected component strip extraction.

- [ ] **Step 4: Run test to verify it passes**
Run: `cargo test --test test_decomposer` (in `src/cegar-fix`)
Expected: PASS (asserting exact strip count 74, hub count 310, hh_edge count 650).

- [ ] **Step 5: Commit**
`git commit -m "feat(decomposer): implement graph topology decomposer in cegar-fix"`

---

### Task 2: Pinpointed Strip Solver with Assumption Core Extraction in `src/cegar-fix`

**Files:**
- Create: `src/cegar-fix/src/pinpointed_strip_solver.rs`
- Test: `src/cegar-fix/tests/test_strip_solver.rs`

**Interfaces:**
- Consumes: `DecompositionResult`, `Graph`, `rustsat::solvers::SolveIncremental` (or `rustsat_cadical::CaDiCaL`)
- Produces:
  - `pub struct PinpointedStripSolver { ... }`
  - `pub fn solve_strip(&mut self, si: usize, dem: &HashMap<i32, usize>, s_hub: Option<i32>, b_hub: Option<i32>, k: usize) -> Result<Vec<Vec<i32>>, Vec<i32>>`

- [ ] **Step 1: Write failing test for strip solving and core extraction**
Create `src/cegar-fix/tests/test_strip_solver.rs` testing SAT on small strips and UNSAT core extraction on conflicting assumptions.

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test --test test_strip_solver`
Expected: FAIL.

- [ ] **Step 3: Implement `pinpointed_strip_solver.rs`**
Implement strip internal path-cover CNF with Sinz sequential counters, assumption activations, and acyclic subtour elimination.

- [ ] **Step 4: Run test to verify it passes**
Run: `cargo test --test test_strip_solver`
Expected: PASS.

- [ ] **Step 5: Commit**
`git commit -m "feat(strip-solver): implement pinpointed strip solver with unsat core extraction in cegar-fix"`

---

### Task 3: Global Demand Coordinator with Indicator Cut-Crossing SECs in `src/cegar-fix`

**Files:**
- Create: `src/cegar-fix/src/global_demand_coordinator.rs`
- Test: `src/cegar-fix/tests/test_coordinator.rs`

**Interfaces:**
- Consumes: `DecompositionResult`, `Graph`, `rustsat_cadical::CaDiCaL`
- Produces:
  - `pub struct GlobalDemandCoordinator { ... }`
  - `pub fn solve_assignment(&mut self) -> Option<(Vec<(i32, i32)>, HashMap<usize, HashMap<i32, usize>>)>`
  - `pub fn add_conflict_clause(&mut self, si: usize, dem: &HashMap<i32, usize>, failed_hubs: &[i32])`
  - `pub fn add_macro_cut(&mut self, cyc_verts: &HashSet<i32>)`

- [ ] **Step 1: Write failing test for coordinator assignment and conflict learning**
Create `src/cegar-fix/tests/test_coordinator.rs` testing exact-2 degree on 310 Hubs, parity bounds, and conflict clause addition.

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test --test test_coordinator`
Expected: FAIL.

- [ ] **Step 3: Implement `global_demand_coordinator.rs`**
Implement Sinz counter exact-2 degree on all Hubs, sequential counter parity indicator bounds, flipped-literal conflict clauses, and indicator Cut-Crossing clauses.

- [ ] **Step 4: Run test to verify it passes**
Run: `cargo test --test test_coordinator`
Expected: PASS.

- [ ] **Step 5: Commit**
`git commit -m "feat(coordinator): implement global demand coordinator with indicator cut-crossing in cegar-fix"`

---

### Task 4: Macro Splicer & Fast 2-Opt Cycle Patching in `src/cegar-fix`

**Files:**
- Create: `src/cegar-fix/src/macro_splicer.rs`
- Test: `src/cegar-fix/tests/test_splicer.rs`

**Interfaces:**
- Consumes: `DecompositionResult`, `Graph`
- Produces:
  - `pub fn verify_tour_on_raw_graph(tour: &[i32], g: &Graph) -> bool`
  - `pub fn patch_cycles_2opt(cycles: Vec<Vec<i32>>, g: &Graph) -> Vec<Vec<i32>>`
  - `pub fn splice_macro_tour(g: &Graph, decomp: &DecompositionResult, hh_edges: &[(i32, i32)], strip_paths: &HashMap<usize, Vec<Vec<i32>>>, strip_demands: &HashMap<usize, HashMap<i32, usize>>) -> (bool, Vec<Vec<i32>>)`

- [ ] **Step 1: Write failing test for macro splicing and 2-opt cycle patching**
Create `src/cegar-fix/tests/test_splicer.rs` testing deterministic boundary matching and 2-opt cycle merging.

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test --test test_splicer`
Expected: FAIL.

- [ ] **Step 3: Implement `macro_splicer.rs`**
Implement local deterministic boundary connector, fast 2-opt cycle patcher, and independent raw tour verifier.

- [ ] **Step 4: Run test to verify it passes**
Run: `cargo test --test test_splicer`
Expected: PASS.

- [ ] **Step 5: Commit**
`git commit -m "feat(splicer): implement macro splicer and 2-opt patcher in cegar-fix"`

---

### Task 5: End-to-End Orchestrator, CLI Integration & `graph950.col` Solve

**Files:**
- Create: `src/cegar-fix/src/two_tier_orchestrator.rs`
- Modify: `src/cegar-fix/src/main.rs`
- Modify: `src/cegar-fix/src/options.rs`
- Test: `src/cegar-fix/tests/test_end_to_end.rs`

**Interfaces:**
- Produces:
  - `pub fn solve_two_tier(g: &Graph, timeout_secs: f64, out_path: Option<&str>) -> bool`

- [ ] **Step 1: Write failing end-to-end test**
Create `src/cegar-fix/tests/test_end_to_end.rs` testing the full closed-loop orchestrator.

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test --test test_end_to_end`
Expected: FAIL.

- [ ] **Step 3: Implement `two_tier_orchestrator.rs` and CLI option `--two-tier`**
Connect full closed loop: Decomposition $\to$ Coordinator $\to$ Strip Solvers $\to$ Splicer/Patcher $\to$ Cut-CEGAR $\to$ Tour Certification.

- [ ] **Step 4: Run full test suite and verify 100% pass**
Run: `cargo test` (in `src/cegar-fix`)
Expected: PASS across all unit and integration tests.

- [ ] **Step 5: Benchmark and solve `graph950.col`**
Run: `cargo run --release -- --input ../../FHCPCS-col/graph950.col --two-tier --timeout 1800`
Expected: High-speed solving (~0.05s/iteration) exploring thousands of macro iterations to output certified `scratch/graph950/found_tour_rust.hcp`.

- [ ] **Step 6: Commit**
`git commit -m "feat(orchestrator): implement full high-speed two-tier rust solver and cli in cegar-fix"`
