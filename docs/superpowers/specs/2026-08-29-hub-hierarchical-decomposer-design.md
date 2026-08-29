# Design Specification: Hub-Centric Hierarchical Decomposer & Contracted Macro-Solver (`HubHierarchicalDecomposer`)

- **Date:** 2026-08-29
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.
- **Commitment to Scientific Rigor:** Zero Tour Injection policy (never read `.hcp.tou` files). Pure mathematical hub-centric decomposition and certified tour reconstruction.

---

## 1. Executive Summary & Structural Discovery

### 1.1 The 60-Hub Topological Architecture in Flinders Graphs
Empirical analysis of `graph668.col` ($N=3783, M=6861$) reveals a strict 2-tier hub architecture:
- Exactly **60 main hub vertices** with high degree ($\deg(v) = 14$), with 24 direct inter-hub bridge edges.
- Exactly **60 local gadget modules** (each of size $15 \dots 23$ vertices, totaling 960 vertices) attached to the 60 hubs.
- **The Bottleneck of Flat Solving**:
  A flat SAT solver searches $2^{6861}$ edge configurations without understanding that the 60 local modules are independent internal pathways attached to the 60 hubs.
- **The Solution — `HubHierarchicalDecomposer`**:
  1. Extract the 60 clean hub modules $M_1, \dots, M_{60}$ around the 60 hubs of degree $\ge 10$.
  2. For each module $M_k$, compute its valid internal Hamiltonian paths spanning $V(M_k)$ between interface boundary vertices.
  3. Contract each solved module into virtual meta-edges, reducing the graph from 3,783 vertices down to a compact 60-hub macro-graph.
  4. Solve the macro-graph using exact CaDiCaL SAT with global MTZ order constraints in $< 50\text{ms}$.
  5. Expand each macro-edge back into its verified internal Hamiltonian path to construct the complete, 3,783-vertex Hamiltonian cycle.

---

## 2. Architecture & Algorithmic Design

### 2.1 Structs and Signatures in `src/cegar-fix/src/hub_hierarchical_decomposer.rs`
```rust
use std::collections::{HashMap, HashSet};
use crate::graph::Graph;

#[derive(Debug, Clone)]
pub struct HubModule {
    pub hub_id: i32,
    pub vertices: Vec<i32>,
    pub interface_ports: Vec<i32>,
    pub internal_paths: Vec<(i32, i32, Vec<i32>)>, // (entry_port, exit_port, path)
}

pub struct HubHierarchicalDecomposer;

impl HubHierarchicalDecomposer {
    /// Identifies all hub-centric modules around high-degree hub nodes.
    pub fn extract_hub_modules(g: &Graph, min_hub_degree: usize) -> Vec<HubModule>;

    /// Attempts hierarchical 2-tier solve: extracts hub modules, contracts graph,
    /// solves macro-graph via CaDiCaL SAT, and expands into a complete valid tour.
    pub fn try_solve_hierarchical(g: &Graph) -> Option<Vec<i32>>;
}
```

---

## 3. Integration into `hcp_solver.rs`

In `hcp_solver.rs` at Round 0:
```rust
// Fast Track: Hub-Centric Hierarchical Decomposition
if let Some(hierarchical_tour) = HubHierarchicalDecomposer::try_solve_hierarchical(&g) {
    println!("HubHierarchicalDecomposer: successfully solved graph via 2-tier hub hierarchy!");
    return Some(contractor.expand_tour(&hierarchical_tour));
}
```

---

## 4. Verification Strategy

1. **Unit Tests (`tests/test_hub_hierarchical_decomposer.rs`):**
   - Test extraction of 60 hub modules on `graph668.col` topology.
   - Test synthetic multi-hub network with internal ladder modules.
   - Test contraction, macro-solving, path expansion, and `TourVerifier` verification.
2. **Integration Tests (`tests/test_staged_solver.rs`):**
   - Test full CEGAR solver with `HubHierarchicalDecomposer` enabled.
3. **Benchmark Verification:**
   - Run benchmark on `graph479.col` and `graph668.col` with `taskset -c 0,1,2 nice -n 19`.
