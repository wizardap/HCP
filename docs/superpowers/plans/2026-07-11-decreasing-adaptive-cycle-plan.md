# Decreasing Adaptive Cycle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the increasing adaptive cycle (1→2→6→30→210) with a decreasing approach (30→6→2→1) using fresh-solver-per-level and SEC carry-forward.

**Architecture:** Start at cycle=30 (most discriminating, 6 bits/node). When subtour repeats, drop to cycle=6 then 2 then 1. Each level gets a fresh CaDiCaL solver + fresh HcpEncoder at that cycle. All SEC clauses ever generated are accumulated in a global store and replayed into each new solver. SEC clauses are encoding-independent (edge vars only), so they're valid at every cycle level.

**Tech Stack:** C++17, CaDiCaL SAT solver, DIMACS CNF encoding, CRE (Chinese Remainder Encoding), SEC (Subtour Elimination Constraints).

## Global Constraints

- Existing baseline mode (`--incremental` without `--adaptive-cycle`) must produce identical results
- No new dependencies
- No filesystem IO for SEC carry-forward

---

## File Structure

| File | Change |
|------|--------|
| `src/AdaptiveCycle.hpp` | Rename advance()→drop(), clear seenSignatures on drop, default sequence {30,6,2,1} |
| `src/Solver.hpp` | Update setAdaptiveCycle() default sequence, remove `cycle = adaptiveCycle->current()` |
| `src/Solver.cpp` | Add fresh-solver outer loop, SEC store, replay SEC at each level |

---

### Task 1: Rewrite AdaptiveCycle class

**Files:**
- Modify: `src/AdaptiveCycle.hpp` — full rewrite

**Interfaces:**
- Consumes: nothing from other tasks
- Produces: `AdaptiveCycle` with `{30,6,2,1}` sequence, `drop()` semantics, per-level repeat tracking

- [ ] Replace the entire file content:

```cpp
#ifndef ADAPTIVECYCLE_HPP
#define ADAPTIVECYCLE_HPP

#include <vector>
#include <set>

class AdaptiveCycle {
public:
    AdaptiveCycle(const std::vector<int>& sequence)
        : seq_(sequence), idx_(0) {}

    int current() const { return seq_[idx_]; }

    bool canDrop() const { return idx_ + 1 < seq_.size(); }

    // Move to next cheaper cycle level. Clears repeat signatures
    // for independent per-level tracking.
    int drop() {
        if (canDrop()) {
            idx_++;
            seenSignatures_.clear();
        }
        return current();
    }

    int numLevels() const { return seq_.size(); }

    const std::vector<int>& sequence() const { return seq_; }

    // Returns true if this exact subtour signature was seen at current level
    bool isRepeat(const std::vector<int>& componentIds) {
        auto [it, inserted] = seenSignatures_.insert(componentIds);
        return !inserted;
    }

    void resetSeen() { seenSignatures_.clear(); }

    void reset() {
        idx_ = 0;
        seenSignatures_.clear();
    }

private:
    std::vector<int> seq_;
    size_t idx_;
    std::set<std::vector<int>> seenSignatures_;
};

#endif
```

- [ ] **Commit**

```bash
git add src/AdaptiveCycle.hpp
git commit -m "refactor: replace advance() with drop(), default sequence {30,6,2,1}"
```

---

### Task 2: Update Solver.hpp

**Files:**
- Modify: `src/Solver.hpp` — change setAdaptiveCycle default sequence

- [ ] Change `setAdaptiveCycle` to use the decreasing sequence:

```cpp
    void setAdaptiveCycle(bool enabled) {
        if (enabled) {
            adaptiveCycle.reset(new AdaptiveCycle({30, 6, 2, 1}));
            // cycle member stays at default (2) for non-adaptive mode
        }
    }
```

- [ ] **Commit**

```bash
git add src/Solver.hpp
git commit -m "feat: update adaptive cycle default sequence to decreasing {30,6,2,1}"
```

---

### Task 3: Rewrite Solver::runIncremental() — fresh-solver-per-level

**Files:**
- Modify: `src/Solver.cpp:52-178` — full runIncremental rewrite

**Interfaces:**
- Consumes: AdaptiveCycle with `{30,6,2,1}`, HcpEncoder::encodeBase(IncrementalSolver&)
- Produces: `bool` (SAT/not), global SEC vector carried across levels

- [ ] **Write the new runIncremental** — replace the body of `bool Solver::runIncremental(int64_t timeLimitMs)`:

```cpp
bool Solver::runIncremental(int64_t timeLimitMs) {
    Graph g;
    if (!g.loadFromFile(graphFile, true)) {
        std::cerr << "c Error: could not open graph file " << graphFile << "\n";
        return false;
    }

    std::unique_ptr<IAtMostOne> amo;
    if (amoOption == AtMostOneOption::PBLIB) {
        amo.reset(new PbLibAtMostOne());
    } else {
        amo.reset(new DefaultAtMostOne());
    }

    std::unique_ptr<ISymmetryBreaker> sym;
    if (symOption == SymmetryOption::DEFAULT) {
        sym.reset(new DefaultSymmetryBreaker());
    } else if (symOption == SymmetryOption::NONE) {
        sym.reset(new NoSymmetryBreaker());
    }

    int sNode = -1;
    if (startNodeOption == StartNodeOption::MIN_DEGREE) sNode = -1;
    else if (startNodeOption == StartNodeOption::MAX_DEGREE) sNode = -2;
    else if (startNodeOption == StartNodeOption::FIRST_NODE) sNode = -3;
    else if (startNodeOption == StartNodeOption::SPECIFIC_NODE) sNode = specificStartNode;

    if (adaptiveCycle) {
        // === Decreasing adaptive cycle mode ===
        std::vector<std::vector<int>> globalSecClauses;
        int totalActions = 0;

        for (size_t levelIdx = 0; levelIdx < adaptiveCycle->numLevels(); levelIdx++) {
            int currentCycle = adaptiveCycle->sequence()[levelIdx];
            VariableManager vm(2 * g.getEdges() + 1);
            IncrementalSolver isolver(timeLimitMs);
            HcpEncoder encoder(g, currentCycle, *amo, *sym, sNode, vm, currentCycle);
            encoder.encodeBase(isolver);

            // Replay all accumulated SEC clauses into the fresh solver
            for (const auto& clause : globalSecClauses) {
                isolver.addClause(clause);
            }

            adaptiveCycle->resetSeen();

            std::cerr << "c cycle " << currentCycle << ": "
                      << isolver.getNumVars() << " vars, "
                      << isolver.getNumClauses() << " clauses\n";

            int actions = 0;
            bool dropLevel = false;

            while (!dropLevel) {
                actions++;
                totalActions++;
                auto result = isolver.solve();

                if (result == IncrementalSolver::Result::UNSAT) {
                    std::cerr << "c UNSAT at cycle " << currentCycle << "\n";
                    std::cerr << "c incremental actions: " << totalActions << "\n";
                    std::cerr << "c total variables: " << isolver.getNumVars() << "\n";
                    std::cerr << "c total clauses: " << isolver.getNumClauses() << "\n";
                    std::cerr << "c final solve time: " << isolver.getFinalSolveTime() << "\n";
                    std::cerr << "c total solver time: " << isolver.getTotalSolverTime() << "\n";
                    isolver.printStatistics();
                    return false;
                }

                if (result == IncrementalSolver::Result::TIMEOUT) {
                    std::cerr << "c TIMEOUT at cycle " << currentCycle << "\n";
                    std::cerr << "c incremental actions: " << totalActions << "\n";
                    std::cerr << "c total variables: " << isolver.getNumVars() << "\n";
                    std::cerr << "c total clauses: " << isolver.getNumClauses() << "\n";
                    std::cerr << "c final solve time: " << isolver.getFinalSolveTime() << "\n";
                    std::cerr << "c total solver time: " << isolver.getTotalSolverTime() << "\n";
                    isolver.printStatistics();
                    return false;
                }

                if (result == IncrementalSolver::Result::SAT) {
                    auto model = isolver.getModel();
                    auto components = SubtourDetector::detect(model, g);

                    if (components.empty()) {
                        std::cerr << "c HAMILTONIAN found at cycle " << currentCycle << "\n";
                        std::string solFile = "solution.sat";
                        std::ofstream solOut(solFile);
                        if (!solOut.is_open() || solOut.fail()) {
                            std::cerr << "c Error: Could not write solution to " << solFile << "\n";
                            return false;
                        }
                        solOut << "s SATISFIABLE\nv ";
                        for (int var = 1; var <= isolver.getNumVars(); ++var) {
                            int val = isolver.getModelValue(var);
                            if (val > 0) solOut << var << " ";
                            else if (val < 0) solOut << -var << " ";
                        }
                        solOut << "0\n";
                        if (solOut.fail()) {
                            std::cerr << "c Error: Failed while writing solution to " << solFile << "\n";
                            solOut.close();
                            return false;
                        }
                        solOut.close();
                        std::cerr << "c incremental actions: " << totalActions << "\n";
                        std::cerr << "c total variables: " << isolver.getNumVars() << "\n";
                        std::cerr << "c total clauses: " << isolver.getNumClauses() << "\n";
                        std::cerr << "c final solve time: " << isolver.getFinalSolveTime() << "\n";
                        std::cerr << "c total solver time: " << isolver.getTotalSolverTime() << "\n";
                        isolver.printStatistics();
                        return true;
                    }

                    // Check for repeat stagnation
                    for (const auto& comp : components) {
                        std::vector<int> vertexIds;
                        for (int v : comp.vertices) {
                            vertexIds.push_back(v);
                        }
                        std::sort(vertexIds.begin(), vertexIds.end());

                        if (adaptiveCycle->isRepeat(vertexIds)) {
                            dropLevel = true;
                            break;
                        }
                    }

                    if (!dropLevel) {
                        SecEncoder secEncoder(g);
                        auto secClauses = secEncoder.encodeSecs(components);
                        for (const auto& clause : secClauses) {
                            isolver.addClause(clause);
                            globalSecClauses.push_back(clause);
                        }
                        std::cerr << "c Iteration: found " << components.size()
                                  << " components, added " << secClauses.size()
                                  << " SEC clauses (cycle " << currentCycle << ")\n";
                    } else {
                        std::cerr << "c cycle " << currentCycle << " -> " 
                                  << adaptiveCycle->sequence()[levelIdx + 1] << " (repeat detected)\n";
                    }
                }
            }
        }

        // Exhausted all levels without finding Hamiltonian
        std::cerr << "c All cycles exhausted without solution\n";
        std::cerr << "c incremental actions: " << totalActions << "\n";
        return false;
    }

    // === Non-adaptive incremental mode (unchanged) ===
    // Original code below...

    // [Keep the entire existing non-adaptive incremental code here,
    //  starting from VariableManager creation and the original loop]
    VariableManager vm(2 * g.getEdges() + 1);
    IncrementalSolver isolver(timeLimitMs);
    HcpEncoder encoder(g, cycle, *amo, *sym, sNode, vm);
    encoder.encodeBase(isolver);

    std::cerr << "c total variables: " << isolver.getNumVars() << "\n";
    std::cerr << "c total clauses: " << isolver.getNumClauses() << "\n";

    int actions = 0;
    while (true) {
        actions++;
        auto result = isolver.solve();
        if (result == IncrementalSolver::Result::UNSAT) {
            std::cerr << "c UNSAT\n";
            std::cerr << "c incremental actions: " << actions << "\n";
            std::cerr << "c total variables: " << isolver.getNumVars() << "\n";
            std::cerr << "c total clauses: " << isolver.getNumClauses() << "\n";
            std::cerr << "c final solve time: " << isolver.getFinalSolveTime() << "\n";
            std::cerr << "c total solver time: " << isolver.getTotalSolverTime() << "\n";
            isolver.printStatistics();
            return false;
        }
        if (result == IncrementalSolver::Result::TIMEOUT) {
            std::cerr << "c TIMEOUT\n";
            std::cerr << "c incremental actions: " << actions << "\n";
            std::cerr << "c total variables: " << isolver.getNumVars() << "\n";
            std::cerr << "c total clauses: " << isolver.getNumClauses() << "\n";
            std::cerr << "c final solve time: " << isolver.getFinalSolveTime() << "\n";
            std::cerr << "c total solver time: " << isolver.getTotalSolverTime() << "\n";
            isolver.printStatistics();
            return false;
        }
        if (result == IncrementalSolver::Result::SAT) {
            auto model = isolver.getModel();
            auto components = SubtourDetector::detect(model, g);
            if (components.empty()) {
                std::cerr << "c HAMILTONIAN found\n";
                std::string solFile = "solution.sat";
                std::ofstream solOut(solFile);
                if (!solOut.is_open() || solOut.fail()) {
                    std::cerr << "c Error: Could not write solution to " << solFile << "\n";
                    return false;
                }
                solOut << "s SATISFIABLE\nv ";
                for (int var = 1; var <= isolver.getNumVars(); ++var) {
                    int val = isolver.getModelValue(var);
                    if (val > 0) {
                        solOut << var << " ";
                    } else if (val < 0) {
                        solOut << -var << " ";
                    }
                }
                solOut << "0\n";
                if (solOut.fail()) {
                    std::cerr << "c Error: Failed while writing solution to " << solFile << "\n";
                    solOut.close();
                    return false;
                }
                solOut.close();

                std::cerr << "c incremental actions: " << actions << "\n";
                std::cerr << "c total variables: " << isolver.getNumVars() << "\n";
                std::cerr << "c total clauses: " << isolver.getNumClauses() << "\n";
                std::cerr << "c final solve time: " << isolver.getFinalSolveTime() << "\n";
                std::cerr << "c total solver time: " << isolver.getTotalSolverTime() << "\n";
                isolver.printStatistics();
                return true;
            } else {
                SecEncoder secEncoder(g);
                auto secClauses = secEncoder.encodeSecs(components);
                for (const auto& clause : secClauses) {
                    isolver.addClause(clause);
                }
                std::cerr << "c Iteration: found " << components.size()
                          << " components, added " << secClauses.size() << " SEC clauses\n";
            }
        }
    }
}
```

- [ ] **Build**

```bash
make -C src
```

Expected: clean compilation (no errors, only pre-existing pblib warnings).

- [ ] **Run non-adaptive baseline** to verify no regression:

```bash
src/hcp-solver /tmp/grid_4x4.edge --incremental --time-limit 10
```

Expected: "c HAMILTONIAN found"

- [ ] **Run decreasing adaptive on 4x4 grid**:

```bash
src/grid-graph 4 4 > /tmp/grid_4x4.edge 2>/dev/null
src/hcp-solver /tmp/grid_4x4.edge --incremental --time-limit 10 --adaptive-cycle
```

Expected: "c cycle 30: ..." then (possibly drops) then "c HAMILTONIAN found at cycle ..."

- [ ] **Commit**

```bash
git add src/Solver.cpp
git commit -m "feat: rewrite runIncremental with decreasing adaptive cycle + fresh-solver-per-level"
```

---

### Task 4: Run tests and verify

- [ ] **Build and run unit tests**:

```bash
make -C src test_graphs
cd src && ./test_graphs
```

Expected: "All graph tests passed successfully!"

- [ ] **Verify CLI help** shows `--adaptive-cycle` flag:

```bash
src/hcp-solver --help
```

Expected: output includes `--adaptive-cycle`

- [ ] **Run graph48 baseline** to confirm no regression:

```bash
src/hcp-solver graphs/graph48.edge --incremental --time-limit 30
```

Expected: "c HAMILTONIAN found"

- [ ] **Commit**

```bash
git commit --allow-empty -m "chore: verify decreasing adaptive cycle tests pass"
```

---

### Task 5: Benchmark on fhcppp dataset

- [ ] **Run on all 18 fhcppp graphs** with 120s limit:

```bash
for f in graphs/fhcppp/*.edge; do
  name=$(basename "$f" .edge)
  echo "=== $name ==="
  timeout 135 src/hcp-solver "$f" --incremental --time-limit 120 --adaptive-cycle 2>&1 \
    | grep -E "(c cycle |c HAMILTONIAN|c TIMEOUT|c UNSAT|c All cycles|c total solver|c total variables|c total clauses|c incremental actions)"
  echo ""
done
```

Record results per graph: status, solve time, which cycle level solved at (or timed out).

- [ ] **Compare with previous benchmark**:
  - Did any graph that timed out before (graph491, graph529) now solve?
  - Is total time lower on graphs where decreasing avoids the 210 cost?
  - Are there new regressions?

- [ ] **Commit results** (if significant):

```bash
git add docs/superpowers/plans/2026-07-11-decreasing-adaptive-cycle-plan.md
git commit -m "docs: add benchmark results for decreasing adaptive cycle"
```
