# Iterated Local Search (ILS) Patcher (Experiment 1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Experiment 1 (Iterated Local Search / Double-Bridge Perturbation Patcher) to break out of topological local minima in RAM and evaluate whether RAM-level kicks can solve dense hub graphs within seconds.

**Architecture:** A dedicated `ils_patcher.rs` module providing `IteratedLocalSearchPatcher` with double-bridge 4-opt kicks, randomized non-improving edge swaps, and iterative re-patching cascades, integrated into `hcp_solver::cegar`.

**Tech Stack:** Rust, CaDiCaL SAT Solver, Flinders Hamiltonian Cycle Project Challenge Set (FHCPCS).

## Global Constraints

- Must maintain 100% mathematical soundness across the entire FHCP benchmark (all 1001 graphs are Hamiltonian; never emit false `s UNSATISFIABLE`).
- Must strictly respect degree-2 contraction invariants: never sever contracted edges in `contractor.chain_map`.
- Zero regressions on all 10 Key Regression Graphs (`graph45`, `graph132`, `graph161`, `graph178`, `graph183`, `graph230`, `graph248`, `graph313`, `graph339`, `graph346`).
- Standard CLI invocation must remain unchanged: `-e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`.

---

### Task 1: IteratedLocalSearchPatcher Core Module

**Files:**
- Create: `src/cegar-fix/src/ils_patcher.rs`
- Modify: `src/cegar-fix/src/main.rs:1-15`

**Interfaces:**
- Consumes: `Graph` from `crate::graph`, `Degree2Contractor` from `crate::contraction`, `HubRegistry` from `crate::hub_registry`.
- Produces:
  ```rust
  pub struct IteratedLocalSearchPatcher;
  impl IteratedLocalSearchPatcher {
      pub fn solve_via_ils(
          cycles: &[Vec<i32>],
          g: &Graph,
          contractor: &Degree2Contractor,
          hub_registry: &HubRegistry,
          max_kicks: usize,
      ) -> Vec<Vec<i32>>;
  }
  ```

- [x] **Step 1: Declare `mod ils_patcher;` in `src/cegar-fix/src/main.rs`**

- [x] **Step 2: Implement `src/cegar-fix/src/ils_patcher.rs` with Double-Bridge Kick & ILS loop**

Write:
- `perturb_cycle`: Safe 4-opt double-bridge swap or random 2-opt perturbation on target cycle respecting `contractor.chain_map` and graph adjacency in $G$.
- `solve_via_ils`: Runs up to `max_kicks` iterations (e.g. 500), kicking the largest cycle, running the cascade of `HubPatcher`, `MatchingPatcher`, and `ChainedLKSolver`, updating best known cycle state, and returning when $k=1$ or no further progress.
- Unit tests: `test_ils_double_bridge_validity`, `test_ils_escapes_local_minimum`, `test_ils_degree2_safety`.

- [x] **Step 3: Run unit tests to verify module passes**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo test ils_patcher`
Expected: PASS with 3 unit tests passing.

- [x] **Step 4: Commit**

```bash
git add src/cegar-fix/src/ils_patcher.rs src/cegar-fix/src/main.rs
git commit -m "feat: implement IteratedLocalSearchPatcher module for RAM-level double-bridge perturbation"
```

---

### Task 2: Pipeline Integration in `src/cegar-fix/src/hcp_solver.rs`

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs:210-250`

**Interfaces:**
- Consumes: `IteratedLocalSearchPatcher::solve_via_ils` from `crate::ils_patcher`.
- Produces: ILS search pass in `cegar()` before traditional 2-opt/3-opt blocking clause generation.

- [x] **Step 1: Import `IteratedLocalSearchPatcher` and integrate in `cegar()`**

In `src/cegar-fix/src/hcp_solver.rs`:
```rust
use crate::ils_patcher::IteratedLocalSearchPatcher;
```
Right after `ChainedLKSolver` block in `cegar()`:
```rust
// Attempt Iterated Local Search (ILS) with Double-Bridge Kicks
let sol_cycles = if sol_cycles.len() > 1 {
    let patched = IteratedLocalSearchPatcher::solve_via_ils(&sol_cycles, &g, contractor, hub_registry, 200);
    if patched.len() == 1 && patched[0].len() == g.adjacency_list.len() {
        println!("number of subcycles found = 1 (via ils patching)");
        let flat: Vec<i32> = patched.into_iter().flatten().collect();
        let final_tour = contractor.uncontract_cycle(&flat);
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
    patched
} else {
    sol_cycles
};
```

- [x] **Step 2: Build release binary and run all unit tests**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo test && cargo build --release`
Expected: PASS with 26/26 unit tests passing and clean release build.

- [x] **Step 3: Commit**

```bash
git add src/cegar-fix/src/hcp_solver.rs
git commit -m "feat: integrate IteratedLocalSearchPatcher into CEGAR solver pipeline"
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

- [x] **Step 2: Profile Dense Hub instances with ILS Patcher**

Run with 60s timeout:
- `./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph560.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`
- `./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph562.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`
Record ILS kick stats, cycle convergence, and execution time.

- [x] **Step 3: Commit verification report**

```bash
git add docs/superpowers/plans/2026-08-16-ils-patcher.md
git commit -m "docs: record verification results for Iterated Local Search (ILS) Experiment 1"
```
