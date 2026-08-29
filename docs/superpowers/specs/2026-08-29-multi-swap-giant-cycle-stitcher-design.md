# Design Specification: Multi-Swap Giant-Cycle & Simultaneous Multi-Cycle SAT Stitcher (`MultiSwapStitcher`)

- **Date:** 2026-08-29
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.

---

## 1. Executive Summary & Root Cause

### 1.1 The Single-Swap Per Cycle Bottleneck
In `MacroCycleStitcher` and `GiantCycleStitcher`, an exact SAT subproblem was formulated over 2-factors to find alternating symmetric difference cycle mergers.
- **The Bottleneck**: The previous formulation enforced an At-Most-One (AMO) clause on removed cycle edges:
  $$\forall C_i \in \mathcal{C}, \quad \sum_{e \in E(C_i)} y_e \le 1$$
  When a giant cycle $C_{\text{giant}}$ ($|C| = 1300 \dots 1800$) is surrounded by multiple gadget subcycles ($15 \times 16$), $C_{\text{giant}}$ was artificially forbidden from removing $> 1$ edge. It could only merge with a single cycle per step, and multi-cycle swaps requiring 2 cuts on $C_{\text{giant}}$ were classified as UNSAT.
- **The Solution — `MultiSwapStitcher`**:
  1. **Unrestricted / Scalable Giant Cycle Cuts**: For any large cycle ($|C| \ge 50$), allow up to $k = \min(32, |C|/4)$ edge cuts, removing the rigid AMO restriction on $C_{\text{giant}}$.
  2. **Simultaneous Multi-Cycle Absorption**: Scale total `max_swaps` from 6 up to 32 swaps across the cycle set.
  3. **Multi-Cycle Clustering**: Cluster nearby subcycles into joint SAT instances to allow simultaneous multi-cycle 3-way, 4-way, and $k$-way absorption in $< 5\text{ms}$.

---

## 2. Mathematical Formulation & Architecture

### 2.1 Vertex Parity & Generalized Cycle-Swap Limits
Given 2-factor $\mathcal{C} = \{C_0, \dots, C_{m-1}\}$ and graph $G$:
1. **Vertex Degree-Parity Preservation**:
   $$\forall v \in V(\mathcal{C}), \quad \sum_{e' \in E_{\text{cross}}(v)} z_{e'} = \sum_{e \in E_{\text{cycle}}(v) \setminus E_{\text{protected}}} y_e \in \{0, 1\}$$
2. **Cycle Swap Limits**:
   - For small subcycles ($|C_i| < 50$): Enforce $\sum_{e \in E(C_i)} y_e \le 2$ (allowing 1 or 2 cuts for path traversals).
   - For giant/large cycles ($|C_{\text{giant}}| \ge 50$): Do NOT enforce pairwise AMO. The vertex parity constraint alone guarantees that cuts on $C_{\text{giant}}$ correspond to valid cross-edge attachments.
3. **Global Non-Triviality & Bounded Swaps**:
   - $\bigvee_{e' \in E_{\text{cross}}} z_{e'}$ (at least one cross edge used).
   - $\sum z_{e'} \le \text{max\_swaps}$ (with `max_swaps` up to 32).
4. **Traversal & Verification**:
   - Verify 2-regularity, lack of subcycles, and valid uncontracted path connectivity.

---

## 3. Interfaces & Implementation Plan

### 3.1 Enhancements in `src/cegar-fix/src/macro_cycle_stitcher.rs` & `src/cegar-fix/src/giant_cycle_stitcher.rs`
- In `macro_cycle_stitcher.rs`: Modify `build_base_cnf` to only enforce AMO for small cycles ($|C| < 50$), leaving giant cycles free.
- In `giant_cycle_stitcher.rs`: Add `absorb_simultaneous_multi_cycle` supporting up to 32 simultaneous cross-edge swaps into $C_{\text{giant}}$.

---

## 4. Verification Strategy

1. **Unit Tests (`tests/test_macro_cycle_stitcher.rs` & `tests/test_giant_cycle_stitcher.rs`):**
   - Test simultaneous absorption of multiple 16-cycle gadgets into a 100-node giant cycle with $> 2$ cuts on the giant cycle.
   - Test 4-way alternating cycle merger.
   - Test protected edge preservation.
2. **Integration Tests (`tests/test_staged_solver.rs`):**
   - Test full CEGAR pipeline with multi-swap stitcher enabled.
3. **Benchmark Verification:**
   - Run benchmark on `graph479.col` and `graph668.col` with `taskset -c 0,1,2 nice -n 19`.
