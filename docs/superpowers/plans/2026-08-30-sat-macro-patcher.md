# Exact SAT Macro-Patching & Multi-Cycle Bridge Solver Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `SatMacroPatcher` to solve an exact SAT spanning tree formulation over all candidate 2-opt bridges when $\le 30$ subcycles remain, discovering simultaneous multi-bridge $k$-opt merges in $< 5\text{ms}$.

**Architecture:** Create `src/cegar-fix/src/sat_macro_patcher.rs`, wire into `giant_cycle_stitcher.rs` and `hcp_solver.rs`.

**Tech Stack:** Rust (2021 edition), CaDiCaL SAT solver (`rustsat`, `rustsat_cadical`).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.
- Empirical Rigor: No overpromising. Maintain strict verification.

---

### Task 1: `SatMacroPatcher` Engine

**Files:**
- Create: `src/cegar-fix/src/sat_macro_patcher.rs`
- Modify: `src/cegar-fix/src/lib.rs`, `src/cegar-fix/src/main.rs` (export `pub mod sat_macro_patcher;`)
- Test: `src/cegar-fix/tests/test_sat_macro_patcher.rs`

**Interfaces:**
```rust
use crate::graph::Graph;
use std::collections::{HashMap, HashSet};

pub struct SatMacroPatcher;

impl SatMacroPatcher {
    pub fn try_patch_all_cycles(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
    ) -> Option<Vec<i32>>;
}
```

- [ ] **Step 1: Write unit tests** in `src/cegar-fix/tests/test_sat_macro_patcher.rs`
- [ ] **Step 2: Run test to verify it fails** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_sat_macro_patcher`)
- [ ] **Step 3: Implement `SatMacroPatcher` in `src/cegar-fix/src/sat_macro_patcher.rs`**
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_sat_macro_patcher`)
- [ ] **Step 5: Commit changes**

---

### Task 2: Wire `SatMacroPatcher` into `GiantCycleStitcher` and `hcp_solver.rs`

**Files:**
- Modify: `src/cegar-fix/src/giant_cycle_stitcher.rs`, `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_staged_solver.rs`

- [ ] **Step 1: Add Step 10 `SatMacroPatcher::try_patch_all_cycles` to `GiantCycleStitcher::repair_until_fixed_point`**
- [ ] **Step 2: Wire direct fast-path return in `hcp_solver.rs` when `SatMacroPatcher` finds a full tour**
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
