# Design Specification: Hemisphere Splicing & Bi-Partition Crossing Cuts Engine (`HemisphereSplicer`)

- **Date:** 2026-08-28
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.

---

## 1. Executive Summary & Empirical Problem Context

### 1.1 The 2-Hemisphere Oscillation Bottleneck
During empirical 1800s testing on `graph479.col` ($N = 1,848$ contracted vertices), the CEGAR solver consistently achieves a critical state at rounds 37–46:
- The entire graph is condensed into **exactly two 924-vertex cycles** ($924 + 924 = 1,848$).
- However, because both subcycles exceed the small-cycle boundary threshold ($924 > 8$), `CutSelector` generates only a single blocking clause ($\neg x_1 \lor \dots \lor \neg x_k$).
- Consequently, CaDiCaL oscillates to alternative symmetric 2-cycle partitions instead of connecting $C_1$ and $C_2$.

### 1.2 The Solution: `HemisphereSplicer`
1. **Direct 2-Opt Splicing for Macro-Components ($k \in [2, 4]$)**:
   - For all pairs of macro-cycles $(C_a, C_b)$, inspect cross-edges $(u, v)$ where $u \in C_a, v \in C_b$.
   - Search for valid 2-edge reconnects $(u_i, u_{i+1}) \in C_a$ and $(v_j, v_{j+1}) \in C_b$ such that $(u_i, v_j) \in E(G)$ and $(u_{i+1}, v_{j+1}) \in E(G)$ (or cross-reversed).
   - If found without violating degree-2 protected chains, directly splice $C_a$ and $C_b$ into a single merged cycle. If the merged cycle spans all vertices, **terminate immediately with certified SATISFIABLE**.
2. **Bi-Partition Crossing Cuts ($\delta^+(C_i) \ge 1, \delta^-(C_i) \ge 1$)**:
   - Whenever the active subcycle count is small ($2 \le k \le 4$), generate directional crossing cut clauses for every macro-cycle $C_i$:
     $$\bigvee_{e \in \delta^+(C_i)} x_e \quad \text{and} \quad \bigvee_{e \in \delta^-(C_i)} x_e$$
   - This mathematically forces the SAT solver to traverse edges crossing between the hemispheres.

---

## 2. Mathematical Soundness

### 2.1 Hamiltonian Connectivity Invariant
Let $S \subset V(G)$ be any non-empty proper subset of vertices ($1 \le |S| < |V(G)|$).
Any valid Hamiltonian cycle $H$ must enter $S$ at least once and exit $S$ at least once:
$$\sum_{e \in \delta^+(S)} x_e \ge 1 \iff \bigvee_{e \in \delta^+(S)} x_e$$
$$\sum_{e \in \delta^-(S)} x_e \ge 1 \iff \bigvee_{e \in \delta^-(S)} x_e$$
When the graph is partitioned into $k \in [2, 4]$ cycles, each cycle $C_i$ is a proper subset. Asserting both entering and exiting crossing clauses is 100% sound and valid for all Hamiltonian cycles.

---

## 3. Architecture & Code Changes

### 3.1 `src/cegar-fix/src/hemisphere_splicer.rs`
```rust
use rustsat::types::Clause;
use crate::graph::Graph;
use crate::encoder::Encoder;
use crate::contraction::Degree2Contractor;

pub struct HemisphereSplicer;

impl HemisphereSplicer {
    /// Attempts direct 2-opt cross-splicing between all pairs of cycles.
    /// Returns Some(merged_cycles) if any merge occurred.
    pub fn try_direct_splice_all(
        cycles: &[Vec<i32>],
        g: &Graph,
        contractor: &Degree2Contractor,
    ) -> Option<Vec<Vec<i32>>>;

    /// Generates directional bi-partition crossing cut clauses for all macro-cycles (k <= 4).
    pub fn generate_hemisphere_crossing_cuts(
        cycles: &[Vec<i32>],
        g: &Graph,
        encoder: &Encoder,
    ) -> Vec<Clause>;
}
```

### 3.2 Integration into `hcp_solver.rs`
- In `solve_hamilton` / `two_opt`:
  - When `_active_cycles.len() <= 4`, invoke `HemisphereSplicer::try_direct_splice_all`.
  - If a single full tour is produced, verify and return `SATISFIABLE`.
  - In `get_blocking_clauses`, if `cycles.len() <= 4`, append `HemisphereSplicer::generate_hemisphere_crossing_cuts`.

---

## 4. Verification Strategy

1. **Unit Tests (`tests/test_hemisphere_splicer.rs`):**
   - Test direct 2-opt splicing on two 10-node cycles connected by two cross-edges $\implies$ produce single 20-node tour.
   - Test bi-partition crossing cut generation for 2-cycle and 3-cycle partitions $\implies$ assert non-empty directed crossing clauses.
2. **Benchmark Verification:**
   - Run benchmark on `graph479.col` to verify whether the two 924-vertex cycles are spliced or constrained to merge.
