# Design Spec: Forced Edges and Component Min-Cut Preprocessing

**Date:** 2026-07-08

## 1. Context and Goals

We want to reduce the total runtime of the incremental SAT Hamiltonian Cycle Problem (HCP) solver to `< 100` seconds per graph for cycle multiplier = 2.
Specifically, we target the sparse, large graphs from the FHCPPP dataset (e.g., `graph424`, `graph446`, `graph470`) which have ~2500 vertices and ~4300 edges (average degree ~3.4) and currently timeout.

To achieve this, we introduce two preprocessing techniques and one in-loop stagnation mitigation strategy:
1. **Degree-2 Preprocessing**: Force selection of both incident edges of any degree-2 vertex.
2. **2-Edge-Cut Preprocessing**: Detect pairs of edges that form a cut. Force both edges to be selected and constrain their directions to be opposite.
3. **Component Contracted Min-Cut**: During stagnation, build a contracted graph where components are super-nodes, compute the minimum s-t cut between the smallest component and neighbors, and add the corresponding SEC.

---

## 2. Preprocessing Algorithms

### 2.1. Degree-2 Vertices
For any vertex $u$ with undirected degree 2, let its neighbors be $v_1, v_2$.
- In any Hamiltonian cycle, both undirected edges $\{u, v_1\}$ and $\{u, v_2\}$ must be selected.
- If $x_{u,v}$ represents the directed edge variable from $u$ to $v$:
  - We add the clause: `x_{u, v1} ∨ x_{v1, u}`
  - We add the clause: `x_{u, v2} ∨ x_{v2, u}`

### 2.2. Bridges and 2-Edge-Cuts
A bridge is an edge whose removal disconnects the graph. A graph with bridges has no Hamiltonian Cycle (UNSAT).
A 2-edge-cut is a pair of edges $\{e_1, e_2\}$ whose removal disconnects the graph.
- Any Hamiltonian cycle must cross the cut exactly twice (once in each direction).
- Thus, both undirected edges $e_1 = \{u_1, v_1\}$ and $e_2 = \{u_2, v_2\}$ must be selected, and their directions must be opposite.

#### 2-Edge-Cut Detection Algorithm:
For each undirected edge $e = \{u_1, v_1\}$ in the graph:
1. Temporarily disable $e$.
2. Run Tarjan's bridge-finding algorithm (DFS) on the remaining graph in $O(V+E)$ time.
3. For each bridge $e' = \{u_2, v_2\}$ found:
   - $\{e, e'\}$ is a 2-edge-cut.
4. Restore $e$.

Since $E \approx 4500$ and $V \approx 2500$, the total complexity is $O(E(V+E)) \approx 3 \times 10^7$ operations, taking $< 0.05$ seconds in C++.

#### Encoding 2-Edge-Cut:
Let $u_1, u_2$ be in the same connected component $A$ after removing $\{e_1, e_2\}$, and $v_1, v_2$ in component $B$.
1. Force both edges:
   - `x_{u1, v1} ∨ x_{v1, u1}`
   - `x_{u2, v2} ∨ x_{v2, u2}`
2. Force opposite directions:
   - `¬x_{u1, v1} ∨ x_{v2, u2}`
   - `x_{u1, v1} ∨ ¬x_{v2, u2}`
   - `¬x_{v1, u1} ∨ x_{u2, v2}`
   - `x_{v1, u1} ∨ ¬x_{u2, v2}`

---

## 3. Component Contracted Min-Cut

When stagnation is detected (Jaccard similarity of edges $\ge 0.85$ for $K$ consecutive iterations):
1. **Build Contracted Graph**:
   - Let $C_1, \dots, C_m$ be the current components.
   - Map each component $C_i$ to a super-node $S_i$.
   - Edges in the contracted graph exist between $S_i$ and $S_j$ if there is at least one edge in the original graph connecting a vertex in $C_i$ to a vertex in $C_j$. The capacity of the super-edge is the number of such connecting edges.
2. **Compute Min s-t Cut**:
   - Let $S_{small}$ be the super-node of the smallest component.
   - For each neighbor $S_j$ of $S_{small}$ in the contracted graph:
     - Run Edmonds-Karp min s-t cut algorithm on the contracted graph between $S_{small}$ and $S_j$.
     - Identify the cut $(A, B)$ where $S_{small} \in A$.
     - Let $V_A = \bigcup_{C_k \in A} C_k$ be the set of original vertices.
     - Add SEC clauses (incoming and outgoing) for the set $V_A$.
3. **Fallback**:
   - If no min-cut is found or if the solver is still stagnant, fallback to the existing `greedy` or `dfj` strategy.

---

## 4. Implementation Details

We will add a helper class/functions to:
- `src/SubtourDetector.hpp` / `src/SubtourDetector.cpp` or a new file to compute bridges, 2-edge-cuts, and contracted min-cuts.
- Update `src/Solver.cpp` to call these preprocessing functions after `HcpEncoder::encodeBase` and add the constraints to `IncrementalSolver`.
- Add a new `--stagnation-strategy mincut` option to wire the contracted min-cut strategy.

---

## 5. Verification Plan

1. **Unit Testing**:
   - Verify bridge detection correctly flags a graph with bridges as UNSAT.
   - Verify degree-2 and 2-edge-cut constraint generator on a small cycle or grid.
2. **Regression Testing**:
   - Run `src/run_experiments.py` on a subset of graphs to verify correctness (all decoded paths are valid Hamiltonian cycles).
3. **Performance Benchmarking**:
   - Measure time on `fhcppp/graph424`, `fhcppp/graph446`, `fhcppp/graph470` to verify runtime is $< 100$ seconds.
