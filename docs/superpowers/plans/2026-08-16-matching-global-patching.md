# Maximum Matching Global Patching (Phase 2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Phase 2 (Maximum Matching Global Patching) to build a Merge Compatibility Graph across non-hub subcycles and execute batch disjoint 2-opt merges simultaneously, accelerating cycle reduction and breaking out of sequential local minima.

**Architecture:** A dedicated `matching_patcher.rs` module providing `MatchingPatcher` with compatibility graph construction, maximum weight matching, and batch simultaneous merging, integrated into `hcp_solver::cegar` directly following `HubPatcher`.

**Tech Stack:** Rust, CaDiCaL SAT Solver, Flinders Hamiltonian Cycle Project Challenge Set (FHCPCS).

## Global Constraints

- Must maintain 100% mathematical soundness across the entire FHCP benchmark (all 1001 graphs are Hamiltonian; never emit false `s UNSATISFIABLE`).
- Must strictly respect degree-2 contraction invariants: never sever contracted edges in `contractor.chain_map`.
- Zero regressions on all 10 Key Regression Graphs (`graph45`, `graph132`, `graph161`, `graph178`, `graph183`, `graph230`, `graph248`, `graph313`, `graph339`, `graph346`).
- Standard CLI invocation must remain unchanged: `-e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`.

---

### Task 1: MatchingPatcher Core Module

**Files:**
- Create: `src/cegar-fix/src/matching_patcher.rs`
- Modify: `src/cegar-fix/src/main.rs:1-15`

**Interfaces:**
- Consumes: `Graph` from `crate::graph`, `Degree2Contractor` from `crate::contraction`, `HubRegistry` from `crate::hub_registry`.
- Produces:
  ```rust
  pub struct MatchingPatcher;
  impl MatchingPatcher {
      pub fn patch_cycles_via_matching(
          cycles: &[Vec<i32>],
          g: &Graph,
          contractor: &Degree2Contractor,
          hub_registry: &HubRegistry,
      ) -> Vec<Vec<i32>>;
      pub fn find_max_weight_matching(
          cycles: &[Vec<i32>],
          g: &Graph,
          contractor: &Degree2Contractor,
          hub_registry: &HubRegistry,
      ) -> Vec<(usize, usize, Vec<i32>)>;
  }
  ```

- [ ] **Step 1: Declare `mod matching_patcher;` in `src/cegar-fix/src/main.rs`**

- [ ] **Step 2: Implement `src/cegar-fix/src/matching_patcher.rs` with Compatibility Graph & Matching logic**

Write:
- `try_find_2opt_merge(c1, c2, g, contractor, hub_registry)`: Finds valid 2-opt reconnection between two subcycles honoring degree-2 chain invariants and edge adjacency in $G$.
- `find_max_weight_matching(cycles, g, contractor, hub_registry)`: Evaluates all pairs $(i, j)$ with $i < j$, sorts candidates by merge weight ($|C_i| + |C_j|$ and cross-edge density), and extracts a maximal disjoint matching $M$.
- `patch_cycles_via_matching(cycles, g, contractor, hub_registry)`: Executes batch merges for all $(i, j) \in M$, gathers unmatched cycles, and iterates until convergence.
- Unit tests: `test_matching_patcher_disjoint_pairs`, `test_matching_patcher_full_convergence`, `test_matching_patcher_degree2_safety`.

- [ ] **Step 3: Run unit tests to verify module passes**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo test matching_patcher`
Expected: PASS with 3 unit tests passing.

- [ ] **Step 4: Commit**

```bash
git add src/cegar-fix/src/matching_patcher.rs src/cegar-fix/src/main.rs
git commit -m "feat: implement MatchingPatcher module for batch disjoint cycle merging"
```

---

### Task 2: Pipeline Integration in `src/cegar-fix/src/hcp_solver.rs`

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs:145-180`

**Interfaces:**
- Consumes: `MatchingPatcher::patch_cycles_via_matching` from `crate::matching_patcher`.
- Produces: Secondary global cycle reduction in `cegar()` before traditional 2-opt/3-opt.

- [ ] **Step 1: Import `MatchingPatcher` and integrate in `cegar()`**

In `src/cegar-fix/src/hcp_solver.rs`:
```rust
use crate::matching_patcher::MatchingPatcher;
```
Right after `HubPatcher` in `cegar()`:
```rust
// Attempt Maximum Matching Global Patching on remaining subcycles
let sol_cycles = if sol_cycles.len() > 1 {
    let patched = MatchingPatcher::patch_cycles_via_matching(&sol_cycles, &g, contractor, hub_registry);
    if patched.len() == 1 && patched[0].len() == g.adjacency_list.len() {
        println!("number of subcycles found = 1 (via matching patching)");
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

- [ ] **Step 2: Build release binary and run all unit tests**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo test && cargo build --release`
Expected: PASS with 20/20 unit tests passing and clean release build.

- [ ] **Step 3: Commit**

```bash
git add src/cegar-fix/src/hcp_solver.rs
git commit -m "feat: integrate MatchingPatcher into CEGAR solver pipeline"
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

- [ ] **Step 2: Profile Dense Hub instances with Matching Patcher**

Run:
- `./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph560.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`
- `./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph562.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`
- `./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph584.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`
Record total subcycle reduction across both Hub & Matching patchers.

- [ ] **Step 3: Commit verification report**

```bash
git add docs/superpowers/plans/2026-08-16-matching-global-patching.md
git commit -m "docs: record verification results for Maximum Matching Global Patching Phase 2"
```
