# Macro-Graph Hierarchical Contraction Solver (Experiment 2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Experiment 2 (Macro-Graph Hierarchical Contraction & Mini-SAT Solver) to contract remaining subcycles into a compact ~80-node macro-graph, solve the macro-tour with Mini-SAT in milliseconds, and expand it into a full Hamiltonian cycle on dense hub graphs.

**Architecture:** A dedicated `macro_solver.rs` module providing `MacroGraphSolver` with macro-graph extraction, cross-connector port mapping, auxiliary CaDiCaL Mini-SAT solving with subtour elimination, and multi-cycle tour expansion, integrated into `hcp_solver::cegar`.

**Tech Stack:** Rust, CaDiCaL SAT Solver, Flinders Hamiltonian Cycle Project Challenge Set (FHCPCS).

## Global Constraints

- Must maintain 100% mathematical soundness across the entire FHCP benchmark (all 1001 graphs are Hamiltonian; never emit false `s UNSATISFIABLE`).
- Must strictly respect degree-2 contraction invariants: never sever contracted edges in `contractor.chain_map`.
- Zero regressions on all 10 Key Regression Graphs (`graph45`, `graph132`, `graph161`, `graph178`, `graph183`, `graph230`, `graph248`, `graph313`, `graph339`, `graph346`).
- Standard CLI invocation must remain unchanged: `-e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`.

---

### Task 1: MacroGraphSolver Core Module

**Files:**
- Create: `src/cegar-fix/src/macro_solver.rs`
- Modify: `src/cegar-fix/src/main.rs:1-15`

**Interfaces:**
- Consumes: `Graph` from `crate::graph`, `Degree2Contractor` from `crate::contraction`, `HubRegistry` from `crate::hub_registry`.
- Produces:
  ```rust
  pub struct MacroGraphSolver;
  impl MacroGraphSolver {
      pub fn solve_via_macro_graph(
          cycles: &[Vec<i32>],
          g: &Graph,
          contractor: &Degree2Contractor,
          hub_registry: &HubRegistry,
      ) -> Option<Vec<i32>>;
  }
  ```

- [ ] **Step 1: Declare `mod macro_solver;` in `src/cegar-fix/src/main.rs`**

- [ ] **Step 2: Implement `src/cegar-fix/src/macro_solver.rs` with Macro-Graph Extraction & Mini-SAT Solving**

Write:
- `build_macro_graph`: Extracts all cross-edges between distinct subcycles $C_i$ and $C_j$, checking degree-2 break safety via `is_safe_to_break`.
- `solve_macro_sat`: Encodes degree-2 and MTZ subtour elimination constraints on the macro-graph using CaDiCaL, extracts the macro-tour sequence, and splices each subcycle along the selected entry/exit ports into a single valid tour.
- Unit tests: `test_macro_graph_construction`, `test_macro_solver_synthetic_grid`, `test_macro_solver_degree2_safety`.

- [ ] **Step 3: Run unit tests to verify module passes**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo test macro_solver`
Expected: PASS with 3 unit tests passing.

- [ ] **Step 4: Commit**

```bash
git add src/cegar-fix/src/macro_solver.rs src/cegar-fix/src/main.rs
git commit -m "feat: implement MacroGraphSolver module for mini-SAT macro-tour expansion"
```

---

### Task 2: Pipeline Integration in `src/cegar-fix/src/hcp_solver.rs`

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs:230-270`

**Interfaces:**
- Consumes: `MacroGraphSolver::solve_via_macro_graph` from `crate::macro_solver`.
- Produces: Macro-graph solving pass in `cegar()` before traditional 2-opt/3-opt blocking clause generation.

- [ ] **Step 1: Import `MacroGraphSolver` and integrate in `cegar()`**

In `src/cegar-fix/src/hcp_solver.rs`:
```rust
use crate::macro_solver::MacroGraphSolver;
```
Right after `IteratedLocalSearchPatcher` in `cegar()`:
```rust
// Attempt Macro-Graph Hierarchical Contraction Solver
if sol_cycles.len() > 1 {
    if let Some(macro_tour) = MacroGraphSolver::solve_via_macro_graph(&sol_cycles, &g, contractor, hub_registry) {
        if macro_tour.len() == g.adjacency_list.len() {
            println!("number of subcycles found = 1 (via macro-graph solver)");
            let final_tour = contractor.uncontract_cycle(&macro_tour);
            let line = final_tour.iter().map(|i| i.to_string()).collect::<Vec<String>>().join(" ");
            let time = now - previous_time;
            let add_block_clauses_time = now - previous_time - sat_solving_time;
            println!("number of added block clauses = {}", clause_count);
            println!("add block clauses time = {:?}", add_block_clauses_time);
            println!("increment time = {:?}", time);
            println!();
            println!("solution: ");
            println!("{}\n", line);
            println!("s SATISFIABLE");
            return (count, clause_count);
        }
    }
}
```

- [ ] **Step 2: Build release binary and run all unit tests**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo test && cargo build --release`
Expected: PASS with 29/29 unit tests passing and clean release build.

- [ ] **Step 3: Commit**

```bash
git add src/cegar-fix/src/hcp_solver.rs
git commit -m "feat: integrate MacroGraphSolver into CEGAR solver pipeline"
```

---

### Task 3: Regression Benchmark & Dense Hub Verification

**Files:**
- Test: FHCPCS benchmarks (`FHCPCS-col/*.col`)

- [ ] **Step 1: Verify 10 Key Regression Graphs**

Run each of the 10 Key Regression graphs:
- `graph45`, `graph132`, `graph161`, `graph178`, `graph183`, `graph230`, `graph248`, `graph313`, `graph339`, `graph346`.
Command: `./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/<graph>.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`
Expected: 10/10 return `s SATISFIABLE`.

- [ ] **Step 2: Profile Dense Hub instances with Macro-Graph Solver**

Run with 120s timeout:
- `./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph560.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`
- `./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph562.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`
- `./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph584.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`
Measure Macro-Graph node count, Mini-SAT solving time, and convergence outcome.

- [ ] **Step 3: Commit verification report**

```bash
git add docs/superpowers/plans/2026-08-16-macro-solver.md
git commit -m "docs: record verification results for Macro-Graph Solver (Experiment 2)"
```
