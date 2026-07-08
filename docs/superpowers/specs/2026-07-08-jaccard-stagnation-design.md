# Jaccard Stagnation Mitigation with DFJ and Union SECs

**Date:** 2026-07-08

## 1. Context and Problem Statement

The existing stagnation mitigation strategy, `greedy` (implemented in [runGreedyBlocking](file:///home/ubuntu/HCP/src/Solver.cpp#L76)), resolves stagnation by running a nested loop of SAT solver calls. In each iteration of this inner loop, the SAT solver is run under assumptions to block components one by one.

While this strategy effectively breaks stagnation loops and resolves TIMEOUT cases, it introduces a **high escalation overhead** due to multiple nested SAT solver calls per stagnation trigger. For large graphs, these nested solves significantly increase the **total runtime**, even if the number of outer iterations (actions) is reduced.

We need **lightweight, solver-agnostic** stagnation mitigation strategies that:
1. Do not modify or affect the internals of the CaDiCaL SAT solver.
2. Require **zero extra SAT solves** during the escalation step (eliminating the nested loop).
3. Do not cause excessive clause database bloat which would slow down future solves.

## 2. Proposed Design: New Stagnation Strategies

We propose three new stagnation strategies to be configured via the existing `--stagnation-strategy` option:

1. **`dfj`** (Dantzig-Fulkerson-Johnson Cycle-Edge Blocking)
2. **`union`** (Adaptive Component Union SECs)
3. **`both`** (Combination of `dfj` and `union` strategies)

### 2.1. Dantzig-Fulkerson-Johnson (DFJ) Cycle-Edge Blocking (`dfj`)

Currently, [SecEncoder::encodeSecs](file:///home/ubuntu/HCP/src/SecEncoder.cpp#L6) adds **cut-set** constraints (e.g., at least one outgoing and one incoming edge must be selected for each component $C$). 

When stagnation occurs, the solver satisfies the cut-set constraint by changing only 1 or 2 edges at the boundary of $C$, leaving the rest of the cycle intact and causing high Jaccard similarity in the next iteration.

To prevent this, the `dfj` strategy adds a **cycle-edge-blocking clause** (the DFJ formulation) for each component $C$:
$$\bigvee_{e \in E_C} \neg x_e$$
where $E_C$ is the set of edge variables selected within component $C$.

*   **Soundness**: This constraint is mathematically sound. In a Hamiltonian cycle, there can be no cycles of length $< N$ (subtours). Since $C$ is a subtour ($|C| < N$), it cannot have all its edges selected simultaneously.
*   **Performance Impact**: These clauses are very short (length equal to the component size $|C|$) and are extremely fast for the SAT solver to process via unit propagation, introducing zero overhead while forcing the solver to break the specific cycle structure.

### 2.2. Adaptive Component Union SECs (`union`)

When stagnation is detected, the solver is often rotating edges between a few small components. We can force these components to connect globally by adding cut-set SECs on their union.

Let $C_1, C_2, \dots, C_m$ be the detected components in the current iteration, sorted by size (number of vertices).
1. Select the $P$ smallest components (we set $P = 3$ to keep clauses short).
2. For each pair $(C_a, C_b)$ where $1 \le a < b \le \min(P, m)$:
   * Form the union vertex set $S = C_a \cup C_b$.
   * Since $S$ is a proper subset of all vertices ($0 < |S| < |V|$), it must satisfy the cut-set constraints.
   * Generate and add:
     - Outgoing SEC: $\sum_{u \in S, v \notin S} x_{u,v} \ge 1$
     - Incoming SEC: $\sum_{u \notin S, v \in S} x_{u,v} \ge 1$

*   **Soundness**: Sound because $S$ is a proper subset of vertices, and any Hamiltonian cycle must enter and leave any proper subset of vertices.
*   **Performance Impact**: Adding cut-set constraints for unions of components forces the solver to globally merge the components, preventing minor local edge-swapping.

### 2.3. Combined Strategy (`both`)

The `both` strategy adds both the `dfj` cycle-edge blocking clauses and the `union` SEC clauses when stagnation is triggered, leveraging the benefits of both representations.

## 3. Detailed Implementation Plan

### 3.1. Extend Solver Class Options

In `src/Solver.hpp`:
* Expose and document the new strategies in the class definition. (Already supported via `stagnationStrategy` string).

### 3.2. Update `runIncremental` in `src/Solver.cpp`

Modify the stagnation detection section in [Solver::runIncremental](file:///home/ubuntu/HCP/src/Solver.cpp#L157):

```cpp
            // ----- STAGNATION DETECTION -----
            if (stagnationK > 0 && !components.empty()) {
                bool changed = prevFingerprint.empty() || partitionChanged(prevFingerprint, components);

                if (changed) {
                    prevFingerprint = computeFingerprint(components);
                    stagnationCount = 0;
                    escalated = false;
                    escalationResult = "";
                } else {
                    stagnationCount++;
                    std::cerr << "c Stagnation count: " << stagnationCount
                              << "/" << stagnationK << "\n";

                    if (stagnationCount >= stagnationK && !escalated) {
                        escalated = true;
                        std::cerr << "c Stagnation detected! Escalating with strategy: "
                                  << stagnationStrategy << "\n";

                        if (stagnationStrategy == "dfj") {
                            // Apply DFJ blocking clauses
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
                            stagnationCount = 0; // Reset count
                            
                            // Log and continue to next iteration
                            if (tracer) {
                                // Log trajectory iteration details...
                            }
                            continue;
                        } 
                        else if (stagnationStrategy == "union") {
                            // Apply Adaptive Component Union SECs
                            int addedCount = 0;
                            SecEncoder secEncoder(g);
                            
                            // Sort components to find the smallest ones
                            std::vector<Component> sortedComps = components;
                            std::sort(sortedComps.begin(), sortedComps.end());
                            
                            int P = std::min(3, static_cast<int>(sortedComps.size()));
                            for (int a = 0; a < P; ++a) {
                                for (int b = a + 1; b < P; ++b) {
                                    Component unionComp;
                                    // Merge vertices
                                    unionComp.vertices = sortedComps[a].vertices;
                                    unionComp.vertices.insert(unionComp.vertices.end(), 
                                                              sortedComps[b].vertices.begin(), 
                                                              sortedComps[b].vertices.end());
                                    
                                    // Generate outgoing & incoming clauses for the union component
                                    auto unionClauses = secEncoder.encodeSecs({unionComp});
                                    for (const auto& clause : unionClauses) {
                                        isolver.addClause(clause);
                                        addedCount++;
                                    }
                                }
                            }
                            std::cerr << "c Escalation (Union): Added " << addedCount << " union SEC clauses\n";
                            escalationResult = "union_added";
                            stagnationCount = 0; // Reset count
                            
                            // Log and continue to next iteration
                            if (tracer) {
                                // Log trajectory iteration details...
                            }
                            continue;
                        }
                        else if (stagnationStrategy == "both") {
                            // Apply both DFJ and Union SECs
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
                            stagnationCount = 0; // Reset count
                            
                            // Log and continue to next iteration
                            if (tracer) {
                                // Log trajectory iteration details...
                            }
                            continue;
                        }
                        else if (runGreedyBlocking(components, isolver, g,
                                              prevFingerprint, prevBlockedComponentIds)) {
                            // Existing greedy fallback
                            // ...
                        }
                    }
                }
            }
```

## 4. Verification and Benchmarking Plan

1. **Compilation Check**:
   Build the project using `make -C src` to ensure clean compilation.
2. **Functional Correctness**:
   * Run the solver on small grid-graphs using different strategies:
     ```bash
     src/hcp-solver graphs/grid_5x5.edge --incremental --stagnation-k 3 --stagnation-strategy dfj
     src/hcp-solver graphs/grid_5x5.edge --incremental --stagnation-k 3 --stagnation-strategy union
     src/hcp-solver graphs/grid_5x5.edge --incremental --stagnation-k 3 --stagnation-strategy both
     ```
   * Decode solutions using `src/hcp-solver -d solution.sat` to verify they form valid Hamiltonian cycles.
3. **Performance Benchmarking**:
   * Run the experimental suite `src/run_experiments.py` or a targeted benchmark subset to compare:
     - `greedy` strategy vs `dfj` vs `union` vs `both` vs `none` (no stagnation mitigation).
     - Metrics to measure: **Total Runtime (seconds)**, **Number of Iterations (actions)**, and **CNF Size (variables/clauses)**.
