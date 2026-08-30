# Max-Vertex SAT Spanning Tree Splicer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `SatMacroPatcher::try_patch_components` to solve exact SAT spanning tree formulations over every connected component of candidate 2-opt bridges when $\le 60$ subcycles remain.

**Architecture:** Extend `src/cegar-fix/src/sat_macro_patcher.rs`, wire into `giant_cycle_stitcher.rs` and `hcp_solver.rs`.

**Tech Stack:** Rust (2021 edition), CaDiCaL SAT solver (`rustsat`, `rustsat_cadical`).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.
- Empirical Rigor: No overpromising. Maintain strict verification.

---

### Task 1: `try_patch_components` in `SatMacroPatcher`

**Files:**
- Modify: `src/cegar-fix/src/sat_macro_patcher.rs`
- Test: `src/cegar-fix/tests/test_sat_macro_patcher.rs`

**Interfaces:**
```rust
use crate::graph::Graph;
use std::collections::{HashMap, HashSet};

pub struct SatMacroPatcher;

impl SatMacroPatcher {
    pub fn try_patch_components(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
    ) -> Vec<Vec<i32>>;
}
```

- [ ] **Step 1: Write unit tests** in `src/cegar-fix/tests/test_sat_macro_patcher.rs`
- [ ] **Step 2: Run test to verify it fails** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_sat_macro_patcher`)
- [ ] **Step 3: Implement `try_patch_components` in `src/cegar-fix/src/sat_macro_patcher.rs`**
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_sat_macro_patcher`)
- [ ] **Step 5: Commit changes**

---

### Task 2: Wire `try_patch_components` into `GiantCycleStitcher` and `hcp_solver.rs`

**Files:**
- Modify: `src/cegar-fix/src/giant_cycle_stitcher.rs`, `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_staged_solver.rs`

- [ ] **Step 1: Update Step 10 in `GiantCycleStitcher::repair_until_fixed_point` to use `try_patch_components` with $\le 60$ threshold**
- [ ] **Step 2: Wire into `hcp_solver.rs` CEGAR loop**
- [ ] **Step 3: Add integration test in `src/cegar-fix/tests/test_staged_solver.rs`**
- [ ] **Step 4: Run full workspace test suite** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 5: Commit changes**

---

### Task 3: Benchmark Verification on `graph479.col` & `graph668.col`

**Files:**
- Verify: `FHCPCS-col/graph479.col` and `FHCPCS-col/graph668.col`

- [ ] **Step 1: Build release binary** (`taskset -c 0,1,2 nice -n 19 cargo build --release`)
- [ ] **Step 2: Run full workspace test suite** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 3: Run benchmark on `graph479.col` and `graph668.col`**
- [ ] **Step 4: Document results and commit**
