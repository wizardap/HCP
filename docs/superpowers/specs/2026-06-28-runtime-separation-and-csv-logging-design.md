# Design Spec: Runtime Separation, CSV Logging, and Path Gathering

**Date:** 2026-06-28  
**Status:** Approved  

---

## 1. Problem Statement

To enhance the evaluation of the Hamiltonian Cycle Problem (HCP) solver, we need to introduce several adjustments:
1. **Total Runtime vs. Solver Time:** Distinguish between the total execution time (process-level runtime covering encoding and solving) and the time taken specifically by the SAT solver (both total time across all incremental steps and the final step's solve time).
2. **CaDiCaL API Variables:** Retrieve the variable count programmatically from the CaDiCaL C API (`ccadical_vars`) inside the C++ solver wrapper.
3. **CSV Output:** Output the experiment statistics as a CSV file to `src/sol.csv` instead of a plain-text aligned table in `sol.log`.
4. **Gathering Solution Paths:** Save the verified node cycle paths (`solution.path`) into a dedicated directory `src/solution_paths/` as `<graph_name>.path` for inspection.
5. **Incremental Mode Default:** Make incremental mode the default execution path in `run_experiments.py`.

---

## 2. Architecture & Data Flow

```
run_experiments.py (Runs on files)
   │
   ├──► [Start Timer]
   ├──► Invoke: ./hcp-solver <graph> (Incremental Mode by default)
   │      │
   │      ├──► C++ Base Encoding
   │      ├──► Incremental Loop:
   │      │      ├──► [Timer Start] -> ccadical_solve() -> [Timer End]
   │      │      └──► Subtour detection & SEC clause additions
   │      │
   │      └──► Print to stderr: total variables (API), total clauses (local),
   │           final solve time (seconds), total solver time (seconds), actions.
   │
   ├──► [End Timer] (Total Runtime)
   ├──► Parse stderr metrics
   ├──► Dual Verification:
   │      ├──► C++ Decoder check
   │      └──► C Decoder check (on clean temp file)
   │
   ├──► If verified: Copy solution.path -> src/solution_paths/<graph>.path
   └──► Log all parsed metrics to src/sol.csv
```

---

## 3. Interfaces & Class Contracts

### 3.1. `IncrementalSolver` Updates
- **`IncrementalSolver::getNumVars() const`**: Modified to call and return `ccadical_vars(solver)`.
- **`IncrementalSolver::getFinalSolveTime() const`**: Returns `finalSolveTime` (duration of the last solve call in seconds).
- **`IncrementalSolver::getTotalSolverTime() const`**: Returns `totalSolverTime` (sum of durations of all solve calls in seconds).

### 3.2. Solver Outputs (`stderr`)
On termination, `Solver::runIncremental` prints:
```
c total variables: <int>
c total clauses: <int>
c final solve time: <float>
c total solver time: <float>
c incremental actions: <int>
```

### 3.3. CSV Logging Column Headers (`src/sol.csv`)
```csv
Graph,Total Variables,Total Clauses,Total Runtime (s),Total Solver Time (s),Final Solve Time (s),Status,Verified,Actions,Conflicts,Decisions,Propagations
```

---

## 4. Test & Verification Plan

1. **Compilation Check:** Compile the C++ solver suite cleanly with `make -C src`.
2. **Dry Run:** Run `python3 src/run_experiments.py` on a single graph and verify:
   - `src/sol.csv` is correctly created and formatted as a CSV.
   - `src/solution_paths/` directory is created, and `<graph_name>.path` is copied there.
   - Total runtime is distinct from total solver time and final solve time in the CSV logs.
3. **Full Run:** Verify all 18 benchmark graphs.
