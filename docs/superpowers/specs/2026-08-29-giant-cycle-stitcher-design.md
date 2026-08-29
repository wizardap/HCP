# Design Specification: Universal Giant-Cycle Local Repair & Adaptive SAT Stitcher (`GiantCycleStitcher`)

- **Date:** 2026-08-29
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.

---

## 1. Executive Summary & Problem Context

### 1.1 The Giant-Cycle Local Repair Opportunity
In combinatorial HCP solving on large graphs (such as `graph479.col`, `graph668.col`, `graph950.col`, and general HCP benchmarks), the CEGAR loop rapidly consolidates $> 50\% - 70\%$ of the graph into a **Giant Cycle** ($C_{\text{giant}}$), accompanied by a collection of isolated local subcycles ($C_{\text{small}}$) of various lengths ($3, 4, 7, 8, 16, \dots$).
- **The Bottleneck**: Standard CEGAR discards this entire 2-factor whenever subcycles remain and forces the SAT solver to search for an entirely new 2-factor from scratch across thousands of variables (taking $100\text{s} - 300\text{s}$).
- **The Solution — `GiantCycleStitcher`**:
  Instead of discarding the 2-factor, we perform targeted, incremental absorption of every $C_{\text{small}}$ into $C_{\text{giant}}$ via exact CaDiCaL SAT subproblems ($< 1\text{ms}$ each), expanding $C_{\text{giant}}$ until it covers all $|V(G)|$ vertices or no further valid alternating connection exists.

---

## 2. Mathematical Formulation & Architecture

### 2.1 Targeted Absorption Subproblem
Given current 2-factor $\mathcal{C} = \{C_0, C_1, \dots, C_{m-1}\}$ and graph $G = (V, E)$:
1. **Identify Giant Cycle**: $C_{\text{giant}} = \arg\max_{C \in \mathcal{C}} |C|$. If $|C_{\text{giant}}| < 50$ and $|\mathcal{C}| > 2$, apply standard multi-cycle stitching.
2. **Sort Candidate Small Cycles**: Sort remaining cycles $C_i$ by number of cross-edges connecting to $C_{\text{giant}}$.
3. **Exact Alternating Symmetric Difference**:
   - For each candidate $C_i$:
     - Formulate a local parity SAT instance between $C_i$ and $C_{\text{giant}}$ (and optionally intermediate neighbor cycles $C_j$).
     - Identify removable edges $E_{\text{removable}} \subseteq (E(C_i) \cup E(C_{\text{giant}})) \setminus E_{\text{protected}}$.
     - Identify cross edges $E_{\text{cross}} \subseteq E(G)$ connecting $C_i$ and $C_{\text{giant}}$.
     - Solve in CaDiCaL with vertex parity ($\sum y_e = \sum z_{e'}$).
     - If SAT and the resulting cycle unifies $C_i \cup C_{\text{giant}}$ without creating subcycles:
       - Update $C_{\text{giant}} \leftarrow C_i \cup C_{\text{giant}}$.
       - Remove $C_i$ from $\mathcal{C}$.
4. **Adaptive Threshold**: Remove the restrictive $\le 35$ cycle limit in `hcp_solver.rs`, allowing `GiantCycleStitcher` to run whenever $|\mathcal{C}| \le 150$ or whenever $|C_{\text{giant}}| \ge 100$.

---

## 3. Interfaces & Implementation Plan

### 3.1 Structs & Signatures in `src/cegar-fix/src/giant_cycle_stitcher.rs`
```rust
use std::collections::HashSet;
use crate::graph::Graph;

pub struct GiantCycleStitcher;

impl GiantCycleStitcher {
    /// Attempts greedy sequential absorption of all candidate subcycles into the giant cycle.
    /// Returns the reduced cycle list (or a single Hamiltonian tour if all subcycles are absorbed).
    pub fn absorb_into_giant_cycle(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
        max_swaps: usize,
    ) -> Vec<Vec<i32>>;

    /// Iterates absorption and multi-cycle stitching until fixed point.
    pub fn repair_until_fixed_point(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
    ) -> Vec<Vec<i32>>;
}
```

---

## 4. Verification Strategy

1. **Unit Tests (`tests/test_giant_cycle_stitcher.rs`):**
   - Test absorption of multiple small subcycles ($3, 4, 8, 16$ vertices) into a giant cycle of 100 vertices.
   - Test protected edge preservation (degree-2 contracted chains).
   - Test complete reduction to a single Hamiltonian cycle.
2. **Integration Tests (`tests/test_staged_solver.rs`):**
   - Test full CEGAR solver with `GiantCycleStitcher` enabled on multi-cycle graphs.
3. **Benchmark Verification:**
   - Run benchmark on `graph479.col` and `graph668.col` with `taskset -c 0,1,2 nice -n 19`.
