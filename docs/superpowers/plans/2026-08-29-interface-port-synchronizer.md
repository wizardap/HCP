# Gadget Interface Port Truth Assignment & Flow Synchronizer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `InterfacePortSynchronizer` to extract internal Hamiltonian dual-paths ($T$ and $F$) for each gadget module, bind them to binary choice literals $x_k \in \{0, 1\}$, and inject interface port flow conservation at Round 0 to collapse the search space to a $K$-variable Boolean instance.

**Architecture:** Create `src/cegar-fix/src/interface_port_synchronizer.rs`, wire into `src/cegar-fix/src/hcp_solver.rs`.

**Tech Stack:** Rust (2021 edition), CaDiCaL SAT solver (`rustsat`, `rustsat_cadical`).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.
- Empirical Rigor: No overpromising. Maintain strict verification.

---

### Task 1: `InterfacePortSynchronizer` Engine

**Files:**
- Create: `src/cegar-fix/src/interface_port_synchronizer.rs`
- Modify: `src/cegar-fix/src/lib.rs`, `src/cegar-fix/src/main.rs` (export `pub mod interface_port_synchronizer;`)
- Test: `src/cegar-fix/tests/test_interface_port_synchronizer.rs`

**Interfaces:**
```rust
use std::collections::{HashMap, HashSet};
use rustsat::instances::Cnf;
use crate::graph::Graph;
use crate::encoder::Encoder;

#[derive(Debug, Clone)]
pub struct GadgetDualPath {
    pub module_id: usize,
    pub vertices: Vec<i32>,
    pub ports: [i32; 2],
    pub true_path_edges: Vec<(i32, i32)>,
    pub false_path_edges: Vec<(i32, i32)>,
}

pub struct InterfacePortSynchronizer;

impl InterfacePortSynchronizer {
    pub fn extract_gadget_dual_paths(g: &Graph, max_module_size: usize) -> Vec<GadgetDualPath>;
    pub fn encode_interface_port_synchronization(dual_paths: &[GadgetDualPath], encoder: &mut Encoder, cnf: &mut Cnf);
}
```

- [ ] **Step 1: Write unit tests** in `src/cegar-fix/tests/test_interface_port_synchronizer.rs`
- [ ] **Step 2: Run test to verify it fails** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_interface_port_synchronizer`)
- [ ] **Step 3: Implement `InterfacePortSynchronizer` in `src/cegar-fix/src/interface_port_synchronizer.rs`**
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_interface_port_synchronizer`)
- [ ] **Step 5: Commit changes**

---

### Task 2: Wire `InterfacePortSynchronizer` into Round 0 in `hcp_solver.rs`

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_staged_solver.rs`

- [ ] **Step 1: Wire `InterfacePortSynchronizer` into Round 0 in `hcp_solver.rs`**
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
