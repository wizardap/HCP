# Multi-Opt (2-Opt + 3-Opt Triangle) SAT Splicer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `MultiOptSatSplicer` to find both 2-opt bridges and 3-cycle triangle swaps, solve an exact SAT spanning forest problem with MTZ ordering in $< 5\text{ms}$ in CaDiCaL, and splice multi-cycle clusters simultaneously into a single Hamiltonian cycle.

**Architecture:** Create `src/cegar-fix/src/multi_opt_sat_splicer.rs`, wire into `src/cegar-fix/src/giant_cycle_stitcher.rs`.

**Tech Stack:** Rust (2021 edition), CaDiCaL SAT solver (`rustsat`, `rustsat_cadical`).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.
- Empirical Rigor: No overpromising. Maintain strict verification.

---

### Task 1: `MultiOptSatSplicer` Engine

**Files:**
- Create: `src/cegar-fix/src/multi_opt_sat_splicer.rs`
- Modify: `src/cegar-fix/src/lib.rs`, `src/cegar-fix/src/main.rs` (export `pub mod multi_opt_sat_splicer;`)
- Test: `src/cegar-fix/tests/test_multi_opt_sat_splicer.rs`

**Interfaces:**
```rust
use std::collections::{HashMap, HashSet};
use crate::graph::Graph;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CycleSwap {
    TwoOpt {
        c1: usize,
        c2: usize,
        rem1: (i32, i32),
        rem2: (i32, i32),
        add1: (i32, i32),
        add2: (i32, i32),
    },
    ThreeOptTriangle {
        c1: usize,
        c2: usize,
        c3: usize,
        rem1: (i32, i32),
        rem2: (i32, i32),
        rem3: (i32, i32),
        add1: (i32, i32),
        add2: (i32, i32),
        add3: (i32, i32),
    },
}

pub struct MultiOptSatSplicer;

impl MultiOptSatSplicer {
    pub fn splice_multi_opt_cycles(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
    ) -> Vec<Vec<i32>>;
}
```

- [ ] **Step 1: Write unit tests** in `src/cegar-fix/tests/test_multi_opt_sat_splicer.rs`
- [ ] **Step 2: Run test to verify it fails** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_multi_opt_sat_splicer`)
- [ ] **Step 3: Implement `MultiOptSatSplicer` in `src/cegar-fix/src/multi_opt_sat_splicer.rs`**
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_multi_opt_sat_splicer`)
- [ ] **Step 5: Commit changes**

---

### Task 2: Wire `MultiOptSatSplicer` into `GiantCycleStitcher`

**Files:**
- Modify: `src/cegar-fix/src/giant_cycle_stitcher.rs`
- Test: `src/cegar-fix/tests/test_staged_solver.rs`

- [ ] **Step 1: Wire `MultiOptSatSplicer::splice_multi_opt_cycles` into `repair_until_fixed_point`**
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
