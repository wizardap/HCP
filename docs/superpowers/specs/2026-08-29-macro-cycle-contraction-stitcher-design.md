# Design Specification: Macro-Cycle Contraction & Exact SAT Cycle Stitcher (`MacroCycleContractionStitcher`)

- **Date:** 2026-08-29
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.

---

## 1. Executive Summary & Problem Context

### 1.1 The Multi-Cycle Merging Bottleneck
In later CEGAR rounds on large gadget graphs (`graph479.col`, `graph668.col`), the solver successfully consolidates the graph into a small number of macro-cycles (e.g., $m \le 28$, with giant cycles $> 1,600$ vertices).
- **The Failure of Greedy 2-opt & LK**: Greedy heuristics fail to merge these macro-cycles because merging 3 or more macro-cycles often requires a simultaneous $k$-edge swap ($k \in [3, 6]$) where no individual 2-edge swap is valid in isolation.
- **The Solution — `MacroCycleContractionStitcher`**:
  Instead of discarding the 2-factor and spending hundreds of seconds in CDCL to generate an entirely new 2-factor, we formulate a lightweight, exact **Alternating 2-Factor Symmetric Difference SAT Subproblem** directly on the candidate cross-edges connecting the cycles.
  This exact subproblem has only a few hundred variables, executes in $< 5\text{ms}$, and discovers multi-hop alternating cycle merges that greedy heuristics cannot see.

---

## 2. Mathematical Formulation

### 2.1 Variables & Degree Parity
Given 2-factor $\mathcal{C} = \{C_0, C_1, \dots, C_{m-1}\}$ and graph $G = (V, E)$:
1. **Candidate Cross-Edges**: $E_{\text{cross}} = \{ (u, v) \in E(G) : u \in C_a, v \in C_b, a \neq b \}$.
2. **Removed Tour Edge Indicators**: For each non-protected tour edge $e = (u, v) \in E(\mathcal{C})$, variable $y_e \in \{0, 1\}$ ($y_e = 1 \iff e$ removed).
3. **Added Cross-Edge Indicators**: For each $e' \in E_{\text{cross}}$, variable $z_{e'} \in \{0, 1\}$ ($z_{e'} = 1 \iff e'$ added).
4. **Vertex Parity Constraint**: For each vertex $v \in V$:
   $$\sum_{e' \in E_{\text{cross}}, e' \ni v} z_{e'} = \sum_{e \in E(\mathcal{C}), e \ni v} y_e$$
   (Since degree in $\mathcal{C}$ is 2, removing $y$ edges requires adding exactly $y$ cross-edges incident to $v$).
5. **Cycle-Crossing Requirement**: For each target subset of cycles $\{C_a, C_b\}$, require at least 2 cross-edges connecting them.
6. **Swap Budget**: $\sum_{e'} z_{e'} \le K_{\text{max}}$ (e.g., $K_{\text{max}} = 6$ or $8$).

### 2.2 Recombination & Uncontracting
When CaDiCaL returns a satisfying alternating set $(E_{\text{added}}, E_{\text{removed}})$:
- Apply symmetric difference: $\mathcal{C}' = (\mathcal{C} \setminus E_{\text{removed}}) \cup E_{\text{added}}$.
- Extract new cycles $\mathcal{C}'$. If $|\mathcal{C}'| < |\mathcal{C}|$, the cycle count has strictly decreased.
- If $|\mathcal{C}'| == 1$, a complete valid Hamiltonian tour is found immediately!

---

## 3. Module & File Architecture

### 3.1 `src/cegar-fix/src/macro_cycle_stitcher.rs`
```rust
pub struct MacroCycleStitcher;

impl MacroCycleStitcher {
    /// Attempts exact multi-cycle alternating patch merging on current 2-factor cycles.
    /// Returns Some(merged_cycles) if cycle count strictly decreased, or None.
    pub fn stitch_cycles(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
        max_swaps: usize,
    ) -> Option<Vec<Vec<i32>>>;

    /// Iteratively stitches cycles until a single tour is obtained or no further merge is possible.
    pub fn stitch_until_fixed_point(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
    ) -> Vec<Vec<i32>>;
}
```

---

## 4. Verification Strategy

1. **Unit Tests (`tests/test_macro_cycle_stitcher.rs`):**
   - Test 2-cycle exact stitch on disjoint triangles with cross bridges.
   - Test 3-cycle simultaneous 3-edge alternating stitch where 2-opt fails.
   - Test protected edge preservation.
2. **Integration Tests (`tests/test_staged_solver.rs`):**
   - Test complete CEGAR loop with `MacroCycleStitcher` active.
3. **Benchmark Verification:**
   - Run benchmark on `graph479.col` and `graph668.col` with `taskset -c 0,1,2 nice -n 19`.
