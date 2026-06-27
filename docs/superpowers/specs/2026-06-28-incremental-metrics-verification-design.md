# Design Spec: Incremental Metrics and Dual Verification

**Date:** 2026-06-28  
**Status:** Approved

---

## 1. Problem Statement

1. **Metrics Collection in Incremental Mode:** The `hcp-solver` incremental mode lack detailed solving statistics (incremental action/iteration count, CaDiCaL conflicts, decisions, and propagations). These need to be measured, printed, and logged by the experiment runner (`run_experiments.py`).
2. **Total Solving Time:** The "Total Solving Time" in both incremental and non-incremental modes should cover the entire pipeline, starting from encoding (including base encoding) until the solver finishes.
3. **Verification Rigor:** SAT solutions from incremental solving need rigorous verification. We must:
   - Independently verify using the original C-based `hcp-decode` binary.
   - Output the traversed Hamiltonian cycle nodes to a readable path file (`solution.path`) to allow visual check or input to visualization tools.
4. **Submodule Protection:** The `refs/` directory (including the CaDiCaL submodule) must not be modified.

---

## 2. Architecture & Data Flow

```
Incremental Mode:
┌─────────────────┐       ┌────────────┐       ┌───────────────┐
│     Graph       │──────▶│ hcp-solver │──────▶│ solution.sat  │
└─────────────────┘       │ (Inc Loop) │       └───────┬───────┘
                          └─────┬──────┘               │
                                │                      ▼
                                │             ┌─────────────────┐
                                │             │   HcpDecoder    │
                                │             │ (Writes path to │
                                │             │  solution.path) │
                                │             └─────────────────┘
                                │                      │
                                ▼                      ▼
                        ┌──────────────┐      ┌─────────────────┐
                        │    Stderr    │      │ original decode │
                        │  (Actions &  │      │   (Independent  │
                        │ CaDiCaL stats│      │  verification)  │
                        └──────┬───────┘      └────────┬────────┘
                               │                       │
                               ▼                       ▼
                           ┌───────────────────────────────┐
                           │      run_experiments.py       │
                           │  - Times total execution      │
                           │  - Parses & displays stats    │
                           │  - Logs results to sol.log    │
                           └───────────────────────────────┘
```

---

## 3. Component Design & Changes

### 3.1 C++ Decoder Path Generation (`src/HcpDecoder.hpp`)
We will capture the exact order of visited nodes during the cycle verification traversal. If verified successfully as a Hamiltonian cycle (length matches `nNode`), the list of vertices is written to `solution.path` as a space-separated sequence.

```cpp
// src/HcpDecoder.hpp
std::vector<int> path;
int a = 1;
for (int i = 1; i <= nNode + 1; i++) {
    path.push_back(a);
    if (visited[a]) {
        if ((i - visited[a]) == nNode) {
            std::cout << "c VERIFIED HCP of size " << nNode << "\n";
            
            // Save the cycle path for visualization
            std::ofstream pathOut("solution.path");
            if (pathOut.is_open()) {
                for (size_t k = 0; k < path.size(); ++k) {
                    pathOut << path[k] << (k == path.size() - 1 ? "" : " ");
                }
                pathOut << "\n";
                pathOut.close();
            }
        } else {
            std::cout << "c ERROR: cycle of size " << (i - visited[a]) << " out of " << nNode << "\n";
        }
        break;
    }
    visited[a] = i;
    a = nextNode[a];
}
```

### 3.2 IncrementalSolver Wrapper (`src/IncrementalSolver.hpp/.cpp`)
We wrap the pre-existing C API `ccadical_print_statistics()` to print solver statistics directly to `stdout`.

```cpp
// src/IncrementalSolver.hpp
void printStatistics() const;

// src/IncrementalSolver.cpp
void IncrementalSolver::printStatistics() const {
    ccadical_print_statistics(solver);
}
```

### 3.3 Solver Integration (`src/Solver.cpp`)
Track the iteration count (`actions`) and output statistics when the incremental loop terminates (SAT, UNSAT, or TIMEOUT).

```cpp
// src/Solver.cpp
int actions = 0;
while (true) {
    actions++;
    auto result = isolver.solve();
    if (result == IncrementalSolver::Result::UNSAT || result == IncrementalSolver::Result::TIMEOUT) {
        std::cerr << "c incremental actions: " << actions << "\n";
        std::cerr << "c total variables: " << isolver.getNumVars() << "\n";
        std::cerr << "c total clauses: " << isolver.getNumClauses() << "\n";
        isolver.printStatistics();
        return false;
    }
    if (result == IncrementalSolver::Result::SAT) {
        auto model = isolver.getModel();
        auto components = SubtourDetector::detect(model, g);
        if (components.empty()) {
            std::cerr << "c HAMILTONIAN found\n";
            // ... write solution.sat ...
            std::cerr << "c incremental actions: " << actions << "\n";
            std::cerr << "c total variables: " << isolver.getNumVars() << "\n";
            std::cerr << "c total clauses: " << isolver.getNumClauses() << "\n";
            isolver.printStatistics();
            return true;
        } else {
            // ... encode SEC clauses and continue ...
        }
    }
}
```

### 3.4 Benchmarking Runner (`src/run_experiments.py`)
1. **Compilation of Original Decoder:** Run `make -C ../refs/ChineseRemainderEncoding hcp-decode` to build the original verification tool.
2. **Total Time Measurement:**
   - **Non-incremental:** Set `t_start` before spawning the `./hcp-solver` encoding subprocess.
   - **Incremental:** Time around the `./hcp-solver` run.
3. **Regex Parsers for Statistics:**
   Use regexes on solver output (or `temp_run.sat` for non-incremental) to extract:
   - `c incremental actions: (\d+)` (or default to `1` for non-incremental)
   - `conflicts:\s+(\d+)`
   - `decisions:\s+(\d+)`
   - `propagations:\s+(\d+)`
4. **Dual Verification:**
   Run both:
   - `./hcp-solver -d <solution>`
   - `../refs/ChineseRemainderEncoding/hcp-decode <graph_path> <solution>`
   Verify both outputs print `"VERIFIED"`.

---

## 4. Verification & Testing

1. Run `make -C src` and verify it builds successfully.
2. Run experiments with `python3 src/run_experiments.py` for both default and `--incremental` modes.
3. Confirm that `solution.path` is written for SAT instances and matches the cycle size.
4. Verify that `sol.log` table contains columns for `Actions`, `Conflicts`, `Decisions`, and `Propagations`.
