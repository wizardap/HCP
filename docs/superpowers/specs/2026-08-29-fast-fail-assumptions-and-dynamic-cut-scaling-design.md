# Design Specification: Fast-Fail Assumptions & Dynamic Cut Scaling

- **Date:** 2026-08-29
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.

---

## 1. Executive Summary & Problem Context

### 1.1 Root-Cause Analysis on Non-Bipartite Instances (`graph668.col`)
In empirical 1800s testing on `graph668.col`:
1. **Unbounded Assumption Drag:** When assumptions are passed to CaDiCaL, proving UNSAT or finding solutions under heavy assumptions takes 30s–130s per round.
2. **Low Cut Budget Throttling:** `CutSelector` was fixed at a 40-cut ceiling, requiring 5+ rounds just to drain 112 initial subcycles.
3. **Odd-Cycle Discovery Lag:** `graph668.col` is non-bipartite and spawns 7- and 8-cycles that were not included in Round 0 static elimination.

### 1.2 The Solution
1. **Fast-Fail Assumption Limit:** Set a strict conflict limit (e.g. 5,000 conflicts) on assumption solving. If interrupted or UNSAT, instantly fall back to unconstrained solving.
2. **Dynamic Cut Budget Scaling:** Dynamically scale `max_cuts_per_round` up to 100 for cycles $\le 16$ vertices when total subcycle count $> 30$.
3. **Extended Static 7- & 8-Cycle Cuts:** Static enumeration and subtour elimination for induced 7-cycles and 8-cycles in `StaticCycleCutter` (capped at 4,000 clauses).

---

## 2. Mathematical Soundness & Completeness

- **Assumption Conflict Limits:** Bounding assumption conflicts in CaDiCaL is purely an incomplete search filter; falling back to `solver.solve()` guarantees 100% SAT/UNSAT completeness.
- **Dynamic SEC Cuts:** Subtour elimination clauses $\bigvee_{e \in C} \neg x_e$ for any subcycle $C$ ($|C| < |V|$) are valid cutting planes. Increasing the round cut budget accelerates convergence without adding invalid constraints.

---

## 3. Architecture & Code Changes

### 3.1 `src/cegar-fix/src/cut_selector.rs`
```rust
#[derive(Debug, Clone)]
pub struct CutSelectorOptions {
    pub max_cycle_len_threshold: usize, // Default: 64
    pub base_max_cuts: usize,           // Default: 40
    pub high_volume_max_cuts: usize,    // Default: 100
    pub tiny_cycle_boundary_len: usize, // Default: 8
}
```

### 3.2 `src/cegar-fix/src/static_cycle_cutter.rs`
- Add chordless 7-cycle and 8-cycle enumeration and SEC generation.

### 3.3 `src/cegar-fix/src/hcp_solver.rs`
- Add conflict-bounded assumption solving in `cegar`.

---

## 4. Verification Strategy

1. **Unit Tests:**
   - Test dynamic cut scaling in `tests/test_cut_selector.rs`.
   - Test 7- and 8-cycle extraction in `tests/test_static_cycle_cutter.rs`.
2. **Integration Test:**
   - Verify full CEGAR loop in `tests/test_staged_solver.rs`.
3. **Benchmark Verification:**
   - Run benchmark on `graph668.col` to verify accelerated convergence and $T_{\text{SAT}} \le 5\text{s}$.
