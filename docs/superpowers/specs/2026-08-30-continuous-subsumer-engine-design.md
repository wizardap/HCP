# Design Specification: Continuous Incremental CNF Subsumer & Chained Multi-Cycle Absorber (`ContinuousSubsumerEngine`)

- **Date:** 2026-08-30
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.
- **Commitment to Scientific Rigor:** Zero Tour Injection policy (never read `.hcp.tou` files). Exact mathematical clause subsumption and multi-cycle chained absorption.

---

## 1. Executive Summary & Problem Context

### 1.1 Incremental CNF Clause Bloat in CEGAR Loops
In deep CEGAR loops, accumulating cut CNFs across rounds without immediate subsumption causes CaDiCaL worker startup and clause-database propagation slowdowns.
- **The Solution — `ContinuousSubsumerEngine`**:
  1. **Continuous Every-Round CNF Subsumption**:
     - Automatically execute `CnfSubsumer::prune_and_subsume_cuts` every round if `accumulated_cut_cnfs.len() >= 10` or `sat_solving_time > 15.0s`.
     - Maintains the CNF database in minimal canonical form ($< 2,000$ non-redundant clauses), ensuring each SAT round completes in $< 30\text{s}$.
  2. **Multi-Cycle Chained Absorption in Stitcher**:
     - Repeatedly chain `CycleChainAbsorber` and `SubcycleAbsorber` within `GiantCycleStitcher` until a strict fixed point is reached.

---

## 2. Architecture & Algorithmic Design

### 2.1 Changes in `src/cegar-fix/src/hcp_solver.rs`
1. Update `SolverReseeder` trigger condition:
   - Reseed when `accumulated_cut_cnfs.len() >= 10 || sat_solving_time >= 15.0s || count % 3 == 0`.
   - Ensures CaDiCaL is reseeded with the leanest non-redundant CNF every 3 rounds or whenever solve time reaches 15s.

---

## 3. Verification Strategy

1. **Unit & Integration Tests (`tests/test_staged_solver.rs`):**
   - Verify continuous reseeding and workspace test suite.
2. **Benchmark Verification:**
   - Run benchmark on `graph479.col` and `graph668.col` with `taskset -c 0,1,2 nice -n 19`.
