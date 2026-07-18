# SEC Loop Improvements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Improve SEC loop convergence via Gomory-Hu tree prioritized cuts and a dedicated 2-component deadlock strategy, targeting graph470 solvable within 120s.

**Architecture:** Gomory-Hu tree on the contracted component graph generates globally optimal cut priorities each iteration (>2 comps). A dedicated 2-component strategy with at-least-4 and vertex-disjoint constraints breaks the oscillation deadlock. Implementation cleanup (model extraction, allocation reuse, hash fix) is bundled.

**Tech Stack:** C++17, CaDiCaL SAT solver, Dinic max-flow (existing in `ContractedMinCut.cpp`)

## Global Constraints

- Build: `make -C src` (g++ -O2 -Wall -std=c++17, links against CaDiCaL and pblib)
- Test: `make -C src test` (runs test_incremental_solver, test_graphs, test_vertex_separator)
- Test pattern: `TEST_ASSERT(cond)` macro, void functions named `testXxx()`, called from `main()`
- Benchmark: `python3 scripts/run_experiments.py --time-limit 120` (18 FHCPP graphs)
- No regressions: all 17 currently-solved graphs must still solve at 120s
- Graph470 target: solve in <120s (currently 514s at 600s limit, TIMEOUT at 120s)
- Existing Dinic struct is defined in `src/ContractedMinCut.cpp:63-132` (file-local, not in header)

---

### Task 1: Gomory-Hu Tree — Implementation and Unit Tests

**Files:**
- Create: `src/GomoryHuTree.hpp`
- Create: `src/GomoryHuTree.cpp`
- Create: `src/test_gomory_hu.cpp`
- Modify: `src/Makefile`

**Interfaces:**
- Consumes: `Dinic` struct from `src/ContractedMinCut.cpp` (needs to be made shared — see step 3), `Component` from `SubtourDetector.hpp`, `Graph` from `Graph.hpp`
- Produces:
  - `struct GHEdge { int u, v; int cutWeight; std::vector<int> sideA; }` — one edge of the Gomory-Hu tree
  - `struct GomoryHuTree { std::vector<GHEdge> edges; }` — sorted by cutWeight ascending
  - `GomoryHuTree computeGomoryHuTree(const std::vector<Component>& components, const Graph& graph)` — main entry point

- [ ] **Step 1: Move Dinic struct to a shared header**

The `Dinic` struct is currently defined inside `src/ContractedMinCut.cpp` (line 63-132) and not accessible from other translation units. Move it to the header.

In `src/ContractedMinCut.hpp`, add before the closing `#endif` (or before the function declarations):

```cpp
#include <queue>
#include <limits>

// Dinic max-flow on adjacency list (sparse). O(E * sqrt(V)) on unit-capacity networks.
struct Dinic {
    struct Edge { int to, rev; int cap; };
    std::vector<std::vector<Edge>> g;
    std::vector<int> level, iter;
    Dinic(int n) : g(n), level(n), iter(n) {}
    void addEdge(int from, int to, int cap) {
        g[from].push_back({to, (int)g[to].size(), cap});
        g[to].push_back({from, (int)g[from].size() - 1, 0});
    }
    void bfs(int s) {
        level.assign(g.size(), -1);
        std::queue<int> q;
        level[s] = 0;
        q.push(s);
        while (!q.empty()) {
            int v = q.front(); q.pop();
            for (auto& e : g[v]) {
                if (e.cap > 0 && level[e.to] < 0) {
                    level[e.to] = level[v] + 1;
                    q.push(e.to);
                }
            }
        }
    }
    int dfs(int v, int t, int f) {
        if (v == t) return f;
        for (int& i = iter[v]; i < (int)g[v].size(); ++i) {
            Edge& e = g[v][i];
            if (e.cap > 0 && level[v] < level[e.to]) {
                int d = dfs(e.to, t, std::min(f, e.cap));
                if (d > 0) {
                    e.cap -= d;
                    g[e.to][e.rev].cap += d;
                    return d;
                }
            }
        }
        return 0;
    }
    int maxFlow(int s, int t) {
        int flow = 0;
        while (true) {
            bfs(s);
            if (level[t] < 0) break;
            iter.assign(g.size(), 0);
            int f;
            while ((f = dfs(s, t, std::numeric_limits<int>::max())) > 0) {
                flow += f;
            }
        }
        return flow;
    }
    std::vector<bool> minCut(int s) {
        std::vector<bool> visited(g.size(), false);
        std::queue<int> q;
        q.push(s);
        visited[s] = true;
        while (!q.empty()) {
            int v = q.front(); q.pop();
            for (auto& e : g[v]) {
                if (e.cap > 0 && !visited[e.to]) {
                    visited[e.to] = true;
                    q.push(e.to);
                }
            }
        }
        return visited;
    }
};
```

In `src/ContractedMinCut.cpp`, remove the `Dinic` struct definition (lines 61-132) and add `#include <queue>` and `#include <limits>` if not already present. The code in `computeInternalMinCut` (line 274: `Dinic dinic(k);`) and `maxFlowDinic` (line 138: `Dinic dinic(n);`) will now use the header version.

- [ ] **Step 2: Verify the Dinic move compiles**

Run:
```bash
make -C src clean && make -C src hcp-solver 2>&1 | tail -5
```
Expected: successful compilation, no errors.

- [ ] **Step 3: Write the Gomory-Hu tree header**

Create `src/GomoryHuTree.hpp`:

```cpp
#pragma once
#include <vector>
#include "SubtourDetector.hpp"

class Graph;

struct GHEdge {
    int u, v;           // component indices
    int cutWeight;       // min s-t cut value in contracted graph
    std::vector<int> sideA;  // component indices on the source side of this cut
};

struct GomoryHuTree {
    std::vector<GHEdge> edges;  // C-1 edges, sorted by cutWeight ascending
};

// Compute Gomory-Hu tree on the contracted component graph.
// Each component becomes a super-node; edge weight = number of directed edges
// between components in the original graph.
// Returns tree with C-1 edges sorted by cutWeight ascending.
// Returns empty tree if components.size() < 2.
GomoryHuTree computeGomoryHuTree(
    const std::vector<Component>& components,
    const Graph& graph);
```

- [ ] **Step 4: Write the Gomory-Hu tree implementation**

Create `src/GomoryHuTree.cpp`:

```cpp
#include "GomoryHuTree.hpp"
#include "Graph.hpp"
#include "ContractedMinCut.hpp"
#include <algorithm>
#include <numeric>
#include <queue>

GomoryHuTree computeGomoryHuTree(
    const std::vector<Component>& components,
    const Graph& graph)
{
    GomoryHuTree result;
    int C = static_cast<int>(components.size());
    if (C < 2) return result;

    int n = graph.getNodes();

    // Map each vertex to its component index
    std::vector<int> vertToComp(n, -1);
    for (int ci = 0; ci < C; ++ci) {
        for (int v : components[ci].vertices) {
            if (v >= 0 && v < n) vertToComp[v] = ci;
        }
    }

    // Build contracted graph capacity matrix (C x C)
    std::vector<std::vector<int>> baseCap(C, std::vector<int>(C, 0));
    for (int u = 0; u < n; ++u) {
        int cu = vertToComp[u];
        if (cu < 0) continue;
        for (auto& [v, _] : graph.getNeighbors(u)) {
            int cv = vertToComp[v];
            if (cv < 0 || cv == cu) continue;
            baseCap[cu][cv]++;
        }
    }

    // Gomory-Hu tree algorithm:
    // tree[i] = parent of node i in the GH tree (tree[0] = -1, root)
    // treeWeight[i] = weight of edge from i to tree[i]
    std::vector<int> tree(C, 0);  // initially all nodes connected to node 0
    std::vector<int> treeWeight(C, 0);

    for (int i = 1; i < C; ++i) {
        int t = tree[i];  // current tree-neighbor of i

        // Run max-flow from i to t on contracted graph
        Dinic dinic(C);
        for (int u = 0; u < C; ++u) {
            for (int v = 0; v < C; ++v) {
                if (baseCap[u][v] > 0) {
                    dinic.addEdge(u, v, baseCap[u][v]);
                }
            }
        }

        int flowVal = dinic.maxFlow(i, t);
        auto sideI = dinic.minCut(i);  // vertices reachable from i in residual

        treeWeight[i] = flowVal;

        // Update tree pointers: for each j > i that is currently connected to t
        // and is on the same side as i, redirect j to point to i instead
        for (int j = i + 1; j < C; ++j) {
            if (tree[j] == t && sideI[j]) {
                tree[j] = i;
            }
        }
    }

    // Convert tree[] to GHEdge list, computing sideA for each edge
    // For each tree edge (i, tree[i]), sideA = subtree rooted at i
    // We compute this by BFS/DFS on the tree structure
    
    // Build tree adjacency
    std::vector<std::vector<int>> treeAdj(C);
    for (int i = 1; i < C; ++i) {
        treeAdj[i].push_back(tree[i]);
        treeAdj[tree[i]].push_back(i);
    }

    for (int i = 1; i < C; ++i) {
        GHEdge edge;
        edge.u = i;
        edge.v = tree[i];
        edge.cutWeight = treeWeight[i];

        // Find sideA = all nodes in the subtree rooted at i (when edge i-tree[i] is removed)
        std::vector<bool> visited(C, false);
        std::queue<int> q;
        q.push(i);
        visited[i] = true;
        while (!q.empty()) {
            int cur = q.front(); q.pop();
            edge.sideA.push_back(cur);
            for (int nb : treeAdj[cur]) {
                if (!visited[nb] && !(cur == i && nb == tree[i]) && !(cur == tree[i] && nb == i)) {
                    visited[nb] = true;
                    q.push(nb);
                }
            }
        }

        result.edges.push_back(std::move(edge));
    }

    // Sort by cutWeight ascending (weakest cuts first = highest priority)
    std::sort(result.edges.begin(), result.edges.end(),
              [](const GHEdge& a, const GHEdge& b) { return a.cutWeight < b.cutWeight; });

    return result;
}
```

- [ ] **Step 5: Write unit tests for Gomory-Hu tree**

Create `src/test_gomory_hu.cpp`:

```cpp
#include <iostream>
#include <vector>
#include <cstdlib>
#include <algorithm>
#include "GomoryHuTree.hpp"
#include "SubtourDetector.hpp"
#include "Graph.hpp"
#include "ContractedMinCut.hpp"

#define TEST_ASSERT(cond) \
    do { \
        if (!(cond)) { \
            std::cerr << "Assertion failed: " << #cond << " at " << __FILE__ << ":" << __LINE__ << "\n"; \
            std::abort(); \
        } \
    } while (0)

// Helper: create a simple graph from edge list
// Edges are undirected: (u, v) creates both (u,v) and (v,u) with edge indices
static Graph makeGraph(int n, const std::vector<std::pair<int,int>>& edges) {
    // Write temp .edge file and load
    std::string tmpFile = "test_gh_tmp.edge";
    {
        std::ofstream f(tmpFile);
        f << n << " " << edges.size() << "\n";
        for (auto& [u, v] : edges) {
            f << (u + 1) << " " << (v + 1) << "\n";  // 1-indexed in DIMACS
        }
    }
    Graph g;
    g.loadFromFile(tmpFile, true);
    std::remove(tmpFile.c_str());
    return g;
}

void testGomoryHuBasic3Components() {
    std::cout << "Testing Gomory-Hu tree with 3 components...\n";

    // 6-node graph: {0,1} -- {2,3} -- {4,5}
    // Edges: 0-1, 2-3, 4-5 (internal), 1-2 (bridge 1), 3-4 (bridge 2)
    Graph g = makeGraph(6, {{0,1},{1,2},{2,3},{3,4},{4,5}});

    // Create 3 components manually
    Component c0, c1, c2;
    c0.vertices = {0, 1};
    c1.vertices = {2, 3};
    c2.vertices = {4, 5};
    std::vector<Component> components = {c0, c1, c2};

    auto ghTree = computeGomoryHuTree(components, g);

    TEST_ASSERT(ghTree.edges.size() == 2);  // C-1 = 2 edges

    // Both cuts should be small (bridged connections)
    // Sorted by cutWeight ascending
    for (const auto& e : ghTree.edges) {
        TEST_ASSERT(e.cutWeight > 0);
        TEST_ASSERT(!e.sideA.empty());
        TEST_ASSERT((int)e.sideA.size() < 3);  // proper partition
    }

    std::cout << "  Tree edges:\n";
    for (const auto& e : ghTree.edges) {
        std::cout << "    comp " << e.u << " -- comp " << e.v
                  << " weight=" << e.cutWeight
                  << " sideA=[";
        for (int c : e.sideA) std::cout << c << " ";
        std::cout << "]\n";
    }

    std::cout << "Gomory-Hu 3-component test passed!\n";
}

void testGomoryHuSingleComponent() {
    std::cout << "Testing Gomory-Hu tree with < 2 components...\n";

    Graph g = makeGraph(4, {{0,1},{1,2},{2,3},{3,0}});
    std::vector<Component> components;  // empty
    auto ghTree = computeGomoryHuTree(components, g);
    TEST_ASSERT(ghTree.edges.empty());

    Component c0;
    c0.vertices = {0, 1, 2, 3};
    components = {c0};
    ghTree = computeGomoryHuTree(components, g);
    TEST_ASSERT(ghTree.edges.empty());

    std::cout << "Gomory-Hu single-component test passed!\n";
}

void testGomoryHuCutWeightsMatchBruteForce() {
    std::cout << "Testing Gomory-Hu cut weights match brute force...\n";

    // 8-node graph with 4 components of 2 nodes each, varying connectivity
    // Comp0={0,1}, Comp1={2,3}, Comp2={4,5}, Comp3={6,7}
    // Edges: 0-1, 2-3, 4-5, 6-7 (internal)
    // 0-2, 1-3 (comp0-comp1: weight 4 directed = 2 undirected × 2 directions)
    // 2-4 (comp1-comp2: weight 2 directed)
    // 4-6, 5-7 (comp2-comp3: weight 4 directed)
    Graph g = makeGraph(8, {{0,1},{2,3},{4,5},{6,7}, {0,2},{1,3}, {2,4}, {4,6},{5,7}});

    Component c0, c1, c2, c3;
    c0.vertices = {0, 1};
    c1.vertices = {2, 3};
    c2.vertices = {4, 5};
    c3.vertices = {6, 7};
    std::vector<Component> components = {c0, c1, c2, c3};

    auto ghTree = computeGomoryHuTree(components, g);
    TEST_ASSERT(ghTree.edges.size() == 3);

    // The weakest cut should be comp1-comp2 with weight 2
    TEST_ASSERT(ghTree.edges[0].cutWeight <= ghTree.edges[1].cutWeight);
    TEST_ASSERT(ghTree.edges[1].cutWeight <= ghTree.edges[2].cutWeight);

    std::cout << "  Sorted cut weights:";
    for (const auto& e : ghTree.edges) {
        std::cout << " " << e.cutWeight;
    }
    std::cout << "\n";

    std::cout << "Gomory-Hu brute-force comparison passed!\n";
}

int main() {
    testGomoryHuSingleComponent();
    testGomoryHuBasic3Components();
    testGomoryHuCutWeightsMatchBruteForce();
    std::cout << "\nAll Gomory-Hu tests passed!\n";
    return 0;
}
```

- [ ] **Step 6: Update Makefile**

Add to `src/Makefile`:

1. Add `GomoryHuTree.cpp GomoryHuTree.hpp` to the `hcp-solver` dependency and compile list.
2. Add a `test_gomory_hu` target.
3. Add `GomoryHuTree.cpp` to `test_incremental_solver` and `test_graphs` compile lines.
4. Add `test_gomory_hu` to the `test` target and `clean` target.

For the `hcp-solver` target (line 6-7), add `GomoryHuTree.cpp GomoryHuTree.hpp` to the dependency list and `GomoryHuTree.cpp` to the compile command.

Add new test target:
```makefile
test_gomory_hu: test_gomory_hu.cpp GomoryHuTree.cpp GomoryHuTree.hpp ContractedMinCut.cpp ContractedMinCut.hpp SubtourDetector.cpp SubtourDetector.hpp Graph.hpp SecEncoder.cpp SecEncoder.hpp
	$(CXX) $(CXXFLAGS) test_gomory_hu.cpp GomoryHuTree.cpp ContractedMinCut.cpp SubtourDetector.cpp SecEncoder.cpp -o test_gomory_hu
```

Update `test:` to include `test_gomory_hu`.
Update `clean:` to include `test_gomory_hu`.

- [ ] **Step 7: Build and run the Gomory-Hu tests**

Run:
```bash
make -C src test_gomory_hu && src/test_gomory_hu
```
Expected: All 3 tests pass.

- [ ] **Step 8: Run existing tests for regression**

Run:
```bash
make -C src test
```
Expected: All existing tests pass (Dinic move didn't break anything).

- [ ] **Step 9: Commit**

```bash
cd /home/ubuntu/HCP && git add src/GomoryHuTree.hpp src/GomoryHuTree.cpp src/test_gomory_hu.cpp src/ContractedMinCut.hpp src/ContractedMinCut.cpp src/Makefile && git commit -m "feat: add Gomory-Hu tree on contracted graph with unit tests"
```

---

### Task 2: Integrate Gomory-Hu Tree into SEC Loop

**Files:**
- Modify: `src/Solver.cpp` (SEC loop in `runIncremental`, ~line 708)
- Modify: `src/Solver.hpp` (add GH threshold constant)

**Interfaces:**
- Consumes: `computeGomoryHuTree()` from Task 1, `DefaultAtLeastK::encode()` from `src/AtLeastK/DefaultAtLeastK.hpp`
- Produces: Enhanced SEC clause generation in the main loop (no new public API)

- [ ] **Step 1: Add include and constant**

In `src/Solver.cpp`, add near the top includes:
```cpp
#include "GomoryHuTree.hpp"
#include "AtLeastK/DefaultAtLeastK.hpp"
```

In `src/Solver.hpp`, add a constant to the `Solver` class private section:
```cpp
int ghAtLeast2Threshold_ = 4;  // Gomory-Hu: use at-least-2 for cuts with weight <= this
```

- [ ] **Step 2: Add Gomory-Hu prioritized SEC generation**

In `src/Solver.cpp`, in `runIncremental()`, locate the SEC encoding block (around line 708-709):
```cpp
iterationSecEncoder.startAuxAt(isolver.getNumVars() + 1);
auto secClauses = iterationSecEncoder.encodeSecs(components, useVertexSep_, vtxSepThreshold_, skipVertexDisjoint_);
```

Insert the Gomory-Hu block **before** this existing code (so GH cuts are added first, then standard per-component SECs follow):

```cpp
// ---- Gomory-Hu prioritized cuts ----
if (components.size() > 2) {
    auto ghTree = computeGomoryHuTree(components, g);
    int ghClausesAdded = 0;

    // Build component membership for crossing-edge collection
    std::vector<int> vertToComp(g.getNodes(), -1);
    for (int ci = 0; ci < (int)components.size(); ++ci) {
        for (int v : components[ci].vertices) {
            if (v >= 0 && v < g.getNodes()) vertToComp[v] = ci;
        }
    }

    for (const auto& edge : ghTree.edges) {
        if (edge.cutWeight > ghAtLeast2Threshold_) break; // sorted ascending; rest are strong enough

        // Build sideA membership
        std::vector<bool> inSideA(components.size(), false);
        for (int ci : edge.sideA) inSideA[ci] = true;

        // Collect all crossing edges (both directions)
        std::vector<int> crossingLits;
        for (int ci : edge.sideA) {
            for (int u : components[ci].vertices) {
                for (auto& [v, edgeIdx] : g.getNeighbors(u)) {
                    int cv = vertToComp[v];
                    if (cv >= 0 && !inSideA[cv]) {
                        crossingLits.push_back(edgeIdx);
                    }
                }
            }
        }

        if ((int)crossingLits.size() >= 4) {
            // At-least-2 on crossing edges
            DefaultAtLeastK atLeastK;
            int auxBase = isolver.getNumVars() + 1;
            auto kClauses = atLeastK.encode(crossingLits, 2, auxBase);
            for (const auto& cl : kClauses) {
                isolver.addClause(cl);
                ghClausesAdded++;
            }
        }
    }

    if (ghClausesAdded > 0) {
        std::cerr << "c Gomory-Hu: added " << ghClausesAdded
                  << " at-least-2 clauses for " << ghTree.edges.size() << " tree edges\n";
    }
}
```

- [ ] **Step 3: Build and run existing tests**

Run:
```bash
make -C src clean && make -C src test
```
Expected: All tests pass. The Gomory-Hu integration only fires during `runIncremental` with real graphs, so unit tests are unaffected.

- [ ] **Step 4: Quick smoke test with a fast graph**

Run:
```bash
src/hcp-solver graphs/fhcppp/graph171.edge --incremental --time-limit 30 2>&1 | grep -E "HAMILTONIAN|Gomory-Hu|incremental actions"
```
Expected: `HAMILTONIAN found`, possibly some `Gomory-Hu: added` lines, completes in <10s.

- [ ] **Step 5: Commit**

```bash
cd /home/ubuntu/HCP && git add src/Solver.cpp src/Solver.hpp && git commit -m "feat: integrate Gomory-Hu prioritized cuts into SEC loop"
```

---

### Task 3: 2-Component Deadlock Strategy

**Files:**
- Modify: `src/Solver.cpp` (add twoCompStreak counter and 2-comp strategy block)
- Modify: `src/Solver.hpp` (add twoCompThreshold constant)

**Interfaces:**
- Consumes: `DefaultAtLeastK::encode()`, `Graph::getNeighbors()`, `Graph::getAdj()`, `IncrementalSolver::addClause()`
- Produces: 2-component deadlock breaking in the main loop (no new public API)

- [ ] **Step 1: Add configuration to Solver.hpp**

In `src/Solver.hpp`, add to the `Solver` class private section:
```cpp
int twoCompThreshold_ = 20;  // trigger 2-comp strategy after this many consecutive 2-comp iterations
```

Add public setter:
```cpp
void setTwoCompThreshold(int t) { twoCompThreshold_ = t; }
```

- [ ] **Step 2: Add twoCompStreak counter in runIncremental**

In `src/Solver.cpp`, in `runIncremental()`, near the other state variables (around line 357-362), add:
```cpp
int twoCompStreak = 0;
```

- [ ] **Step 3: Add streak tracking after subtour detection**

In `src/Solver.cpp`, after the subtour detection line `auto components = SubtourDetector::detect(model, g);` (line 411) and after the Jaccard/stagnation block, before the SEC encoding section, add the streak tracking:

```cpp
// Track consecutive 2-component iterations
if (components.size() == 2) {
    twoCompStreak++;
} else {
    twoCompStreak = 0;
}
```

- [ ] **Step 4: Add the 2-component strategy block**

Insert the following block after the streak tracking and before the Gomory-Hu/SEC encoding section. This should go right after the oscillation-guided cuts block (after line ~706):

```cpp
// ---- 2-COMPONENT DEADLOCK STRATEGY ----
if (components.size() == 2 && twoCompStreak >= twoCompThreshold_) {
    std::cerr << "c 2-comp deadlock detected (streak=" << twoCompStreak
              << "), applying vertex-separator strengthening\n";
    twoCompStreak = 0;  // reset to allow re-triggering after threshold

    // Build component A membership
    std::vector<bool> inA(g.getNodes(), false);
    for (int v : components[0].vertices) {
        if (v >= 0 && v < g.getNodes()) inA[v] = true;
    }

    // Collect all crossing edges (both A→B and B→A)
    std::vector<int> allCrossing;
    for (int u : components[0].vertices) {
        if (u < 0 || u >= g.getNodes()) continue;
        for (auto& [v, edgeIdx] : g.getNeighbors(u)) {
            if (!inA[v] && edgeIdx > 0) allCrossing.push_back(edgeIdx);
        }
    }
    for (int u : components[1].vertices) {
        if (u < 0 || u >= g.getNodes()) continue;
        for (auto& [v, edgeIdx] : g.getNeighbors(u)) {
            if (inA[v] && edgeIdx > 0) allCrossing.push_back(edgeIdx);
        }
    }

    int twoCompClauses = 0;

    // At-least-4 on all crossing edges
    if ((int)allCrossing.size() >= 4) {
        DefaultAtLeastK atLeastK;
        int auxBase = isolver.getNumVars() + 1;
        auto kClauses = atLeastK.encode(allCrossing, 4, auxBase);
        for (const auto& cl : kClauses) {
            isolver.addClause(cl);
            twoCompClauses++;
        }
    }

    // Vertex-disjoint constraints on boundary vertices of component B
    for (int bv : components[1].vertices) {
        if (bv < 0 || bv >= g.getNodes()) continue;
        bool isBoundary = false;
        for (auto& [v, _] : g.getNeighbors(bv)) {
            if (inA[v]) { isBoundary = true; break; }
        }
        if (!isBoundary) continue;

        // Edges from A into bv
        std::vector<int> edgesIn;
        for (auto& [u, edgeIdx] : g.getNeighbors(bv)) {
            if (inA[u]) {
                int lit = g.getAdj(u, bv);
                if (lit > 0) edgesIn.push_back(lit);
            }
        }
        // Edges from bv to A
        std::vector<int> edgesOut;
        for (auto& [v, edgeIdx] : g.getNeighbors(bv)) {
            if (inA[v] && edgeIdx > 0) {
                edgesOut.push_back(edgeIdx);
            }
        }
        // Pairwise mutex: HC cannot enter from A and exit to A through same vertex
        for (int eIn : edgesIn) {
            for (int eOut : edgesOut) {
                isolver.addClause({-eIn, -eOut});
                twoCompClauses++;
            }
        }
    }

    if (twoCompClauses > 0) {
        std::cerr << "c 2-comp strategy: added " << twoCompClauses
                  << " clauses (at-least-4 + vertex-disjoint)\n";
    }
}
```

- [ ] **Step 5: Add CLI option for twoCompThreshold**

In `src/Solver.cpp`, in the `main()` argument parsing section (after line ~870, near other `--` options), add:
```cpp
} else if (arg == "--two-comp-threshold" && i + 1 < argc) {
    solver.setTwoCompThreshold(std::stoi(argv[++i]));
```

- [ ] **Step 6: Build and run tests**

Run:
```bash
make -C src clean && make -C src test
```
Expected: All tests pass.

- [ ] **Step 7: Test with graph470 at 600s to verify no regression**

Run:
```bash
src/hcp-solver graphs/fhcppp/graph470.edge --incremental --time-limit 600 2>&1 | grep -E "HAMILTONIAN|2-comp|incremental actions|total solver"
```
Expected: `HAMILTONIAN found`. Should see `2-comp deadlock detected` messages. Check that total time is ≤ 514s (current baseline).

- [ ] **Step 8: Test with graph470 at 120s (the target)**

Run:
```bash
src/hcp-solver graphs/fhcppp/graph470.edge --incremental --time-limit 120 2>&1 | grep -E "HAMILTONIAN|TIMEOUT|2-comp|incremental actions"
```
Expected: ideally `HAMILTONIAN found` (success!). If still TIMEOUT, record the iteration count and compare with baseline (3,421 at 600s). Any reduction in iterations indicates progress.

- [ ] **Step 9: Commit**

```bash
cd /home/ubuntu/HCP && git add src/Solver.cpp src/Solver.hpp && git commit -m "feat: 2-component deadlock strategy with at-least-4 and vertex-disjoint constraints"
```

---

### Task 4: Implementation Cleanup (Model Extraction, Allocation Reuse, Hash Fix)

**Files:**
- Modify: `src/IncrementalSolver.hpp` (add `getModel(int maxVar)` overload)
- Modify: `src/IncrementalSolver.cpp` (implement overload)
- Modify: `src/Solver.cpp` (use edge-only model, fix oscillation hash)
- Modify: `src/SecEncoder.hpp` (add member vectors)
- Modify: `src/SecEncoder.cpp` (reuse member vectors)

**Interfaces:**
- Consumes: existing `IncrementalSolver`, `SecEncoder`, `Solver` APIs
- Produces: `IncrementalSolver::getModel(int maxEdgeVar)` — returns model for vars 1..maxEdgeVar only

- [ ] **Step 1: Add getModel overload**

In `src/IncrementalSolver.hpp`, add after the existing `getModel()` declaration:
```cpp
// Returns a partial model covering only variables 1..maxEdgeVar.
// Useful when only edge variables are needed (avoids querying auxiliary variables).
std::vector<int> getModel(int maxEdgeVar) const;
```

In `src/IncrementalSolver.cpp`, add:
```cpp
std::vector<int> IncrementalSolver::getModel(int maxEdgeVar) const {
    if (state != SolverState::SAT) {
        throw std::logic_error("Cannot get model: solver is not in SAT state");
    }
    int limit = std::min(maxEdgeVar, max_var);
    std::vector<int> model(limit + 1, 0);
    for (int i = 1; i <= limit; ++i) {
        int val = getModelValue(i);
        if (val == 1) {
            model[i] = i;
        } else if (val == -1) {
            model[i] = -i;
        }
    }
    return model;
}
```

- [ ] **Step 2: Use edge-only model in Solver.cpp**

In `src/Solver.cpp`, find the model extraction line in the main loop (line ~410):
```cpp
auto model = isolver.getModel();
```

Change to:
```cpp
int maxEdgeVar = 2 * g.getEdges();
auto model = isolver.getModel(maxEdgeVar);
```

Note: The `tracer` block also uses `model` — it accesses `model[edgeIdx]` which is within edge var range. The Hamiltonian cycle extraction block (line ~650) also uses `model[edgeIdx]`. Both are safe with the edge-only model.

However, the tracer log block at line 601-607 iterates `for (int v = 1; v <= numVars; ++v)` — this needs to change to `for (int v = 1; v <= maxEdgeVar; ++v)` to match the reduced model size.

- [ ] **Step 3: Fix oscillation hash to be order-independent**

In `src/Solver.cpp`, find the oscillation hash computation (around line 680-683):
```cpp
uint64_t hash = 0;
for (int v : comp.vertices) {
    hash ^= std::hash<int>{}(v) + 0x9e3779b9 + (hash << 6) + (hash >> 2);
}
```

Replace with order-independent hash:
```cpp
uint64_t hash = 0;
for (int v : comp.vertices) {
    hash ^= std::hash<int>{}(v) * 0x9e3779b97f4a7c15ULL;
}
```

- [ ] **Step 4: SecEncoder allocation reuse**

In `src/SecEncoder.hpp`, add private member vectors:
```cpp
mutable std::vector<bool> inComponent_;
mutable std::vector<bool> isBoundary_;
```

In `src/SecEncoder.cpp`, in the constructor, pre-allocate:
```cpp
SecEncoder::SecEncoder(const Graph& graph) : graph_(graph), nextAuxBase_(0) {
    int numNodes = graph_.getNodes();
    inComponent_.resize(numNodes, false);
    isBoundary_.resize(numNodes, false);
    inAdj_.resize(numNodes);
    // ... rest unchanged
}
```

In `getOutgoingLiterals()`, replace:
```cpp
std::vector<bool> inComponent(numNodes, false);
```
with:
```cpp
std::fill(inComponent_.begin(), inComponent_.end(), false);
```
And use `inComponent_` instead of `inComponent` throughout the function.

In `getIncomingLiterals()`, same change: replace local `inComponent` with member `inComponent_`.

In `encodeSecs()`, replace local `inComponent` (line 45) and `isBoundary` (line 51) with `inComponent_` and `isBoundary_`, using `std::fill` to clear them.

- [ ] **Step 5: Build and run all tests**

Run:
```bash
make -C src clean && make -C src test
```
Expected: All tests pass.

- [ ] **Step 6: Smoke test a fast graph**

Run:
```bash
src/hcp-solver graphs/fhcppp/graph171.edge --incremental --time-limit 30 2>&1 | grep -E "HAMILTONIAN|incremental"
```
Expected: `HAMILTONIAN found`.

- [ ] **Step 7: Commit**

```bash
cd /home/ubuntu/HCP && git add src/IncrementalSolver.hpp src/IncrementalSolver.cpp src/Solver.cpp src/SecEncoder.hpp src/SecEncoder.cpp && git commit -m "perf: model extraction optimization, SecEncoder allocation reuse, order-independent oscillation hash"
```

---

### Task 5: Full Regression and Benchmark

**Files:**
- No code changes — verification only

**Interfaces:**
- Consumes: all changes from Tasks 1-4
- Produces: benchmark results confirming no regressions and (hopefully) graph470 improvement

- [ ] **Step 1: Clean build**

Run:
```bash
make -C src clean && make -C src hcp-solver 2>&1 | tail -3
```
Expected: successful build.

- [ ] **Step 2: Run all unit tests**

Run:
```bash
make -C src test
```
Expected: All tests pass (test_incremental_solver, test_graphs, test_vertex_separator, test_gomory_hu).

- [ ] **Step 3: Run full 18-graph benchmark at 120s**

Run:
```bash
python3 scripts/run_experiments.py --time-limit 120 2>&1 | tee experiments/fhcppp/post_improvement_results.txt
```
Expected:
- 17 previously-solved graphs: all still SAT within 120s
- graph470: check status — SAT (goal) or TIMEOUT with reduced iteration count (progress)

- [ ] **Step 4: Compare results**

Manually compare iteration counts and solve times against baseline in `experiments/fhcppp/sol.csv`. Key metrics:
- graph470: time and iteration count vs baseline (514.7s, 3421 iters)
- Average iteration count across all graphs
- Any graphs that got slower (regression indicator)

- [ ] **Step 5: If graph470 still times out — tune twoCompThreshold**

If graph470 still times out at 120s, try lower thresholds:
```bash
# Try threshold=10
src/hcp-solver graphs/fhcppp/graph470.edge --incremental --time-limit 120 --two-comp-threshold 10 2>&1 | tail -10
# Try threshold=5
src/hcp-solver graphs/fhcppp/graph470.edge --incremental --time-limit 120 --two-comp-threshold 5 2>&1 | tail -10
```

Note: The `--two-comp-threshold` CLI option needs to be added to `main()` argument parsing if not already present. Add it as a simple integer option in the argument parsing section of `Solver.cpp` (after line ~870).

- [ ] **Step 6: Commit benchmark results**

```bash
cd /home/ubuntu/HCP && git add experiments/ && git commit -m "bench: post-improvement FHCPP benchmark results"
```

- [ ] **Step 7: Update AGENTS.md with new results**

Update the results tables and "Changes This Session" section in `AGENTS.md` to reflect the new benchmark data.

```bash
cd /home/ubuntu/HCP && git add AGENTS.md && git commit -m "docs: update AGENTS.md with SEC loop improvement results"
```
