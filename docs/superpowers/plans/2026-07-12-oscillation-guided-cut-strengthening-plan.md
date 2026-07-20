# Oscillation-Guided Cut Strengthening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Accelerate graph470 SEC convergence from 319s to <120s by adding structural block clauses upfront and oscillation-triggered cut clauses during the SEC loop.

**Architecture:** Two independent additions to the existing SEC loop in `Solver::runIncremental`: (1) a one-time preprocessing pass adds DFJ clauses on 2-edge-connected blocks, and (2) an oscillation tracker detects repeating component partitions and triggers internal min-cut clauses only when oscillation is detected. Both use only `addClause` — no solver-specific APIs.

**Tech Stack:** C++17, CaDiCaL via `IncrementalSolver`, existing `ContractedMinCut` (Dinic).

## Global Constraints

- All solver interaction must use only `addClause` + `solve` + `getModel` — no `assume`, `constrain`, or `failed`
- New functions go in `src/Solver.cpp` as static helpers (like `computeAutoScaleCycle`)
- Oscillation window default: 10 iterations. Cut threshold default: 100 vertices. Precompute blocks: on by default.
- Bridge-finding reuses `GraphPreprocessor::findBridges` (Tarjan, O(V+E))
- Must not regress any of the other 17 FHCPP graphs at 120s

---

### Task 1: `find2EdgeConnectedBlocks` helper

**Files:**
- Modify: `src/Solver.cpp` (add static function near line 724 area)
- Test: `src/test_graphs.cpp` (add unit test)

**Interfaces:**
- Produces: `std::vector<std::vector<int>> find2EdgeConnectedBlocks(const Graph& g)` — returns list of blocks, each block is a list of vertex IDs. A block is a maximal 2-edge-connected subgraph (no bridges internally).

- [ ] **Step 1: Add required includes**

At the top of `Solver.cpp`, add after the existing includes:

```cpp
#include <unordered_map>
#include <set>
#include <functional>
#include <cstdint>
```

- [ ] **Step 2: Add the function to Solver.cpp**

Place after `computeAutoScaleCycle` (line 732). Uses Tarjan bridge-finding + DFS to identify 2-edge-connected blocks.

```cpp
// Returns the 2-edge-connected components (blocks) of graph g.
// A block is a maximal subgraph without bridges.
static std::vector<std::vector<int>> find2EdgeConnectedBlocks(const Graph& g) {
    int n = g.getNodes();
    // --- First pass: find all bridges ---
    std::vector<int> disc(n, -1), low(n, -1), parent(n, -1);
    std::vector<std::pair<int,int>> bridges;
    int timer = 0;
    std::function<void(int)> dfs = [&](int u) {
        disc[u] = low[u] = timer++;
        for (auto& [v, _] : g.getNeighbors(u)) {
            if (disc[v] == -1) {
                parent[v] = u;
                dfs(v);
                low[u] = std::min(low[u], low[v]);
                if (low[v] > disc[u]) {
                    int bu = std::min(u, v), bv = std::max(u, v);
                    bridges.push_back({bu, bv});
                }
            } else if (v != parent[u]) {
                low[u] = std::min(low[u], disc[v]);
            }
        }
    };
    for (int i = 0; i < n; ++i)
        if (disc[i] == -1) dfs(i);

    // --- Second pass: assign block IDs via DFS skipping bridges ---
    // Build adjacency set for fast bridge lookup
    std::set<std::pair<int,int>> bridgeSet(bridges.begin(), bridges.end());
    std::vector<int> blockId(n, -1);
    int blockCount = 0;
    for (int i = 0; i < n; ++i) {
        if (blockId[i] >= 0) continue;
        // DFS from i, skipping any edge in bridgeSet
        std::vector<int> stack = {i};
        blockId[i] = blockCount;
        std::vector<int> vertices;
        while (!stack.empty()) {
            int u = stack.back(); stack.pop_back();
            vertices.push_back(u);
            for (auto& [v, _] : g.getNeighbors(u)) {
                if (blockId[v] >= 0) continue;
                int a = std::min(u, v), b = std::max(u, v);
                if (bridgeSet.count({a, b})) continue;
                blockId[v] = blockCount;
                stack.push_back(v);
            }
        }
        blockCount++;
    }

    // Collect vertices per block
    std::vector<std::vector<int>> blocks(blockCount);
    for (int v = 0; v < n; ++v)
        blocks[blockId[v]].push_back(v);
    return blocks;
}
```

- [ ] **Step 2: Write the failing test**

In `test_graphs.cpp`, add after existing tests:

```cpp
static void test2EdgeConnectedBlocks() {
    // Simple triangle (3-cycle): no bridges, one block with all vertices
    {
        Graph g(3, 3);
        g.addEdge(0, 1);
        g.addEdge(1, 2);
        g.addEdge(2, 0);
        auto blocks = find2EdgeConnectedBlocks(g);
        TEST_ASSERT(blocks.size() == 1);
        TEST_ASSERT(blocks[0].size() == 3);
    }
    // Two triangles connected by a single bridge edge:
    // Triangle A (0-1-2), bridge 2-3, triangle B (3-4-5)
    // Expected: 2 blocks, block 0 = {0,1,2}, block 1 = {3,4,5}
    {
        Graph g(6, 7);
        g.addEdge(0, 1); g.addEdge(1, 2); g.addEdge(2, 0);  // triangle A
        g.addEdge(2, 3);  // bridge
        g.addEdge(3, 4); g.addEdge(4, 5); g.addEdge(5, 3);  // triangle B
        auto blocks = find2EdgeConnectedBlocks(g);
        TEST_ASSERT(blocks.size() == 2);
        for (auto& b : blocks) TEST_ASSERT(b.size() == 3);
    }
    std::cerr << "PASS: test2EdgeConnectedBlocks\n";
}
```

In `main()` of test_graphs.cpp, add a call to `test2EdgeConnectedBlocks()`.

- [ ] **Step 3: Build and run the test**

```bash
cd src && g++ -O0 -g -std=c++17 -I../refs/cadical/src test_graphs.cpp Solver.cpp IncrementalSolver.cpp SubtourDetector.cpp SecEncoder.cpp VariableManager.cpp TrajectoryLogger.cpp GraphPreprocessor.cpp ContractedMinCut.cpp -o test_graphs -L../refs/cadical/build -lcadical && ./test_graphs 2>&1 | grep -E "(PASS|FAIL)"
```

Expected: `PASS: test2EdgeConnectedBlocks` (plus other existing passes).

- [ ] **Step 4: Commit**

```bash
git add src/Solver.cpp src/test_graphs.cpp
git commit -m "feat: add find2EdgeConnectedBlocks helper

Tarjan bridge-finding + DFS for 2-edge-connected component
decomposition. Returns blocks for Phase 0 structural cut clauses."
```

---

### Task 2: Phase 0 — precomputed 2-EC block DFJ clauses

**Files:**
- Modify: `src/Solver.cpp` (`runIncremental` method, after `encodeBase`)

**Interfaces:**
- Consumes: `find2EdgeConnectedBlocks` from Task 1
- Produces: Block clauses added before the SEC loop

- [ ] **Step 1: Write the integration test (failing)**

In `test_graphs.cpp`, add:

```cpp
static void testPrecomputedBlockClauses() {
    // Two triangles with bridge 2-3
    Graph g(6, 7);
    g.addEdge(0, 1); g.addEdge(1, 2); g.addEdge(2, 0);
    g.addEdge(2, 3);
    g.addEdge(3, 4); g.addEdge(4, 5); g.addEdge(5, 3);

    auto blocks = find2EdgeConnectedBlocks(g);
    TEST_ASSERT(blocks.size() == 2);  // two blocks

    // For each block, find outgoing edges
    int totalClauses = 0;
    for (auto& block : blocks) {
        if ((int)block.size() == g.getNodes()) continue; // not proper subset
        std::set<int> blockSet(block.begin(), block.end());
        std::vector<int> clause;
        for (int u : block) {
            for (auto& [v, _] : g.getNeighbors(u)) {
                if (!blockSet.count(v)) {
                    clause.push_back(-1); // placeholder, just count
                }
            }
        }
        if (clause.size() >= 2) totalClauses++;
    }
    // Triangle A has 1 outgoing edge (2→3), Triangle B has 1 incoming (3→2).
    // Each directed edge gets a separate clause, but we skip <2 literals.
    // So: 0 clauses (each block has only 1 outgoing directed edge).
    // With both directions counted: 2 outgoing per block → 2 clauses.
    std::cerr << "PASS: testPrecomputedBlockClauses (clauses=" << totalClauses << ")\n";
}
```

Call in `main()`.

- [ ] **Step 2: Add Phase 0 code to `runIncremental`**

After line 285 (`std::cerr << "c total clauses: " << isolver.getNumClauses() << "\n";`) and before the skip-var reservation, add:

```cpp
    // ---- PHASE 0: Precomputed 2-EC block DFJ clauses ----
    int blockClauseCount = 0;
    if (precomputeBlocks_) {
        auto blocks = find2EdgeConnectedBlocks(g);
        for (const auto& block : blocks) {
            if ((int)block.size() >= g.getNodes()) continue; // not proper subset
            std::vector<bool> inBlock(g.getNodes(), false);
            for (int v : block) inBlock[v] = true;
            std::vector<int> clause;
            for (int u : block) {
                for (auto& [v, _] : g.getNeighbors(u)) {
                    if (!inBlock[v]) {
                        int lit = g.getAdj(u, v);
                        if (lit > 0) clause.push_back(-lit);
                    }
                }
            }
            if (clause.size() >= 2) {
                isolver.addClause(clause);
                blockClauseCount++;
            }
        }
        std::cerr << "c Phase 0: added " << blockClauseCount
                  << " DFJ clauses for " << blocks.size() << " 2-EC blocks\n";
    }
```

Add member `bool precomputeBlocks_ = true;` to `Solver` class (in Solver.hpp or as a member — check how other options like `useVertexSep_` are stored).

- [ ] **Step 3: Build and run test**

```bash
cd src && g++ -O0 -g -std=c++17 -I../refs/cadical/src test_graphs.cpp Solver.cpp IncrementalSolver.cpp SubtourDetector.cpp SecEncoder.cpp VariableManager.cpp TrajectoryLogger.cpp GraphPreprocessor.cpp ContractedMinCut.cpp -o test_graphs -L../refs/cadical/build -lcadical && ./test_graphs 2>&1 | grep -E "(PASS|FAIL)"
```

Expected: PASS for all tests.

- [ ] **Step 4: Commit**

```bash
git add src/Solver.cpp src/test_graphs.cpp
git commit -m "feat: add Phase 0 precomputed 2-EC block DFJ clauses

Adds structural DFJ clauses on 2-edge-connected block boundaries
before the SEC loop begins. Permanent clauses, zero iteration cost."
```

---

### Task 3: `OscillationTracker` struct + `buildBoundaryClause` helper

**Files:**
- Modify: `src/Solver.cpp` (add struct + helper before `runIncremental`)
- Test: `src/test_graphs.cpp` (add unit test)

**Interfaces:**
- Produces: `struct OscillationTracker { bool isOscillating(uint64_t, int); void record(uint64_t, int); }`
- Produces: `std::vector<int> buildBoundaryClause(const std::vector<int>& sideA, const Component& fullComp, const Graph& graph)`

- [ ] **Step 1: Add `OscillationTracker` struct and `buildBoundaryClause`**

Add before `runIncremental` (around line 170):

```cpp
struct OscillationTracker {
    int window;
    int minCutThreshold;
    int maxCutSize;
    std::unordered_map<uint64_t, int> history;

    OscillationTracker(int win, int minC, int maxC)
        : window(win), minCutThreshold(minC), maxCutSize(maxC) {}

    bool isOscillating(uint64_t hash, int currentIter) const {
        auto it = history.find(hash);
        if (it == history.end()) return false;
        return (currentIter - it->second) < window;
    }

    void record(uint64_t hash, int currentIter) {
        history[hash] = currentIter;
    }
};

static std::vector<int> buildBoundaryClause(
    const std::vector<int>& sideA_vertices,
    const Component& fullComponent,
    const Graph& graph)
{
    std::vector<bool> inSideA(graph.getNodes(), false);
    for (int v : sideA_vertices) inSideA[v] = true;

    // Collect all vertices outside sideA (B ∪ rest of graph)
    std::vector<bool> inFullComp(graph.getNodes(), false);
    for (int v : fullComponent.vertices) inFullComp[v] = true;

    std::vector<int> clause;
    for (int u : sideA_vertices) {
        for (auto& [v, _] : graph.getNeighbors(u)) {
            // Edge from sideA to outside sideA (to B or to rest of graph)
            if (!inSideA[v]) {
                int lit = graph.getAdj(u, v);
                if (lit > 0) clause.push_back(-lit);
            }
        }
    }
    return clause;
}

// Note: This clause covers ALL outgoing edges of sideA (to B + to rest of
// graph), making it an independently sound DFJ constraint on the proper
// subset sideA. The normal weak SEC clause on fullComponent is redundant
// with this for sideA's boundary, but harmless.
```

- [ ] **Step 2: Write the failing test**

```cpp
static void testOscillationTracker() {
    OscillationTracker tracker(10, 100, 10);

    uint64_t h1 = 0xAAAA;
    uint64_t h2 = 0xBBBB;

    // Not seen yet: not oscillating
    TEST_ASSERT(!tracker.isOscillating(h1, 0));

    // Record h1 at iter 0
    tracker.record(h1, 0);
    TEST_ASSERT(tracker.isOscillating(h1, 5));  // within window
    TEST_ASSERT(!tracker.isOscillating(h1, 10)); // exactly at edge (10 - 0 < 10? no)

    // Record again at iter 10
    tracker.record(h1, 10);
    TEST_ASSERT(tracker.isOscillating(h1, 15));

    // Different hash: not oscillating
    TEST_ASSERT(!tracker.isOscillating(h2, 15));

    // Record h2, h1 disappears from window
    tracker.record(h2, 100);
    TEST_ASSERT(!tracker.isOscillating(h1, 100)); // too old

    std::cerr << "PASS: testOscillationTracker\n";
}
```

- [ ] **Step 3: Write a test for `buildBoundaryClause`**

```cpp
static void testBuildBoundaryClause() {
    // Simple graph: triangle 0-1-2
    Graph g(3, 3);
    g.addEdge(0, 1, 10); g.addEdge(1, 0, 11);
    g.addEdge(1, 2, 12); g.addEdge(2, 1, 13);
    g.addEdge(2, 0, 14); g.addEdge(0, 2, 15);

    // Full component = all 3 vertices
    Component fullComp;
    fullComp.vertices = {0, 1, 2};
    fullComp.edges = {10, 11, 12, 13, 14, 15};

    // Side A = {0}
    auto clause = buildBoundaryClause({0}, fullComp, g);
    // {0}'s outgoing edges to V\{0} = 0→1, 0→2 → literals 10, 15
    // Clause: -10, -15
    TEST_ASSERT(clause.size() == 2);
    TEST_ASSERT(std::find(clause.begin(), clause.end(), -10) != clause.end());
    TEST_ASSERT(std::find(clause.begin(), clause.end(), -15) != clause.end());

    std::cerr << "PASS: testBuildBoundaryClause\n";
}
```

- [ ] **Step 4: Build and run**

```bash
cd src && g++ -O0 -g -std=c++17 -I../refs/cadical/src test_graphs.cpp Solver.cpp IncrementalSolver.cpp SubtourDetector.cpp SecEncoder.cpp VariableManager.cpp TrajectoryLogger.cpp GraphPreprocessor.cpp ContractedMinCut.cpp -o test_graphs -L../refs/cadical/build -lcadical && ./test_graphs 2>&1 | grep -E "(PASS|FAIL)"
```

Expected: PASS for all tests.

- [ ] **Step 5: Commit**

```bash
git add src/Solver.cpp src/test_graphs.cpp
git commit -m "feat: add OscillationTracker + buildBoundaryClause

OscillationTracker detects repeating component partitions within
a configurable iteration window. buildBoundaryClause constructs
a DFJ clause on sideA's outgoing edges within its parent component."
```

---

### Task 4: Wire oscillation escalation into SEC loop

**Files:**
- Modify: `src/Solver.cpp` (inside `runIncremental` SEC loop, around lines 600-650)

**Interfaces:**
- Consumes: `OscillationTracker`, `buildBoundaryClause`, existing `computeInternalMinCut`
- Produces: Oscillation-based cut clauses added during the SEC loop

- [ ] **Step 1: Add oscillation tracking to the SEC loop**

In `runIncremental`, after detecting components from the model (around line 357, after `auto components = SubtourDetector::detect(model, g);`) and before `encodeSecs`, add:

```cpp
        // ---- Phase 1: oscillation-guided cut escalation ----
        int oscClausesAdded = 0;
        for (const auto& comp : components) {
            if ((int)comp.vertices.size() < oscillationTracker_.minCutThreshold)
                continue;

            uint64_t hash = 0;
            for (int v : comp.vertices) {
                hash ^= std::hash<int>{}(v) + 0x9e3779b9 + (hash << 6) + (hash >> 2);
            }

            if (oscillationTracker_.isOscillating(hash, actions)) {
                auto mcr = computeInternalMinCut(comp, g, maxFlowVertLimit);
                if (mcr.cutSize >= 2 && mcr.cutSize <= oscillationTracker_.maxCutSize
                    && !mcr.sideA_vertices.empty()
                    && (int)mcr.sideA_vertices.size() < (int)comp.vertices.size())
                {
                    auto clause = buildBoundaryClause(mcr.sideA_vertices, comp, g);
                    if ((int)clause.size() >= 2) {
                        isolver.addClause(clause);
                        oscClausesAdded++;
                    }
                }
            }

            oscillationTracker_.record(hash, actions);
        }
        if (oscClausesAdded > 0) {
            std::cerr << "c Iteration: oscillation cut added for "
                      << oscClausesAdded << " components\n";
        }
```

Place this before the existing `encodeSecs` call. The iteration counter `actions` is incremented at the top of the while loop (line 331), so `actions` at this point is the current iteration number.

Add `OscillationTracker oscillationTracker_;` as a member of the Solver class, initialized in the constructor:

```cpp
// In Solver class (Solver.hpp if exists, or inline in Solver.cpp before runIncremental)
OscillationTracker oscillationTracker_{10, 100, 10};
```

Or just declare it locally at the top of `runIncremental` (simpler, no header change):

```cpp
OscillationTracker oscillationTracker_(oscillationWindow_, cutThreshold_, 10);
```

Where `oscillationWindow_` and `cutThreshold_` are new Solver member variables.

- [ ] **Step 2: Add Solver member variables**

In the `Solver` class definition (check `Solver.hpp` or `Solver.cpp` class definition):

```cpp
// Default: on
bool precomputeBlocks_ = true;
// Oscillation parameters
int oscillationWindow_ = 10;
int cutThreshold_ = 100;
```

Add setter methods:
```cpp
void setPrecomputeBlocks(bool b) { precomputeBlocks_ = b; }
void setOscillationWindow(int w) { oscillationWindow_ = w; }
void setCutThreshold(int t) { cutThreshold_ = t; }
```

- [ ] **Step 3: Build and run sanity test**

```bash
cd src && make && ./hcp-solver ../graphs/graph48.edge --incremental --cycle auto --time-limit 60 2>&1 | grep -E "(Phase 0|oscillation|HAMILTONIAN|TIMEOUT)" | head -10
```

Expected: Should see "Phase 0: added N DFJ clauses" and eventually HAMILTONIAN.

- [ ] **Step 4: Commit**

```bash
git add src/Solver.cpp
git commit -m "feat: wire oscillation-guided cut escalation into SEC loop

Adds stronger DFJ clauses on internal min-cuts only when component
partition fingerprints repeat within the oscillation window. Targets
the specific oscillation failure mode documented for graph470."
```

---

### Task 5: CLI flags and wiring

**Files:**
- Modify: `src/Solver.cpp` (main function, around lines 750-895)

- [ ] **Step 1: Add CLI flag parsing**

After the existing `--cut-size` / `--monitor-secs` / etc. flags (around line 885, before the `if (!solFile.empty())` block), add:

```cpp
        } else if (arg == "--precompute-blocks") {
            solver.setPrecomputeBlocks(true);
        } else if (arg == "--no-precompute-blocks") {
            solver.setPrecomputeBlocks(false);
        } else if (arg == "--oscillation-window") {
            if (i + 1 < argc) {
                solver.setOscillationWindow(std::stoi(argv[++i]));
            } else {
                std::cerr << "Error: --oscillation-window requires an integer\n";
                return 1;
            }
        } else if (arg == "--cut-threshold") {
            if (i + 1 < argc) {
                solver.setCutThreshold(std::stoi(argv[++i]));
            } else {
                std::cerr << "Error: --cut-threshold requires an integer\n";
                return 1;
            }
        }
```

Also update `printHelp` (if it exists) to document these flags.

- [ ] **Step 2: Build test**

```bash
cd src && make && ./hcp-solver ../graphs/graph48.edge --incremental --cycle auto --no-precompute-blocks --oscillation-window 5 --cut-threshold 50 --time-limit 30 2>&1 | grep -E "(Phase 0|oscillation)" | head -5
```

Expected: No "Phase 0" output (blocks disabled), should still solve.

- [ ] **Step 3: Commit**

```bash
git add src/Solver.cpp
git commit -m "feat: add CLI flags for oscillation-guided cut strengthening

--precompute-blocks / --no-precompute-blocks: toggle Phase 0
--oscillation-window N: iterations for partition repeat detection
--cut-threshold N: minimum component size for cut escalation"
```

---

### Task 6: Benchmark validation

**Files:**
- Modify: `docs/AGENTS.md` (update results table)
- Modify: `docs/superpowers/specs/2026-07-12-oscillation-guided-cut-strengthening-design.md` (update status)
- Run: Full FHCPP 18-graph benchmark at 120s

- [ ] **Step 1: Build production binary**

```bash
cd src && make clean && make
```

- [ ] **Step 2: Test graph470 with 120s limit**

```bash
cd src && timeout 130 ./hcp-solver ../graphs/graph470.edge --incremental --cycle auto --time-limit 120 2>&1 | grep -E "(Phase 0|oscillation|HAMILTONIAN|TIMEOUT|actions|total solver)" | tail -10
```

Expected: Should see "Phase 0: added N DFJ clauses", oscillation escalation messages, and ideally "HAMILTONIAN found" within 120s.

- [ ] **Step 3: Run full benchmark**

```bash
for g in graph48 graph162 graph171 graph197 graph223 graph237 graph249 graph252 graph254 graph255 graph424 graph446 graph470 graph491 graph506 graph522 graph526 graph529; do
    echo "=== $g ==="
    timeout 130 ./hcp-solver ../graphs/$g.edge --incremental --cycle auto --time-limit 120 2>&1 | grep -E "(Phase 0|HAMILTONIAN|TIMEOUT|OSCILLATION|actions|total solver)" | tail -5
    echo
done
```

- [ ] **Step 4: Update AGENTS.md**

Update the results table and graph470 row. If graph470 solves <120s, move it from TIMEOUT to SAT and update the open problems section.

- [ ] **Step 5: Final commit**

```bash
git add docs/AGENTS.md
git commit -m "docs: update benchmark with oscillation-guided cut results"
```
