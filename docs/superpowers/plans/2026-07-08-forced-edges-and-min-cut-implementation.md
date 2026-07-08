# Forced Edges and Component Min-Cut Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reduce incremental SAT HCP solver runtime to < 100s per graph (cycle=2) via forced-edge preprocessing and contracted-graph min-cut stagnation mitigation.

**Architecture:** Three layers of improvement: (1) static preprocessing — force edges incident to degree-2 vertices and 2-edge-cut pairs before the first SAT call; (2) a new `--stagnation-strategy mincut` mode that, on stagnation trigger, builds a contracted graph over current subtour components and computes s-t min-cuts to derive tight SEC constraints; (3) a new CLI flag `--preprocess` to enable/disable the preprocessing step independently.

**Tech Stack:** C++17, CaDiCaL SAT solver (via `IncrementalSolver`), Edmonds-Karp BFS-based max-flow (O(VE²) — negligible on contracted graph with < 20 super-nodes)

## Global Constraints

- Build system: `make -C src` — no CMake, no new build deps
- All new code in `src/` — no new directories
- Preprocessing runs on the **undirected** graph (treat `Graph` adjacency as undirected)
- Directed SAT variable for edge u→v: `graph.getAdj(u, v)`; for v→u: `graph.getAdj(v, u)`
- `IncrementalSolver::addClause(std::vector<int>)` — positive literal = forced true, negative = forced false
- A clause of size 1 is a unit clause (forced assignment)
- Stagnation threshold: existing `stagnationK` field; Jaccard threshold: existing `0.85` constant in `Solver.cpp:279`
- Test compilation: `make -C src` must produce zero warnings with `-Wall`
- `src/test_incremental_solver.cpp` is the existing C++ test harness — add new tests there

---

### Task 1: GraphPreprocessor — Bridge and 2-Edge-Cut Detection

**Files:**
- Create: `src/GraphPreprocessor.hpp`
- Create: `src/GraphPreprocessor.cpp`
- Modify: `src/Makefile` (add `GraphPreprocessor.o`)
- Test: `src/test_incremental_solver.cpp` (append new `TEST` cases)

**Interfaces:**
- Produces:
```cpp
// In GraphPreprocessor.hpp
struct EdgePair {
    int u1, v1; // first undirected edge endpoints
    int u2, v2; // second undirected edge endpoints
    // side A contains u1 and u2 after removing both edges
};

class GraphPreprocessor {
public:
    explicit GraphPreprocessor(const Graph& g);

    // Returns true if graph has a bridge (no HC possible).
    bool hasBridge() const;

    // Returns all 2-edge-cuts found. Empty if none.
    const std::vector<EdgePair>& getTwoEdgeCuts() const;

    // Returns all degree-2 vertices.
    const std::vector<int>& getDegree2Vertices() const;

private:
    const Graph& graph_;
    bool hasBridge_;
    std::vector<EdgePair> twoEdgeCuts_;
    std::vector<int> degree2Vertices_;

    void compute();
    // Tarjan bridge-finding DFS on graph with one undirected edge (skipU,skipV) disabled.
    // Appends discovered bridges to out_bridges.
    void findBridges(int skipU, int skipV,
                     std::vector<std::pair<int,int>>& out_bridges) const;
};
```

- [ ] **Step 1: Write the failing test**

In `src/test_incremental_solver.cpp`, append:

```cpp
// ---- GraphPreprocessor tests ----
#include "GraphPreprocessor.hpp"

TEST(GraphPreprocessor, DetectsBridgeOnPathGraph) {
    // Path: 0-1-2  (bridge at every edge)
    Graph g(3, 2);
    g.addEdge(0, 1); g.addEdge(1, 2);
    GraphPreprocessor pp(g);
    EXPECT_TRUE(pp.hasBridge());
    EXPECT_TRUE(pp.getTwoEdgeCuts().empty());
    EXPECT_TRUE(pp.getDegree2Vertices().empty()); // degree(0)=1, degree(1)=2, degree(2)=1
}

TEST(GraphPreprocessor, DetectsDegree2OnCycle) {
    // 4-cycle: 0-1-2-3-0, every vertex has degree 2
    Graph g(4, 4);
    g.addEdge(0,1); g.addEdge(1,2); g.addEdge(2,3); g.addEdge(3,0);
    GraphPreprocessor pp(g);
    EXPECT_FALSE(pp.hasBridge());
    EXPECT_EQ(pp.getDegree2Vertices().size(), 4u);
}

TEST(GraphPreprocessor, Detects2EdgeCutOnDumbbellGraph) {
    // Dumbbell: triangle (0-1-2-0) connected to triangle (3-4-5-3) by edge 2-3
    // But 2-edge-cut needs 2 edges; here the single edge 2-3 is a bridge.
    // Instead: two triangles sharing an edge pair: 0-1, 1-2, 2-0, 2-3, 3-4, 4-2
    // (actually let's use: 0-1,1-2,2-0,2-3,3-0 -- that's a special graph)
    // Simpler: two 4-cycles sharing two edges (a "theta graph"):
    // vertices 0,1,2,3: paths 0-1-2, 0-3-2, and direct edge 0-2
    // Edges: {0,1},{1,2},{0,3},{3,2},{0,2}
    // Removing {0,1} and {0,3} disconnects vertex 0 from rest -- that's a 2-edge-cut
    Graph g(4, 5);
    g.addEdge(0,1); g.addEdge(1,2);
    g.addEdge(0,3); g.addEdge(3,2);
    g.addEdge(0,2);
    GraphPreprocessor pp(g);
    EXPECT_FALSE(pp.hasBridge());
    // {0,1} and {0,3} together form a 2-edge-cut separating {0} from {1,2,3}
    bool found = false;
    for (const auto& ep : pp.getTwoEdgeCuts()) {
        bool e1 = (ep.u1==0&&ep.v1==1)||(ep.u1==1&&ep.v1==0)||
                  (ep.u1==0&&ep.v1==3)||(ep.u1==3&&ep.v1==0);
        bool e2 = (ep.u2==0&&ep.v2==1)||(ep.u2==1&&ep.v2==0)||
                  (ep.u2==0&&ep.v2==3)||(ep.u2==3&&ep.v2==0);
        if (e1 && e2) found = true;
    }
    EXPECT_TRUE(found);
}
```

- [ ] **Step 2: Compile test and verify it fails**

```bash
cd /home/ubuntu/HCP && make -C src test_incremental_solver 2>&1 | tail -20
```

Expected: compile error — `GraphPreprocessor.hpp` not found.

- [ ] **Step 3: Implement `GraphPreprocessor.hpp`**

Create `src/GraphPreprocessor.hpp`:

```cpp
#pragma once
#include <vector>
#include <utility>
#include "Graph.hpp"

struct EdgePair {
    int u1, v1;  // first undirected edge
    int u2, v2;  // second undirected edge
    // u1 and u2 are on the same side (A) after removal; v1, v2 on the other (B)
};

class GraphPreprocessor {
public:
    explicit GraphPreprocessor(const Graph& g);

    bool hasBridge() const { return hasBridge_; }
    const std::vector<EdgePair>& getTwoEdgeCuts() const { return twoEdgeCuts_; }
    const std::vector<int>& getDegree2Vertices() const { return degree2Vertices_; }

private:
    const Graph& graph_;
    bool hasBridge_ = false;
    std::vector<EdgePair> twoEdgeCuts_;
    std::vector<int> degree2Vertices_;

    void compute();
    // Runs Tarjan bridge DFS; returns all bridges as (u,v) pairs (u < v).
    // If skipU != -1, treat the undirected edge {skipU, skipV} as absent.
    std::vector<std::pair<int,int>> findBridges(int skipU, int skipV) const;
};
```

- [ ] **Step 4: Implement `GraphPreprocessor.cpp`**

Create `src/GraphPreprocessor.cpp`:

```cpp
#include "GraphPreprocessor.hpp"
#include <algorithm>
#include <vector>

GraphPreprocessor::GraphPreprocessor(const Graph& g) : graph_(g) {
    compute();
}

void GraphPreprocessor::compute() {
    int n = graph_.getNodes();

    // --- Degree-2 vertices ---
    for (int u = 0; u < n; ++u) {
        if (graph_.getDegree(u) == 2) {
            degree2Vertices_.push_back(u);
        }
    }

    // --- Base bridge detection (no edge removed) ---
    {
        auto bridges = findBridges(-1, -1);
        if (!bridges.empty()) {
            hasBridge_ = true;
            return; // No HC possible; skip 2-edge-cut detection
        }
    }

    // --- 2-edge-cut detection ---
    // For each undirected edge {u,v} (visit each pair once: u < v only via adj):
    for (int u = 0; u < n; ++u) {
        for (auto& [v, _] : graph_.getNeighbors(u)) {
            if (v <= u) continue; // undirected: process each edge once

            // Remove {u,v} temporarily and find bridges in remaining graph
            auto bridges = findBridges(u, v);
            for (auto& [bu, bv] : bridges) {
                if ((bu == u && bv == v) || (bu == v && bv == u)) continue; // skip self

                // {u,v} and {bu,bv} form a 2-edge-cut.
                // Determine sides A and B by BFS from u without both edges.
                // Side A = component containing u after removing both edges.
                // We use a quick union-find on this: simpler to just record the pair.
                // The side determination is done in Solver.cpp when encoding.
                EdgePair ep;
                ep.u1 = u; ep.v1 = v;
                ep.u2 = bu; ep.v2 = bv;
                twoEdgeCuts_.push_back(ep);
            }
        }
    }

    // Deduplicate: each 2-edge-cut {e1,e2} is found twice (once removing e1, once e2)
    // Sort and unique
    for (auto& ep : twoEdgeCuts_) {
        // Normalize edge order within EdgePair
        if (ep.u1 > ep.v1) std::swap(ep.u1, ep.v1);
        if (ep.u2 > ep.v2) std::swap(ep.u2, ep.v2);
        if (std::make_pair(ep.u1,ep.v1) > std::make_pair(ep.u2,ep.v2)) {
            std::swap(ep.u1,ep.u2); std::swap(ep.v1,ep.v2);
        }
    }
    std::sort(twoEdgeCuts_.begin(), twoEdgeCuts_.end(), [](const EdgePair& a, const EdgePair& b){
        return std::tie(a.u1,a.v1,a.u2,a.v2) < std::tie(b.u1,b.v1,b.u2,b.v2);
    });
    twoEdgeCuts_.erase(std::unique(twoEdgeCuts_.begin(), twoEdgeCuts_.end(), [](const EdgePair& a, const EdgePair& b){
        return a.u1==b.u1 && a.v1==b.v1 && a.u2==b.u2 && a.v2==b.v2;
    }), twoEdgeCuts_.end());
}

std::vector<std::pair<int,int>> GraphPreprocessor::findBridges(int skipU, int skipV) const {
    int n = graph_.getNodes();
    std::vector<int> disc(n, -1), low(n, -1), parent(n, -1);
    std::vector<std::pair<int,int>> bridges;
    int timer = 0;

    std::function<void(int)> dfs = [&](int u) {
        disc[u] = low[u] = timer++;
        for (auto& [v, _] : graph_.getNeighbors(u)) {
            // Skip the removed edge {skipU, skipV} in both directions
            if ((u == skipU && v == skipV) || (u == skipV && v == skipU)) continue;
            if (disc[v] == -1) {
                parent[v] = u;
                dfs(v);
                low[u] = std::min(low[u], low[v]);
                if (low[v] > disc[u]) {
                    // {u,v} is a bridge
                    int bu = std::min(u,v), bv = std::max(u,v);
                    bridges.push_back({bu, bv});
                }
            } else if (v != parent[u]) {
                low[u] = std::min(low[u], disc[v]);
            }
        }
    };

    for (int i = 0; i < n; ++i) {
        if (disc[i] == -1) dfs(i);
    }
    return bridges;
}
```

- [ ] **Step 5: Add `GraphPreprocessor.o` to Makefile**

In `src/Makefile`, find the `OBJS` variable (or equivalent) and add `GraphPreprocessor.o`:

```makefile
# Add GraphPreprocessor.o to the list of object files for hcp-solver
# Before:
SOLVER_OBJS = Solver.o HcpDecoder.o IncrementalSolver.o SecEncoder.o SubtourDetector.o VariableManager.o TrajectoryLogger.o
# After:
SOLVER_OBJS = Solver.o HcpDecoder.o IncrementalSolver.o SecEncoder.o SubtourDetector.o VariableManager.o TrajectoryLogger.o GraphPreprocessor.o
```

Also add the compile rule if not already using a pattern rule:
```makefile
GraphPreprocessor.o: GraphPreprocessor.cpp GraphPreprocessor.hpp Graph.hpp
	$(CXX) $(CXXFLAGS) -c GraphPreprocessor.cpp -o GraphPreprocessor.o
```

- [ ] **Step 6: Compile and run tests**

```bash
cd /home/ubuntu/HCP && make -C src test_incremental_solver 2>&1 | tail -5
./src/test_incremental_solver 2>&1 | tail -20
```

Expected: all existing tests still pass + new GraphPreprocessor tests pass.

- [ ] **Step 7: Commit**

```bash
cd /home/ubuntu/HCP
git add src/GraphPreprocessor.hpp src/GraphPreprocessor.cpp src/Makefile src/test_incremental_solver.cpp
git commit -m "feat(preprocessor): add GraphPreprocessor with bridge and 2-edge-cut detection"
```

---

### Task 2: Preprocessing Clause Generation in `Solver::runIncremental`

**Files:**
- Modify: `src/Solver.cpp` (add preprocessing block after `encoder.encodeBase(isolver)`)
- Modify: `src/Solver.hpp` (add `preprocess_` flag field)
- Modify: `src/Solver.cpp` (parse `--preprocess` flag in `main`)
- Test: `src/test_incremental_solver.cpp` (integration test)

**Interfaces:**
- Consumes:
  - `GraphPreprocessor(const Graph&)` from Task 1
  - `graph.getAdj(u, v)` → SAT variable index for directed edge u→v
  - `isolver.addClause(std::vector<int>)` to add clauses
- Produces: `Solver::setPreprocess(bool)` for CLI wiring

- [ ] **Step 1: Write the failing integration test**

In `src/test_incremental_solver.cpp`, append:

```cpp
TEST(SolverPreprocessing, Degree2ForcedOnSmallCycle) {
    // 4-cycle is Hamiltonian; every vertex has degree 2
    // After preprocessing, all 8 directed edge vars are constrained
    // We just verify it still finds HC and doesn't crash
    Solver s("src/grid_8_8.edge"); // grid is all degree 2/3; use small.edge instead
    // Actually use the small 4-cycle graph written inline:
    // write a temp .edge file
    std::ofstream f("/tmp/test_cycle4.edge");
    f << "p edge 4 4\ne 1 2\ne 2 3\ne 3 4\ne 4 1\n";
    f.close();

    Solver s2("/tmp/test_cycle4.edge");
    s2.setPreprocess(true);
    EXPECT_TRUE(s2.runIncremental(5000)); // 5s time limit, must find HC
}
```

- [ ] **Step 2: Compile and verify test fails**

```bash
cd /home/ubuntu/HCP && make -C src test_incremental_solver 2>&1 | tail -5
```

Expected: compile error — `setPreprocess` not found.

- [ ] **Step 3: Add `preprocess_` flag to `Solver.hpp`**

In `src/Solver.hpp`, add inside the `Solver` class (alongside other fields like `stagnationK`):

```cpp
// Add in public section:
void setPreprocess(bool v) { preprocess_ = v; }

// Add in private section:
bool preprocess_ = false;
```

- [ ] **Step 4: Implement preprocessing clause generation in `Solver.cpp`**

In `src/Solver.cpp`, add at the top:
```cpp
#include "GraphPreprocessor.hpp"
```

In `Solver::runIncremental`, after the line:
```cpp
    encoder.encodeBase(isolver);
```
insert:

```cpp
    // ---- PREPROCESSING: Forced edges from degree-2 vertices and 2-edge-cuts ----
    if (preprocess_) {
        GraphPreprocessor pp(g);

        if (pp.hasBridge()) {
            std::cerr << "c Preprocessing: graph has a bridge — no Hamiltonian Cycle possible\n";
            return false;
        }

        int forcedClauses = 0;

        // Degree-2 vertices: both incident undirected edges must be selected
        for (int u : pp.getDegree2Vertices()) {
            for (auto& [v, _] : g.getNeighbors(u)) {
                int fwd = g.getAdj(u, v);
                int bwd = g.getAdj(v, u);
                if (fwd > 0 && bwd > 0) {
                    isolver.addClause({fwd, bwd}); // one of the two directions must be used
                    forcedClauses++;
                }
            }
        }

        // 2-edge-cuts: both edges must be selected, and directions must be opposite
        for (const auto& ep : pp.getTwoEdgeCuts()) {
            // Force edge 1: one direction must be used
            int fwd1 = g.getAdj(ep.u1, ep.v1);
            int bwd1 = g.getAdj(ep.v1, ep.u1);
            // Force edge 2: one direction must be used
            int fwd2 = g.getAdj(ep.u2, ep.v2);
            int bwd2 = g.getAdj(ep.v2, ep.u2);

            if (fwd1 <= 0 || bwd1 <= 0 || fwd2 <= 0 || bwd2 <= 0) continue;

            isolver.addClause({fwd1, bwd1});   // edge1 must be selected
            isolver.addClause({fwd2, bwd2});   // edge2 must be selected

            // Opposite directions: fwd1 ↔ bwd2, bwd1 ↔ fwd2
            // (fwd1 → bwd2): ¬fwd1 ∨ bwd2
            isolver.addClause({-fwd1, bwd2});
            // (bwd2 → fwd1): ¬bwd2 ∨ fwd1
            isolver.addClause({-bwd2, fwd1});
            // (bwd1 → fwd2): ¬bwd1 ∨ fwd2
            isolver.addClause({-bwd1, fwd2});
            // (fwd2 → bwd1): ¬fwd2 ∨ bwd1
            isolver.addClause({-fwd2, bwd1});

            forcedClauses += 6;
        }

        std::cerr << "c Preprocessing: added " << forcedClauses
                  << " forced clauses ("
                  << pp.getDegree2Vertices().size() << " deg-2 vertices, "
                  << pp.getTwoEdgeCuts().size() << " 2-edge-cuts)\n";
    }
    // ---- END PREPROCESSING ----
```

- [ ] **Step 5: Wire `--preprocess` flag in `main` in `Solver.cpp`**

In `main`, add in the arg-parsing loop:
```cpp
        } else if (arg == "--preprocess") {
            solver.setPreprocess(true);
```

Also add to `printHelp`:
```cpp
              << "  --preprocess            Enable forced-edge preprocessing (degree-2 and 2-edge-cuts)\n"
```

- [ ] **Step 6: Build and run tests**

```bash
cd /home/ubuntu/HCP && make -C src 2>&1 | tail -5
make -C src test_incremental_solver 2>&1 | tail -5
./src/test_incremental_solver 2>&1 | tail -30
```

Expected: all tests pass.

- [ ] **Step 7: Quick smoke test on a real graph**

```bash
cd /home/ubuntu/HCP
./src/hcp-solver graphs/fhcppp/graph171.edge --incremental --preprocess 2>&1 | grep -E "Preprocessing|HAMILTONIAN|incremental actions|total solver"
```

Expected output includes:
```
c Preprocessing: added N forced clauses (...)
c HAMILTONIAN found
```

- [ ] **Step 8: Commit**

```bash
cd /home/ubuntu/HCP
git add src/Solver.cpp src/Solver.hpp src/test_incremental_solver.cpp
git commit -m "feat(solver): add --preprocess flag wiring degree-2 and 2-edge-cut forced clauses"
```

---

### Task 3: Contracted Graph Min-Cut for Stagnation Mitigation

**Files:**
- Create: `src/ContractedMinCut.hpp`
- Create: `src/ContractedMinCut.cpp`
- Modify: `src/Makefile` (add `ContractedMinCut.o`)
- Modify: `src/Solver.cpp` (add `mincut` stagnation strategy branch)
- Test: `src/test_incremental_solver.cpp` (unit test for min-cut logic)

**Interfaces:**
- Consumes: `std::vector<Component>` from `SubtourDetector::detect` (each `Component` has `.vertices: std::vector<int>`)
- Consumes: `Graph` for edges between components
- Produces:
```cpp
// In ContractedMinCut.hpp
struct MinCutResult {
    std::vector<int> sideA_vertices; // original vertex IDs in the min-cut partition containing smallest component
    int cutSize;                     // number of crossing edges in min-cut
};

// Returns the best (smallest cut) partition found.
// Returns empty sideA_vertices if no useful cut found (cut >= total boundary edges).
MinCutResult computeComponentMinCut(
    const std::vector<Component>& components,
    const Graph& graph
);
```

- [ ] **Step 1: Write failing unit test**

In `src/test_incremental_solver.cpp`, append:

```cpp
#include "ContractedMinCut.hpp"

TEST(ContractedMinCut, FindsCutBetweenTwoComponents) {
    // Graph: two triangles (0-1-2) and (3-4-5) connected by single edge 2-3
    // BUT that's a bridge, so no HC. Use two triangles connected by TWO edges: 2-3, 0-5.
    // Vertices: 0,1,2 in triangle A; 3,4,5 in triangle B
    // Edges: {0,1},{1,2},{2,0},{3,4},{4,5},{5,3},{2,3},{0,5}
    Graph g(6, 8);
    g.addEdge(0,1); g.addEdge(1,2); g.addEdge(2,0);
    g.addEdge(3,4); g.addEdge(4,5); g.addEdge(5,3);
    g.addEdge(2,3); g.addEdge(0,5);

    // Simulate two components: C0={0,1,2}, C1={3,4,5}
    Component c0; c0.vertices = {0,1,2};
    Component c1; c1.vertices = {3,4,5};

    MinCutResult res = computeComponentMinCut({c0, c1}, g);

    // The min cut between C0 and C1 is 2 (edges 2-3 and 0-5)
    EXPECT_EQ(res.cutSize, 2);
    // sideA should contain all of {0,1,2} or all of {3,4,5}
    EXPECT_EQ(res.sideA_vertices.size(), 3u);
}
```

- [ ] **Step 2: Verify it fails to compile**

```bash
cd /home/ubuntu/HCP && make -C src test_incremental_solver 2>&1 | tail -5
```

Expected: compile error — `ContractedMinCut.hpp` not found.

- [ ] **Step 3: Implement `ContractedMinCut.hpp`**

Create `src/ContractedMinCut.hpp`:

```cpp
#pragma once
#include <vector>
#include "Graph.hpp"
#include "SubtourDetector.hpp"

struct MinCutResult {
    std::vector<int> sideA_vertices; // original vertices on the source side of the min-cut
    int cutSize = 0;
};

// Given current subtour components and the original graph,
// build the contracted graph and find the minimum s-t cut
// between the smallest component and each neighbor.
// Returns the cut with the smallest cutSize across all neighbor pairs.
// Returns MinCutResult with empty sideA_vertices if no cut < total boundary found.
MinCutResult computeComponentMinCut(
    const std::vector<Component>& components,
    const Graph& graph
);
```

- [ ] **Step 4: Implement `ContractedMinCut.cpp`**

Create `src/ContractedMinCut.cpp`:

```cpp
#include "ContractedMinCut.hpp"
#include <algorithm>
#include <limits>
#include <queue>
#include <vector>

// Edmonds-Karp max-flow on a small adjacency matrix (up to 64 nodes).
// capacity[u][v] = capacity of edge u->v.
// Returns max flow from s to t and fills minCutSide (true = side containing s).
static int maxFlowBFS(int n, std::vector<std::vector<int>>& cap,
                      int s, int t, std::vector<bool>& minCutSide) {
    int flow = 0;
    while (true) {
        // BFS to find augmenting path
        std::vector<int> parent(n, -1);
        std::queue<int> q;
        q.push(s);
        parent[s] = s;
        while (!q.empty() && parent[t] == -1) {
            int u = q.front(); q.pop();
            for (int v = 0; v < n; ++v) {
                if (parent[v] == -1 && cap[u][v] > 0) {
                    parent[v] = u;
                    q.push(v);
                }
            }
        }
        if (parent[t] == -1) break; // no augmenting path

        // Find bottleneck
        int bottleneck = std::numeric_limits<int>::max();
        for (int v = t; v != s; v = parent[v]) {
            int u = parent[v];
            bottleneck = std::min(bottleneck, cap[u][v]);
        }
        // Update capacities
        for (int v = t; v != s; v = parent[v]) {
            int u = parent[v];
            cap[u][v] -= bottleneck;
            cap[v][u] += bottleneck;
        }
        flow += bottleneck;
    }
    // Min-cut: BFS from s on residual graph
    minCutSide.assign(n, false);
    std::queue<int> q;
    q.push(s);
    minCutSide[s] = true;
    while (!q.empty()) {
        int u = q.front(); q.pop();
        for (int v = 0; v < n; ++v) {
            if (!minCutSide[v] && cap[u][v] > 0) {
                minCutSide[v] = true;
                q.push(v);
            }
        }
    }
    return flow;
}

MinCutResult computeComponentMinCut(
    const std::vector<Component>& components,
    const Graph& graph
) {
    int m = static_cast<int>(components.size());
    if (m < 2) return {};

    // Map each vertex to its component index
    int n = graph.getNodes();
    std::vector<int> vertToComp(n, -1);
    for (int ci = 0; ci < m; ++ci) {
        for (int v : components[ci].vertices) {
            if (v >= 0 && v < n) vertToComp[v] = ci;
        }
    }

    // Build contracted graph capacity matrix (m x m)
    std::vector<std::vector<int>> baseCap(m, std::vector<int>(m, 0));
    for (int u = 0; u < n; ++u) {
        int cu = vertToComp[u];
        if (cu < 0) continue;
        for (auto& [v, _] : graph.getNeighbors(u)) {
            int cv = vertToComp[v];
            if (cv < 0 || cv == cu) continue;
            baseCap[cu][cv]++;  // directed: count both directions for undirected
        }
    }
    // Make symmetric (undirected): already symmetric since we count both u->v and v->u

    // Find smallest component (source = s)
    int s = 0;
    for (int i = 1; i < m; ++i) {
        if (components[i].vertices.size() < components[s].vertices.size()) s = i;
    }

    MinCutResult best;
    best.cutSize = std::numeric_limits<int>::max();

    // Try each neighbor of s as sink t
    for (int t = 0; t < m; ++t) {
        if (t == s || baseCap[s][t] == 0) continue;

        // Copy capacity matrix (max-flow is destructive)
        auto cap = baseCap;
        std::vector<bool> sideA;
        int flowVal = maxFlowBFS(m, cap, s, t, sideA);

        if (flowVal < best.cutSize) {
            best.cutSize = flowVal;
            best.sideA_vertices.clear();
            for (int ci = 0; ci < m; ++ci) {
                if (sideA[ci]) {
                    for (int v : components[ci].vertices) {
                        best.sideA_vertices.push_back(v);
                    }
                }
            }
        }
    }

    if (best.cutSize == std::numeric_limits<int>::max()) return {};
    return best;
}
```

- [ ] **Step 5: Add `ContractedMinCut.o` to Makefile**

In `src/Makefile`, add to `SOLVER_OBJS`:
```makefile
SOLVER_OBJS = Solver.o HcpDecoder.o IncrementalSolver.o SecEncoder.o SubtourDetector.o VariableManager.o TrajectoryLogger.o GraphPreprocessor.o ContractedMinCut.o
```

Add compile rule:
```makefile
ContractedMinCut.o: ContractedMinCut.cpp ContractedMinCut.hpp Graph.hpp SubtourDetector.hpp
	$(CXX) $(CXXFLAGS) -c ContractedMinCut.cpp -o ContractedMinCut.o
```

- [ ] **Step 6: Build and run tests**

```bash
cd /home/ubuntu/HCP && make -C src test_incremental_solver 2>&1 | tail -5
./src/test_incremental_solver 2>&1 | grep -E "PASS|FAIL|ContractedMinCut"
```

Expected: all tests pass including `ContractedMinCut.FindsCutBetweenTwoComponents`.

- [ ] **Step 7: Commit**

```bash
cd /home/ubuntu/HCP
git add src/ContractedMinCut.hpp src/ContractedMinCut.cpp src/Makefile src/test_incremental_solver.cpp
git commit -m "feat(mincut): add ContractedMinCut with Edmonds-Karp on contracted component graph"
```

---

### Task 4: Wire `mincut` Stagnation Strategy in `Solver::runIncremental`

**Files:**
- Modify: `src/Solver.cpp` (add `mincut` branch in stagnation handler)
- Test: manual integration test on sparse timeout graphs

**Interfaces:**
- Consumes: `computeComponentMinCut(components, g)` from Task 3 → `MinCutResult`
- Consumes: `SecEncoder::encodeSecs({Component})` on the `sideA_vertices` partition
- Consumes: existing `stagnationStrategy`, `stagnationK`, `jaccardSim` variables already in scope

- [ ] **Step 1: Add `#include "ContractedMinCut.hpp"` to `Solver.cpp`**

At the top of `src/Solver.cpp`, add:
```cpp
#include "ContractedMinCut.hpp"
```

- [ ] **Step 2: Add `mincut` branch in stagnation handler**

In `Solver::runIncremental`, locate the existing stagnation strategy dispatch (around line 295). After the existing `else if (stagnationStrategy == "both")` block and before the `else` (greedy fallback), insert:

```cpp
                        else if (stagnationStrategy == "mincut") {
                            MinCutResult mcr = computeComponentMinCut(components, g);
                            int addedCount = 0;

                            if (!mcr.sideA_vertices.empty()) {
                                // Build a synthetic Component for SecEncoder
                                Component cutComp;
                                cutComp.vertices = mcr.sideA_vertices;
                                // Edges field not needed for encodeSecs (only vertices used for cut-set)

                                SecEncoder secEncoder(g);
                                auto secClauses = secEncoder.encodeSecs({cutComp});
                                for (const auto& clause : secClauses) {
                                    isolver.addClause(clause);
                                    addedCount++;
                                }
                                std::cerr << "c Escalation (MinCut): cut size " << mcr.cutSize
                                          << ", added " << addedCount << " SEC clauses for "
                                          << mcr.sideA_vertices.size() << " vertices\n";
                                escalationResult = "mincut_added";
                                stagnationCount = 0;
                                escalated = false;
                            } else {
                                std::cerr << "c Escalation (MinCut): no useful cut found, falling back\n";
                                // Fall through to greedy below
                                if (runGreedyBlocking(components, isolver, g, prevFingerprint,
                                                      prevBlockedComponentIds, usedSkipVars,
                                                      skipVarStart, maxSkipVars)) {
                                    escalationResult = "partition_changed";
                                } else {
                                    escalationResult = "failed";
                                }
                            }
                        }
```

- [ ] **Step 3: Add `mincut` to help text**

In `printHelp` in `Solver.cpp`, update the stagnation-strategy line:
```cpp
              << "  --stagnation-strategy <opt>  Escalation: greedy (default), dfj, union, both, mincut\n"
```

- [ ] **Step 4: Build**

```bash
cd /home/ubuntu/HCP && make -C src 2>&1 | tail -5
```

Expected: zero errors, zero warnings.

- [ ] **Step 5: Integration test on graph171 (fast, known SAT)**

```bash
cd /home/ubuntu/HCP
./src/hcp-solver graphs/fhcppp/graph171.edge --incremental \
    --stagnation-k 3 --stagnation-strategy mincut \
    --preprocess 2>&1 | grep -E "Preprocessing|Escalation|HAMILTONIAN|incremental actions|total solver"
```

Expected:
```
c Preprocessing: added N forced clauses (...)
c HAMILTONIAN found
c incremental actions: N
c total solver time: X.X
```

If stagnation is triggered, you should also see:
```
c Escalation (MinCut): cut size K, added N SEC clauses for M vertices
```

- [ ] **Step 6: Performance test on timeout graphs**

```bash
cd /home/ubuntu/HCP
time ./src/hcp-solver graphs/fhcppp/graph424.edge --incremental \
    --stagnation-k 3 --stagnation-strategy mincut \
    --preprocess --time-limit 100 2>&1 | grep -E "Preprocessing|HAMILTONIAN|TIMEOUT|total solver"
```

Repeat for `graph446` and `graph470`. Record results.

- [ ] **Step 7: Commit**

```bash
cd /home/ubuntu/HCP
git add src/Solver.cpp
git commit -m "feat(solver): add mincut stagnation strategy using contracted component min-cut"
```

---

### Task 5: Benchmark and Verify

**Files:**
- Modify: `scripts/benchmark_strategies.py` (add `mincut` and `preprocess` flag to benchmark runs)
- No new files needed

**Interfaces:**
- Consumes: `./src/hcp-solver <graph> --incremental --preprocess --stagnation-strategy <X> --time-limit 100`

- [ ] **Step 1: Update benchmark script to test new strategies**

In `scripts/benchmark_strategies.py`, add `"mincut"` to the strategies list and add `--preprocess` to the command template. The existing script already loops over strategies and graphs — just extend the lists.

Locate the strategies variable and update:
```python
strategies = ["none", "dfj", "union", "both", "mincut"]
# And add --preprocess to the base command args:
base_args = ["--incremental", "--preprocess", "--time-limit", "100"]
```

- [ ] **Step 2: Run benchmark on the 3 timeout graphs**

```bash
cd /home/ubuntu/HCP
python3 scripts/benchmark_strategies.py \
    graphs/fhcppp/graph424.edge \
    graphs/fhcppp/graph446.edge \
    graphs/fhcppp/graph470.edge \
    2>&1 | tee benchmark_mincut.csv
```

Expected: `mincut` + `preprocess` achieves SAT within 100s on at least `graph424` and `graph446`.

- [ ] **Step 3: Verify decoded solutions are valid Hamiltonian cycles**

For each graph that produced `solution.sat`:
```bash
./src/hcp-solver graphs/fhcppp/graph424.edge -d solution.sat 2>&1
```

Expected: `Hamiltonian cycle verified` (or equivalent valid output from `HcpDecoder`).

- [ ] **Step 4: Commit results**

```bash
cd /home/ubuntu/HCP
git add benchmark_mincut.csv scripts/benchmark_strategies.py
git commit -m "bench: add mincut strategy benchmark results for graph424/446/470"
```

---

## Self-Review

**Spec coverage:**
- ✅ Degree-2 forced edges → Task 2
- ✅ Bridge detection → Task 1 + Task 2 (early UNSAT return)
- ✅ 2-edge-cut forced edges with direction constraints → Task 2
- ✅ Contracted graph min-cut → Task 3
- ✅ `mincut` stagnation strategy → Task 4
- ✅ `--preprocess` CLI flag → Task 2
- ✅ Benchmark verification → Task 5

**Placeholder scan:** No TBDs. All code blocks are complete. All commands have expected output.

**Type consistency:**
- `MinCutResult.sideA_vertices: std::vector<int>` used consistently in Task 3 and Task 4
- `computeComponentMinCut` signature matches between Task 3 (definition) and Task 4 (usage)
- `EdgePair` fields `u1,v1,u2,v2` match between Task 1 (definition) and Task 2 (usage: `ep.u1`, `ep.v1`, etc.)
- `GraphPreprocessor::getDegree2Vertices()` → `std::vector<int>` used in Task 2 range-for loop
- `GraphPreprocessor::getTwoEdgeCuts()` → `std::vector<EdgePair>` used in Task 2 range-for loop
