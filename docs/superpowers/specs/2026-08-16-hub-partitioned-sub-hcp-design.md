# Design Document: Hub-Partitioned Sub-HCP (Divide-and-Conquer) Solver

**Document ID**: `2026-08-16-hub-partitioned-sub-hcp-design`  
**Target Systems**: `src/cegar-fix`  
**Status**: APPROVED DESIGN SPEC

---

## 1. Executive Summary & Objective

Dense Hub HCP graphs (`graph560` – `graph684`) feature 3,310+ vertices with 5 super-hubs ($deg \approx 662$) and 25 sub-hubs ($deg \approx 133$) connecting to ~3,100 satellite vertices. Monolithic SAT encodings over the entire 3,310-vertex graph generate 166,800+ clauses, taking 40–100+ seconds per CEGAR iteration.

This specification designs **Hub-Partitioned Sub-HCP (Divide-and-Conquer)** as a structured, mathematically sound solver for dense hub graphs. By partitioning the non-hub vertices into $K$ localized clusters ($|V_i| \approx 500-650$ vertices) anchored around the super-hubs, we solve localized Hamiltonian Paths in $< 0.1$s per cluster using Mini-SAT, then stitch the paths across the super-hubs into a single full Hamiltonian cycle in $< 5$s total.

---

## 2. Architectural Design

```
                     [ Input Graph G: 3,310 vertices ]
                     (5 Super-Hubs H₁..H₅, 3,100 satellites)
                                    │
                                    ▼
                 [ Step 1: Super-Hub Cluster Partitioning ]
                      Assign vertices to K clusters V₁..V_K
                      (|V_i| ≈ 500–650 vertices per cluster)
                                    │
                                    ▼
                [ Step 2: Localized Sub-HCP Path Solving ]
                      For each cluster V_i:
                      Find Hamiltonian Path P_i from in(V_i) to out(V_i)
                      via Mini-SAT (CaDiCaL in RAM, ~0.05s / cluster)
                                    │
                                    ▼
                 [ Step 3: Super-Hub Boundary Stitching ]
                      Connect: P₁ ──► H₁ ──► P₂ ──► H₂ ... ──► H_K ──► P₁
                      Validate full tour length and edge existence
                                    │
                                    ▼
                     [ Return Full Hamiltonian Cycle ]
```

---

## 3. Data Structures & Interfaces

In `src/cegar-fix/src/hub_sub_hcp.rs` (new module):

```rust
use crate::graph::Graph;
use crate::contraction::Degree2Contractor;
use crate::hub_registry::HubRegistry;

pub struct HubPartitionedSolver;

impl HubPartitionedSolver {
    /// Solves Dense Hub instances via divide-and-conquer cluster partitioning.
    /// Returns Some(full_tour) if successful, or None to fall back to CEGAR.
    pub fn solve_via_hub_partition(
        g: &Graph,
        contractor: &Degree2Contractor,
        hub_registry: &HubRegistry,
    ) -> Option<Vec<i32>>;

    /// Partitions satellite vertices into clusters anchored to super-hubs.
    fn partition_clusters(
        g: &Graph,
        hub_registry: &HubRegistry,
    ) -> Vec<HubCluster>;

    /// Solves a Hamiltonian Path on an induced cluster subgraph using Mini-SAT.
    fn solve_cluster_hamiltonian_path(
        cluster: &HubCluster,
        g: &Graph,
        in_vertex: i32,
        out_vertex: i32,
    ) -> Option<Vec<i32>>;
}

pub struct HubCluster {
    pub hub_id: i32,
    pub vertices: Vec<i32>,
    pub entry_candidates: Vec<i32>,
    pub exit_candidates: Vec<i32>,
}
```

---

## 4. Invariants & Safety Constraints

1. **Strict 100% Mathematical Soundness**:
   - The reconstructed global cycle is verified against `g.adjacency_list` with `is_valid_cycle(&tour, g)` and length $|V(G)|$.
   - Never emits false `s UNSATISFIABLE`.
2. **Degree-2 Path Invariant**:
   - When uncontracting via `contractor.uncontract_cycle`, all degree-2 chains remain valid.
3. **Clean Fallback**:
   - If any cluster path cannot be constructed within timeout (e.g. 2.0s), the solver returns `None`, allowing normal CEGAR execution without side effects.

---

## 5. Verification & Benchmark Plan

1. **Unit Tests (`hub_sub_hcp.rs`)**:
   - `test_hub_partition_clustering`: Verifies disjoint partitioning covering all non-hub vertices.
   - `test_hub_partition_synthetic_star_graph`: Tests synthetic multi-cluster hub graph solved in $< 50$ms.
   - `test_hub_partition_degree2_safety`: Verifies degree-2 contraction uncontracting.
2. **Regression Benchmark**:
   - Run 10 Key Regression Graphs (`graph45`, `graph132`, etc.) $\implies 10/10$ `s SATISFIABLE`.
3. **Dense Hub Profiling**:
   - Benchmark on `graph560.col`, `graph562.col`, `graph584.col` with target execution time $< 1800$s (aiming for $< 30$s).
