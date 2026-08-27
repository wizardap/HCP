# Giant Cycle Preservation & Deep Absorption Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prevent CaDiCaL from shattering near-complete giant subcycles ($\ge 50\% - 95\%$ of vertices) into fragmented symmetric blocks, locking internal backbone edges as SAT assumptions and expanding deep alternating cycle absorption.

**Architecture:** Adaptive backbone freezing in `BackboneFreezer` triggered dynamically on large cycle length ($|C| \ge 0.50 \times N$) or subcycle count ($m \le 25$), paired with multi-point alternating absorption in `CycleChainAbsorber` and integrated into the CEGAR loop in `hcp_solver.rs`.

**Tech Stack:** Rust (2021 edition), CaDiCaL SAT solver (`rustsat`, `rustsat_cadical`).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.

---

### Task 1: Adaptive BackboneFreezer Engine

**Files:**
- Modify: `src/cegar-fix/src/backbone_freezer.rs`
- Test: `src/cegar-fix/tests/test_backbone_freezer.rs`

**Interfaces:**
- Produces:
  ```rust
  impl BackboneFreezer {
      pub fn extract_backbone_assumptions(
          cycles: &[Vec<i32>],
          g: &Graph,
          encoder: &Encoder,
          min_giant_ratio: f64,
          max_cycle_count_trigger: usize,
      ) -> Vec<Lit>;
  }
  ```

- [ ] **Step 1: Write failing unit tests** in `src/cegar-fix/tests/test_backbone_freezer.rs` (testing $\ge 50\%$ giant cycle preservation on 6-cycle cases)
- [ ] **Step 2: Run test to verify it fails** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_backbone_freezer`)
- [ ] **Step 3: Implement adaptive thresholds in `src/cegar-fix/src/backbone_freezer.rs`**
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_backbone_freezer`)
- [ ] **Step 5: Commit changes**

---

### Task 2: Wire Adaptive Freezing into CEGAR Solver

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_staged_solver.rs`

**Interfaces:**
- Consumes: `BackboneFreezer::extract_backbone_assumptions`
- Behavior: In `solve_hcp_with_cegar`, trigger backbone assumptions whenever `max_cycle_len >= total_v / 2 || _active_cycles.len() <= 25`.

- [ ] **Step 1: Write integration test in `test_staged_solver.rs`**
- [ ] **Step 2: Run test to verify baseline** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_staged_solver`)
- [ ] **Step 3: Integrate adaptive freezing in `src/cegar-fix/src/hcp_solver.rs`**
- [ ] **Step 4: Run full workspace tests to verify they pass** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 5: Commit changes**

---

### Task 3: Benchmark Verification on `graph479.col`

**Files:**
- Verify: `FHCPCS-col/graph479.col`
- Command: `taskset -c 0,1,2 nice -n 19 ./src/cegar-fix/target/release/cegar-fix --input FHCPCS-col/graph479.col --auto 1`

- [ ] **Step 1: Build release binary** (`taskset -c 0,1,2 nice -n 19 cargo build --release`)
- [ ] **Step 2: Run full workspace test suite** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 3: Run benchmark on `graph479.col` and verify giant cycle preservation**
- [ ] **Step 4: Document results and commit**
