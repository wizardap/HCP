# Independent Module Solvers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement and evaluate the 3 proposed approaches independently and sequentially on hard benchmark graphs (`graph788`, `graph710`, `graph717`, `graph479`):
1. **Approach 1 (Direct Inverse 3-SAT)**: Extract 44-vertex modules and dual $T_i/F_i$ paths, solve the $k$-variable 3-SAT in <5ms, and synthesize the tour directly.
2. **Approach 2 (Module State Equivalence Clauses in CEGAR)**: Inject selector variables $b_i \leftrightarrow T_i, \neg b_i \leftrightarrow F_i$ into CEGAR `base_cnf` to eliminate intra-module 16-cycles and guide CDCL search.
3. **Approach 3 (Macro-Variable Dynamic Routing via Crossover Splicer)**: Use local auxiliary SAT to flip module truth states and merge boundary macro-cycles.

**Architecture:** A modular pipeline where shared module detection (`BipartiteModuleDetector`) and dual-path extraction (`ModuleDualPathExtractor`) feed into 3 cleanly separated solving engines that can each be toggled on/off independently via CLI flags or configuration.

**Tech Stack:** Rust, `rustsat`, `rustsat-cadical`, standard library.

## Global Constraints
- Core 3 is 100% strictly reserved for the user (`taskset -c 0,1,2 nice -n 19 ...`).
- Never touch files or configuration outside `/home/ubuntu/HCP`.
- Never read, hardcode, or inject `.hcp.tou` solution tours into the solver.
- Follow strict TDD (Red-Green-Refactor).

---

### Task 1: Module Detection & Dual-Path Extraction (`BipartiteModuleDetector` & `ModuleDualPathExtractor`)

**Files:**
- Create: `src/cegar-fix/src/bipartite_module_detector.rs`
- Create: `src/cegar-fix/src/module_dual_path_extractor.rs`
- Modify: `src/cegar-fix/src/lib.rs`
- Modify: `src/cegar-fix/src/main.rs`
- Test: `src/cegar-fix/tests/test_bipartite_module_detector.rs`
- Test: `src/cegar-fix/tests/test_module_dual_path_extractor.rs`

- [ ] **Step 1: Write the failing tests**
  - Create `tests/test_bipartite_module_detector.rs`: Construct a synthetic graph with two 44-vertex modules connected by external cross-edges and verify that `BipartiteModuleDetector::detect_44_modules` detects exactly 2 modules of size 44.
  - Create `tests/test_module_dual_path_extractor.rs`: For a detected 44-vertex module, extract $T_i$ and $F_i$ paths and assert both are valid Hamiltonian paths across all 44 vertices using 100% of the 22 virtual edges.

- [ ] **Step 2: Run tests to confirm failure (RED)**
  ```bash
  cargo test --test test_bipartite_module_detector
  cargo test --test test_module_dual_path_extractor
  ```

- [ ] **Step 3: Implement `BipartiteModuleDetector` (GREEN)**
  - Use virtual edge quotient projection: build quotient graph of virtual edges connected by real edges.
  - Connected components form the 44-vertex modules.

- [ ] **Step 4: Implement `ModuleDualPathExtractor` (GREEN)**
  - Identify boundary interface ports with external edges.
  - Bounded DFS inside the 44-vertex induced subgraph finding $T_i$ and $F_i$ spanning all 44 vertices.

- [ ] **Step 5: Verify tests pass (GREEN)**
  ```bash
  cargo test --test test_bipartite_module_detector
  cargo test --test test_module_dual_path_extractor
  ```

- [ ] **Step 6: Commit changes**
  ```bash
  git add src/cegar-fix/src/bipartite_module_detector.rs src/cegar-fix/src/module_dual_path_extractor.rs src/cegar-fix/src/lib.rs src/cegar-fix/src/main.rs src/cegar-fix/tests/test_bipartite_module_detector.rs src/cegar-fix/tests/test_module_dual_path_extractor.rs
  git commit -m "feat(module): implement BipartiteModuleDetector and ModuleDualPathExtractor"
  ```

---

### Task 2: Evaluate Approach 1 (Direct Inverse 3-SAT Solver)

**Files:**
- Modify: `src/cegar-fix/src/inverse_3sat_synthesizer.rs`
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_inverse_3sat_synthesizer.rs`

- [ ] **Step 1: Write the failing unit test**
  - Create `tests/test_inverse_3sat_synthesizer.rs` testing end-to-end de-reduction and synthesis on a synthetic 3-variable formula.

- [ ] **Step 2: Run test to confirm failure (RED)**
  ```bash
  cargo test --test test_inverse_3sat_synthesizer
  ```

- [ ] **Step 3: Implement 44-vertex module synthesis in `Inverse3SatSynthesizer` (GREEN)**
  - Integrate `BipartiteModuleDetector` and `ModuleDualPathExtractor`.
  - Formulate $k$-variable SAT instance.
  - Solve in CaDiCaL (<5ms).
  - Stitch true/false paths into full tour.

- [ ] **Step 4: Verify test passes (GREEN)**
  ```bash
  cargo test --test test_inverse_3sat_synthesizer
  ```

- [ ] **Step 5: Run Independent Benchmark on `graph479.col` and `graph788.col`**
  ```bash
  taskset -c 0,1,2 nice -n 19 timeout 60 ./src/cegar-fix/target/release/cegar-fix --input FHCPCS-col/graph479.col --auto 1 --timeout 45
  taskset -c 0,1,2 nice -n 19 timeout 60 ./src/cegar-fix/target/release/cegar-fix --input FHCPCS-col/graph788.col --auto 1 --timeout 45
  ```
  - Record execution time, verification result, and any edge-case gaps.

- [ ] **Step 6: Commit changes**
  ```bash
  git add src/cegar-fix/src/inverse_3sat_synthesizer.rs src/cegar-fix/src/hcp_solver.rs src/cegar-fix/tests/test_inverse_3sat_synthesizer.rs
  git commit -m "feat(solver): evaluate Approach 1 direct inverse 3-SAT synthesis"
  ```

---

### Task 3: Evaluate Approach 2 (Module State Equivalence Clauses in CEGAR)

**Files:**
- Create: `src/cegar-fix/src/module_state_cnf_encoder.rs`
- Modify: `src/cegar-fix/src/lib.rs`
- Modify: `src/cegar-fix/src/main.rs`
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_module_state_cnf_encoder.rs`

- [ ] **Step 1: Write the failing unit test**
  - Create `tests/test_module_state_cnf_encoder.rs`: Verify that state selector literals $b_i$ properly enforce $T_i$ and $F_i$ in CNF and forbid spurious internal cycles.

- [ ] **Step 2: Run test to confirm failure (RED)**
  ```bash
  cargo test --test test_module_state_cnf_encoder
  ```

- [ ] **Step 3: Implement `ModuleStateCnfEncoder` (GREEN)**
  - Allocate $k$ selector variables $b_1, \dots, b_k$.
  - Add equivalence clauses: $b_i \implies T_i$ edges, $\neg b_i \implies F_i$ edges.
  - Forbid edges inside module that do not belong to $T_i \cup F_i$.

- [ ] **Step 4: Verify test passes (GREEN)**
  ```bash
  cargo test --test test_module_state_cnf_encoder
  ```

- [ ] **Step 5: Run Independent Benchmark on `graph788.col`, `graph710.col`, `graph717.col`**
  ```bash
  taskset -c 0,1,2 nice -n 19 timeout 60 ./src/cegar-fix/target/release/cegar-fix --input FHCPCS-col/graph788.col --auto 1 --timeout 45
  taskset -c 0,1,2 nice -n 19 timeout 60 ./src/cegar-fix/target/release/cegar-fix --input FHCPCS-col/graph710.col --auto 1 --timeout 45
  taskset -c 0,1,2 nice -n 19 timeout 60 ./src/cegar-fix/target/release/cegar-fix --input FHCPCS-col/graph717.col --auto 1 --timeout 45
  ```
  - Record whether intra-module 16-cycles are eliminated and how many CEGAR rounds are needed.

- [ ] **Step 6: Commit changes**
  ```bash
  git add src/cegar-fix/src/module_state_cnf_encoder.rs src/cegar-fix/src/lib.rs src/cegar-fix/src/main.rs src/cegar-fix/src/hcp_solver.rs src/cegar-fix/tests/test_module_state_cnf_encoder.rs
  git commit -m "feat(encoder): evaluate Approach 2 module state equivalence encoding"
  ```

---

### Task 4: Evaluate Approach 3 (Macro-Variable Dynamic Routing via Crossover Splicer)

**Files:**
- Modify: `src/cegar-fix/src/macro_crossover_splicer.rs`
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_macro_crossover_splicer.rs`

- [ ] **Step 1: Write test for module-level state flips during crossover**
  - Add test in `tests/test_macro_crossover_splicer.rs` where macro-cycles can merge by flipping entire module $T_i \leftrightarrow F_i$ states.

- [ ] **Step 2: Implement module flip variables in `MacroCrossoverSplicer`**
  - Allow boundary crossover SAT to choose whether each module operates in $T_i$ or $F_i$ mode.

- [ ] **Step 3: Run tests to confirm GREEN**
  ```bash
  cargo test --test test_macro_crossover_splicer
  ```

- [ ] **Step 4: Run Independent Benchmark**
  - Test on `graph788.col`, `graph710.col`, `graph717.col`.

- [ ] **Step 5: Commit changes**
  ```bash
  git add src/cegar-fix/src/macro_crossover_splicer.rs src/cegar-fix/src/hcp_solver.rs src/cegar-fix/tests/test_macro_crossover_splicer.rs
  git commit -m "feat(splicer): evaluate Approach 3 macro-variable dynamic routing"
  ```

---

### Task 5: Comparative Analysis & Synthesis
- Compare the metrics across Approach 1, 2, and 3:
  - Solve time
  - Number of CEGAR rounds
  - Generalizability across pure modular vs hybrid hub graphs
- Generate a summary report and propose the best unified configuration.
