# Twin Giant Cycle Splicer & Bicomponent Cut Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `TwinGiantSplicer` to discover direct 2-opt and 3-opt intermediate bridge splices between twin giant cycles ($|C_1|, |C_2| \ge |V| / 4$), and inject exact bicomponent cut clauses to force CaDiCaL to cross the bicomponent boundary.

**Architecture:** Create `src/cegar-fix/src/twin_giant_splicer.rs`, wire into `giant_cycle_stitcher.rs` and `hcp_solver.rs`.

**Tech Stack:** Rust (2021 edition), CaDiCaL SAT solver (`rustsat`, `rustsat_cadical`).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.
- Empirical Rigor: No overpromising. Maintain strict verification.

---

### Task 1: `TwinGiantSplicer` Engine

**Files:**
- Create: `src/cegar-fix/src/twin_giant_splicer.rs`
- Modify: `src/cegar-fix/src/lib.rs`, `src/cegar-fix/src/main.rs` (export `pub mod twin_giant_splicer;`)
- Test: `src/cegar-fix/tests/test_twin_giant_splicer.rs`

**Interfaces:**
```rust
use crate::graph::Graph;
use rustsat::types::{Clause, Lit};
use std::collections::{HashMap, HashSet};

pub struct TwinGiantSplicer;

impl TwinGiantSplicer {
    pub fn try_splice_twin_giants(
        cycles: &[Vec<i32>],
        g: &Graph,
        total_v: usize,
    ) -> Option<Vec<Vec<i32>>>;

    pub fn generate_bicomponent_cut_clauses(
        c1: &[i32],
        c2: &[i32],
        g: &Graph,
        graph_lit_map: &HashMap<(i32, i32), Lit>,
    ) -> Vec<Clause>;
}
```

- [ ] **Step 1: Write unit tests** in `src/cegar-fix/tests/test_twin_giant_splicer.rs`
- [ ] **Step 2: Run test to verify it fails** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_twin_giant_splicer`)
- [ ] **Step 3: Implement `TwinGiantSplicer` in `src/cegar-fix/src/twin_giant_splicer.rs`**
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_twin_giant_splicer`)
- [ ] **Step 5: Commit changes**

---

### Task 2: Wire `TwinGiantSplicer` into `GiantCycleStitcher` and `hcp_solver.rs`

**Files:**
- Modify: `src/cegar-fix/src/giant_cycle_stitcher.rs`, `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_staged_solver.rs`

- [ ] **Step 1: Add Step 8 `TwinGiantSplicer::try_splice_twin_giants` to `GiantCycleStitcher::repair_until_fixed_point`**
- [ ] **Step 2: Wire `generate_bicomponent_cut_clauses` into `hcp_solver.rs` CEGAR loop when twin giants exist**
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
