# Bounded Backbone Freezer & Extended Static Cycle Cutter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate Heavy Assumption Drag on CaDiCaL by bounding backbone edge freezing to $\le 250$ edges with adaptive relaxation, and eliminate 6-cycle noise upfront via extended static subtour cuts at Round 0.

**Architecture:** Enhancements to `BackboneFreezer` and `StaticCycleCutter`, integrated into `hcp_solver.rs`.

**Tech Stack:** Rust (2021 edition), CaDiCaL SAT solver (`rustsat`, `rustsat_cadical`).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.

---

### Task 1: Bounded Backbone Freezer Engine

**Files:**
- Modify: `src/cegar-fix/src/backbone_freezer.rs`
- Test: `src/cegar-fix/tests/test_backbone_freezer.rs`

**Interfaces:**
- Enhance:
  ```rust
  #[derive(Debug, Clone)]
  pub struct FreezerOptions {
      pub ratio_threshold: f64,
      pub max_subcycles_trigger: usize,
      pub max_frozen_edges: usize, // Default: 250
      pub adaptive_relax_time_secs: f64, // Default: 10.0
  }

  impl BackboneFreezer {
      pub fn select_adaptive_frozen_assumptions(
          cycles: &[Vec<i32>],
          g: &Graph,
          encoder: &Encoder,
          contractor: &Degree2Contractor,
          opts: &FreezerOptions,
          last_sat_time_secs: f64,
      ) -> Vec<Lit>;
  }
  ```

- [ ] **Step 1: Write unit tests** in `src/cegar-fix/tests/test_backbone_freezer.rs`
- [ ] **Step 2: Run test to verify it fails** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_backbone_freezer`)
- [ ] **Step 3: Implement bounding and adaptive relaxation in `src/cegar-fix/src/backbone_freezer.rs`**
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_backbone_freezer`)
- [ ] **Step 5: Commit changes**

---

### Task 2: Extended Static Cycle Cutter (6-Cycles)

**Files:**
- Modify: `src/cegar-fix/src/static_cycle_cutter.rs`
- Test: `src/cegar-fix/tests/test_static_cycle_cutter.rs`

**Interfaces:**
- Add 6-cycle extraction and subtour elimination clause generation (capped at 4,000 clauses).

- [ ] **Step 1: Write unit tests** in `src/cegar-fix/tests/test_static_cycle_cutter.rs`
- [ ] **Step 2: Run test to verify it fails** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_static_cycle_cutter`)
- [ ] **Step 3: Implement 6-cycle extraction in `src/cegar-fix/src/static_cycle_cutter.rs`**
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_static_cycle_cutter`)
- [ ] **Step 5: Commit changes**

---

### Task 3: Wire into Solver & Benchmark Verification

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Verify: `FHCPCS-col/graph479.col`

- [ ] **Step 1: Wire `last_sat_time_secs` and `FreezerOptions` into `hcp_solver.rs`**
- [ ] **Step 2: Run full workspace test suite** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 3: Run benchmark on `graph479.col` and verify $T_{\text{SAT}} \le 5\text{s}$ and 6-cycle elimination**
- [ ] **Step 4: Document results and commit**
