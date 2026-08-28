# Hemisphere Splicing & Bi-Partition Crossing Cuts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Merge 2-to-4 macro-hemisphere cycles into a single Hamiltonian cycle via direct 2-opt cross-splicing and directional bi-partition crossing cuts ($\delta^+(C) \ge 1, \delta^-(C) \ge 1$).

**Architecture:** A standalone `HemisphereSplicer` module managing pairwise macro-cycle 2-opt splicing and directional bi-partition crossing cut generation, integrated into `hcp_solver.rs`.

**Tech Stack:** Rust (2021 edition), CaDiCaL SAT solver (`rustsat`, `rustsat_cadical`).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.

---

### Task 1: HemisphereSplicer Engine

**Files:**
- Create: `src/cegar-fix/src/hemisphere_splicer.rs`
- Modify: `src/cegar-fix/src/lib.rs` (export `pub mod hemisphere_splicer;`)
- Modify: `src/cegar-fix/src/main.rs` (register `pub mod hemisphere_splicer;`)
- Test: `src/cegar-fix/tests/test_hemisphere_splicer.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct HemisphereSplicer;

  impl HemisphereSplicer {
      pub fn try_direct_splice_all(
          cycles: &[Vec<i32>],
          g: &Graph,
          contractor: &Degree2Contractor,
      ) -> Option<Vec<Vec<i32>>>;

      pub fn generate_hemisphere_crossing_cuts(
          cycles: &[Vec<i32>],
          g: &Graph,
          encoder: &Encoder,
      ) -> Vec<Clause>;
  }
  ```

- [ ] **Step 1: Write unit tests** in `src/cegar-fix/tests/test_hemisphere_splicer.rs`
- [ ] **Step 2: Run test to verify it fails** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_hemisphere_splicer`)
- [ ] **Step 3: Implement `src/cegar-fix/src/hemisphere_splicer.rs`**
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_hemisphere_splicer`)
- [ ] **Step 5: Commit changes**

---

### Task 2: Wire HemisphereSplicer into CEGAR Solver

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_staged_solver.rs`

**Interfaces:**
- Consumes: `HemisphereSplicer::try_direct_splice_all` and `HemisphereSplicer::generate_hemisphere_crossing_cuts`
- Behavior: In `two_opt`, if active cycles $k \in [2, 4]$, invoke `try_direct_splice_all`. In `get_blocking_clauses`, if $k \in [2, 4]$, append crossing cuts.

- [ ] **Step 1: Wire splicing and cuts into `src/cegar-fix/src/hcp_solver.rs`**
- [ ] **Step 2: Run full workspace test suite** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 3: Commit changes**

---

### Task 3: Benchmark Verification on `graph479.col`

**Files:**
- Verify: `FHCPCS-col/graph479.col`

- [ ] **Step 1: Build release binary** (`taskset -c 0,1,2 nice -n 19 cargo build --release`)
- [ ] **Step 2: Run full workspace test suite** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 3: Run benchmark on `graph479.col` and observe 2-hemisphere splicing/cuts**
- [ ] **Step 4: Document results and commit**
