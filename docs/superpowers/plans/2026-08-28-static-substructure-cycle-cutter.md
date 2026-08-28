# Static Substructure Cycle Cutter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Permanently prevent small-cycle subtour discovery oscillations (e.g. 519 4-cycles on `graph479.col`) by statically extracting all induced 3-cycles and 4-cycles and injecting directional subtour elimination clauses into `base_cnf` at Round 0.

**Architecture:** A standalone `StaticCycleCutter` module managing static cycle extraction and clause generation, integrated into `solve_hamilton` in `hcp_solver.rs`.

**Tech Stack:** Rust (2021 edition), CaDiCaL SAT solver (`rustsat`, `rustsat_cadical`).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.

---

### Task 1: StaticCycleCutter Engine

**Files:**
- Create: `src/cegar-fix/src/static_cycle_cutter.rs`
- Modify: `src/cegar-fix/src/lib.rs` (export `pub mod static_cycle_cutter;`)
- Modify: `src/cegar-fix/src/main.rs` (register `pub mod static_cycle_cutter;`)
- Test: `src/cegar-fix/tests/test_static_cycle_cutter.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct StaticCycleCutter;

  impl StaticCycleCutter {
      pub fn generate_static_small_cycle_cuts(
          g: &Graph,
          encoder: &Encoder,
      ) -> Cnf;
  }
  ```

- [ ] **Step 1: Write unit tests** in `src/cegar-fix/tests/test_static_cycle_cutter.rs`
- [ ] **Step 2: Run test to verify it fails** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_static_cycle_cutter`)
- [ ] **Step 3: Implement `src/cegar-fix/src/static_cycle_cutter.rs`**
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_static_cycle_cutter`)
- [ ] **Step 5: Commit changes**

---

### Task 2: Wire StaticCycleCutter into Solver Base Encoding

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_staged_solver.rs`

**Interfaces:**
- Consumes: `StaticCycleCutter::generate_static_small_cycle_cuts`
- Behavior: In `solve_hamilton`, call `generate_static_small_cycle_cuts(&g, &encoder)` and append clauses to `cnf` before solver loading.

- [ ] **Step 1: Wire static cuts into `src/cegar-fix/src/hcp_solver.rs`**
- [ ] **Step 2: Run full workspace test suite** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 3: Commit changes**

---

### Task 3: Benchmark Verification on `graph479.col`

**Files:**
- Verify: `FHCPCS-col/graph479.col`

- [ ] **Step 1: Build release binary** (`taskset -c 0,1,2 nice -n 19 cargo build --release`)
- [ ] **Step 2: Run full workspace test suite** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 3: Run benchmark on `graph479.col` and verify Round 0 eliminates 4-cycles**
- [ ] **Step 4: Document results and commit**
