# Dynamic Solver Re-seeding & Inprocessing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate $T_{\text{SAT}}$ latency creep in outer CEGAR loops by dynamically refreshing the CaDiCaL SAT solver with clean base constraints and accumulated budgeted cuts whenever solving time exceeds thresholds or periodic round intervals occur.

**Architecture:** A standalone `SolverReseeder` module managing re-seeding conditions and clean solver recreation, integrated into `solve_hcp_with_cegar` in `hcp_solver.rs`.

**Tech Stack:** Rust (2021 edition), CaDiCaL SAT solver (`rustsat`, `rustsat_cadical`).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.

---

### Task 1: SolverReseeder Engine

**Files:**
- Create: `src/cegar-fix/src/solver_reseeder.rs`
- Modify: `src/cegar-fix/src/lib.rs` (export `pub mod solver_reseeder;`)
- Modify: `src/cegar-fix/src/main.rs` (register `pub mod solver_reseeder;`)
- Test: `src/cegar-fix/tests/test_solver_reseeder.rs`

**Interfaces:**
- Produces:
  ```rust
  #[derive(Debug, Clone)]
  pub struct ReseederOptions {
      pub max_sat_time_threshold_secs: f64, // Default: 15.0s
      pub periodic_interval_rounds: usize,  // Default: 10 rounds
      pub enable_reseeding: bool,           // Default: true
  }

  pub struct SolverReseeder;

  impl SolverReseeder {
      pub fn should_reseed(
          last_sat_time_secs: f64,
          current_round: usize,
          options: &ReseederOptions,
      ) -> bool;

      pub fn reseed_solver(
          base_cnf: &Cnf,
          accumulated_cuts: &[Cnf],
          cadical_config: i32,
      ) -> CaDiCaL<'static, 'static>;
  }
  ```

- [ ] **Step 1: Write unit tests** in `src/cegar-fix/tests/test_solver_reseeder.rs`
- [ ] **Step 2: Run test to verify it fails** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_solver_reseeder`)
- [ ] **Step 3: Implement `src/cegar-fix/src/solver_reseeder.rs`**
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_solver_reseeder`)
- [ ] **Step 5: Commit changes**

---

### Task 2: Wire SolverReseeder into CEGAR Loop

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_staged_solver.rs`

**Interfaces:**
- Consumes: `SolverReseeder::should_reseed` and `SolverReseeder::reseed_solver`
- Behavior: In `solve_hcp_with_cegar`, keep track of `accumulated_cut_cnfs`. When re-seeding triggers, re-instantiate `solver` cleanly and log the refresh event.

- [ ] **Step 1: Wire re-seeding into `src/cegar-fix/src/hcp_solver.rs`**
- [ ] **Step 2: Run full workspace test suite** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 3: Commit changes**

---

### Task 3: Benchmark Verification on `graph479.col` & `graph668.col`

**Files:**
- Verify: `FHCPCS-col/graph479.col` and `FHCPCS-col/graph668.col`

- [ ] **Step 1: Build release binary** (`taskset -c 0,1,2 nice -n 19 cargo build --release`)
- [ ] **Step 2: Run full workspace test suite** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 3: Run benchmark on `graph479.col` and observe re-seeding and $T_{\text{SAT}}$ stability**
- [ ] **Step 4: Document results and commit**
