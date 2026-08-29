# Transitive Macro-Cycle Graph Splicer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `TransitiveMacroSplicer` to construct a macro-graph $\mathcal{M}$ over the remaining disjoint 2-factor cycles ($m \le 60$), find a global transitive bridging structure, and splice all cycles into a single unified Hamiltonian tour via exact SAT in $< 2\text{ms}$.

**Architecture:** Create `src/cegar-fix/src/transitive_macro_splicer.rs`, wire into `src/cegar-fix/src/giant_cycle_stitcher.rs` and `src/cegar-fix/src/hcp_solver.rs`.

**Tech Stack:** Rust (2021 edition), CaDiCaL SAT solver (`rustsat`, `rustsat_cadical`).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.

---

### Task 1: `TransitiveMacroSplicer` Engine

**Files:**
- Create: `src/cegar-fix/src/transitive_macro_splicer.rs`
- Modify: `src/cegar-fix/src/lib.rs`, `src/cegar-fix/src/main.rs` (export `pub mod transitive_macro_splicer;`)
- Test: `src/cegar-fix/tests/test_transitive_macro_splicer.rs`

**Interfaces:**
```rust
use std::collections::HashSet;
use crate::graph::Graph;

pub struct TransitiveMacroSplicer;

impl TransitiveMacroSplicer {
    pub fn splice_transitive_macro_graph(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
    ) -> Vec<Vec<i32>>;
}
```

- [ ] **Step 1: Write unit tests** in `src/cegar-fix/tests/test_transitive_macro_splicer.rs`
- [ ] **Step 2: Run test to verify it fails** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_transitive_macro_splicer`)
- [ ] **Step 3: Implement `TransitiveMacroSplicer` in `src/cegar-fix/src/transitive_macro_splicer.rs`**
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_transitive_macro_splicer`)
- [ ] **Step 5: Commit changes**

---

### Task 2: Wire `TransitiveMacroSplicer` into CEGAR Loop

**Files:**
- Modify: `src/cegar-fix/src/giant_cycle_stitcher.rs`, `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_staged_solver.rs`

- [ ] **Step 1: Wire `TransitiveMacroSplicer` into `repair_until_fixed_point` and `hcp_solver.rs`**
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
