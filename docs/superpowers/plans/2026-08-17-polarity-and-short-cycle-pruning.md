# Polarity Phase Hints & Short-Cycle Pruning Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Global Short-Cycle Pruning (forbidding 3-cycles and 4-cycles in initial CNF) and Polarity Phase Hints (via RustSAT `PhaseLit` for aggregate cycles) to accelerate CaDiCaL CEGAR iterations from ~150s down to < 5s on dense hub graphs.

**Architecture:** Integrated directly into `src/cegar-fix/src/hcp_solver.rs` with `add_global_short_cycle_cuts` during CNF initialization and `solver.phase_lit(lit)` injection inside `cegar()`.

**Tech Stack:** Rust, CaDiCaL SAT Solver, RustSAT `PhaseLit` trait, Flinders Hamiltonian Cycle Project Challenge Set (FHCPCS).

## Global Constraints

- Must maintain 100% mathematical soundness across the entire FHCP benchmark (all 1001 graphs are Hamiltonian; never emit false `s UNSATISFIABLE`).
- Must strictly respect degree-2 contraction invariants: never sever contracted edges in `contractor.chain_map`.
- Zero regressions on all 10 Key Regression Graphs (`graph45`, `graph132`, `graph161`, `graph178`, `graph183`, `graph230`, `graph248`, `graph313`, `graph339`, `graph346`).
- Standard CLI invocation must remain unchanged: `-e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`.

---

### Task 1: Global Short-Cycle Pruning Implementation

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs:50-120`

**Interfaces:**
- Consumes: `Graph` from `crate::graph`, `Encoder` from `crate::encoder`, `Cnf` from `rustsat::instances::Cnf`.
- Produces:
  ```rust
  pub fn add_global_short_cycle_cuts(
      g: &Graph,
      encoder: &Encoder,
      cnf: &mut Cnf,
      max_cycle_len: usize,
  ) -> usize;
  ```

- [x] **Step 1: Implement `add_global_short_cycle_cuts` in `src/cegar-fix/src/hcp_solver.rs`**

Write:
- Triangle extraction: finds all triangles $(u, v, w)$ with $u < v < w$ and adds `(!x_uv | !x_vw | !x_wu)` and reverse `(!x_uw | !x_wv | !x_vu)`.
- 4-cycle extraction: finds all chordless 4-cycles $(u, v, w, z)$ and adds 4-cycle prohibition clauses.
- Integration: Call `add_global_short_cycle_cuts` when `loop_prohibition >= 1` or when `hub_registry.hub_vertices.len() >= 3`.
- Unit test: `test_short_cycle_pruning_triangles_and_quads`.

- [x] **Step 2: Run unit tests to verify module passes**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo test test_short_cycle`
Expected: PASS with unit tests passing.

- [x] **Step 3: Commit**

```bash
git add src/cegar-fix/src/hcp_solver.rs
git commit -m "feat: implement global short-cycle pruning for triangles and 4-cycles"
```

---

### Task 2: Polarity Phase Hints via RustSAT `PhaseLit`

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs:180-260`

**Interfaces:**
- Consumes: `PhaseLit` trait from `rustsat::solvers::PhaseLit`.
- Produces: Polarity phase hints injection inside the CEGAR loop before next `solver.solve()`.

- [x] **Step 1: Import `PhaseLit` and inject phase hints in `cegar()`**

In `src/cegar-fix/src/hcp_solver.rs`:
```rust
use rustsat::solvers::PhaseLit;
```
Inside `cegar()` right after subcycle merging:
```rust
// Inject positive polarity phase hints for edges in aggregate cycles
for cycle in &sol_cycles {
    if cycle.len() >= 10 {
        for i in 0..cycle.len() {
            let u = cycle[i];
            let v = cycle[(i + 1) % cycle.len()];
            if let Some(var_idx) = encoder.edge_to_var.get(&(u, v)).copied() {
                let lit = rustsat::types::Lit::positive(var_idx);
                let _ = solver.phase_lit(lit);
            }
        }
    }
}
```

- [x] **Step 2: Build release binary and run all unit tests**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo test && cargo build --release`
Expected: PASS with 34/34 unit tests passing and clean release build.

- [x] **Step 3: Commit**

```bash
git add src/cegar-fix/src/hcp_solver.rs
git commit -m "feat: implement polarity phase hints for CaDiCaL CEGAR acceleration"
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

- [x] **Step 2: Profile Dense Hub instances with Short-Cycle Pruning and Polarity Hints**

Run with 120s timeout:
- `./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph560.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`
- `./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph562.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`
- `./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph584.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`
Measure CEGAR iteration count and SAT solving time acceleration.

- [x] **Step 3: Commit verification report**

```bash
git add docs/superpowers/plans/2026-08-17-polarity-and-short-cycle-pruning.md
git commit -m "docs: record verification results for Polarity Phase Hints & Short-Cycle Pruning"
```
