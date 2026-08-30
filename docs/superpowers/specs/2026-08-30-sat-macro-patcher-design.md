# Design Specification: Exact SAT Macro-Patching & Multi-Cycle Bridge Solver (`SatMacroPatcher`)

- **Date:** 2026-08-30
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.
- **Commitment to Scientific Rigor:** Zero Tour Injection policy (never read `.hcp.tou` files). Exact mathematical SAT encoding over macro-cycle spanning trees and edge replacements.

---

## 1. Executive Summary & Problem Context

### 1.1 Late-Round Plateau: Bounded Number of Remaining Cycles ($\le 25$ Cycles)
In deep CEGAR rounds on `graph668.col` (e.g. Round 15), the number of subcycles drops to **21 cycles**:
`{7: 1, 8: 3, 9: 3, 27: 1, 36: 1, 42: 1, 68: 1, 84: 1, 106: 1, 121: 1, 124: 1, 144: 1, 161: 1, 325: 2, 616: 1, 625: 1}`
- **The Bottleneck**:
  Greedy 2-opt and 3-opt heuristic stitchers test pairs or triplets sequentially.
  If merging requires a simultaneous coordinated swap of 4 or 5 bridges across a cluster of 5 cycles ($k$-opt), sequential greedy heuristics fail.
- **The Solution — `SatMacroPatcher`**:
  1. **Exact SAT Spanning Formulation over All Active Cycles**:
     - When `cycles.len() <= 30`:
     - Extract all valid 2-opt cross-bridges $B_{ij} = (e_i, e_j, x_1, x_2)$ between all pairs of cycles $(C_i, C_j)$.
     - Formulate an exact Spanning Tree SAT formula in CaDiCaL:
       * **Edge Exclusivity**: At most one bridge can cut any cycle edge $e \in C_i$.
       * **Degree & Spanning Tree Constraints**: Tree of $K$ cycles requires exactly $K-1$ chosen bridges.
       * **Topological Acyclicity**: MTZ rank variables $u_i \in [0, K-1]$ ensuring the chosen bridges form a single connected spanning tree with no cycles of cycles.
     - Solve in CaDiCaL in $< 5\text{ms}$.
     - Reconstruct the unified Hamiltonian tour from the simultaneous multi-bridge tree!
  2. **Direct Fast-Path in `GiantCycleStitcher` and `hcp_solver.rs`**:
     - If `SatMacroPatcher` finds a spanning tree, the tour is verified with `TourVerifier` and returned immediately as a full solution!

---

## 2. Architecture & Algorithmic Design

### 2.1 Structs and Methods in `src/cegar-fix/src/sat_macro_patcher.rs`
```rust
use crate::graph::Graph;
use std::collections::{HashMap, HashSet};

pub struct SatMacroPatcher;

impl SatMacroPatcher {
    /// Solves an exact SAT spanning tree formulation over all candidate 2-opt bridges between the cycles.
    /// Returns Some(single_hamiltonian_cycle) if a valid simultaneous bridge set exists, or None.
    pub fn try_patch_all_cycles(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
    ) -> Option<Vec<i32>>;
}
```

---

## 3. Integration into `giant_cycle_stitcher.rs` and `hcp_solver.rs`

1. **In `GiantCycleStitcher::repair_until_fixed_point`**:
   Add Step 10: When `current_cycles.len() <= 30`, invoke `SatMacroPatcher::try_patch_all_cycles`.
2. **In `hcp_solver.rs`**:
   Before cut selection, if `_active_cycles.len() <= 30`, call `SatMacroPatcher::try_patch_all_cycles` and if a single tour is found, uncontract and return `SATISFIABLE`.

---

## 4. Verification Strategy

1. **Unit Tests (`tests/test_sat_macro_patcher.rs`):**
   - Test simultaneous 4-cycle ring merge.
   - Test 10-cycle tree merge.
   - Test disjoint multi-cluster graphs.
2. **Integration Tests (`tests/test_staged_solver.rs`):**
   - Test end-to-end CEGAR solve with `SatMacroPatcher`.
3. **Benchmark Verification:**
   - Run benchmark on `graph479.col` and `graph668.col` with `taskset -c 0,1,2 nice -n 19`.
