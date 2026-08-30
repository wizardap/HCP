# Design Specification: Gadget Hamiltonian Path Absorber (`GadgetPathAbsorber`)

- **Date:** 2026-08-30
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.
- **Commitment to Scientific Rigor:** Zero Tour Injection policy (never read `.hcp.tou` files). Exact mathematical Hamiltonian path discovery within induced subgraphs.

---

## 1. Executive Summary & Problem Context

### 1.1 Small Gadget Cycle Satellites in Flinders 3-SAT Reduction Graphs
In late CEGAR rounds (e.g. Round 12 on `graph668.col`), 95% of vertices are in giant/macro cycles, but 10-15 small satellite cycles (lengths 7, 8, 9, 15, 16) remain.
- **The Bottleneck**:
  Standard 2-opt only tests deleting a single edge $e \in C_{\text{small}}$ to form a path.
  In 3-SAT gadget subgraphs (e.g. 8-cycles and 16-cycles), a Hamiltonian path through all vertices of $C_{\text{small}}$ may require traversing an internal chord or cross-gadget edge.
- **The Solution — `GadgetPathAbsorber`**:
  1. **All-Pairs Hamiltonian Path Enumeration**:
     - For each small cycle $C_s$ ($|C_s| \le 16$), compute all valid Hamiltonian paths $P = (p_1, p_2, \dots, p_k)$ in the induced subgraph $G[V(C_s)]$.
  2. **Boundary Arc Splice into Giant/Macro Cycles**:
     - For every candidate path $P$ with endpoints $p_1$ and $p_k$:
       * Search for an edge $(u, v) \in C_{\text{target}}$ such that $(u, p_1) \in E(G)$ and $(p_k, v) \in E(G)$ (or $(u, p_k) \in E(G)$ and $(p_1, v) \in E(G)$) and neither edge is protected.
       * Splice $P$ into $C_{\text{target}}$ by replacing $(u, v)$ with $(u \to P \to v)$.
     - Automatically absorbs all 8-cycles, 7-cycles, 9-cycles, 16-cycles into the giant cycle!

---

## 2. Architecture & Algorithmic Design

### 2.1 Structs and Methods in `src/cegar-fix/src/gadget_path_absorber.rs`
```rust
use crate::graph::Graph;
use std::collections::{HashMap, HashSet};

pub struct GadgetPathAbsorber;

impl GadgetPathAbsorber {
    /// Attempts to absorb small satellite subcycles (|C| <= 16) into larger cycles by discovering
    /// Hamiltonian paths in the induced subgraphs of the small cycles.
    pub fn try_absorb_gadgets(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
    ) -> Vec<Vec<i32>>;
}
```

---

## 3. Integration into `giant_cycle_stitcher.rs` and `hcp_solver.rs`

In `GiantCycleStitcher::repair_until_fixed_point`:
Add Step 11: `GadgetPathAbsorber::try_absorb_gadgets`.

---

## 4. Verification Strategy

1. **Unit Tests (`tests/test_gadget_path_absorber.rs`):**
   - Test 8-cycle chorded gadget absorption into a 20-vertex cycle.
   - Test multiple satellite gadget absorption (3 gadgets absorbed in one pass).
   - Test protected edge preservation.
2. **Integration Tests (`tests/test_staged_solver.rs`):**
   - Test CEGAR loop integration with chorded gadget graphs.
3. **Benchmark Verification:**
   - Run benchmark on `graph479.col` and `graph668.col` with `taskset -c 0,1,2 nice -n 19`.
