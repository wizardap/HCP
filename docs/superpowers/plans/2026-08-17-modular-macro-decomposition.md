# Modular Macro-Decomposition Solver Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Modular Macro-Decomposition in `src/modular_solver.rs` to decompose Dense Hub graphs into 25 satellite modules of 125 vertices, solve localized Hamiltonian paths via Mini-SAT, contract into a ~55-node macro-graph, solve the macro-tour, and reconstruct the full 3,311-vertex Hamiltonian cycle in < 1 second.

**Architecture:** New module `src/modular_solver.rs` integrated as the top-level structural solver in `src/cegar-fix/src/hcp_solver.rs` prior to the CEGAR loop when $\ge 5$ dense hubs are detected.

**Tech Stack:** Rust, CaDiCaL/Mini-SAT SAT Solver, RustSAT, Flinders Hamiltonian Cycle Project Challenge Set (FHCPCS).

## Global Constraints

- Must maintain 100% mathematical soundness across the entire FHCP benchmark (all 1001 graphs are Hamiltonian; never emit false `s UNSATISFIABLE`).
- Must strictly respect degree-2 contraction invariants: never sever contracted edges in `contractor.chain_map`.
- Zero regressions on all 10 Key Regression Graphs (`graph45`, `graph132`, `graph161`, `graph178`, `graph183`, `graph230`, `graph248`, `graph313`, `graph339`, `graph346`).
- Standard CLI invocation must remain unchanged: `-e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`.

---

### Task 1: Implement `ModularSolver` Module

**Files:**
- Create: `src/cegar-fix/src/modular_solver.rs`
- Modify: `src/cegar-fix/src/main.rs:1-25`

**Interfaces:**
- Consumes: `Graph` from `crate::graph`, `Degree2Contractor` from `crate::contraction`, `HubRegistry` from `crate::hub_registry`.
- Produces:
  ```rust
  pub struct ModularSolver;
  impl ModularSolver {
      pub fn solve_via_modular_decomposition(
          g: &Graph,
          contractor: &Degree2Contractor,
          hub_registry: &HubRegistry,
      ) -> Option<Vec<i32>>;
      pub fn extract_satellite_modules(
          g: &Graph,
          hub_registry: &HubRegistry,
      ) -> Vec<SatelliteModule>;
      pub fn solve_module_hamiltonian_path(
          module: &SatelliteModule,
          g: &Graph,
          in_vertex: i32,
          out_vertex: i32,
      ) -> Option<Vec<i32>>;
  }
  ```

- [x] **Step 1: Declare `mod modular_solver;` in `src/cegar-fix/src/main.rs`**

Add `pub mod modular_solver;` to `src/cegar-fix/src/main.rs`.

- [x] **Step 2: Implement `src/cegar-fix/src/modular_solver.rs`**

Write:
- `SatelliteModule` struct containing `module_id`, `vertices: HashSet<i32>`, `internal_adj`, `hub_connections: HashMap<i32, Vec<i32>>`.
- `extract_satellite_modules`: BFS/DFS on induced subgraph $G[V \setminus \text{Hubs}]$, extracting components with $\ge 5$ vertices and their hub boundary edges.
- `solve_module_hamiltonian_path`: Mini-SAT formulation with directed arc variables, degree-2 (endpoints degree-1) constraints, and CEGAR subtour cuts to find directed $u_{in} \to u_{out}$ Hamiltonian path across all vertices in the module.
- `solve_via_modular_decomposition`: Contracts each module into macro-edges, builds macro-graph of size $\le 60$, solves macro-tour via Mini-SAT, uncontracts paths and degree-2 chains, verifies validity with `is_valid_cycle`.
- Unit tests: `test_satellite_module_extraction`, `test_module_hamiltonian_path_solving`, `test_modular_solver_end_to_end`.

- [x] **Step 3: Run unit tests to verify module passes**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo test modular_solver`
Expected: PASS with unit tests passing.

- [x] **Step 4: Commit**

```bash
git add src/cegar-fix/src/modular_solver.rs src/cegar-fix/src/main.rs
git commit -m "feat: implement ModularSolver module for dense hub module decomposition"
```

---

### Task 2: Pipeline Integration into CEGAR Solver

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs:160-200`

**Interfaces:**
- Consumes: `ModularSolver` from `crate::modular_solver`.
- Produces: Top-level structural decomposition check in `cegar()`.

- [x] **Step 1: Integrate `ModularSolver` into `cegar()`**

In `src/cegar-fix/src/hcp_solver.rs`:
```rust
use crate::modular_solver::ModularSolver;
```
Inside `cegar()`, right before the CEGAR loop:
```rust
// Attempt Modular Macro-Decomposition when dense hubs are detected
if hub_registry.hub_vertices.len() >= 5 {
    if let Some(tour) = ModularSolver::solve_via_modular_decomposition(&g, contractor, hub_registry) {
        println!("s SATISFIABLE (via Modular Macro-Decomposition)");
        println!("overall incremented number = 0");
        print_tour(&tour, contractor);
        return;
    }
}
```

- [x] **Step 2: Build release binary and run all unit tests**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo test && cargo build --release`
Expected: PASS with 36/36 unit tests passing and clean release build.

- [x] **Step 3: Commit**

```bash
git add src/cegar-fix/src/hcp_solver.rs
git commit -m "feat: integrate ModularSolver into CEGAR solver pipeline"
```

---

### Task 3: Regression Benchmark & Dense Hub Verification

**Files:**
- Test: FHCPCS benchmarks (`FHCPCS-col/*.col`)

- [x] **Step 1: Verify 10 Key Regression Graphs**

Run each of the 10 Key Regression graphs:
- `graph45`, `graph132`, `graph161`, `graph178`, `graph183`, `graph230`, `graph248`, `graph313`, `graph339`, `graph346`.
Command: `./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/<graph>.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`
Expected: 10/10 return `s SATISFIABLE`.

- [x] **Step 2: Profile Dense Hub instances**

Run with 120s timeout:
- `./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph560.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`
- `./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph562.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`
- `./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph584.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`
Measure solve time and verify rapid convergence.

- [x] **Step 3: Commit verification report**

```bash
git add docs/superpowers/plans/2026-08-17-modular-macro-decomposition.md
git commit -m "docs: record verification results for Modular Macro-Decomposition Solver"
```
