# HCP Solver Bottleneck Optimizations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Optimize performance bottlenecks in getIncomingLiterals traversal and internal min-cut flow computations in the C++ HCP solver.

**Architecture:** Switch `getIncomingLiterals` to traverse only the component boundary, and switch `computeInternalMinCut` to use a sparse adjacency list capacity representation with Dinic max-flow, capped at 10 boundary sink nodes.

**Tech Stack:** C++17, CaDiCaL SAT solver

## Global Constraints

- Must compile and build successfully using the existing Makefile.
- All unit tests must pass.
- No regression in correctness or final solve validity.

---

### Task 1: Optimize getIncomingLiterals Traversal

**Files:**
- Modify: `src/SecEncoder.cpp`
- Modify: `src/test_incremental_solver.cpp`

**Interfaces:**
- Consumes: None
- Produces: Optimized `SecEncoder::getIncomingLiterals`

- [ ] **Step 1: Write a unit test verifying getIncomingLiterals correctness via encodeSecs**
  Add a new test `testGetIncomingLiterals` to `src/test_incremental_solver.cpp` and register it in `main`.
  ```cpp
  void testGetIncomingLiterals() {
      std::cout << "Testing getIncomingLiterals correctness...\n";
      Graph g(3, 3);
      g.addEdge(0, 1, 1); g.addEdge(1, 0, 2);
      g.addEdge(1, 2, 3); g.addEdge(2, 1, 4);
      g.addEdge(2, 0, 5); g.addEdge(0, 2, 6);

      Component c;
      c.vertices = {1, 2};
      
      SecEncoder secEncoder(g);
      auto clauses = secEncoder.encodeSecs({c}, false);
      TEST_ASSERT(clauses.size() == 2);
      TEST_ASSERT(clauses[0] == std::vector<int>({2, 5}));
      TEST_ASSERT(clauses[1] == std::vector<int>({1, 6}));
      std::cout << "testGetIncomingLiterals passed!\n";
  }
  ```
  Register in `main()` of `src/test_incremental_solver.cpp`:
  ```cpp
  testGetIncomingLiterals();
  ```

- [ ] **Step 2: Run test to verify it passes with current implementation**
  Run: `make -C src && ./src/test_incremental_solver`
  Expected: PASS

- [ ] **Step 3: Modify getIncomingLiterals to return an empty vector to verify test fails**
  In `src/SecEncoder.cpp` modify the function to:
  ```cpp
  std::vector<int> SecEncoder::getIncomingLiterals(const Component& component) {
      return {};
  }
  ```
  Run: `make -C src && ./src/test_incremental_solver`
  Expected: FAIL (assertion on clauses size or content)

- [ ] **Step 4: Implement optimized boundary traversal**
  In `src/SecEncoder.cpp`:
  ```cpp
  std::vector<int> SecEncoder::getIncomingLiterals(const Component& component) {
      int numNodes = graph_.getNodes();
      std::vector<bool> inComponent(numNodes, false);
      int totalDegree = 0;
      for (int v : component.vertices) {
          if (v >= 0 && v < numNodes) {
              inComponent[v] = true;
              totalDegree += graph_.getDegree(v);
          }
      }

      std::vector<int> literals;
      literals.reserve(totalDegree);

      for (int v : component.vertices) {
          if (v < 0 || v >= numNodes) continue;
          for (auto& [u, _] : graph_.getNeighbors(v)) {
              if (u >= 0 && u < numNodes && !inComponent[u]) {
                  int edgeIdx = graph_.getAdj(u, v);
                  if (edgeIdx > 0) {
                      literals.push_back(edgeIdx);
                  }
              }
          }
      }
      return literals;
  }
  ```

- [ ] **Step 5: Run tests to verify the optimized version passes**
  Run: `make -C src && ./src/test_incremental_solver`
  Expected: PASS

- [ ] **Step 6: Commit**
  Run: `git commit -am "feat: optimize getIncomingLiterals boundary traversal"`

---

### Task 2: Optimize computeInternalMinCut Flow Computations

**Files:**
- Modify: `src/ContractedMinCut.cpp`
- Modify: `src/test_incremental_solver.cpp`

**Interfaces:**
- Consumes: `computeInternalMinCut`
- Produces: Optimized `computeInternalMinCut` with sparse representation and Dinic max-flow, capped at 10 boundary sink nodes.

- [ ] **Step 1: Write a unit test for computeInternalMinCut**
  Add a new test `testInternalMinCut` to `src/test_incremental_solver.cpp` and register it in `main`.
  ```cpp
  void testInternalMinCut() {
      std::cout << "Testing computeInternalMinCut...\n";
      Graph g2(6, 6);
      g2.addEdge(0, 1); g2.addEdge(1, 0);
      g2.addEdge(1, 2); g2.addEdge(2, 1);
      g2.addEdge(2, 3); g2.addEdge(3, 2);
      g2.addEdge(3, 0); g2.addEdge(0, 3);
      g2.addEdge(0, 4); g2.addEdge(4, 0);
      g2.addEdge(3, 5); g2.addEdge(5, 3);

      Component comp2;
      comp2.vertices = {0, 1, 2, 3};
      
      auto mcr = computeInternalMinCut(comp2, g2, 100);
      TEST_ASSERT(mcr.cutSize == 2);
      TEST_ASSERT(mcr.sideA_vertices.size() >= 1);
      TEST_ASSERT(mcr.sideA_vertices.size() <= 3);
      std::cout << "testInternalMinCut passed!\n";
  }
  ```
  Register in `main()` of `src/test_incremental_solver.cpp`:
  ```cpp
  testInternalMinCut();
  ```

- [ ] **Step 2: Run test to verify it passes with current implementation**
  Run: `make -C src && ./src/test_incremental_solver`
  Expected: PASS

- [ ] **Step 3: Modify computeInternalMinCut to return empty MinCutResult to verify test fails**
  In `src/ContractedMinCut.cpp` modify the function to:
  ```cpp
  MinCutResult computeInternalMinCut(
      const Component& component,
      const Graph& graph,
      int maxFlowVertLimit
  ) {
      return {};
  }
  ```
  Run: `make -C src && ./src/test_incremental_solver`
  Expected: FAIL (assertion on cutSize)

- [ ] **Step 4: Implement sparse internal min-cut and capped Dinic**
  In `src/ContractedMinCut.cpp`, update `computeInternalMinCut` to:
  ```cpp
  MinCutResult computeInternalMinCut(
      const Component& component,
      const Graph& graph,
      int maxFlowVertLimit
  ) {
      int k = static_cast<int>(component.vertices.size());
      if (k < 4 || k > maxFlowVertLimit) return {};

      // Map component vertices to local indices 0..k-1
      std::vector<int> globalToLocal(graph.getNodes(), -1);
      for (int i = 0; i < k; ++i) {
          int v = component.vertices[i];
          if (v >= 0 && v < static_cast<int>(graph.getNodes())) {
              globalToLocal[v] = i;
          }
      }

      auto& localToGlobal = component.vertices;

      // Build sparse capacity representation for edges WITHIN the component
      std::vector<std::vector<std::pair<int, int>>> localAdj(k);
      for (int vi = 0; vi < k; ++vi) {
          int u = localToGlobal[vi];
          if (u < 0) continue;
          for (auto& [v, _] : graph.getNeighbors(u)) {
              int vj = globalToLocal[v];
              if (vj >= 0 && vj != vi) {
                  localAdj[vi].push_back({vj, 1});
              }
          }
      }

      // Find boundary vertices
      std::vector<int> boundary;
      for (int vi = 0; vi < k; ++vi) {
          int u = localToGlobal[vi];
          for (auto& [v, _] : graph.getNeighbors(u)) {
              int vj = globalToLocal[v];
              if (vj < 0) {
                  boundary.push_back(vi);
                  break;
              }
          }
      }

      if (boundary.size() < 2) return {};

      int s = boundary[0];
      MinCutResult best;
      best.cutSize = std::numeric_limits<int>::max();

      // Cap boundary sinks to at most 10
      size_t maxSinks = 10;
      size_t step = std::max<size_t>(1, (boundary.size() - 1) / maxSinks);

      for (size_t ti = 1; ti < boundary.size(); ti += step) {
          int t = boundary[ti];
          
          Dinic dinic(k);
          for (int u = 0; u < k; ++u) {
              for (auto& [v, capVal] : localAdj[u]) {
                  dinic.addEdge(u, v, capVal);
              }
          }

          int flowVal = dinic.maxFlow(s, t);
          std::vector<bool> sideA_local = dinic.minCut(s);

          if (flowVal > 0 && flowVal < best.cutSize) {
              best.cutSize = flowVal;
              best.sideA_vertices.clear();
              for (int vi = 0; vi < k; ++vi) {
                  if (sideA_local[vi]) {
                      best.sideA_vertices.push_back(localToGlobal[vi]);
                  }
              }

              if (best.sideA_vertices.empty() ||
                  static_cast<int>(best.sideA_vertices.size()) >= k) {
                  best.cutSize = std::numeric_limits<int>::max();
                  best.sideA_vertices.clear();
              }
          }
      }

      if (best.cutSize == std::numeric_limits<int>::max()) return {};
      return best;
  }
  ```

- [ ] **Step 5: Run tests to verify the optimized version passes**
  Run: `make -C src && ./src/test_incremental_solver && ./src/test_vertex_separator`
  Expected: PASS

- [ ] **Step 6: Commit**
  Run: `git commit -am "feat: optimize internal min-cut flow computation and sparse representation"`
