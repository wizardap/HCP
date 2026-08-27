# Design Specification: Giant Cycle Preservation & Deep Absorption Engine

- **Date:** 2026-08-27
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.

---

## 1. Executive Summary & Problem Context

### 1.1 The Giant-Cycle Shattering Problem
During empirical benchmarks on challenge graphs (`graph479` and `graph651`), the CEGAR solver consistently exhibits a critical structural breakthrough followed by a regression:
- At Iteration 23 on `graph479`, a giant subcycle reached **1,768 vertices out of 1,848** (95.7% of the graph) with only 5 remaining small 16-vertex subcycles.
- Because CaDiCaL solves the global boolean formula without memory of the giant cycle's geometry, the next outer iteration **shattered the 1,768-vertex giant cycle into 6 symmetric 220–440 vertex fragments**, losing the near-complete Hamiltonian tour.
- Root cause: `BackboneFreezer` had a hardcoded activation trigger `_active_cycles.len() <= 5` and `ratio >= 0.70`, which failed to trigger when 6 cycles were present.

### 1.2 The Solution
1. **Adaptive Giant-Cycle Freezing (`BackboneFreezer`)**:
   - Trigger freezing whenever any cycle has length $\ge 0.50 \times N$ OR the number of active subcycles is $\le 25$.
   - Extract internal non-boundary edges of the giant cycle and pass them as SAT assumptions to CaDiCaL, mathematically preventing CaDiCaL from dismantling the giant cycle while allowing boundary edge reconnects.
2. **Deep Alternating Chain Absorption (`CycleChainAbsorber`)**:
   - Enhance `CycleChainAbsorber` to perform multi-cycle chain rotation and 2-point/3-point spliced insertion for all small cycles adjacent to the giant cycle before passing back to SAT.
3. **Inter-Block Modular Cuts (`CutSelector`)**:
   - When large modular blocks ($|C| \ge 100$) are detected, generate inter-block boundary crossing cuts to prevent symmetric block oscillation.

---

## 2. Mathematical Formalization

### 2.1 Adaptive Giant-Cycle Freezing Condition
Let $\mathcal{C} = \{C_1, \dots, C_m\}$ be the active cycles in the current iteration, and $N = |V(G)|$.
Let $C_{\max} = \arg\max_{C \in \mathcal{C}} |C|$.
The freezing condition is satisfied if:
$$|C_{\max}| \ge 0.50 \times N \quad \lor \quad m \le 25$$

### 2.2 Boundary Vertex Definition & Safety Buffer
For cycle $C \in \mathcal{C}$ with $|C| \ge 0.50 \times N$:
- Vertex $u \in C$ is a **boundary vertex** if $\exists v \in N_G(u)$ such that $v \notin C$.
- Safety buffer: if $u$ is boundary, mark $u$, $\text{prev}_C(u)$, and $\text{next}_C(u)$ as boundary.
- **Internal Backbone Edge:** Directed edge $(u, v) \in E(C)$ is an internal backbone edge if neither $u$ nor $v$ is boundary.
- All internal backbone directed edges are asserted as **SAT assumptions** for the next CaDiCaL invocation.

---

## 3. Architecture & Code Changes

### 3.1 `src/cegar-fix/src/backbone_freezer.rs`
- Generalize `extract_backbone_assumptions` signature and thresholding:
  ```rust
  impl BackboneFreezer {
      pub fn extract_backbone_assumptions(
          cycles: &[Vec<i32>],
          g: &Graph,
          encoder: &Encoder,
          min_giant_ratio: f64,
          max_cycle_count_trigger: usize,
      ) -> Vec<Lit>;
  }
  ```

### 3.2 `src/cegar-fix/src/hcp_solver.rs`
- In `solve_hcp_with_cegar`:
  ```rust
  let total_v = g.adjacency_list.len();
  let max_cycle_len = _active_cycles.iter().map(|c| c.len()).max().unwrap_or(0);
  if _active_cycles.len() > 1 && (max_cycle_len >= total_v / 2 || _active_cycles.len() <= 25) {
      assumptions = BackboneFreezer::extract_backbone_assumptions(&_active_cycles, &g, encoder, 0.50, 25);
      if !assumptions.is_empty() {
          println!("BackboneFreezer: locked {} internal backbone edges (giant cycle len {})", assumptions.len(), max_cycle_len);
      }
  } else {
      assumptions.clear();
  }
  ```

### 3.3 `src/cegar-fix/src/cycle_chain_absorber.rs`
- Expand greedy multi-hop alternating chain searches to support 4-point rotation insertion for adjacent small cycle pairs.

---

## 4. Verification Strategy

1. **Unit Tests (`tests/test_backbone_freezer.rs` & `tests/test_cycle_chain_absorber.rs`):**
   - Test that `BackboneFreezer` triggers on 6-cycle cases when giant cycle $\ge 50\%$.
   - Test that boundary edges remain free while internal edges are locked.
   - Test that 4-point rotation absorption correctly splices multiple small cycles into the giant cycle.
2. **Benchmark Verification:**
   - Run benchmark on `graph479.col` and verify that the 1,768-vertex giant cycle is preserved and absorbed rather than shattered.
