# Empirical Backbone Frequency Tracker & Aggressive SEC Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `EmpiricalBackboneTracker` to track edge frequencies across recent SAT solutions, lock backbone edges of giant cycles with frequency $\ge 85\%$, and inject 100% comprehensive SEC clauses for all non-giant subcycles to accelerate late-round CaDiCaL convergence.

**Architecture:** Create `src/cegar-fix/src/empirical_backbone_cutter.rs`, wire into `src/cegar-fix/src/hcp_solver.rs`.

**Tech Stack:** Rust (2021 edition), CaDiCaL SAT solver (`rustsat`, `rustsat_cadical`).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.
- Empirical Rigor: No overpromising. Maintain strict verification.

---

### Task 1: `EmpiricalBackboneCutter` Engine

**Files:**
- Create: `src/cegar-fix/src/empirical_backbone_cutter.rs`
- Modify: `src/cegar-fix/src/lib.rs`, `src/cegar-fix/src/main.rs` (export `pub mod empirical_backbone_cutter;`)
- Test: `src/cegar-fix/tests/test_empirical_backbone_cutter.rs`

**Interfaces:**
```rust
use std::collections::{HashMap, HashSet};
use rustsat::types::Lit;

#[derive(Debug, Clone, Default)]
pub struct EmpiricalBackboneTracker {
    pub history_window: usize,
    pub edge_history: Vec<HashSet<(i32, i32)>>,
    pub total_rounds_recorded: usize,
}

impl EmpiricalBackboneTracker {
    pub fn new(window_size: usize) -> Self;
    pub fn record_solution_edges(&mut self, cycles: &[Vec<i32>]);
    pub fn get_frequent_backbone_edges(&self, threshold: f64) -> HashSet<(i32, i32)>;
}

pub struct EmpiricalBackboneCutter;

impl EmpiricalBackboneCutter {
    pub fn generate_comprehensive_sec_clauses(
        cycles: &[Vec<i32>],
        giant_threshold: usize,
        lit_map: &HashMap<(i32, i32), Lit>,
    ) -> Vec<Vec<Lit>>;
}
```

- [ ] **Step 1: Write unit tests** in `src/cegar-fix/tests/test_empirical_backbone_cutter.rs`
- [ ] **Step 2: Run test to verify it fails** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_empirical_backbone_cutter`)
- [ ] **Step 3: Implement `EmpiricalBackboneCutter` in `src/cegar-fix/src/empirical_backbone_cutter.rs`**
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_empirical_backbone_cutter`)
- [ ] **Step 5: Commit changes**

---

### Task 2: Wire `EmpiricalBackboneCutter` into `hcp_solver.rs`

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_staged_solver.rs`

- [ ] **Step 1: Wire `EmpiricalBackboneTracker` and comprehensive SEC cuts into `hcp_solver.rs` CEGAR loop**
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
