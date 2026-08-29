# Design Specification: Exact SAT-Based 3-Opt Triangle & Multi-Cycle Alternating Splicer (`MultiOptSatSplicer`)

- **Date:** 2026-08-29
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.
- **Commitment to Scientific Rigor:** Zero Tour Injection policy (never read `.hcp.tou` files). Pure mathematical alternating cycle transformation and certified tour reconstruction.

---

## 1. Executive Summary & Problem Context

### 1.1 The 2-Opt Bottleneck on Flinders 3-SAT Reduction Graphs
In `graph668.col`, CEGAR and `GiantCycleStitcher` succeed in compressing 113 subcycles down to **28 cycles** (with two massive macro-cycles covering 2,047 vertices / $71.5\%$ of the graph).
- **The Bottleneck**:
  Standard 2-opt splicing only looks for pairs of cycles $(C_i, C_j)$ that share a direct 4-cycle crossing $(u_1, u_2) \in C_i, (v_1, v_2) \in C_j, (u_1, v_1) \in E(G), (u_2, v_2) \in E(G)$.
  In 3-regular cubic structures, cycles often cannot be merged with just 2 edges, but can be merged via:
  1. **3-Cycle Triangle Swaps (3-Opt)**: Removing $(u_1, u_2) \in C_1, (v_1, v_2) \in C_2, (w_1, w_2) \in C_3$ and adding $(u_1, v_2), (v_1, w_2), (w_1, u_2)$, merging all 3 cycles into one in a single step!
  2. **Intra-2-Cycle 3-Opt Swaps**: Removing 2 edges from $C_1$ and 1 edge from $C_2$ (or vice-versa) and replacing with 3 cross edges.
- **The Solution — `MultiOptSatSplicer`**:
  - Enumerate both 2-opt bridges and 3-opt cycle triplets across the $m \le 60$ cycles.
  - Formulate an exact SAT spanning forest problem in CaDiCaL SAT with:
    - Vertex parity conservation (no edge collisions).
    - MTZ ladder ordering variables $u_i \in [1, m]$ guaranteeing tree acyclicity.
  - Solve in $< 5\text{ms}$ in CaDiCaL and splice all components into a single unified tour.

---

## 2. Architecture & Algorithmic Design

### 2.1 Structs and Signatures in `src/cegar-fix/src/multi_opt_sat_splicer.rs`
```rust
use std::collections::{HashMap, HashSet};
use crate::graph::Graph;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CycleSwap {
    TwoOpt {
        c1: usize,
        c2: usize,
        rem1: (i32, i32),
        rem2: (i32, i32),
        add1: (i32, i32),
        add2: (i32, i32),
    },
    ThreeOptTriangle {
        c1: usize,
        c2: usize,
        c3: usize,
        rem1: (i32, i32),
        rem2: (i32, i32),
        rem3: (i32, i32),
        add1: (i32, i32),
        add2: (i32, i32),
        add3: (i32, i32),
    },
}

pub struct MultiOptSatSplicer;

impl MultiOptSatSplicer {
    /// Attempts multi-opt (2-opt + 3-opt triangle) SAT-based cycle splicing across all m cycles.
    pub fn splice_multi_opt_cycles(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
    ) -> Vec<Vec<i32>>;
}
```

---

## 3. Integration into `giant_cycle_stitcher.rs`

In `giant_cycle_stitcher.rs` `repair_until_fixed_point`:
```rust
// Step 3b: Multi-Opt SAT Splicer (2-opt + 3-opt triangle spanning forest)
let multi_opt_spliced = MultiOptSatSplicer::splice_multi_opt_cycles(&current_cycles, g, &protected);
if multi_opt_spliced.len() < current_cycles.len() {
    current_cycles = multi_opt_spliced;
    continue;
}
```

---

## 4. Verification Strategy

1. **Unit Tests (`tests/test_multi_opt_sat_splicer.rs`):**
   - Test 3-cycle triangle configuration that cannot be merged by 2-opt, verifying that 3-opt triangle merge succeeds.
   - Test mixed 2-opt + 3-opt spanning tree over 6 cycles.
   - Test protected edge preservation.
2. **Integration Tests (`tests/test_staged_solver.rs`):**
   - Test CEGAR pipeline with `MultiOptSatSplicer` active.
3. **Benchmark Verification:**
   - Run benchmark on `graph479.col` and `graph668.col` with `taskset -c 0,1,2 nice -n 19`.
