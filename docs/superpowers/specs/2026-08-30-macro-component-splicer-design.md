# Design Specification: Macro-Component Spanning Tree Splicer & Generalized Bicomponent Cutter (`MacroComponentSplicer`)

- **Date:** 2026-08-30
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.
- **Commitment to Scientific Rigor:** Zero Tour Injection policy (never read `.hcp.tou` files). Pure mathematical spanning-tree cycle merging and valid cut constraints.

---

## 1. Executive Summary & Problem Context

### 1.1 Multi-Hop Cycle Chaining in CEGAR on `graph668.col`
In deep CEGAR rounds:
- $C_1$: 1,131 vertices
- $C_2$: 794 vertices
- Small cycles: $C_3, C_4, \dots$
- Together, $C_1 \cup C_2$ contain $> 1,925$ vertices ($67.3\%$ of the graph).
- **The Bottleneck**:
  Direct 2-opt between $C_1$ and $C_2$ may not exist directly, but $C_1$ connects to $C_3$, $C_3$ connects to $C_4$, and $C_4$ connects to $C_2$.
  A 1-hop search fails to connect $C_1$ and $C_2$.
- **The Solution — `MacroComponentSplicer`**:
  1. **Macro-Graph Spanning Forest Splicing**:
     - Construct a meta-graph where each cycle $C_i$ is a vertex, and an undirected edge $(C_i, C_j)$ exists with an explicit 2-opt bridge $(e_i, e_j, x_1, x_2)$.
     - Compute the maximum spanning tree for each connected component.
     - Recursively splice each tree from leaves to root using validated 2-opt operations.
     - Merges entire multi-hop chains ($C_1 \leftrightarrow C_3 \leftrightarrow C_4 \leftrightarrow C_2$) into a single giant cycle covering $> 80-90\%$ of the graph.
  2. **Generalized Bicomponent Cut Injection**:
     - For any two large cycles $C_i, C_j$ with $|C_i|, |C_j| \ge |V| / 5$, inject the directional boundary cuts between $V(C_i)$ and $V \setminus V(C_i)$ regardless of how many small cycles exist.

---

## 2. Architecture & Algorithmic Design

### 2.1 Structs and Methods in `src/cegar-fix/src/macro_component_splicer.rs`
```rust
use crate::graph::Graph;
use std::collections::{HashMap, HashSet, VecDeque};

pub struct MacroComponentSplicer;

impl MacroComponentSplicer {
    /// Discovers the macro-adjacency graph of 2-opt bridges and merges entire connected spanning trees.
    pub fn splice_spanning_components(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
    ) -> Vec<Vec<i32>>;
}
```

---

## 3. Integration into `giant_cycle_stitcher.rs`

In `GiantCycleStitcher::repair_until_fixed_point`:
Add Step 9: `MacroComponentSplicer::splice_spanning_components`.

---

## 4. Verification Strategy

1. **Unit Tests (`tests/test_macro_component_splicer.rs`):**
   - Test multi-hop chain $C_1 \leftrightarrow C_2 \leftrightarrow C_3 \leftrightarrow C_4$ all merging into a single cycle.
   - Test tree-structured cycle networks (star, tree).
   - Test protected edge preservation.
2. **Integration Tests (`tests/test_staged_solver.rs`):**
   - Test CEGAR loop integration with multi-hop cycle chains.
3. **Benchmark Verification:**
   - Run benchmark on `graph479.col` and `graph668.col` with `taskset -c 0,1,2 nice -n 19`.
