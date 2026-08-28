# Boundary Alternating Patcher Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bridge and merge multi-hop separated macro-hemisphere cycles into a single Hamiltonian tour via bounded-depth Alternating Reconnection Paths (Lin-Kernighan / k-opt augmentation).

**Architecture:** A standalone `BoundaryAlternatingPatcher` module managing multi-hop alternating path discovery and symmetric difference cycle merging, integrated into `hcp_solver.rs`.

**Tech Stack:** Rust (2021 edition), CaDiCaL SAT solver (`rustsat`, `rustsat_cadical`).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.

---

### Task 1: BoundaryAlternatingPatcher Engine

**Files:**
- Create: `src/cegar-fix/src/boundary_alternating_patcher.rs`
- Modify: `src/cegar-fix/src/lib.rs` (export `pub mod boundary_alternating_patcher;`)
- Modify: `src/cegar-fix/src/main.rs` (register `pub mod boundary_alternating_patcher;`)
- Test: `src/cegar-fix/tests/test_boundary_alternating_patcher.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct BoundaryAlternatingPatcher;

  impl BoundaryAlternatingPatcher {
      pub fn try_patch_macro_hemispheres(
          cycles: &[Vec<i32>],
          g: &Graph,
          contractor: &Degree2Contractor,
          max_search_depth: usize,
      ) -> Option<Vec<Vec<i32>>>;
  }
  ```

- [ ] **Step 1: Write unit tests** in `src/cegar-fix/tests/test_boundary_alternating_patcher.rs`
- [ ] **Step 2: Run test to verify it fails** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_boundary_alternating_patcher`)
- [ ] **Step 3: Implement `src/cegar-fix/src/boundary_alternating_patcher.rs`**
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_boundary_alternating_patcher`)
- [ ] **Step 5: Commit changes**

---

### Task 2: Wire BoundaryAlternatingPatcher into CEGAR Loop

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_staged_solver.rs`

**Interfaces:**
- Consumes: `BoundaryAlternatingPatcher::try_patch_macro_hemispheres`
- Behavior: In `cegar` solving loop, when $k \in [2, 4]$, invoke `try_patch_macro_hemispheres`. If full tour is produced, return certified SATISFIABLE.

- [ ] **Step 1: Wire patcher into `src/cegar-fix/src/hcp_solver.rs`**
- [ ] **Step 2: Run full workspace test suite** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 3: Commit changes**

---

### Task 3: Benchmark Verification on `graph479.col`

**Files:**
- Verify: `FHCPCS-col/graph479.col`

- [ ] **Step 1: Build release binary** (`taskset -c 0,1,2 nice -n 19 cargo build --release`)
- [ ] **Step 2: Run full workspace test suite** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 3: Run benchmark on `graph479.col` and observe alternating patcher execution**
- [ ] **Step 4: Document results and commit**
