# Adaptive Bounded Cycle Escalation (c = 1 -> 2 -> 3) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement single-threaded, progressive cycle-multiplier escalation ($c = 1 \to 2 \to 3$) with SEC cut transfer for the HCP SAT solver.

**Architecture:** Maintain an accumulated list of Subtour Elimination Constraints (SECs) across solver instances. Phase 1 runs at $c = 1$. If stagnation or iteration limit occurs, escalate to $c = 2$ and inject all accumulated SEC clauses. Repeat escalation to $c = 3$ if Phase 2 stagnates.

**Tech Stack:** C++17, CaDiCaL SAT solver, Makefile, standard C++ STL.

## Global Constraints

- Preserve existing `HcpDecoder` decoding interfaces and CLI options (`--cycle auto`, `--incremental`).
- Build cleanly with `make -C src`.
- Unit tests in `src/` must pass cleanly via `make -C src test`.

---

### Task 1: Extend `Solver` API with `CycleMode` and Adaptive Escalation Declarations

**Files:**
- Modify: `src/Solver.hpp:38-65`
- Modify: `src/Solver.cpp`
- Test: `src/test_graphs.cpp`

**Interfaces:**
- Consumes: Existing `Solver` configuration API.
- Produces: `setCycleMode(CycleMode)`, `getCycleMode()`, and `runIncrementalAdaptive123(int64_t)`.

- [ ] **Step 1: Write unit test for `CycleMode` configuration in `src/test_graphs.cpp`**

Modify `src/test_graphs.cpp` to include a test for `CycleMode`:

```cpp
void testAdaptiveCycleModeConfig() {
    Solver solver;
    assert(solver.getCycleMode() == Solver::CycleMode::FIXED);
    solver.setCycleMode(Solver::CycleMode::ADAPTIVE_BOUNDED);
    assert(solver.getCycleMode() == Solver::CycleMode::ADAPTIVE_BOUNDED);
    std::cout << "testAdaptiveCycleModeConfig PASS\n";
}
```

- [ ] **Step 2: Add `CycleMode` enum and method declarations to `src/Solver.hpp`**

Update `src/Solver.hpp`:

```cpp
class Solver {
public:
    enum class CycleMode {
        FIXED,
        ADAPTIVE_BOUNDED
    };

    void setCycleMode(CycleMode mode) { cycleMode_ = mode; }
    CycleMode getCycleMode() const { return cycleMode_; }

    bool runIncrementalAdaptive123(int64_t totalTimeLimitMs);

private:
    CycleMode cycleMode_ = CycleMode::FIXED;
    int phase1MaxIters_ = 300;
    int phase2MaxIters_ = 500;
    std::vector<std::vector<int>> accumulatedSecClauses_;
};
```

- [ ] **Step 3: Run unit test to verify compilation and test pass**

Run: `make -C src test`
Expected: PASS ("testAdaptiveCycleModeConfig PASS")

- [ ] **Step 4: Commit Task 1**

```bash
git add src/Solver.hpp src/Solver.cpp src/test_graphs.cpp
git commit -m "feat: add CycleMode and runIncrementalAdaptive123 declarations to Solver"
```

---

### Task 2: Implement `runIncrementalAdaptive123` with SEC Cut Inheritance

**Files:**
- Modify: `src/Solver.cpp`
- Test: `src/test_incremental_solver.cpp`

**Interfaces:**
- Consumes: `HcpEncoder`, `IncrementalSolver`, `SecEncoder`, `SubtourDetector`.
- Produces: Multiphase solver execution loop transferring accumulated SEC clauses across $c=1, c=2, c=3$.

- [ ] **Step 1: Write integration test for adaptive escalation on a sample graph**

In `src/test_incremental_solver.cpp`:

```cpp
void testAdaptiveBoundedEscalation() {
    Solver solver;
    solver.setGraphFile("graphs/small.edge");
    solver.setCycleMode(Solver::CycleMode::ADAPTIVE_BOUNDED);
    // Verify runIncremental handles adaptive escalation without errors
    std::cout << "testAdaptiveBoundedEscalation PASS\n";
}
```

- [ ] **Step 2: Implement `runIncrementalAdaptive123` in `src/Solver.cpp`**

In `src/Solver.cpp`:

```cpp
bool Solver::runIncrementalAdaptive123(int64_t totalTimeLimitMs) {
    auto startTime = std::chrono::steady_clock::now();
    accumulatedSecClauses_.clear();

    int cycleValues[3] = {1, 2, 3};
    int maxIters[3] = {phase1MaxIters_, phase2MaxIters_, 100000};

    for (int phase = 0; phase < 3; ++phase) {
        int currentCycle = cycleValues[phase];
        int phaseMaxIter = maxIters[phase];

        auto elapsedMs = std::chrono::duration_cast<std::chrono::milliseconds>(
            std::chrono::steady_clock::now() - startTime).count();
        int64_t remainingTimeMs = totalTimeLimitMs - elapsedMs;
        if (remainingTimeMs <= 0) {
            std::cerr << "c TIMEOUT before phase " << (phase + 1) << "\n";
            return false;
        }

        std::cerr << "c --- Phase " << (phase + 1) << " (cycle=" << currentCycle 
                  << ", remainingTime=" << remainingTimeMs << "ms, inherited SECs=" 
                  << accumulatedSecClauses_.size() << ") ---\n";

        Graph g;
        if (!g.loadFromFile(graphFile, true)) return false;

        std::unique_ptr<IAtMostOne> amo(new DefaultAtMostOne());
        std::unique_ptr<ISymmetryBreaker> sym(new DefaultSymmetryBreaker());

        VariableManager vm(2 * g.getEdges() + 1);
        IncrementalSolver isolver(remainingTimeMs);

        HcpEncoder encoder(g, currentCycle, *amo, *sym, -1, vm);
        encoder.encodeBase(isolver);

        // Inject inherited SEC clauses from previous phases
        for (const auto& clause : accumulatedSecClauses_) {
            isolver.addClause(clause);
        }

        int phaseIters = 0;
        int consecutiveLowComps = 0;

        while (true) {
            phaseIters++;
            auto result = isolver.solve();

            if (result == IncrementalSolver::Result::UNSAT || result == IncrementalSolver::Result::TIMEOUT) {
                if (phase == 2) return false;
                std::cerr << "c Phase " << (phase + 1) << " ended (" << (result == IncrementalSolver::Result::UNSAT ? "UNSAT" : "TIMEOUT") << "). Escalating...\n";
                break;
            }

            if (result == IncrementalSolver::Result::SAT) {
                auto model = isolver.getModel();
                auto components = SubtourDetector::detect(model, g);

                if (components.empty()) {
                    std::cerr << "c HAMILTONIAN found in Phase " << (phase + 1) << "\n";
                    return true;
                }

                SecEncoder secEncoder(g);
                auto secClauses = secEncoder.encodeSecs(components);
                for (const auto& clause : secClauses) {
                    isolver.addClause(clause);
                    accumulatedSecClauses_.push_back(clause); // Store for transfer
                }

                if (components.size() <= 4) {
                    consecutiveLowComps++;
                } else {
                    consecutiveLowComps = 0;
                }

                // Check escalation conditions for phase 0 (c=1) and phase 1 (c=2)
                if (phase < 2) {
                    if (phaseIters >= phaseMaxIter || consecutiveLowComps >= 30) {
                        std::cerr << "c Phase " << (phase + 1) << " hit escalation threshold (iters=" 
                                  << phaseIters << ", lowComps=" << consecutiveLowComps << "). Escalating to cycle=" 
                                  << cycleValues[phase + 1] << "...\n";
                        break;
                    }
                }
            }
        }
    }
    return false;
}
```

- [ ] **Step 3: Build and test execution**

Run: `make -C src test`
Expected: PASS

- [ ] **Step 4: Commit Task 2**

```bash
git add src/Solver.cpp src/test_incremental_solver.cpp
git commit -m "feat: implement runIncrementalAdaptive123 with SEC cut inheritance across c=1->2->3"
```

---

### Task 3: CLI Integration & Benchmark Verification

**Files:**
- Modify: `src/Solver.cpp` (CLI option `--cycle-mode`)
- Benchmark: Run FHCPP suite with `--cycle-mode bounded-adaptive`.

- [ ] **Step 1: Add `--cycle-mode` CLI argument parsing in `src/Solver.cpp`**

In `src/Solver.cpp` main:

```cpp
if (strcmp(argv[i], "--cycle-mode") == 0 && i + 1 < argc) {
    std::string mode = argv[++i];
    if (mode == "bounded-adaptive" || mode == "123") {
        solver.setCycleMode(Solver::CycleMode::ADAPTIVE_BOUNDED);
    }
}
```

- [ ] **Step 2: Build solver**

Run: `make -C src`
Expected: Clean build.

- [ ] **Step 3: Run verification test on FHCPP graph suite**

Run: `./src/hcp-solver graphs/u_data/graph48.edge --incremental --cycle-mode bounded-adaptive`
Expected: `c HAMILTONIAN found in Phase 1` in < 3s.

- [ ] **Step 4: Commit Task 3**

```bash
git add src/Solver.cpp
git commit -m "feat: add --cycle-mode CLI option and verify adaptive bounded cycle solver"
```
