# Localized Window Splicer & Pure Monotonic CEGAR Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Compute, export, and independently verify a 100% exact, deterministic certified Hamiltonian tour for Flinders Class 1 instance `graph868.col` ($N=5,544$) within 1,800.0s on Core 0.

**Architecture:** Combine a rapid **Localized Window Splicer** (absorbing 8-block gadgets into the Giant and medium cycles via local SAT windows of 15–25 blocks in <5ms) with an active **Pure Monotonic CEGAR** solver anchored to best-known polarities. Once cycle count reaches 1, the uncontracted tour is exported to TSPLIB HCP format and verified via an independent Stage 4 verifier.

**Tech Stack:** Rust (edition 2021), CaDiCaL SAT solver (`rustsat_cadical`), `rustsat`, Python 3 (verification).

## Global Constraints
- Pinned strictly to Core 0 (`taskset -c 0 nice -n 19`), single worker (`num_workers = 1`).
- Deterministic execution (`seed = 777`).
- Total wall-clock time limit $\le 1,800.0$s.
- 100% exact combinatorial method: zero heuristics (no LKH, Concorde), zero reading `.tou` files, zero tour injection.
- Export uncontracted tour to `scratch/graph868/found_tour_graph868.hcp`.
- Independent verification via `python3 scratch/test_stage4_verification.py FHCPCS-col/graph868.col scratch/graph868/found_tour_graph868.hcp`.

---

### Task 1: Local Gadget Topology & Multi-Connection Mapper

**Files:**
- Create: `src/cegar-fix/src/gadget_window_mapper.rs`
- Modify: `src/cegar-fix/src/lib.rs`
- Test: `src/cegar-fix/tests/test_gadget_window_mapper.rs`

**Interfaces:**
- Consumes: `Graph`, `Degree2Contractor` from `cegar_fix`.
- Produces:
  ```rust
  pub struct WindowCandidate {
      pub subcycle_id: usize,
      pub target_cycle_id: usize,
      pub window_start_pos: usize,
      pub window_len: usize,
      pub sub_blocks: Vec<usize>,
      pub window_target_blocks: Vec<usize>,
  }
  pub struct GadgetWindowMapper;
  impl GadgetWindowMapper {
      pub fn find_candidate_windows(
          cycs: &[Vec<usize>],
          dir_adj: &[Vec<usize>],
          max_distance: usize,
      ) -> Vec<WindowCandidate>;
  }
  ```

- [ ] **Step 1: Write the failing integration test**
  Create `src/cegar-fix/tests/test_gadget_window_mapper.rs` testing candidate window extraction on `scratch/best_edges_graph868.txt`.

- [ ] **Step 2: Run test to verify it fails**
  Run: `cargo test --test test_gadget_window_mapper --manifest-path src/cegar-fix/Cargo.toml`
  Expected: FAIL with module/type not found.

- [ ] **Step 3: Implement `gadget_window_mapper.rs`**
  Implement `GadgetWindowMapper::find_candidate_windows` which iterates through all subcycles, maps their boundary ports to positions on the Giant (and medium cycles), and identifies pairs of connections with distance $\le \text{max\_distance}$ on the target cycle.

- [ ] **Step 4: Run test to verify it passes**
  Run: `cargo test --test test_gadget_window_mapper --manifest-path src/cegar-fix/Cargo.toml`
  Expected: PASS, finding candidate windows for all 20 gadgets.

- [ ] **Step 5: Commit changes**
  Commit `src/cegar-fix/src/gadget_window_mapper.rs` and the test.

---

### Task 2: Localized Window Splicer

**Files:**
- Create: `src/cegar-fix/src/localized_window_splicer.rs`
- Modify: `src/cegar-fix/src/lib.rs`
- Test: `src/cegar-fix/tests/test_localized_window_splicer.rs`

**Interfaces:**
- Consumes: `WindowCandidate`, `Degree2Contractor`, `Graph`.
- Produces:
  ```rust
  pub struct LocalizedWindowSplicer;
  impl LocalizedWindowSplicer {
      pub fn try_splice_window(
          cand: &WindowCandidate,
          cycs: &mut Vec<Vec<usize>>,
          dir_adj: &[Vec<usize>],
          n_dir: usize,
      ) -> bool;

      pub fn splice_all(
          cycs: &mut Vec<Vec<usize>>,
          dir_adj: &[Vec<usize>],
          n_dir: usize,
      ) -> usize;
  }
  ```

- [ ] **Step 1: Write the failing unit test**
  Create `src/cegar-fix/tests/test_localized_window_splicer.rs` verifying that `try_splice_window` merges an 8-block gadget into a 2-factor without introducing illegal degrees or disconnected cycles.

- [ ] **Step 2: Run test to verify it fails**
  Run: `cargo test --test test_localized_window_splicer --manifest-path src/cegar-fix/Cargo.toml`
  Expected: FAIL.

- [ ] **Step 3: Implement `localized_window_splicer.rs`**
  Build a localized CNF encoding:
  - Internal degree constraints for each block port in the window $W = S \cup G_W$.
  - Fixed boundary ports entering from the predecessor on the target cycle and exiting to the successor on the target cycle.
  - Subcycle elimination constraints on $W$ to ensure a single Hamiltonian path through $W$.
  - Solve with CaDiCaL (<5ms per window). If SAT, rewrite the cycle representation in `cycs`.

- [ ] **Step 4: Run test to verify it passes**
  Run: `cargo test --test test_localized_window_splicer --manifest-path src/cegar-fix/Cargo.toml`
  Expected: PASS, successfully reducing cycle count.

- [ ] **Step 5: Commit changes**
  Commit `src/cegar-fix/src/localized_window_splicer.rs` and the test.

---

### Task 3: Integrate Splicer into Production CEGAR Solver

**Files:**
- Modify: `src/cegar-fix/examples/solve_class1_exact.rs`
- Test: `scratch/test_stage4_verification.py`

**Interfaces:**
- Consumes: `LocalizedWindowSplicer::splice_all`, `GiantCycleStitcher`, `CaDiCaL`.
- Produces: Final verified Hamiltonian tour in TSPLIB HCP format.

- [ ] **Step 1: Update `solve_class1_exact.rs` loop**
  - In each round, before global cut injection, invoke `LocalizedWindowSplicer::splice_all(&mut cycs, ...)`.
  - If `cycs.len() == 1`, uncontract and verify tour.
  - Re-anchor CaDiCaL polarity `phase_lit` to the spliced configuration.
  - If subcycles remain, add CEGAR subtour and DFJ boundary cuts to all remaining non-giant cycles.

- [ ] **Step 2: Run short 30s test run**
  Run: `./src/cegar-fix/target/release/examples/solve_class1_exact FHCPCS-col/graph868.col scratch/graph868/test_tour.hcp 30.0`
  Verify that cycles monotonically reduce towards 1.

- [ ] **Step 3: Commit changes**
  Commit `src/cegar-fix/examples/solve_class1_exact.rs`.

---

### Task 4: Official 1,800s Benchmark & Independent Tour Verification

**Files:**
- Output: `scratch/graph868/found_tour_graph868.hcp`
- Verification script: `scratch/test_stage4_verification.py`

- [ ] **Step 1: Launch official benchmark on Core 0**
  Run command:
  ```bash
  mkdir -p scratch/graph868 && taskset -c 0 nice -n 19 ./src/cegar-fix/target/release/examples/solve_class1_exact FHCPCS-col/graph868.col scratch/graph868/found_tour_graph868.hcp 1800.0 > scratch/graph868/run_1800s.log 2>&1
  ```

- [ ] **Step 2: Monitor until completion**
  Verify process completes and produces `scratch/graph868/found_tour_graph868.hcp`.

- [ ] **Step 3: Run independent verification**
  Run:
  ```bash
  python3 scratch/test_stage4_verification.py FHCPCS-col/graph868.col scratch/graph868/found_tour_graph868.hcp
  ```
  Expected: "100% HAMILTONIAN TOUR VERIFIED: 5544 vertices, valid edges, 0 duplicates, exactly 1 cycle."
