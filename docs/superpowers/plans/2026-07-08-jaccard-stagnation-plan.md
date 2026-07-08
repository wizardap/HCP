# Jaccard Stagnation Mitigation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement lightweight, solver-agnostic stagnation mitigation strategies (DFJ and Union SECs) to reduce total solving runtime and iterations.

**Architecture:** Add consecutive Jaccard similarity computation based on component edge variables in `Solver::runIncremental`. Implement three new strategies (`dfj` cycle-edge blocking, `union` component-union cuts, and `both` combination) to break stagnation loops without triggering extra SAT solver calls during escalation.

**Tech Stack:** C++17, CaDiCaL SAT Solver, Standard Template Library (std::set_intersection, std::vector, std::sort)

## Global Constraints

- Must compile with C++17 (`-std=c++17`).
- Do not modify CaDiCaL solver source code or its wrapper interface `ccadical.h`.
- Maintain full correctness of the Hamiltonian Cycle solver (all additions must be mathematically sound).
- Use exact file paths and verify compilation using `make -C src` at the end of each task.

---

### Task 1: Stagnation Strategy Support and Jaccard Computation

**Files:**
- Modify: `src/Solver.cpp:200-306`
- Test: Build `src/hcp-solver` and run it on a small graph

**Interfaces:**
- Consumes: `components` (`std::vector<Component>`) at each iteration.
- Produces: `jaccardSim` (`double`) representing the edge Jaccard similarity between iterations.

- [ ] **Step 1: Write code to compute edge Jaccard similarity**
  In [src/Solver.cpp](file:///home/ubuntu/HCP/src/Solver.cpp) inside `Solver::runIncremental` before the main loop, declare `prevComponents` to store components from the previous iteration. Inside the `Result::SAT` block, compute the edge-based Jaccard similarity:
  ```cpp
  // Add in src/Solver.cpp around line 200:
  std::vector<Component> prevComponents;
  ```
  And inside the `if (result == IncrementalSolver::Result::SAT)` block:
  ```cpp
  // Add after component detection:
  double jaccardSim = 0.0;
  if (!prevComponents.empty() && !components.empty()) {
      std::vector<int> prevEdges;
      for (const auto& comp : prevComponents) {
          prevEdges.insert(prevEdges.end(), comp.edges.begin(), comp.edges.end());
      }
      std::sort(prevEdges.begin(), prevEdges.end());
      prevEdges.erase(std::unique(prevEdges.begin(), prevEdges.end()), prevEdges.end());

      std::vector<int> currEdges;
      for (const auto& comp : components) {
          currEdges.insert(currEdges.end(), comp.edges.begin(), comp.edges.end());
      }
      std::sort(currEdges.begin(), currEdges.end());
      currEdges.erase(std::unique(currEdges.begin(), currEdges.end()), currEdges.end());

      std::vector<int> intersectionEdges;
      std::set_intersection(prevEdges.begin(), prevEdges.end(),
                            currEdges.begin(), currEdges.end(),
                            std::back_inserter(intersectionEdges));

      size_t unionSize = prevEdges.size() + currEdges.size() - intersectionEdges.size();
      if (unionSize > 0) {
          jaccardSim = static_cast<double>(intersectionEdges.size()) / unionSize;
      }
  }
  ```

- [ ] **Step 2: Update stagnation trigger condition**
  Update the stagnation detection trigger in [src/Solver.cpp](file:///home/ubuntu/HCP/src/Solver.cpp) to check if `jaccardSim >= 0.85` instead of `!changed`:
  ```cpp
  // Replace the stagnation check section:
  if (stagnationK > 0 && !components.empty()) {
      bool isStagnant = !prevComponents.empty() && (jaccardSim >= 0.85);

      if (!isStagnant) {
          stagnationCount = 0;
          escalated = false;
          escalationResult = "";
      } else {
          stagnationCount++;
          std::cerr << "c Stagnation count: " << stagnationCount
                    << "/" << stagnationK << " (Jaccard: " << jaccardSim << ")\n";
          
          if (stagnationCount >= stagnationK && !escalated) {
              escalated = true;
              std::cerr << "c Stagnation detected! Escalating with strategy: "
                        << stagnationStrategy << "\n";
              // Escalation actions to be implemented in Task 2
          }
      }
  }
  ```
  At the end of the iteration loop (right before `continue` or end of `SAT` block), assign:
  ```cpp
  prevComponents = components;
  ```

- [ ] **Step 3: Build the code to verify compilation**
  Run: `make -C src`
  Expected: Success compile of `hcp-solver`

- [ ] **Step 4: Commit changes**
  ```bash
  git add src/Solver.cpp
  git commit -m "feat: add edge Jaccard computation and stagnation count update"
  ```

---

### Task 2: Implement DFJ, Union, and Both Stagnation Strategies

**Files:**
- Modify: `src/Solver.cpp:250-290`
- Test: Build `src/hcp-solver` and run it on a small graph

**Interfaces:**
- Consumes: `stagnationStrategy` (`std::string`) configured via CLI option.
- Produces: Correct clauses added to `isolver` corresponding to `dfj`, `union`, or `both` strategy.

- [ ] **Step 1: Write the escalation action logic**
  In [src/Solver.cpp](file:///home/ubuntu/HCP/src/Solver.cpp) inside the `stagnationCount >= stagnationK && !escalated` block, replace the greedy check with the check for the new strategies:
  ```cpp
  if (stagnationStrategy == "dfj") {
      int addedCount = 0;
      for (const auto& comp : components) {
          if (comp.edges.empty()) continue;
          std::vector<int> clause;
          clause.reserve(comp.edges.size());
          for (int e : comp.edges) {
              clause.push_back(-e);
          }
          isolver.addClause(clause);
          addedCount++;
      }
      std::cerr << "c Escalation (DFJ): Added " << addedCount << " cycle-blocking clauses\n";
      escalationResult = "dfj_added";
      stagnationCount = 0;
  } 
  else if (stagnationStrategy == "union") {
      int addedCount = 0;
      SecEncoder secEncoder(g);
      std::vector<Component> sortedComps = components;
      std::sort(sortedComps.begin(), sortedComps.end());
      
      int P = std::min(3, static_cast<int>(sortedComps.size()));
      for (int a = 0; a < P; ++a) {
          for (int b = a + 1; b < P; ++b) {
              Component unionComp;
              unionComp.vertices = sortedComps[a].vertices;
              unionComp.vertices.insert(unionComp.vertices.end(), 
                                        sortedComps[b].vertices.begin(), 
                                        sortedComps[b].vertices.end());
              
              auto unionClauses = secEncoder.encodeSecs({unionComp});
              for (const auto& clause : unionClauses) {
                  isolver.addClause(clause);
                  addedCount++;
              }
          }
      }
      std::cerr << "c Escalation (Union): Added " << addedCount << " union SEC clauses\n";
      escalationResult = "union_added";
      stagnationCount = 0;
  }
  else if (stagnationStrategy == "both") {
      int addedDfj = 0;
      for (const auto& comp : components) {
          if (comp.edges.empty()) continue;
          std::vector<int> clause;
          clause.reserve(comp.edges.size());
          for (int e : comp.edges) {
              clause.push_back(-e);
          }
          isolver.addClause(clause);
          addedDfj++;
      }
      
      int addedUnion = 0;
      SecEncoder secEncoder(g);
      std::vector<Component> sortedComps = components;
      std::sort(sortedComps.begin(), sortedComps.end());
      
      int P = std::min(3, static_cast<int>(sortedComps.size()));
      for (int a = 0; a < P; ++a) {
          for (int b = a + 1; b < P; ++b) {
              Component unionComp;
              unionComp.vertices = sortedComps[a].vertices;
              unionComp.vertices.insert(unionComp.vertices.end(), 
                                        sortedComps[b].vertices.begin(), 
                                        sortedComps[b].vertices.end());
              
              auto unionClauses = secEncoder.encodeSecs({unionComp});
              for (const auto& clause : unionClauses) {
                  isolver.addClause(clause);
                  addedUnion++;
              }
          }
      }
      std::cerr << "c Escalation (Both): Added " << addedDfj << " DFJ and " << addedUnion << " union SEC clauses\n";
      escalationResult = "both_added";
      stagnationCount = 0;
  }
  else {
      // Fallback to greedy blocking
      if (runGreedyBlocking(components, isolver, g, prevFingerprint, prevBlockedComponentIds)) {
          escalationResult = "partition_changed";
          if (tracer) {
              std::vector<int> modelEdgeVars;
              int numVars = isolver.getNumVars();
              for (int v = 1; v <= numVars; ++v) {
                  if (isolver.getModelValue(v) > 0) {
                      modelEdgeVars.push_back(v);
                  }
              }
              tracer->logIteration(actions, actions, isolver.getFinalSolveTime(),
                                   totalTime, 0, 0, 0,
                                   components, modelEdgeVars, prevBlockedComponentIds,
                                   stagnationCount, escalated,
                                   stagnationStrategy, escalationResult);
          }
          prevComponents = components; // Update prevComponents
          continue; 
      } else {
          escalationResult = "failed";
      }
  }
  ```

- [ ] **Step 2: Update subsequent cycle iteration tracking**
  If `dfj`, `union`, or `both` are executed, we should add normal SEC clauses for the current components at the end of the iteration, or skip it?
  Since we also want normal SEC clauses to block the current components, we let the loop proceed to the end of the `SAT` block to add the standard SECs. This is handled naturally by letting the loop continue (we do not `continue` early for `dfj`, `union`, or `both` since they are *additional* constraints, unlike `greedy` which replaces the normal SEC addition with the new partition's SECs).

- [ ] **Step 3: Build the code**
  Run: `make -C src`
  Expected: Success compile.

- [ ] **Step 4: Commit changes**
  ```bash
  git add src/Solver.cpp
  git commit -m "feat: implement dfj, union, and both stagnation strategies"
  ```

---

### Task 3: Verification and Integration Testing

**Files:**
- Create: `graphs/small.edge` (if not exists)
- Modify: `src/test_graphs.cpp`

**Interfaces:**
- Consumes: Solver binary `src/hcp-solver`
- Produces: Correct solved Hamiltonian cycle.

- [ ] **Step 1: Test compile and execute unit tests**
  Run: `make -C src test`
  Expected: All unit tests pass.

- [ ] **Step 2: Test CLI on small graph with all strategies**
  Ensure we can solve `graphs/small.edge` using each strategy:
  Run:
  ```bash
  ./src/hcp-solver src/small.edge --incremental --stagnation-k 2 --stagnation-strategy dfj
  ./src/hcp-solver src/small.edge --incremental --stagnation-k 2 --stagnation-strategy union
  ./src/hcp-solver src/small.edge --incremental --stagnation-k 2 --stagnation-strategy both
  ```
  Expected: Solver exits with `s SATISFIABLE` or finishes successfully.

- [ ] **Step 3: Verify solutions**
  Decode solutions of the test runs:
  Run: `./src/hcp-solver -d solution.sat`
  Expected: Successful path decoding output.

- [ ] **Step 4: Run comprehensive benchmarks**
  Run the benchmark script to verify there are no performance regressions and compare strategies:
  Run: `python3 src/run_experiments.py` or compile individual test cases.

- [ ] **Step 5: Commit any test files**
  ```bash
  git commit -am "test: verify compilation and run hcp-solver stagnation strategies"
  ```
