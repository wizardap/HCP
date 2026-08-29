# Universal Giant-Cycle Local Repair & Adaptive SAT Stitcher Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `GiantCycleStitcher` to perform targeted, exact CaDiCaL SAT absorption of local subcycles into the dominant giant cycle across any HCP graph, avoiding long full-graph SAT restarts.

**Architecture:** New module `src/cegar-fix/src/giant_cycle_stitcher.rs`, integration into `src/cegar-fix/src/hcp_solver.rs`.

**Tech Stack:** Rust (2021 edition), CaDiCaL SAT solver (`rustsat`, `rustsat_cadical`).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.

---

### Task 1: `GiantCycleStitcher` Engine

**Files:**
- Create: `src/cegar-fix/src/giant_cycle_stitcher.rs`
- Modify: `src/cegar-fix/src/lib.rs`, `src/cegar-fix/src/main.rs` (export `pub mod giant_cycle_stitcher;`)
- Test: `src/cegar-fix/tests/test_giant_cycle_stitcher.rs`

**Interfaces:**
```rust
use std::collections::HashSet;
use crate::graph::Graph;

pub struct GiantCycleStitcher;

impl GiantCycleStitcher {
    pub fn absorb_into_giant_cycle(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
        max_swaps: usize,
    ) -> Vec<Vec<i32>>;

    pub fn repair_until_fixed_point(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
    ) -> Vec<Vec<i32>>;
}
```

- [ ] **Step 1: Write unit tests** in `src/cegar-fix/tests/test_giant_cycle_stitcher.rs`
- [ ] **Step 2: Run test to verify it fails** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_giant_cycle_stitcher`)
- [ ] **Step 3: Implement `GiantCycleStitcher` in `src/cegar-fix/src/giant_cycle_stitcher.rs`**
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_giant_cycle_stitcher`)
- [ ] **Step 5: Commit changes**

---

### Task 2: Wire `GiantCycleStitcher` into CEGAR Loop in `hcp_solver.rs`

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_staged_solver.rs`

- [ ] **Step 1: Modify `hcp_solver.rs` to invoke `GiantCycleStitcher::repair_until_fixed_point` in the CEGAR patching pipeline with adaptive thresholds**
- [ ] **Step 2: Add integration test in `src/cegar-fix/tests/test_staged_solver.rs`**
- [ ] **Step 3: Run full workspace test suite** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 4: Commit changes**

---

### Task 3: Benchmark Verification on `graph479.col` & `graph668.col`

**Files:**
- Verify: `FHCPCS-col/graph479.col` and `FHCPCS-col/graph668.col`

- [ ] **Step 1: Build release binary** (`taskset -c 0,1,2 nice -n 19 cargo build --release`)
- [ ] **Step 2: Run full workspace test suite** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 3: Run benchmark on `graph479.col` and `graph668.col`**
- [ ] **Step 4: Document results and commit**
