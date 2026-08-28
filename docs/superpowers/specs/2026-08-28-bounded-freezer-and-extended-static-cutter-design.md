# Design Specification: Bounded Backbone Freezer & Extended Static Cycle Cutter

- **Date:** 2026-08-28
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.

---

## 1. Executive Summary & Problem Context

### 1.1 The Dual Bottlenecks
In empirical 1800s testing on `graph479.col`:
1. **Heavy Assumption Drag:** Locking $> 800 - 1,055$ internal edges in `BackboneFreezer` severely over-constrains CaDiCaL, causing $T_{\text{SAT}}$ to balloon to $265\text{s} - 534\text{s}$ per round and limiting 1800s runs to only 22 iterations.
2. **Small 6-Cycle Residual Noise:** Despite 4-cycle elimination, 6-cycles and 8-cycles still spawn at Round 0, taking 10–15 rounds to refute.

### 1.2 The Solution
1. **`BoundedFreezer` & Adaptive Relaxation (`backbone_freezer.rs`):**
   - Introduce a strict budget cap $K_{\text{freeze\_max}} = 250$ edges for assumptions.
   - If candidate backbone edges exceed the cap, subsample an evenly-spaced stride of $K_{\text{freeze\_max}}$ edges along the giant cycle.
   - If `last_sat_time >= 10.0s`, dynamically downscale to $K_{\text{freeze\_max}} = 100$ edges, keeping $T_{\text{SAT}} \le 5\text{s}$.
2. **Extended `StaticCycleCutter` (`static_cycle_cutter.rs`):**
   - Statically extract all induced 6-cycles (hexagons) and add directional subtour elimination clauses upfront to `base_cnf` (capped at 4,000 clauses).

---

## 2. Mathematical Soundness & Completeness

### 2.1 Invariant Preservation
- **Backbone Subsampling:** Assumptions in CaDiCaL are purely directional heuristics that narrow the search space without asserting permanent unit clauses. Subsampling a bounded subset of 250 edges preserves completeness via solver fallback.
- **6-Cycle Static Elimination:** For any cycle $C$ with $|C| = 6 < |V(G)|$, asserting $\bigvee_{e \in C} \neg x_e$ for both traversal orientations is mathematically sound for all valid Hamiltonian tours.

---

## 3. Architecture & Code Changes

### 3.1 `src/cegar-fix/src/backbone_freezer.rs`
```rust
#[derive(Debug, Clone)]
pub struct FreezerOptions {
    pub ratio_threshold: f64,
    pub max_subcycles_trigger: usize,
    pub max_frozen_edges: usize, // Default: 250
    pub adaptive_relax_time_secs: f64, // Default: 10.0
}

impl BackboneFreezer {
    pub fn select_adaptive_frozen_assumptions(
        cycles: &[Vec<i32>],
        g: &Graph,
        encoder: &Encoder,
        contractor: &Degree2Contractor,
        opts: &FreezerOptions,
        last_sat_time_secs: f64,
    ) -> Vec<Lit>;
}
```

### 3.2 `src/cegar-fix/src/static_cycle_cutter.rs`
- Add `find_6_cycles` helper to extract canonical 6-cycles $(u_1, \dots, u_6)$ with $u_1 < \min(u_2..u_6)$ and $u_2 < u_6$.
- Add clauses to `Cnf` up to 4,000 clauses.

---

## 4. Verification Strategy

1. **Unit Tests:**
   - Test bounded subsampling and adaptive relaxation in `tests/test_backbone_freezer.rs`.
   - Test 6-cycle detection in `tests/test_static_cycle_cutter.rs`.
2. **Integration Test:**
   - Full CEGAR verification in `tests/test_staged_solver.rs`.
3. **Benchmark Verification:**
   - Run benchmark on `graph479.col` to verify $T_{\text{SAT}} \le 5\text{s}$ throughout.
