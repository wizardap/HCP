# Design Document: Macro-Graph Hierarchical Contraction Solver (Experiment 2)

**Document ID**: `2026-08-16-macro-solver-design`  
**Target Systems**: `src/cegar-fix`  
**Status**: APPROVED DESIGN SPEC (Experiment 2 of 3)

---

## 1. Executive Summary & Objective

In Dense Hub graphs (`graph560` – `graph684`), the initial SAT solve produces ~250 subcycles, which fast RAM patchers reduce to ~80–99 subcycles. However, pairwise/triplet local search cannot bridge the remaining subcycles because their interconnections form complex multi-hop topologies across the entire graph.

This specification designs **Macro-Graph Hierarchical Contraction** as an **independent optimization technique** (Experiment 2). By contracting all $k$ subcycles into a compact Macro-Graph $\mathcal{M} = (V_{\mathcal{M}}, E_{\mathcal{M}})$ where $|V_{\mathcal{M}}| \approx 80$, this method formulates and solves a mini-Hamiltonian macro-tour problem using an auxiliary CaDiCaL SAT instance in $< 0.1$ seconds, then expands the macro-tour back into a single 100% valid Hamiltonian cycle spanning all 3,311 vertices.

---

## 2. Architectural Design

```
             [ Set of k subcycles: C₀, C₁, ..., C_{k-1} ]
                                   │
                                   ▼
              [ Build Macro-Graph ℳ = (V_ℳ, E_ℳ) ]
                 - |V_ℳ| = k (e.g. ~80 vertices)
                 - E_ℳ: cross-edges between subcycles in G
                                   │
                                   ▼
        [ Formulate & Solve Mini-SAT on ℳ (CaDiCaL in RAM) ]
                 - Degree-2 constraints on macro-nodes
                 - MTZ / SECs to enforce single connected tour
                 - Solving time: < 0.05s
                                   │
                                   ▼
           [ Expand Macro-Tour into Single Hamiltonian Cycle ]
                 - Splice each cycle C_i along entry/exit ports
                 - Check degree-2 contraction safety invariants
                 - Verify is_valid_cycle(&tour, g)
                                   │
                                   ▼
                     [ Return Full Tour Solution ]
```

---

## 3. Data Structures & Interfaces

In `src/cegar-fix/src/macro_solver.rs` (new module):

```rust
use crate::graph::Graph;
use crate::contraction::Degree2Contractor;
use crate::hub_registry::HubRegistry;

pub struct MacroGraphSolver;

impl MacroGraphSolver {
    /// Attempts to solve and merge all subcycles into a single Hamiltonian tour
    /// using Macro-Graph Hierarchical Contraction and Mini-SAT.
    /// Returns Some(full_tour) if successful, or None to fall back cleanly.
    pub fn solve_via_macro_graph(
        cycles: &[Vec<i32>],
        g: &Graph,
        contractor: &Degree2Contractor,
        hub_registry: &HubRegistry,
    ) -> Option<Vec<i32>>;

    /// Constructs the macro-graph adjacency and cross-connector map.
    fn build_macro_graph(
        cycles: &[Vec<i32>],
        g: &Graph,
        contractor: &Degree2Contractor,
    ) -> MacroGraph;

    /// Solves the macro-graph cycle cover with Mini-SAT.
    fn solve_macro_sat(
        macro_graph: &MacroGraph,
    ) -> Option<Vec<(usize, usize, i32, i32)>>;
}

pub struct MacroGraph {
    pub num_macro_nodes: usize,
    pub macro_adj: Vec<Vec<usize>>,
    pub connectors: std::collections::HashMap<(usize, usize), Vec<(i32, i32)>>,
}
```

---

## 4. Invariants & Safety Constraints

1. **Strict 100% Mathematical Soundness**:
   - `MacroGraphSolver` constructs a valid candidate Hamiltonian tour and verifies every edge against `g.adjacency_list` before returning `Some(tour)`.
   - Never outputs false `s UNSATISFIABLE`.
2. **Degree-2 Path Invariant**:
   - Splicing entry and exit ports must never sever contracted paths in `contractor.chain_map`.
3. **Clean Fallback**:
   - If the macro-graph cannot find a valid simultaneous splice in Mini-SAT, it returns `None`, allowing normal CEGAR execution without side effects.

---

## 5. Verification & Benchmark Plan

1. **Unit Tests (`macro_solver.rs`)**:
   - `test_macro_graph_construction`: Verifies macro-node and connector extraction.
   - `test_macro_solver_synthetic_grid`: Tests synthetic 6-subcycle grid graph solved to 1 full Hamiltonian cycle via Mini-SAT in milliseconds.
   - `test_macro_solver_degree2_safety`: Verifies preservation of contracted degree-2 chains.
2. **Regression Benchmark**:
   - Run 10 Key Regression Graphs (`graph45`, `graph132`, etc.) $\implies 10/10$ `s SATISFIABLE`.
3. **Dense Hub Profiling**:
   - Benchmark on `graph560.col`, `graph562.col`, `graph584.col` and measure whether Mini-SAT solves the 80-node macro-graph in $< 1$s.
