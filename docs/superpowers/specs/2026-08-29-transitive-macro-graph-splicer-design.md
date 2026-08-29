# Design Specification: Transitive Macro-Cycle Graph Splicer (`TransitiveMacroSplicer`)

- **Date:** 2026-08-29
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.

---

## 1. Executive Summary & Root Cause

### 1.1 The Transitive Cycle Merging Challenge
In late CEGAR iterations on large graphs (such as `graph668.col` at Round 23), the 2-factor consolidates into a small number of massive macro-cycles ($828, 616, 572, 370$, covering $> 83\%$ of vertices) plus a few satellite cycles.
- **The Bottleneck**: Existing pairwise absorbers attempt direct merges between $C_{\text{giant}}$ and individual subcycles $C_i$. If $C_{\text{giant}}$ and $C_i$ are separated by intermediate macro-cycles (e.g. $C_0 \leftrightarrow C_1 \leftrightarrow C_2 \leftrightarrow C_3$), direct 2-cycle absorption fails because there are zero direct cross-edges between $C_0$ and $C_3$.
- **The Solution — `TransitiveMacroSplicer`**:
  Construct a **Macro-Cycle Graph** $\mathcal{M} = (V_{\mathcal{M}}, E_{\mathcal{M}})$ where each cycle is a supernode ($|V_{\mathcal{M}}| = m \le 60$).
  1. Detect all viable 2-opt crossing bridges between adjacent cycles in $\mathcal{M}$.
  2. Compute a spanning tree / Hamiltonian traversal of the macro-cycles in $\mathcal{M}$.
  3. Formulate an exact CaDiCaL SAT instance to select compatible 2-opt edge removals across the entire macro-tree simultaneously ($< 1\text{ms}$).
  4. Splice all $m$ cycles along the macro-tree into **one single unified Hamiltonian cycle**.

---

## 2. Mathematical Formulation & Architecture

### 2.1 Macro-Graph Construction
Given 2-factor $\mathcal{C} = \{C_0, C_1, \dots, C_{m-1}\}$ and graph $G = (V, E)$:
1. **Macro-Vertices**: $V_{\mathcal{M}} = \{0, 1, \dots, m-1\}$.
2. **Crossing Candidate Pairs**:
   For each pair of cycles $(C_i, C_j)$, find all 2-opt merge opportunities:
   - Non-protected edge $e_i = (u_i, v_i) \in E(C_i) \setminus E_{\text{protected}}$.
   - Non-protected edge $e_j = (u_j, v_j) \in E(C_j) \setminus E_{\text{protected}}$.
   - Cross-edges: $(u_i, u_j) \in E(G)$ and $(v_i, v_j) \in E(G)$ (or cross orientation).
   - If at least one valid 2-opt bridge exists, add undirected meta-edge $(i, j) \in E_{\mathcal{M}}$.
3. **Exact Global Splicing SAT Subproblem**:
   - For each 2-opt bridge $b = (e_i, e_j, e'_1, e'_2)$ on meta-edge $(i, j)$, allocate variable $B_b \in \{0, 1\}$.
   - **Edge Exclusivity**: Each non-protected cycle edge $e \in E(\mathcal{C})$ can participate in at most one active bridge ($\sum_{b \ni e} B_b \le 1$).
   - **Spanning Tree Connectivity**: Enforce that the selected bridges form a connected spanning forest/tree on $\mathcal{M}$ (or maximize the number of merged components).
   - CaDiCaL solves the instance in $< 2\text{ms}$.
4. **Splicing & Tour Verification**:
   - Apply all selected 2-opt swaps simultaneously.
   - Verify 2-regularity, vertex uniqueness, and single-cycle connectivity on the uncontracted graph.

---

## 3. Interfaces & Implementation Plan

### 3.1 Structs & Signatures in `src/cegar-fix/src/transitive_macro_splicer.rs`
```rust
use std::collections::HashSet;
use crate::graph::Graph;

pub struct TransitiveMacroSplicer;

impl TransitiveMacroSplicer {
    /// Attempts transitive global macro-graph splicing across all m cycles.
    /// Returns the reduced cycle list (or a single Hamiltonian tour if all cycles are spliced).
    pub fn splice_transitive_macro_graph(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
    ) -> Vec<Vec<i32>>;
}
```

---

## 4. Verification Strategy

1. **Unit Tests (`tests/test_transitive_macro_splicer.rs`):**
   - Test transitive 4-cycle chain ($C_0 \leftrightarrow C_1 \leftrightarrow C_2 \leftrightarrow C_3$) where $C_0$ and $C_3$ share zero cross-edges.
   - Test protected degree-2 chain preservation.
   - Test complete reduction to a single Hamiltonian cycle.
2. **Integration Tests (`tests/test_staged_solver.rs`):**
   - Test full CEGAR solver with `TransitiveMacroSplicer` enabled.
3. **Benchmark Verification:**
   - Run benchmark on `graph479.col` and `graph668.col` with `taskset -c 0,1,2 nice -n 19`.
