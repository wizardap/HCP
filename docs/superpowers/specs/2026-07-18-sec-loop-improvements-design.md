# SEC Loop Improvements: Gomory-Hu Tree + 2-Component Deadlock Fix

**Date:** 2026-07-18  
**Status:** Draft  
**Goal:** Solve graph470 within 120s and improve SEC loop convergence across all graphs.

---

## 1. Problem Statement

The HCP solver's incremental SEC loop has two critical bottlenecks:

1. **Weak cuts:** Current SEC constraints (≥1 outgoing, ≥1 incoming edge per component) leave the SAT solver enormous freedom. There is no mechanism to prioritize tighter cuts or exploit global min-cut structure.

2. **2-component deadlock:** When the solver converges to 2 giant oscillating components (graph470's failure mode), all escalation mechanisms are disabled by the `components > 4` guard. The solver burns 3,421 iterations (~514s) oscillating without progress.

### Current Results (baseline)

- **17/18** FHCPP graphs solved at 120s timeout
- **graph470** (2,740 vertices, 4,509 edges): TIMEOUT at 120s. Solvable at 600s (514.7s, 3,421 iterations).
- Root cause: avg degree 1.65 (sparsest graph), 2-component oscillation resists SEC convergence.

### Success Criteria

- graph470 solves in <120s
- All 17 currently-solved graphs continue to solve at 120s (no regressions)
- Measurable reduction in average iteration count across the benchmark suite

---

## 2. Design Overview

Three changes, ordered by impact:

1. **Gomory-Hu tree on contracted graph** — prioritize tightest cuts, add stronger cardinality constraints on weak cuts
2. **2-component deadlock strategy** — dedicated escalation when stuck at 2 components for N iterations
3. **Implementation cleanup** — model extraction optimization, allocation reuse, hash fix

```
┌─────────────────── Enhanced SEC Loop ───────────────────┐
│                                                          │
│  SAT Solve → Extract Model → Union-Find Subtour Detect  │
│                      │                                   │
│              ┌───────┴────────┐                          │
│              │ Components > 2 │                          │
│              ├───Yes──────────┤                          │
│              │                │                          │
│    ┌─────────▼──────────┐    │                          │
│    │ Gomory-Hu Tree on  │    │                          │
│    │ contracted graph   │    │                          │
│    │ → prioritized SECs │    │                          │
│    │ → at-least-2 on    │    │                          │
│    │   weak cuts        │    │                          │
│    └─────────┬──────────┘    │                          │
│              │          ┌────┴──No (= 2)────┐           │
│              │          │                    │           │
│              │    ┌─────▼──────────────┐     │           │
│              │    │ 2-Comp Strategy    │     │           │
│              │    │ → vertex separator │     │           │
│              │    │ → at-least-4       │     │           │
│              │    │ → vtx-disjoint     │     │           │
│              │    └─────┬──────────────┘     │           │
│              │          │                    │           │
│              └────┬─────┘                    │           │
│                   │                          │           │
│         Oscillation Check (fixed hash)       │           │
│                   │                          │           │
│            Add Clauses → CaDiCaL             │           │
│                   │                          │           │
│              Continue Loop                   │           │
└──────────────────────────────────────────────┘
```

---

## 3. Gomory-Hu Tree on Contracted Graph

### 3.1 What

Compute the Gomory-Hu tree on the contracted component graph each iteration (when >2 components). Use it to generate SEC constraints prioritized by cut tightness, with stronger cardinality constraints on weak cuts.

### 3.2 Why

Currently, `SecEncoder::encodeSecs()` generates flat SEC constraints for each component independently. It has no notion of which cuts are globally weakest. A component with only 2 crossing edges to the rest of the graph gets the same ≥1 constraint as one with 50 crossing edges.

The Gomory-Hu tree gives the **all-pairs min-cut structure** in C-1 max-flow computations. This lets us:
- Identify the weakest cuts (most likely to persist as subtour boundaries)
- Add stronger constraints (at-least-2) specifically on those weak cuts
- Skip redundant constraints on cuts that are already well-constrained

### 3.3 Algorithm

```
Input: components[] from SubtourDetector, graph G
Output: prioritized SEC clauses

1. Build contracted graph:
   - Super-nodes: one per component (0..C-1)
   - Edge weight w(ci, cj) = number of directed edges between components ci and cj in G
   - Reuses existing logic from computeComponentMinCut (Solver.cpp:159-177)

2. Compute Gomory-Hu tree:
   - Standard algorithm: pick arbitrary root, for i = 1..C-1:
     a. Find current tree-neighbor t of component i
     b. Run max-flow (Dinic) from i to t on contracted graph
     c. Record cut value and partition (side-A, side-B)
     d. Update tree edges
   - Result: tree with C-1 edges, each labeled with min-cut value

3. Extract cuts sorted by weight (ascending):
   - Each tree edge (ci, cj, w) represents a partition of components
   - Traverse tree to find side-A for each cut

4. Generate SEC clauses per cut:
   - w ≤ GH_ATLEAST2_THRESHOLD (default 4):  at-least-2 on merged crossing edges
            (outgoing ∪ incoming) via DefaultAtLeastK::encode(boundary_lits, 2, auxBase)
   - w > GH_ATLEAST2_THRESHOLD:  standard SEC (≥1 outgoing clause, ≥1 incoming clause)
   Note: GH_ATLEAST2_THRESHOLD = 4 is conservative. Cuts with weight ≤ 4 have
   few crossing edges, so at-least-2 is a strong and cheap constraint.
   
5. Also generate standard per-component SECs (current behavior)
   to maintain the individual component constraints.
```

### 3.4 When to Apply

- Every iteration where `components.size() > 2`
- Replaces the current flat-only SEC generation at `Solver.cpp:708-709`
- The per-component SECs are still generated (additive, not replacement)
- The Gomory-Hu cuts provide additional cross-partition constraints

### 3.5 Complexity

- Contracted graph: O(V + E) construction (same as current)
- Gomory-Hu tree: O(C × Dinic(C, C²)) = O(C³·√C) worst case. For C < 100 typical: <1ms
- Cut clause generation: O(C × boundary_edges). Same order as current SEC generation.
- **No per-iteration regression** — all overhead is on the small contracted graph.

### 3.6 Implementation

**New file:** `src/GomoryHuTree.cpp` / `src/GomoryHuTree.hpp`

```cpp
struct GHEdge {
    int u, v;         // component indices
    int cutWeight;    // min-cut value
    std::vector<int> sideA_components;  // component indices on side A
};

struct GomoryHuTree {
    std::vector<GHEdge> edges;  // C-1 edges, sorted by cutWeight ascending
};

// Compute Gomory-Hu tree on the contracted component graph.
// Uses Dinic max-flow (already in ContractedMinCut.cpp).
GomoryHuTree computeGomoryHuTree(
    const std::vector<Component>& components,
    const Graph& graph);
```

**Modified file:** `src/Solver.cpp` — integration into SEC loop

```cpp
// After subtour detection, before SEC encoding:
if (components.size() > 2) {
    auto ghTree = computeGomoryHuTree(components, g);
    
    // Generate prioritized cuts from Gomory-Hu tree
    for (const auto& edge : ghTree.edges) {
        // Collect crossing edges for this cut partition
        auto crossingLits = collectCrossingLiterals(edge.sideA_components, components, g);
        
        if (edge.cutWeight <= 4 && crossingLits.size() >= 4) {
            // Strong constraint: at-least-2 on merged boundary
            auto kClauses = atLeastK.encode(crossingLits, 2, globalAuxBase);
            for (auto& cl : kClauses) isolver.addClause(cl);
        }
        // Standard per-component SECs still generated below
    }
}

// Existing per-component SEC generation continues unchanged
iterationSecEncoder.startAuxAt(isolver.getNumVars() + 1);
auto secClauses = iterationSecEncoder.encodeSecs(components, ...);
```

### 3.7 Constraints

- The Gomory-Hu tree is only useful when C > 2 (at 2 components, it degenerates to a single edge — handled by Section 4)
- The at-least-2 constraint is always sound for Hamiltonian cycles (HC must cross any proper cut at least twice)
- We keep existing per-component SECs as a baseline — the Gomory-Hu cuts are additive

---

## 4. 2-Component Deadlock Strategy

### 4.1 What

A dedicated escalation strategy for when the solver is stuck oscillating between 2 large components. Computes the vertex separator between the components and enforces at-least-4 on crossing edges plus vertex-disjoint path constraints.

### 4.2 Why

The current solver has zero escalation capability at ≤4 components:
- Stagnation detection: gated at `components > 4` (Solver.cpp:438)
- DFJ push: skipped at `curComps ≤ 4` (Solver.cpp:754)
- Union SEC: only fires at `> 10` components (Solver.cpp:715)

Graph470 spends its entire runtime at 2 components, oscillating between near-identical vertex partitions.

### 4.3 Trigger Condition

```cpp
int twoCompStreak = 0;  // consecutive iterations with exactly 2 components
const int twoCompThreshold = 20;  // trigger after 20 consecutive 2-comp iterations

// In main loop, after subtour detection:
if (components.size() == 2) {
    twoCompStreak++;
} else {
    twoCompStreak = 0;
}

if (components.size() == 2 && twoCompStreak >= twoCompThreshold) {
    // Apply 2-component strategy
    twoCompStreak = 0;  // reset to allow re-triggering
}
```

**Why 20 iterations?** Conservative enough to avoid firing during normal convergence (where component count may pass through 2 briefly). On graph470, the solver hits 2 components early and stays there for thousands of iterations — 20 is easily reached.

### 4.4 Algorithm

```
Input: 2 components (compA, compB), graph G
Output: strengthened SEC clauses

1. Collect ALL crossing edges:
   - edgesAB = directed edges from A to B (edge variables)
   - edgesBA = directed edges from B to A (edge variables)
   - allCrossing = edgesAB ∪ edgesBA

2. At-least-4 constraint:
   - A Hamiltonian cycle uses exactly 2 undirected crossings
   - Each undirected crossing = 1 forward + 1 backward directed edge variable
   - Therefore: at-least-4 directed crossing edges must be selected
   - Encode: DefaultAtLeastK::encode(allCrossing, 4, auxBase)

3. Vertex separator identification:
   - boundaryA = vertices in A with neighbors in B
   - boundaryB = vertices in B with neighbors in A
   - For each boundary vertex v in boundaryB:
     a. Collect edgesIn(v) = edges from A to v
     b. Collect edgesOut(v) = edges from v to A
     c. Add pairwise mutex: for each (eIn, eOut): clause {-eIn, -eOut}
        (HC cannot both enter and exit through the same vertex in the same direction)

4. Add all clauses to CaDiCaL
```

### 4.5 Soundness Argument

- **At-least-4 on crossing edges:** A Hamiltonian cycle visits every vertex exactly once. It must leave A at least once (≥1 edge A→B) and enter A at least once (≥1 edge B→A), and symmetrically for B. Total ≥ 4 directed crossing edges. This is a valid lower bound — it can never eliminate a valid HC.

- **Vertex-disjoint constraints:** If a boundary vertex v has an incoming edge from A and an outgoing edge to A both selected, the HC would enter v from A and immediately return to A, creating a U-turn. This is impossible in a Hamiltonian cycle through a proper vertex separator. The pairwise mutex `{-eIn, -eOut}` prevents this.

- **Regression safety:** Unlike greedy blocking (which adds extra SAT solves with assumptions), this only adds sound constraints. Unlike DFJ on large components (which was unsound for partitioned variants), these are structurally correct. The 20-iteration guard ensures it only fires during true stagnation.

### 4.6 Implementation

**Modified file:** `src/Solver.cpp`

New code block after the current stagnation detection section (~line 597), before the SEC encoding section:

```cpp
// --- 2-COMPONENT DEADLOCK STRATEGY ---
if (components.size() == 2 && twoCompStreak >= twoCompThreshold) {
    std::cerr << "c 2-comp deadlock detected (streak=" << twoCompStreak 
              << "), applying vertex-separator strengthening\n";
    twoCompStreak = 0;  // reset
    
    // Collect all crossing edges
    std::vector<bool> inA(g.getNodes(), false);
    for (int v : components[0].vertices) inA[v] = true;
    
    std::vector<int> allCrossing;
    for (int u : components[0].vertices) {
        for (auto& [v, edgeIdx] : g.getNeighbors(u)) {
            if (!inA[v]) allCrossing.push_back(edgeIdx);
        }
    }
    for (int u : components[1].vertices) {
        for (auto& [v, edgeIdx] : g.getNeighbors(u)) {
            if (inA[v]) allCrossing.push_back(edgeIdx);
        }
    }
    
    // At-least-4 on crossing edges
    if (allCrossing.size() >= 4) {
        iterationSecEncoder.startAuxAt(isolver.getNumVars() + 1);
        DefaultAtLeastK atLeastK;
        int auxBase = isolver.getNumVars() + 1;
        auto kClauses = atLeastK.encode(allCrossing, 4, auxBase);
        for (auto& cl : kClauses) isolver.addClause(cl);
        std::cerr << "c 2-comp: added at-least-4 on " << allCrossing.size() 
                  << " crossing edges (" << kClauses.size() << " clauses)\n";
    }
    
    // Vertex-disjoint constraints on boundary vertices of B
    for (int bv : components[1].vertices) {
        bool isBoundary = false;
        for (auto& [v, _] : g.getNeighbors(bv)) {
            if (inA[v]) { isBoundary = true; break; }
        }
        if (!isBoundary) continue;
        
        // Edges from A into bv
        std::vector<int> edgesIn;
        for (auto& [u, edgeIdx] : g.getNeighbors(bv)) {
            if (inA[u]) edgesIn.push_back(g.getAdj(u, bv));
        }
        // Edges from bv back to A
        std::vector<int> edgesOut;
        for (auto& [v, edgeIdx] : g.getNeighbors(bv)) {
            if (inA[v]) edgesOut.push_back(edgeIdx);
        }
        // Pairwise mutex: can't enter from A and exit to A through same vertex
        for (int eIn : edgesIn) {
            for (int eOut : edgesOut) {
                if (eIn > 0 && eOut > 0) {
                    isolver.addClause({-eIn, -eOut});
                }
            }
        }
    }
}
```

---

## 5. Implementation Cleanup

### 5.1 Model Extraction Optimization

**File:** `src/IncrementalSolver.hpp` / `.cpp`

Add overload:
```cpp
// Only query variables 1..maxVar, ignoring aux variables beyond that
std::vector<int> getModel(int maxVar) const;
```

**File:** `src/Solver.cpp`

Change:
```cpp
// BEFORE
auto model = isolver.getModel();
// AFTER  
auto model = isolver.getModel(2 * g.getEdges());  // only edge variables
```

### 5.2 SecEncoder Allocation Reuse

**File:** `src/SecEncoder.hpp`

Add member vectors:
```cpp
class SecEncoder {
    // ...
    std::vector<bool> inComponent_;  // reused, cleared per component
    std::vector<bool> isBoundary_;   // reused, cleared per component
};
```

**File:** `src/SecEncoder.cpp`

In constructor, pre-allocate to `numNodes`. In `getOutgoingLiterals` / `getIncomingLiterals` / `encodeSecs`, replace local `vector<bool> inComponent(numNodes, false)` with member clear-and-reuse.

### 5.3 Oscillation Hash Fix

**File:** `src/Solver.cpp` (line 680-683)

```cpp
// BEFORE: order-dependent
uint64_t hash = 0;
for (int v : comp.vertices) {
    hash ^= std::hash<int>{}(v) + 0x9e3779b9 + (hash << 6) + (hash >> 2);
}

// AFTER: order-independent (commutative XOR)
uint64_t hash = 0;
for (int v : comp.vertices) {
    hash ^= std::hash<int>{}(v) * 0x9e3779b97f4a7c15ULL;
}
```

---

## 6. Files Changed

| File | Change Type | Description |
|------|------------|-------------|
| `src/GomoryHuTree.cpp` | **New** | Gomory-Hu tree computation on contracted graphs |
| `src/GomoryHuTree.hpp` | **New** | Header for Gomory-Hu tree |
| `src/Solver.cpp` | Modified | Integrate GH tree, 2-comp strategy, oscillation hash fix, twoCompStreak counter |
| `src/SecEncoder.cpp` | Modified | Reuse `inComponent_` / `isBoundary_` member vectors |
| `src/SecEncoder.hpp` | Modified | Add member vectors |
| `src/IncrementalSolver.cpp` | Modified | Add `getModel(int maxVar)` overload |
| `src/IncrementalSolver.hpp` | Modified | Declare `getModel(int maxVar)` overload |
| `src/Makefile` | Modified | Add `GomoryHuTree.o` to build |

---

## 7. Testing Plan

| Test | Type | Pass Criteria |
|------|------|--------------|
| Gomory-Hu tree correctness | Unit | Known contracted graphs produce correct tree weights; all-pairs min-cuts match brute-force |
| 2-comp at-least-4 clauses | Unit | Correct clause generation for known 2-component inputs |
| Order-independent hash | Unit | Same vertex sets in different orders → same hash value |
| FHCPP 17 solved graphs | Regression | All 17 solve within 120s (no regressions) |
| graph470 target | Performance | Solves in <120s |
| Full 18-graph benchmark | Comparison | Total time ≤ current; iteration counts ≤ current for each graph |

---

## 8. Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Gomory-Hu cuts are redundant with per-component SECs | Low — just adds overhead, no regression | Gomory-Hu cuts are additive; can disable if no benefit |
| At-least-4 constraint too restrictive for sparse boundaries | Could cause UNSAT on valid instances | Only applied when `allCrossing.size() >= 4`; at-least-4 is provably sound for HC |
| 2-comp strategy fires too early (during normal convergence) | Adds unnecessary clauses, may slow down | 20-iteration threshold; reset counter on component count change |
| Gomory-Hu tree overhead for large C | Extra per-iteration cost | O(C³√C) on contracted graph; C rarely exceeds 100; negligible vs SAT time |

---

## 9. Out of Scope

- Clause compaction / periodic solver reset (Approach C from analysis)
- Comb inequalities or other polyhedral cuts
- Adaptive cycle parameter search (Approach B)
- Changes to the CRE base encoding
- CaDiCaL configuration tuning
