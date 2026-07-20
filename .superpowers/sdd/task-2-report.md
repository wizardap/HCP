# Task 2 Report: Implement `runIncrementalAdaptive123` with SEC Cut Inheritance

## Summary
- **Status:** DONE
- **Commit:** `7bfe9be`
- **Files Modified:**
  - `src/Solver.hpp` (added `setGraphFile` setter method)
  - `src/Solver.cpp` (implemented multiphase `runIncrementalAdaptive123` execution loop transferring accumulated SEC clauses across $c=1 \to 2 \to 3$)
  - `src/test_incremental_solver.cpp` (added integration test `testAdaptiveBoundedEscalation`)

## Implementation Details

1. **Multiphase Cycle Escalation Loop (`runIncrementalAdaptive123`):**
   - Implemented 3-phase cycle escalation loop traversing cycle values $c \in \{1, 2, 3\}$.
   - Phase limits configured via `phase1MaxIters_` (default 300) and `phase2MaxIters_` (default 500), with Phase 3 unbound (up to 100,000 iterations).
   - In each phase, graph loading, `HcpEncoder` base encoding (with current cycle value $c$), and graph preprocessor forced edge clauses are set up cleanly.

2. **SEC Cut Inheritance across Phases:**
   - Accumulated SEC clauses in `accumulatedSecClauses_` are preserved across phase transitions.
   - When moving from phase $i$ to phase $i+1$, all previously generated SEC clauses (which act on base edge variables $1 \dots 2E$) are injected directly into the new `IncrementalSolver` instance via `isolver.addClause(clause)`.
   - Variable index independence between base edge variables ($1 \dots 2E$) and cycle position auxiliary variables ensures syntactical and mathematical validity of transferred clauses.

3. **Escalation Triggers & Controls:**
   - Phase escalation triggers when `phaseIters >= phaseMaxIter` OR `consecutiveLowComps >= 30` (stagnation with $\le 4$ components).
   - Total wall-clock time limit is tracked continuously across phases to enforce prompt termination when time budget expires.

4. **Integration Testing:**
   - Added `testAdaptiveBoundedEscalation()` in `src/test_incremental_solver.cpp`.
   - Implemented fallback path resolution: checks `graphs/small.edge` (when run from root) and falls back to `../graphs/small.edge` (when run from `src/`).
   - Verified that solving a sample graph (`graphs/small.edge`) in `CycleMode::ADAPTIVE_BOUNDED` executes Phase 1 ($c=1$), accumulates SECs (1788 clauses), escalates to Phase 2 ($c=2$), and successfully finds the Hamiltonian cycle.

## Verification
- Ran `make -C src test`.
- All tests (`test_incremental_solver`, `test_graphs`, `test_vertex_separator`, `test_gomory_hu`, `test_sec_encoder`) passed completely without any assertion failures.
