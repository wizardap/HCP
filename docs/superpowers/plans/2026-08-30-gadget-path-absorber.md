# Gadget Hamiltonian Path Absorber Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `GadgetPathAbsorber` to discover Hamiltonian paths in small satellite subcycles ($|C_s| \le 16$) and splice them directly into larger cycles.

**Architecture:** Create `src/cegar-fix/src/gadget_path_absorber.rs`, wire into `giant_cycle_stitcher.rs` and `hcp_solver.rs`.

**Tech Stack:** Rust (2021 edition).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.
- Empirical Rigor: No overpromising. Maintain strict verification.

---

### Task 1: `GadgetPathAbsorber` Engine

**Files:**
- Create: `src/cegar-fix/src/gadget_path_absorber.rs`
- Modify: `src/cegar-fix/src/lib.rs`, `src/cegar-fix/src/main.rs` (export `pub mod gadget_path_absorber;`)
- Test: `src/cegar-fix/tests/test_gadget_path_absorber.rs`

**Interfaces:**
```rust
use crate::graph::Graph;
use std::collections::{HashMap, HashSet};

pub struct GadgetPathAbsorber;

impl GadgetPathAbsorber {
    pub fn try_absorb_gadgets(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
    ) -> Vec<Vec<i32>>;
}
```

- [ ] **Step 1: Write unit tests** in `src/cegar-fix/tests/test_gadget_path_absorber.rs`
- [ ] **Step 2: Run test to verify it fails** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_gadget_path_absorber`)
- [ ] **Step 3: Implement `GadgetPathAbsorber` in `src/cegar-fix/src/gadget_path_absorber.rs`**
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_gadget_path_absorber`)
- [ ] **Step 5: Commit changes**

---

### Task 2: Wire `GadgetPathAbsorber` into `GiantCycleStitcher` and `hcp_solver.rs`

**Files:**
- Modify: `src/cegar-fix/src/giant_cycle_stitcher.rs`, `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_staged_solver.rs`

- [ ] **Step 1: Add Step 11 `GadgetPathAbsorber::try_absorb_gadgets` to `GiantCycleStitcher::repair_until_fixed_point`**
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
