# Hub-Partitioned Sub-HCP (Divide-and-Conquer) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Hub-Partitioned Sub-HCP (Divide-and-Conquer) to decompose dense hub graphs into $K$ localized ~500-vertex clusters, solve localized Hamiltonian paths in $< 0.1$s per cluster via Mini-SAT, and stitch the paths through super-hubs into a single full Hamiltonian cycle in $< 1800$s (targeting $< 30$s).

**Architecture:** A dedicated `hub_sub_hcp.rs` module providing `HubPartitionedSolver` with cluster partitioning, Mini-SAT cluster path solving, and super-hub cycle stitching, integrated into `hcp_solver::cegar` / `solve_hamilton`.

**Tech Stack:** Rust, CaDiCaL SAT Solver, Flinders Hamiltonian Cycle Project Challenge Set (FHCPCS).

## Global Constraints

- Must maintain 100% mathematical soundness across the entire FHCP benchmark (all 1001 graphs are Hamiltonian; never emit false `s UNSATISFIABLE`).
- Must strictly respect degree-2 contraction invariants: never sever contracted edges in `contractor.chain_map`.
- Zero regressions on all 10 Key Regression Graphs (`graph45`, `graph132`, `graph161`, `graph178`, `graph183`, `graph230`, `graph248`, `graph313`, `graph339`, `graph346`).
- Standard CLI invocation must remain unchanged: `-e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`.

---

### Task 1: HubPartitionedSolver Core Module

**Files:**
- Create: `src/cegar-fix/src/hub_sub_hcp.rs`
- Modify: `src/cegar-fix/src/main.rs:1-15`

**Interfaces:**
- Consumes: `Graph` from `crate::graph`, `Degree2Contractor` from `crate::contraction`, `HubRegistry` from `crate::hub_registry`.
- Produces:
  ```rust
  pub struct HubPartitionedSolver;
  impl HubPartitionedSolver {
      pub fn solve_via_hub_partition(
          g: &Graph,
          contractor: &Degree2Contractor,
          hub_registry: &HubRegistry,
      ) -> Option<Vec<i32>>;
  }
  ```

- [x] **Step 1: Declare `mod hub_sub_hcp;` in `src/cegar-fix/src/main.rs`**

- [x] **Step 2: Implement `src/cegar-fix/src/hub_sub_hcp.rs` with Cluster Partitioning & Mini-SAT Path Solving**

Write:
- `partition_clusters`: Partitions non-hub vertices into $K$ disjoint clusters assigned to super-hubs based on graph adjacency and hop distance.
- `solve_cluster_hamiltonian_path`: Encodes Hamiltonian path from $u_{in}$ to $u_{out}$ spanning all vertices in cluster $V_i$ using CaDiCaL in RAM with subtour elimination cuts.
- `solve_via_hub_partition`: Solves paths across all $K$ clusters, stitches them through super-hubs $P_1 \to H_1 \to P_2 \to H_2 \dots \to P_K \to H_K \to P_1$, and verifies with `is_valid_cycle`.
- Unit tests: `test_hub_partition_clustering`, `test_hub_partition_synthetic_star_graph`, `test_hub_partition_degree2_safety`.

- [x] **Step 3: Run unit tests to verify module passes**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo test hub_sub_hcp`
Expected: PASS with 3 unit tests passing.

- [x] **Step 4: Commit**

```bash
git add src/cegar-fix/src/hub_sub_hcp.rs src/cegar-fix/src/main.rs
git commit -m "feat: implement HubPartitionedSolver module for divide-and-conquer cluster solving"
```

---

### Task 2: Pipeline Integration in `src/cegar-fix/src/hcp_solver.rs`

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs:50-90`

**Interfaces:**
- Consumes: `HubPartitionedSolver::solve_via_hub_partition` from `crate::hub_sub_hcp`.
- Produces: Fast divide-and-conquer pre-pass for Dense Hub graphs before entering full-graph CEGAR.

- [x] **Step 1: Import `HubPartitionedSolver` and integrate in `cegar()`**

In `src/cegar-fix/src/hcp_solver.rs`:
```rust
use crate::hub_sub_hcp::HubPartitionedSolver;
```
At the start of `cegar()` (if graph has super-hubs):
```rust
// Attempt Hub-Partitioned Divide-and-Conquer for Dense Hub Graphs
if !hub_registry.hub_vertices.is_empty() && hub_registry.hub_vertices.len() >= 3 {
    if let Some(partition_tour) = HubPartitionedSolver::solve_via_hub_partition(&g, contractor, hub_registry) {
        if partition_tour.len() == g.adjacency_list.len() {
            println!("number of subcycles found = 1 (via hub-partitioned sub-hcp)");
            let final_tour = contractor.uncontract_cycle(&partition_tour);
            let line = final_tour.iter().map(|i| i.to_string()).collect::<Vec<String>>().join(" ");
            let time = now.elapsed();
            println!("overall time = {:?}", time);
            println!();
            println!("solution: ");
            println!("{}\n", line);
            println!("s SATISFIABLE");
            return (0, 0);
        }
    }
}
```

- [x] **Step 2: Build release binary and run all unit tests**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo test && cargo build --release`
Expected: PASS with 32/32 unit tests passing and clean release build.

- [x] **Step 3: Commit**

```bash
git add src/cegar-fix/src/hcp_solver.rs
git commit -m "feat: integrate HubPartitionedSolver into CEGAR solver pipeline"
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

- [x] **Step 2: Profile Dense Hub instances with Hub-Partitioned Solver**

Run with 120s timeout:
- `./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph560.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`
- `./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph562.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`
- `./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph584.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`
Measure solving time and verify if dense hub instances solve within seconds.

- [x] **Step 3: Commit verification report**

```bash
git add docs/superpowers/plans/2026-08-16-hub-sub-hcp.md

git commit -m "docs: record verification results for Hub-Partitioned Sub-HCP Solver"
```
