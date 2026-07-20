# Internal Min-Cut Component Splitting Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Speed up graph470 convergence from 314s to <120s by replacing weak outgoing-edge SEC clauses with internal min-cut splitting for giant components.

**Architecture:** Add Dinic max-flow to `ContractedMinCut.cpp`, increase `maxFlowVertLimit` to 2000, wire internal min-cut splitting into the main SEC iteration loop in `Solver.cpp`.

**Tech Stack:** C++17, CaDiCaL SAT solver.

## Global Constraints

- No new third-party dependencies
- All changes in existing files: `src/ContractedMinCut.cpp`, `src/ContractedMinCut.hpp`, `src/Solver.cpp`
- Unit capacities (all edges weight 1)
- Must build with `make -C src`

---

### Task 1: Add Dinic max-flow to ContractedMinCut.cpp

**Files:**
- Modify: `src/ContractedMinCut.cpp`
- Test: `src/test_incremental_solver.cpp` (adds test to existing test suite)

**Interfaces:**
- Consumes: `std::vector<std::vector<int>>& cap` (n×n capacity matrix), `int s`, `int t`
- Produces: new static function `maxFlowDinic(n, cap, s, t, minCutSide)` — same signature as existing `maxFlowBFS`

- [ ] **Step 1: Add Dinic implementation**

Add a `struct Dinic` with adjacency-list representation and `addEdge(u, v, cap)`, `bfs(level)`, `dfs(v, t, f)` methods. The struct converts the n×n capacity matrix to adjacency list, then runs standard Dinic.

```cpp
// Dinic max-flow on adjacency list (sparse). Converted from n x n capacity matrix.
// O(E * sqrt(V)) on unit-capacity networks.
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
    // Returns vertices reachable from s in residual graph
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

- [ ] **Step 2: Add `maxFlowDinic` wrapper function**

Place it right after `maxFlowBFS`, before `computeComponentMinCut`:

```cpp
// Dinic max-flow wrapper matching maxFlowBFS signature.
// Converts capacity matrix to adjacency list.
static int maxFlowDinic(int n, std::vector<std::vector<int>>& cap,
                        int s, int t, std::vector<bool>& minCutSide) {
    Dinic dinic(n);
    for (int u = 0; u < n; ++u) {
        for (int v = 0; v < n; ++v) {
            if (cap[u][v] > 0) {
                dinic.addEdge(u, v, cap[u][v]);
            }
        }
    }
    int flow = dinic.maxFlow(s, t);
    minCutSide = dinic.minCut(s);
    return flow;
}
```

- [ ] **Step 3: Add test for Dinic**

Add to `src/test_incremental_solver.cpp`:

```cpp
#include "ContractedMinCut.hpp"

void testDinicMaxFlow() {
    std::cout << "Testing Dinic max-flow...\n";
    // Simple 4-node graph: 0-1-2-3 with unit capacities
    // Min cut between 0 and 3 is 1 (edge 2-3 or 1-2 or 0-1)
    int n = 4;
    std::vector<std::vector<int>> cap(n, std::vector<int>(n, 0));
    cap[0][1] = cap[1][0] = 1;
    cap[1][2] = cap[2][1] = 1;
    cap[2][3] = cap[3][2] = 1;
    std::vector<bool> sideA;
    int flow = maxFlowDinic(n, cap, 0, 3, sideA);
    TEST_ASSERT(flow == 1);
    TEST_ASSERT(sideA.size() == 4);
    // s=0 must be on side A, t=3 must not
    TEST_ASSERT(sideA[0] == true);
    TEST_ASSERT(sideA[3] == false);
    std::cout << "Dinic max-flow passed!\n";
}
```

Call `testDinicMaxFlow()` from `main()` in the test file.

- [ ] **Step 4: Run test to verify it passes**

```bash
cd src && make test_incremental_solver && ./test_incremental_solver
```
Expected: `Dinic max-flow passed!`

- [ ] **Step 5: Commit**

```bash
git add src/ContractedMinCut.cpp src/test_incremental_solver.cpp
git commit -m "feat: add Dinic max-flow for internal min-cut on large components"
```

---

### Task 2: Use Dinic for large components in computeInternalMinCut

**Files:**
- Modify: `src/ContractedMinCut.hpp`
- Modify: `src/ContractedMinCut.cpp`

**Interfaces:**
- Consumes: `maxFlowDinic`, `maxFlowBFS` (both static)
- Produces: updated `computeInternalMinCut` using Dinic for k > 500

- [ ] **Step 1: Update maxFlowVertLimit default**

In `src/ContractedMinCut.hpp`, change default from 500 to 2000:

```cpp
MinCutResult computeInternalMinCut(
    const Component& component,
    const Graph& graph,
    int maxFlowVertLimit = 2000
);
```

- [ ] **Step 2: Update computeInternalMinCut to use Dinic for large k**

In `src/ContractedMinCut.cpp`, change the flow computation inside `computeInternalMinCut` (line ~183):

Replace:
```cpp
        int flowVal = maxFlowBFS(k, capCopy, s, t, sideA_local);
```

With:
```cpp
        int flowVal;
        if (k > 500) {
            flowVal = maxFlowDinic(k, capCopy, s, t, sideA_local);
        } else {
            flowVal = maxFlowBFS(k, capCopy, s, t, sideA_local);
        }
```

- [ ] **Step 3: Build and test**

```bash
cd src && make test_incremental_solver && ./test_incremental_solver
```
Expected: all tests pass (including existing ones)

- [ ] **Step 4: Commit**

```bash
git add src/ContractedMinCut.cpp src/ContractedMinCut.hpp
git commit -m "perf: use Dinic for large-component internal min-cut (k>500)"
```

---

### Task 3: Wire internal min-cut splitting into SEC iteration loop

**Files:**
- Modify: `src/Solver.cpp`

**Interfaces:**
- Consumes: `computeInternalMinCut(component, graph, 2000)`, `iterationSecEncoder.encodeSecs({splitComp}, useVertexSep, vtxSepThreshold, skipVertexDisjoint)`
- Produces: stronger SEC clauses for large components (split via internal min-cut)

- [ ] **Step 1: Add include if missing**

`src/Solver.cpp` already includes `ContractedMinCut.hpp` (via `Solver.cpp` → `GraphPreprocessor.hpp` or similar). Verify by checking the build works.

- [ ] **Step 2: Insert min-cut splitting before encodeSecs call**

After line 617 (`prevBlockedComponentIds = std::move(currentComponentIds);`) and before line 619 (`iterationSecEncoder.startAuxAt(...)`):

```cpp
                // ----- LARGE COMPONENT SPLITTING via internal min-cut -----
                // For components >100 vertices, find internal min-cut and
                // encode SEC on the smaller side. This creates much stronger
                // constraints than weak outgoing-edge SEC for giant components.
                std::vector<Component> splitSubjects;
                for (const auto& comp : components) {
                    if (static_cast<int>(comp.vertices.size()) > 100) {
                        auto mcr = computeInternalMinCut(comp, g, 2000);
                        if (!mcr.sideA_vertices.empty()) {
                            Component splitComp;
                            splitComp.vertices = std::move(mcr.sideA_vertices);
                            splitSubjects.push_back(std::move(splitComp));
                        } else {
                            splitSubjects.push_back(comp);
                        }
                    } else {
                        splitSubjects.push_back(comp);
                    }
                }
```

Then change line 620 from:
```cpp
                auto secClauses = iterationSecEncoder.encodeSecs(components, useVertexSep_, vtxSepThreshold_, skipVertexDisjoint_);
```
To:
```cpp
                auto secClauses = iterationSecEncoder.encodeSecs(splitSubjects, useVertexSep_, vtxSepThreshold_, skipVertexDisjoint_);
```

- [ ] **Step 3: Build**

```bash
cd src && make
```
Expected: clean build (no errors)

- [ ] **Step 4: Quick smoke test — graph470 with 120s**

```bash
cd src && timeout 130 ./hcp-solver ../graphs/graph470.edge --incremental --cycle auto --time-limit 120 2>&1 | grep -E "(Auto cycle|HAMILTONIAN|TIMEOUT|UNSAT|Iteration|actions)" | tail -20
```
Expected: HAMILTONIAN within 120s (vs TIMEOUT before)

- [ ] **Step 5: Commit**

```bash
git add src/Solver.cpp
git commit -m "feat: internal min-cut splitting for giant components in SEC loop"
```

---

### Task 4: Full benchmark validation

**Files:**
- None modified

**Interfaces:**
- Consumes: `src/hcp-solver` binary

- [ ] **Step 1: Full FHCPP 18-graph benchmark at 120s**

```bash
cd src && for g in graph48 graph162 graph171 graph197 graph223 graph237 graph249 graph252 graph254 graph255 graph424 graph446 graph470 graph491 graph506 graph522 graph526 graph529; do echo "=== $g ==="; timeout 130 ./hcp-solver ../graphs/$g.edge --incremental --cycle auto --time-limit 120 2>&1 | grep -E "(Auto cycle|HAMILTONIAN|TIMEOUT|UNSAT|Iteration|actions)" | tail -5; done
```
Expected: 18/18 SAT (graph470 now solves within 120s)

- [ ] **Step 2: Verify total SEC iterations for graph470 < 500**

```bash
cd src && ./hcp-solver ../graphs/graph470.edge --incremental --cycle auto --time-limit 300 2>&1 | grep "incremental actions"
```
Expected: `c incremental actions: < 500` (vs ~2843 before)

- [ ] **Step 3: Update AGENTS.md with new results**

Replace the graph470 row in the results table.

- [ ] **Step 4: Commit**

```bash
git add AGENTS.md
git commit -m "docs: update benchmark — graph470 solved at 120s via internal min-cut"
```
