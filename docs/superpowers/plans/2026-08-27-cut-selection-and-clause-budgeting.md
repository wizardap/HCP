# Cut Selection & Clause Budgeting Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Dynamic Cut Selection and Clause Budgeting (`CutSelector`) in the CEGAR solver to prioritize high-information short cycle cuts, cap per-round clause generation ($K_{\max} \le 40$), and keep SAT solving latency $T_{\text{SAT}} \le 1 - 3\text{s}$ per iteration on large graphs.

**Architecture:** A standalone `CutSelector` module that sorts detected subcycles by length, selects the top-$K_{\max}$ shortest cycles, generates strong boundary cuts for tiny cycles ($\le 8$ vertices) and tight direct exclusion clauses for standard small cycles ($\le 64$ vertices), dropping ineffective long-cycle clauses.

**Tech Stack:** Rust (2021 edition), CaDiCaL SAT solver (`rustsat`, `rustsat_cadical`).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.

---

### Task 1: CutSelector Engine

**Files:**
- Create: `src/cegar-fix/src/cut_selector.rs`
- Modify: `src/cegar-fix/src/lib.rs` (export `pub mod cut_selector;`)
- Modify: `src/cegar-fix/src/main.rs` (register `pub mod cut_selector;`)
- Test: `src/cegar-fix/tests/test_cut_selector.rs`

**Interfaces:**
- Produces:
  ```rust
  use rustsat::types::Clause;
  use crate::graph::Graph;
  use crate::encoder::Encoder;

  #[derive(Debug, Clone)]
  pub struct CutSelectorOptions {
      pub max_cuts_per_round: usize,
      pub max_cycle_len_for_cut: usize,
      pub small_cycle_threshold: usize,
      pub enable_boundary_cuts: bool,
  }

  impl Default for CutSelectorOptions {
      fn default() -> Self {
          Self {
              max_cuts_per_round: 40,
              max_cycle_len_for_cut: 64,
              small_cycle_threshold: 8,
              enable_boundary_cuts: true,
          }
      }
  }

  pub struct CutSelector;

  impl CutSelector {
      pub fn select_and_generate_cuts(
          cycles: &[Vec<i32>],
          g: &Graph,
          encoder: &Encoder,
          options: &CutSelectorOptions,
      ) -> (Vec<Clause>, Vec<Vec<i32>>);
  }
  ```

- [ ] **Step 1: Write the failing unit tests** in `src/cegar-fix/tests/test_cut_selector.rs`
- [ ] **Step 2: Run test to verify it fails** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_cut_selector`)
- [ ] **Step 3: Implement `CutSelector`** in `src/cegar-fix/src/cut_selector.rs` and export in `lib.rs` and `main.rs`
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_cut_selector`)
- [ ] **Step 5: Commit changes**

---

### Task 2: Integrate CutSelector into CEGAR Solver

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_staged_solver.rs`

**Interfaces:**
- Consumes: `CutSelector::select_and_generate_cuts`
- Behavior: In `solve_hcp_with_cegar`, apply `CutSelector` to `_active_cycles` when generating blocking clauses, replacing the unconstrained clause flood.

- [ ] **Step 1: Write integration test in `test_staged_solver.rs`**
- [ ] **Step 2: Run test to verify baseline** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_staged_solver`)
- [ ] **Step 3: Integrate `CutSelector` into `src/cegar-fix/src/hcp_solver.rs`**
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 5: Commit changes**

---

### Task 3: Benchmark Verification on `graph651.col`

**Files:**
- Verify: `FHCPCS-col/graph651.col`
- Command: `taskset -c 0,1,2 nice -n 19 ./src/cegar-fix/target/release/cegar-fix --input FHCPCS-col/graph651.col --auto 1`

- [ ] **Step 1: Build release binary** (`taskset -c 0,1,2 nice -n 19 cargo build --release`)
- [ ] **Step 2: Run full workspace test suite** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 3: Run benchmark on `graph651.col` for 60s and verify stable $T_{\text{SAT}} \le 1 - 3\text{s}$ per iteration**
- [ ] **Step 4: Document results and commit**
