# Design Specification: Max-Vertex SAT Spanning Tree Splicer (`MaxVertexSatSplicer`)

- **Date:** 2026-08-30
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.
- **Commitment to Scientific Rigor:** Zero Tour Injection policy (never read `.hcp.tou` files). Exact mathematical SAT encoding over sub-arborescences and simultaneous multi-cycle merges.

---

## 1. Executive Summary & Problem Context

### 1.1 All-or-Nothing vs Partial Spanning Forest in Deep CEGAR
In deep CEGAR rounds (e.g. Round 13 on `graph668.col`), 31 subcycles exist:
- 1 giant cycle: 1,595 vertices ($55.7\%$ of graph)
- 1 macro cycle: 340 vertices ($11.9\%$ of graph)
- Medium cycles: 136, 112, 110, 90, 78, 72, 42, 37, 33 ($\sim 28\%$ of graph)
- Small satellite subcycles: $8 \times 11, 16 \times 6$
- **The Bottleneck**:
  An all-or-nothing spanning tree of all 31 cycles requires every single 8-cycle to have a bridge. If even one 8-cycle is isolated in the current 2-factor, full spanning tree SAT is UNSAT.
  However, a spanning tree connecting the giant cycle, the 340-cycle, and all medium cycles covers **$> 2,500$ vertices ($> 87\%$ of the graph)**!
- **The Solution — `MaxVertexSatSplicer`**:
  1. **Threshold Increase**: Support $\le 60$ cycles in SAT macro-patching.
  2. **Max-Subtree / Component Splicing**:
     - Extract all valid bridges between all cycles.
     - For each connected component in the bridge graph that contains $\ge 2$ cycles:
       * Formulate an exact spanning tree SAT problem for that component.
       * Solve in CaDiCaL in $< 5\text{ms}$.
       * Splicing merges all cycles in that component into a single large cycle.
     - Automatically absorbs the 340-cycle and all medium cycles into the giant cycle, leaving only a few isolated 8-cycles for the next quick CEGAR increment.

---

## 2. Architecture & Algorithmic Design

### 2.1 Structs and Methods in `src/cegar-fix/src/sat_macro_patcher.rs`
```rust
use crate::graph::Graph;
use std::collections::{HashMap, HashSet};

pub struct SatMacroPatcher;

impl SatMacroPatcher {
    /// Attempts to merge cycles in every connected component of the 2-opt bridge graph.
    /// Returns the consolidated list of cycles with reduced cycle count if any component was merged.
    pub fn try_patch_components(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
    ) -> Vec<Vec<i32>>;
}
```

---

## 3. Integration into `giant_cycle_stitcher.rs` and `hcp_solver.rs`

1. **In `GiantCycleStitcher::repair_until_fixed_point`**:
   Step 10: When `current_cycles.len() <= 60`, invoke `SatMacroPatcher::try_patch_components`.
2. **In `hcp_solver.rs`**:
   Before cut selection, if `_active_cycles.len() <= 60`, attempt `try_patch_components`.

---

## 4. Verification Strategy

1. **Unit Tests (`tests/test_sat_macro_patcher.rs`):**
   - Test partial component merge (4 cycles in component A, 3 cycles in component B $\to$ merges into 2 cycles).
   - Test full tree merge ($\le 60$ cycles).
2. **Integration Tests (`tests/test_staged_solver.rs`):**
   - Test CEGAR loop integration with partial component merges.
3. **Benchmark Verification:**
   - Run benchmark on `graph479.col` and `graph668.col` with `taskset -c 0,1,2 nice -n 19`.
