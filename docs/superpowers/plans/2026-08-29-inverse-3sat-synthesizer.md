# Inverse 3-SAT Gadget De-reduction & Tour Synthesizer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `Inverse3SatSynthesizer` to automatically de-reduce 3-SAT reduction graphs into their underlying Boolean satisfiability formulas, solve them in CaDiCaL in $< 50\text{ms}$, and directly synthesize the exact Hamiltonian Tour.

**Architecture:** Create `src/cegar-fix/src/inverse_3sat_synthesizer.rs`, wire into `src/cegar-fix/src/hcp_solver.rs`.

**Tech Stack:** Rust (2021 edition), CaDiCaL SAT solver (`rustsat`, `rustsat_cadical`).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.
- Empirical Rigor: No overpromising. Maintain strict verification.

---

### Task 1: `Inverse3SatSynthesizer` Engine

**Files:**
- Create: `src/cegar-fix/src/inverse_3sat_synthesizer.rs`
- Modify: `src/cegar-fix/src/lib.rs`, `src/cegar-fix/src/main.rs` (export `pub mod inverse_3sat_synthesizer;`)
- Test: `src/cegar-fix/tests/test_inverse_3sat_synthesizer.rs`

**Interfaces:**
```rust
use std::collections::{HashMap, HashSet};
use crate::graph::Graph;

#[derive(Debug, Clone)]
pub struct DeReducedVariable {
    pub var_id: usize,
    pub vertices: Vec<i32>,
    pub port_in: i32,
    pub port_out: i32,
    pub true_path: Vec<i32>,
    pub false_path: Vec<i32>,
}

#[derive(Debug, Clone)]
pub struct DeReducedClause {
    pub clause_id: usize,
    pub clause_vertices: Vec<i32>,
    pub literal_hooks: Vec<(usize, bool, i32, i32)>, // (var_id, is_positive, enter_rung, exit_rung)
}

pub struct Inverse3SatSynthesizer;

impl Inverse3SatSynthesizer {
    pub fn try_solve_via_inverse_3sat(g: &Graph) -> Option<Vec<i32>>;
}
```

- [ ] **Step 1: Write unit tests** in `src/cegar-fix/tests/test_inverse_3sat_synthesizer.rs`
- [ ] **Step 2: Run test to verify it fails** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_inverse_3sat_synthesizer`)
- [ ] **Step 3: Implement `Inverse3SatSynthesizer` in `src/cegar-fix/src/inverse_3sat_synthesizer.rs`**
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_inverse_3sat_synthesizer`)
- [ ] **Step 5: Commit changes**

---

### Task 2: Wire `Inverse3SatSynthesizer` Fast Track into `hcp_solver.rs`

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_staged_solver.rs`

- [ ] **Step 1: Wire `Inverse3SatSynthesizer::try_solve_via_inverse_3sat` into Round 0 in `hcp_solver.rs`**
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
