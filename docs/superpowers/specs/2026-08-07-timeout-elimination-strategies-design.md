# Spec: Advanced Strategies for Eliminating Timeout in SAT-based CEGAR HCP Solver

**Date**: 2026-08-07  
**Status**: Proposal / Reference Specification  
**Target**: Resolve the 75 timeout testcases in the FHCP Challenge Set (`FHCPCS-col`) for `cegar-fix`.

---

## 1. Background & Root Cause Summary

- **Phenomenon**: 75 out of 1001 graphs in `FHCPCS-col` time out at 1800s (30 minutes).
- **Analysis**:
  - `2-opt` and candidate-graph `3-opt` efficiently merge subcycles during early iterations.
  - When local search gets stuck at local optima (no more mergeable 2-cycle or 3-cycle combinations exist), ~500 disconnected subcycles remain.
  - The default ASP cut-arc blocking strategy (`-b 3`) adds clauses demanding out/in edges from subcycles. However, the current SAT assignment already satisfies these cut-edge requirements.
  - Consequently, the SAT solver repeatedly returns the exact same subcycle assignment across thousands of CEGAR iterations without progress.

---

## 2. Proposed Strategies

### Strategy 1: CEGAR Hard Blocking Fallback (Recommended / Priority 1)
- **Concept**: When both 2-opt and 3-opt fail to merge any subcycles in the current iteration, trigger `cegar_blocking_clauses` on the remaining active subcycles alongside ASP blocking.
- **Logic**: CEGAR blocking imposes an `at_most_n(len - 1)` constraint on the edges forming each subcycle, explicitly forbidding the solver from reproducing the exact same cycle assignment.
- **Impact**: Forces the SAT solver to search a completely new region of the solution space.

### Strategy 2: Adaptive Multi-tier Blocking (Priority 2)
- **Concept**: Differentiate blocking rules based on subcycle lengths.
- **Logic**: Apply CEGAR Hard Blocking to short subcycles ($\le 5$ vertices) which act as rigid local optima traps, while using ASP Cut-arc Blocking for longer subcycles to prevent clause explosion.
- **Impact**: Leverages existing `-b 6/7/8` options in `hcp_solver.rs`.

### Strategy 3: Dynamic Subcycle Perturbation / Random Restart (Priority 3)
- **Concept**: Detect infinite CEGAR loops dynamically.
- **Logic**: Track the number of subcycles over iterations. If subcycle count remains static for $N \ge 3$ consecutive iterations, temporarily forbid a randomly selected subset of active edges or inject pseudo-random unit clauses for $M$ iterations.
- **Impact**: Escapes complex attractor basins in ultra-hard graphs.

### Strategy 4: Double-Bridge 4-Opt / Iterated Local Search (Priority 4)
- **Concept**: Expand local search capabilities beyond 3-opt.
- **Logic**: Implement a 4-cycle Double-Bridge reconnection move as a non-improving perturbation step when 3-opt stalls, then resume 2-opt/3-opt passes.
- **Impact**: Discovers complex 4-cycle interconnections.

---

## 3. Recommended Next Steps

1. Implement Strategy 1 (CEGAR Hard Blocking Fallback) in `src/cegar-fix/src/hcp_solver.rs`.
2. Benchmark on known timeout graphs (e.g. `graph339.col`, `graph479.col`, `graph560.col`).
