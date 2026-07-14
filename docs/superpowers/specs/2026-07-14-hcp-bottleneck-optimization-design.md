# Design Spec: HCP Solver Bottleneck Optimizations

This document outlines the design and plan for optimizing performance bottlenecks in the C++ Hamiltonian Cycle Problem (HCP) solver.

## 1. Objectives

- **Reduce traversal overhead** in `SecEncoder::getIncomingLiterals` by switching from a full-graph $O(V + E)$ scan to a component-relative boundary traversal.
- **Speed up min-cut computation** in `computeInternalMinCut` by replacing the dense capacity matrix representation with a sparse adjacency representation, using Dinic for all graph sizes, and capping the number of sink vertices evaluated.

---

## 2. Detailed Designs

### 2.1 Optimization of `SecEncoder::getIncomingLiterals`
Currently, finding incoming edges to a component requires traversing all vertices in the graph to check if their neighbors lie within the component. This is highly inefficient for small components.

#### New Algorithm
We will change `getIncomingLiterals` to:
1. Build a boolean mask `inComponent` for the vertices in the component.
2. Iterate only over the vertices $v$ in `component.vertices`.
3. For each neighbor $u$ of $v$, check if $u \notin C$ using the mask.
4. If $u$ is outside the component, retrieve the directed edge index for $u \to v$ using `graph_.getAdj(u, v)`.
5. Add the edge index to the output vector.

This reduces complexity from $O(V + E)$ to $O(|C| \cdot \text{degree} \cdot \log(\text{degree}))$, making it extremely fast for small components.

---

### 2.2 Optimization of `computeInternalMinCut`
Currently, min-cut partition checks build a dense capacity matrix of size $k \times k$ (where $k$ is the component size) and run Edmonds-Karp (`maxFlowBFS`) repeatedly for all boundary vertices.

#### New Algorithm
1. **Unify Flow Algorithm**: Completely remove `maxFlowBFS` (Edmonds-Karp) and use `Dinic` for all graph sizes.
2. **Sparse Adjacency**: Build a sparse adjacency capacity representation (e.g., `std::vector<std::vector<std::pair<int, int>>>`) for vertices inside the component instead of a dense matrix.
3. **Optimized Dinic Construction**: Instantiate the `Dinic` solver directly from the sparse capacity representation, avoiding $O(k^2)$ loops.
4. **Cap Boundary Evaluated Sinks**: When searching for a min-cut separating the boundary, we set the source $s = \text{boundary}[0]$. Instead of testing all other boundary vertices as sinks $t$, we will cap the number of tested sinks to at most 10. We will sample the sinks evenly from the boundary list:
   ```cpp
   size_t step = std::max<size_t>(1, (boundary.size() - 1) / 10);
   for (size_t ti = 1; ti < boundary.size(); ti += step) {
       int t = boundary[ti];
       // ... run Dinic max-flow ...
   }
   ```
   This limits the number of max-flow runs to at most 10.

---

## 3. Verification & Testing

- **Correctness**: Run the existing unit tests (`test_incremental_solver`, `test_vertex_separator`) to ensure no regressions in correctness.
- **Performance**: Benchmark the solver on `graph48.edge` and `graph470.edge` using the benchmark script `scripts/run_experiments.py` to compare solve time and iteration overhead before and after the change.
