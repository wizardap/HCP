# Global Cycle Patching Framework Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Phase 1 (Multi-Subcycle Hub Patching) to rapidly splice satellite subcycles into primary cycles through super-hubs in $O(\sum |C_i|)$ time, resolving cycle fragmentation on dense hub graphs without violating 100% solver soundness.

**Architecture:** A standalone `patching.rs` module providing `HubPatcher` with Karp-style star-splicing algorithms, degree-2 contracted edge guards, and safe cycle validation, integrated directly before 2-opt/3-opt in `hcp_solver::cegar`.

**Tech Stack:** Rust, CaDiCaL SAT Solver, Flinders Hamiltonian Cycle Project Challenge Set (FHCPCS).

## Global Constraints

- Must maintain 100% mathematical soundness across the entire FHCP benchmark (all 1001 graphs are Hamiltonian; never emit `s UNSATISFIABLE`).
- Must strictly respect degree-2 contraction invariants: never sever contracted edges in `contractor.chain_map`.
- Zero regressions on all 10 Key Regression Graphs (`graph45`, `graph132`, `graph161`, `graph178`, `graph183`, `graph230`, `graph248`, `graph313`, `graph339`, `graph346`).
- Standard CLI invocation must remain unchanged: `-e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`.

---

### Task 1: HubPatcher Core Module

**Files:**
- Create: `src/cegar-fix/src/patching.rs`
- Modify: `src/cegar-fix/src/main.rs:1-15`

**Interfaces:**
- Consumes: `Graph` from `crate::graph`, `Degree2Contractor` from `crate::contraction`, `HubRegistry` from `crate::hub_registry`.
- Produces:
  ```rust
  pub struct HubPatcher;
  impl HubPatcher {
      pub fn patch_cycles_via_hubs(
          cycles: &[Vec<i32>],
          g: &Graph,
          contractor: &Degree2Contractor,
          hub_registry: &HubRegistry,
      ) -> Vec<Vec<i32>>;
      pub fn try_splice_subcycle_at_hub(
          main_cycle: &mut Vec<i32>,
          satellite_cycle: &[i32],
          hub: i32,
          g: &Graph,
          contractor: &Degree2Contractor,
      ) -> bool;
  }
  ```

- [x] **Step 1: Declare `mod patching;` in `src/cegar-fix/src/main.rs`**

```rust
mod contraction;
mod encoder;
mod graph;
mod hcp_solver;
mod hub_registry;
mod parallel_sub_hcp;
mod patching;
```

- [x] **Step 2: Implement `src/cegar-fix/src/patching.rs` with Hub Splicing logic and Unit Tests**

Write the complete implementation:
- `is_safe_to_break(u, v, contractor)` helper.
- `try_splice_subcycle_at_hub`: checks both Orientations across both adjacent sides of `hub` in `main_cycle`.
- `patch_cycles_via_hubs`: selects the longest cycle as `main_cycle`, identifies all incident hubs, and sequentially splices all candidate satellite cycles.
- Unit tests: `test_hub_patcher_single_splice`, `test_hub_patcher_multi_satellite`, `test_hub_patcher_degree2_guard`.

- [x] **Step 3: Run unit tests to verify module passes**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo test patching`
Expected: PASS with 3 unit tests passing.

- [x] **Step 4: Commit**

```bash
git add src/cegar-fix/src/patching.rs src/cegar-fix/src/main.rs
git commit -m "feat: implement HubPatcher module for star-topology cycle splicing"
```

---

### Task 2: Pipeline Integration in `src/cegar-fix/src/hcp_solver.rs`

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs:130-170`

**Interfaces:**
- Consumes: `HubPatcher::patch_cycles_via_hubs` from `crate::patching`.
- Produces: Seamless preprocessing of `sol_cycles` in `cegar()` before 2-opt/3-opt.

- [x] **Step 1: Import `HubPatcher` and integrate in `cegar()`**

In `src/cegar-fix/src/hcp_solver.rs`:
```rust
use crate::patching::HubPatcher;
```
Inside `cegar()` right after finding `sol_cycles`:
```rust
// Attempt Multi-Subcycle Hub Patching
let sol_cycles = if sol_cycles.len() > 1 && !hub_registry.hub_vertices.is_empty() {
    let patched = HubPatcher::patch_cycles_via_hubs(&sol_cycles, &g, contractor, hub_registry);
    if patched.len() == 1 && patched[0].len() == g.adjacency_list.len() {
        println!("number of subcycles found = 1 (via hub patching)");
        let final_tour = contractor.uncontract_cycle(&patched[0]);
        println!("solution: ");
        for node in &final_tour {
            print!("{} ", node);
        }
        println!("\n\ns SATISFIABLE");
        println!("overall incremented number = {}", incremented_number);
        let solving_time = now.elapsed();
        println!("solving time = {}.{:09}s", solving_time.as_secs(), solving_time.subsec_nanos());
        return;
    }
    patched
} else {
    sol_cycles
};
```

- [x] **Step 2: Build release binary and run all unit tests**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo test && cargo build --release`
Expected: PASS with all unit tests passing and clean release build.

- [x] **Step 3: Commit**

```bash
git add src/cegar-fix/src/hcp_solver.rs
git commit -m "feat: integrate HubPatcher into CEGAR solver pipeline"
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

Run:
- `./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph560.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`
- `./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph562.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`
- `./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph584.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`
Record execution time and patched cycle stats.

- [x] **Step 3: Commit verification report**

```bash
git add docs/superpowers/plans/2026-08-16-global-cycle-patching.md
git commit -m "docs: record verification results for Global Cycle Patching Phase 1"
```
