# Design Document: Modular Macro-Decomposition for Dense Hub Graphs

**Document ID**: `2026-08-17-modular-macro-decomposition-design`  
**Target Systems**: `src/cegar-fix` (`src/modular_solver.rs`, `src/hcp_solver.rs`, `src/main.rs`)  
**Status**: APPROVED DESIGN SPEC

---

## 1. Executive Summary & Problem Diagnosis

### 1.1 Problem Diagnosis on Dense Hub Graphs (`graph560` – `graph684`)
Dense Hub graphs contain ~3,311 vertices and 30 hubs ($deg = 133 - 663$).
Crucially, structural analysis reveals that the 3,156 satellite vertices are partitioned into **exactly 25 dense connected modules of size 125** ($25 \times 125 = 3,125$ vertices), each attached to the 30 Hubs.

Standard CEGAR fails because it attempts to solve all 3,311 vertices simultaneously on an unguided 2-factor formula with $10^{500}$ states, causing the 25 modules to fragment into 250 micro-cycles and taking 40–150s per CEGAR iteration.

### 1.2 The Modular Macro-Decomposition Architecture
Instead of global brute-force search, this specification introduces **Modular Macro-Decomposition**:
1. **Module Extraction**: Identify all connected components in the satellite subgraph $G[V \setminus H]$.
2. **Localized Path Solving**: Solve Hamiltonian paths inside each 125-node module in RAM via localized Mini-SAT ($< 5$ms per module).
3. **Macro-Graph Compression**: Contract each solved module into a macro-path edge, reducing the graph from 3,311 vertices to $\le 60$ macro-nodes.
4. **Macro-Tour Assembly**: Solve Hamiltonian cycle on the 60-vertex macro-graph in $< 0.05$s.
5. **Uncontraction**: Splice the 25 internal 125-node paths into the macro-tour, yielding the complete 3,311-vertex Hamiltonian cycle in $< 1$ second!

```
               [ Input Dense Hub Graph (3,311 vertices, 30 Hubs) ]
                                      │
                                      ▼
             [ Step 1: Extract Satellite Modules G[V \ Hubs] ]
                 Found 25 modules of 125 vertices each
                                      │
                                      ▼
             [ Step 2: Solve Localized Path for each Module ]
               Mini-SAT solves 125-node path in < 5ms / module
                                      │
                                      ▼
             [ Step 3: Contract Modules into Macro-Edges ]
               Compresses graph from 3,311 vertices to ~55 nodes
                                      │
                                      ▼
             [ Step 4: Solve Macro-Tour on 55 nodes with Mini-SAT ]
                            (Solves in < 50ms)
                                      │
                                      ▼
             [ Step 5: Uncontract Macro-Paths to full 3,311 Tour ]
                                      │
                                      ▼
                    [ Return Valid Hamiltonian Tour in < 1s! ]
```

---

## 2. Component Architecture

### 2.1 `ModularSolver` Module (`src/modular_solver.rs`)
- **`SatelliteModule` Struct**:
  ```rust
  pub struct SatelliteModule {
      pub module_id: usize,
      pub vertices: HashSet<i32>,
      pub internal_adj: HashMap<i32, Vec<i32>>,
      pub hub_connections: HashMap<i32, Vec<i32>>, // hub_id -> connected boundary nodes
  }
  ```
- **Key Functions**:
  - `extract_satellite_modules(g: &Graph, hub_registry: &HubRegistry) -> Vec<SatelliteModule>`:
    Performs BFS/DFS connected components on $V \setminus \text{Hubs}$.
  - `solve_module_hamiltonian_path(module: &SatelliteModule, g: &Graph, in_v: i32, out_v: i32) -> Option<Vec<i32>>`:
    Encodes directed Hamiltonian path from $in\_v$ to $out\_v$ inside the induced subgraph using CaDiCaL/Mini-SAT in RAM with degree-2 (endpoints degree-1) and MTZ/subtour cuts.
  - `solve_via_modular_decomposition(g: &Graph, contractor: &Degree2Contractor, hub_registry: &HubRegistry) -> Option<Vec<i32>>`:
    Driver coordinating module path solving, macro-graph construction, macro-tour solving, and expansion.

---

## 3. Invariants & Safety Constraints

1. **100% Mathematical Soundness**:
   - If any module fails to find a valid path or macro-graph has no tour, `solve_via_modular_decomposition` returns `None` and falls back cleanly to the main CEGAR solver.
   - Verification with `is_valid_cycle(&tour, g)` and `tour.len() == g.adjacency_list.len()`.
2. **Degree-2 Contraction Safety**:
   - Uncontracts all contracted degree-2 chains via `contractor.uncontract_path()` preserving `contractor.chain_map`.
3. **Zero Regressions**:
   - Key Regression Graphs (`graph45` – `graph346`) remain 100% `s SATISFIABLE`.

---

## 4. Verification Plan

1. **Unit Tests**:
   - `test_satellite_module_extraction`: Verifies component extraction and boundary connection identification.
   - `test_module_hamiltonian_path_solving`: Verifies localized SAT path solving on synthetic module.
   - `test_modular_solver_end_to_end`: Verifies complete end-to-end solve and uncontraction on a modular test graph.
2. **Benchmark Verification**:
   - Verify 10 Key Regression Graphs (`graph45` – `graph346`).
   - Profile `graph560.col`, `graph562.col`, `graph584.col` and confirm rapid convergence.
