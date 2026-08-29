# Hub-Centric Hierarchical Decomposer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `HubHierarchicalDecomposer` to decompose large hub-centric graphs (like `graph668.col` with 60 degree-14 hubs) into 2 tiers: solve localized internal Hamiltonian paths on the 60 hub modules ($< 1\text{ms}$ each), contract into a compact 60-hub macro-graph, solve globally via CaDiCaL SAT, and expand into the complete Hamiltonian tour.

**Architecture:** Create `src/cegar-fix/src/hub_hierarchical_decomposer.rs`, wire into `src/cegar-fix/src/hcp_solver.rs`.

**Tech Stack:** Rust (2021 edition), CaDiCaL SAT solver (`rustsat`, `rustsat_cadical`).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.
- Empirical Rigor: No overpromising. Maintain strict verification.

---

### Task 1: `HubHierarchicalDecomposer` Engine

**Files:**
- Create: `src/cegar-fix/src/hub_hierarchical_decomposer.rs`
- Modify: `src/cegar-fix/src/lib.rs`, `src/cegar-fix/src/main.rs` (export `pub mod hub_hierarchical_decomposer;`)
- Test: `src/cegar-fix/tests/test_hub_hierarchical_decomposer.rs`

**Interfaces:**
```rust
use std::collections::{HashMap, HashSet};
use crate::graph::Graph;

#[derive(Debug, Clone)]
pub struct HubModule {
    pub hub_id: i32,
    pub vertices: Vec<i32>,
    pub interface_ports: Vec<i32>,
    pub internal_paths: Vec<(i32, i32, Vec<i32>)>,
}

pub struct HubHierarchicalDecomposer;

impl HubHierarchicalDecomposer {
    pub fn extract_hub_modules(g: &Graph, min_hub_degree: usize) -> Vec<HubModule>;
    pub fn try_solve_hierarchical(g: &Graph) -> Option<Vec<i32>>;
}
```

- [ ] **Step 1: Write unit tests** in `src/cegar-fix/tests/test_hub_hierarchical_decomposer.rs`
- [ ] **Step 2: Run test to verify it fails** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_hub_hierarchical_decomposer`)
- [ ] **Step 3: Implement `HubHierarchicalDecomposer` in `src/cegar-fix/src/hub_hierarchical_decomposer.rs`**
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_hub_hierarchical_decomposer`)
- [ ] **Step 5: Commit changes**

---

### Task 2: Wire `HubHierarchicalDecomposer` Fast Track into `hcp_solver.rs`

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_staged_solver.rs`

- [ ] **Step 1: Wire `HubHierarchicalDecomposer::try_solve_hierarchical` into Round 0 in `hcp_solver.rs`**
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
