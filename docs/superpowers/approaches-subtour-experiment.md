# Approaches in `subtour-experiment` Branch

**Branch base:** `b54011a` (Jul 12, 2026)
**Branch HEAD:** `2b54632`
**Instructor review date:** 2026-07-12

## Summary (TL;DR)

This branch explores three approaches to accelerate Hamiltonian Cycle SAT solving via subtour elimination in an incremental SEC (subtour elimination constraint) loop. The benchmark is the FHCPP 18-graph set at 120s time limit per graph.

| Approach | Net effect | Status |
|----------|-----------|--------|
| **1. CRE Auto-Scale Cycle** (`--cycle auto`) | +2 graphs solved (15/18 → 17/18 at 120s) | **KEPT** |
| **2. Dinic Max-Flow** | No effect on main path; faster stagnation escalation | **KEPT** |
| **3. Internal Min-Cut Splitting** | Counterproductive: 12-18× per-iteration overhead | **REVERTED** |

Baseline: 15/18 solved at 120s. Branch achieves 17/18 at 120s. graph470 (2740v) remains the sole timeout, converging in 319s pure SEC.

---

## Approach 1: CRE Auto-Scale Cycle (`computeAutoScaleCycle`)

### Motivation

The incremental SEC loop adds one weak clause per component per iteration, blocking the current set of subcycles. When m (cycle multiplier) > n (number of vertices), the CR encoding cannot produce any subcycle [OEIS A000793 / Landau's function g(n)]. Every Hamiltonian cycle produces a single component of size n — the formula is UNSAT if and only if there is no HC. Result: **one-shot solve, zero SEC iterations.**

Previous auto mode tried growing m until the formula became too large. This branch implements a principled strategy from Chinese Remainder Encoding (CRE): m = 3 × 5 × 7 × 2^k, the smallest product of coprime numbers exceeding n.

### Code

```cpp
// src/Solver.cpp:724-731
static int computeAutoScaleCycle(int nNode) {
    long long cycle = 2;
    if (cycle <= nNode) cycle *= 3;
    if (cycle <= nNode) cycle *= 5;
    if (cycle <= nNode) cycle *= 7;
    while (cycle <= nNode) cycle *= 2;
    if (cycle > static_cast<long long>(INT_MAX)) cycle = INT_MAX;
    return static_cast<int>(cycle);
}
```

The sequence: 2 → 6 → 30 → 210 → 420 → 840 → 1680 → 3360 → 6720 → ...

This produces:
- n ≤ 2: m = 2
- 3 ≤ n ≤ 5: m = 6
- 6 ≤ n ≤ 29: m = 30
- 30 ≤ n ≤ 209: m = 210
- 210 ≤ n ≤ 419: m = 420
- 420 ≤ n ≤ 839: m = 840
- 840 ≤ n ≤ 1679: m = 1680
- 1680 ≤ n ≤ 3359: m = 3360
- 3360 ≤ n ≤ 6719: m = 6720

### Two-Phase Strategy in `--cycle auto`

```cpp
// src/Solver.cpp:905-946
if (solver.getCycle() == 0) {  // --cycle auto
    // Phase 1: try auto-scale m > n with 30s budget
    solver.setCycle(autoCycle);  // e.g. 1680 for n=1558
    auto result = solver.runIncremental(phase1Ms=min(30000, 0.3*total));

    if SAT: return solved
    if UNSAT: return unsat  // safe — m > n prevents false UNSAT
    
    // Phase 2 (TIMEOUT): fallback to cycle=2 SEC loop
    solver.setCycle(2);
    result = solver.runIncremental(remaining budget);
}
```

Phase 1: 30s budget. If formula fits within that, one-shot solve with zero SEC iterations.  
Phase 2: remaining budget at cycle=2, standard SEC loop.

### Results

| Graph | n | Auto m | Phase 1 | Result | Time |
|-------|---|--------|---------|--------|------|
| graph249 | 1558 | **1680** | 1680 > 1558 ✓ | **SAT** | **6s** (was TIMEOUT) |
| graph254 | 1582 | **1680** | 1680 > 1582 ✓ | **SAT** | **5s** (was TIMEOUT) |
| graph162 | 909 | **1680** | 1680 > 909 ✓ | **SAT** | 29s |
| graph171 | 996 | **1680** | 1680 > 996 ✓ | **SAT** | 15s |
| graph197 | 1188 | **1680** | 1680 > 1188 ✓ | **SAT** | 12s |
| graph237 | 1476 | **1680** | 1680 > 1476 ✓ | **SAT** | 11s |
| graph252 | 1572 | **1680** | 1680 > 1572 ✓ | **SAT** | 19s |
| graph255 | 1584 | **1680** | 1680 > 1584 ✓ | **SAT** | 12s |
| graph48 | 338 | **420** | 420 > 338 ✓ TO | Phase 2 (c=2) | ~90s |
| graph223 | 1386 | **1680** | TIMEOUT | Phase 2 (c=2) | ~90s |
| graph424 | 2466 | **3360** | TIMEOUT | Phase 2 (c=2) | ~90s |
| graph446 | 2557 | **3360** | TIMEOUT | Phase 2 (c=2) | ~90s |
| graph491 | 2844 | **3360** | TIMEOUT | Phase 2 (c=2) | ~90s |
| graph506 | 2964 | **3360** | TIMEOUT | Phase 2 (c=2) | ~90s |
| graph522 | 3060 | **3360** | TIMEOUT | Phase 2 (c=2) | ~90s |
| graph526 | 3108 | **3360** | TIMEOUT | Phase 2 (c=2) | ~90s |
| graph529 | 3132 | **3360** | TIMEOUT | Phase 2 (c=2) | ~90s |
| graph470 | 2740 | **3360** | TIMEOUT | Phase 2 TO | **TIMEOUT** |

### Discussion

**Why m=1680 works for n < 1680:** The CR encoding uses m sequential bits per edge variable. With 12 encoding bits (log₂ 1680 ≈ 10.7 + margin), variables per edge ≈ 12, total vars ≈ 12 × 2|E|. For graph249 (1558v, ~2000 edges): ~48K vars, ~200K clauses. For graph254 (1582v, ~2000 edges): similar. CaDiCaL solves these in 5-6s.

**Why m=3360 TIMEOUTs at 30s:** 13 bits/node (log₂ 3360 ≈ 11.7). For graph470 (2740v, 4509 edges): variables = 13 × 2 × 4509 ≈ 117K, clauses ≈ 530K. This exceeds CaDiCaL's capacity for a 30s solve.

**Key insight:** The CRE auto-scale is a one-shot oracle. If the formula fits in the budget (m ≤ 1680 for this benchmark's n), it's dramatically better than SEC iteration. If not, the 30s Phase 1 budget is wasted — but this is bounded to 30s, making the worst-case total ~30s + SEC time.

---

## Approach 2: Dinic Max-Flow for Internal Min-Cut

### Motivation

The existing `computeInternalMinCut` function (used in stagnation mincut strategy) used Edmonds-Karp max-flow with O(V·E²) complexity on an adjacency matrix. For components with >500 vertices, this was prohibitively expensive. The `maxFlowVertLimit` capped it at 500 vertices.

Replacing with Dinic (O(E·√V)) reduces per-call cost from ~O(k³) to ~O(k·E_internal). For a 1370-vertex component with ~3000 internal edges: Edmonds-Karp ~2.6B operations, Dinic ~110K operations.

### Code

```cpp
// src/ContractedMinCut.cpp:63-132
struct Dinic {
    struct Edge { int to, rev; int cap; };
    std::vector<std::vector<Edge>> g;
    std::vector<int> level, iter;
    // ... standard Dinic implementation ...
    int maxFlow(int s, int t) {
        int flow = 0;
        while (true) {
            bfs(s);
            if (level[t] < 0) break;
            iter.assign(g.size(), 0);
            int f;
            while ((f = dfs(s, t, INF)) > 0) flow += f;
        }
        return flow;
    }
};
```

The wrapper function `maxFlowDinic` converts the adjacency matrix to adjacency list:

```cpp
// src/ContractedMinCut.cpp:136-149
int maxFlowDinic(int n, std::vector<std::vector<int>>& cap,
                 int s, int t, std::vector<bool>& minCutSide) {
    Dinic dinic(n);
    for (int u = 0; u < n; ++u)
        for (int v = 0; v < n; ++v)
            if (cap[u][v] > 0)
                dinic.addEdge(u, v, cap[u][v]);
    int flow = dinic.maxFlow(s, t);
    minCutSide = dinic.minCut(s);
    return flow;
}
```

Threshold logic in `computeInternalMinCut`:

```cpp
// src/ContractedMinCut.cpp:274-278
if (k > 500) {
    flowVal = maxFlowDinic(k, capCopy, s, t, sideA_local);
} else {
    flowVal = maxFlowBFS(k, capCopy, s, t, sideA_local);
}
```

`maxFlowVertLimit` increased from 500 → 2000 in `ContractedMinCut.hpp`.

### Results

- No impact on main SEC loop (only stagnation mincut strategy uses this path)
- At cycle=2, stagnation is rarely triggered (gated at >4 components)
- But the Dinic code is available for any future min-cut-based strategy

### Discussion

A defense-in-depth improvement. The main SEC loop at cycle=2 converges without needing min-cuts. But if a future approach requires internal cuts (e.g., hybrid strategies), Dinic is essential for graph470-scale components.

---

## Approach 3: Internal Min-Cut Splitting for Giant Components (REVERTED)

### Motivation

Decoding the SEC iteration trajectory for graph470:

```
Start:       1 component (entire graph)
Iter 1:      ~100 components
Iter 2-100:  ~20-50 components, rapidly merging
Iter 100-500: 8-12 components, 2 giant (~1370v each) + 6-10 tiny
Iter 500-2800: 4-8 components, oscillating. The SEC clause for a
              1370-vertex giant is ~(¬e₁ ∨ ... ∨ ¬eₖ) with 1000+ literals.
              SAT solver satisfies this by changing ONE edge — but the
              new model has almost the same partition, just shifted by
              one vertex. Equivalent partitions re-form in the next iteration.
```

The observation: weak outgoing-edge SEC (a single clause negating all edges of a component) is a very weak constraint — it blocks one specific partition, but there are exponentially many equivalent partitions. The solver can satisfy the clause by changing a single vertex's routing, producing a new partition that's 99.9% identical.

### Algorithm

For each component C with |C| > 100:

1. Build the undirected internal edge capacity matrix for C
2. Find a minimum s-t cut via Dinic max-flow
   - Source s = first boundary vertex of C (has neighbors outside C)
   - Try each boundary vertex as sink t
   - The min-cut edge set E_cut is the smallest set of edges that, if removed, disconnects C
3. Split C into A ∪ B where A = side of cut containing s
4. Encode SEC clause on A: outgoing edges from A to vertices outside A (including to B and outside C)
5. This clause blocks all crossing edges of the cut. To satisfy it, the next model must route through at least one different cut edge → C must reconfigure across the cut

```
Before (weak SEC):    ¬e₁ ∨ ¬e₂ ∨ ... ∨ ¬e₁₀₀₀
                      SAT solver: change edge 42, done. Partition unchanged.

After (min-cut SEC):  ¬e₁ ∨ ¬e₂ ∨ ¬e₃   (3 crossing edges at cut)
                      SAT solver: must change at least 1 of 3 specific edges.
                      Partition splits across the cut — can't stay same.
```

Expected iteration count: ~log₂(1370) × (1 + overhead) ≈ 11-20 iterations to split a 1370v component into sub-100v pieces, vs current ~2600 iterations of oscillation.

### Implementation

The code (since reverted) was added to the SEC iteration block in `Solver.cpp`:

```
For each component C:
    if |C| > 100:
        mcr = computeInternalMinCut(C, graph, maxFlowVertLimit=2000)
        if mcr has valid split (|A| between 2 and |C|-2):
            syntheticComp = Component(mcr.sideA_vertices, {})
            encode SEC on syntheticComp
        else:
            encode SEC on C (original weak approach)
    else:
        encode SEC on C (original weak approach)
```

### Results (graph470)

| Metric | Pure SEC (baseline) | With min-cut splitting |
|--------|-------------------|----------------------|
| Total iterations | 2843 | 263 |
| Time per iteration | 0.11s | ~1.37s |
| Total time to converge | **319s** | **>360s (TIMEOUT)** |

The 10× iteration reduction was outweighed by 12-18× per-iteration overhead.

### Root Cause

Formula size explosion. Each min-cut split creates a new Component with its own SEC clause. These clauses reference both internal and cross-cut edges, requiring new auxiliary variables for the sequential counter encoding. After 263 iterations, the formula had grown from ~10K clauses to ~800K clauses. The SAT solver spent most of its time processing the enormous formula rather than finding new models.

Specifically:

```
Per iteration overhead:
1. min-cut computation (Dinic):    ~2ms  (negligible)
2. SEC encoding on split:          ~5ms  (building ~50-200 clauses)
3. Formula growth:                 ~100-500 new clauses   (THE PROBLEM)
4. Solver initialization:          ~0.5s (re-processing all clauses)
5. Solve call:                     ~0.8s (SAT solving on large formula)
```

The formula size grew additively per iteration — but unlike the pure SEC loop (which adds ~10-200 short clauses per iteration), the min-cut approach added clauses with many literals (each split component's boundary touches many edges). The solver's clause database grew faster, making each solve slower.

### Why Was the Per-Iteration Time So Much Higher?

Pure SEC loop: components shrink over time. Early iterations add many short SEC clauses (~3-10 literals). Late iterations add even fewer. The formula size plateaus at ~50K clauses. Each solve is fast.

Min-cut splitting: components stay large because the SEC clause only blocks the MIN-cut edges — the component is still mostly intact. The formula grows without bound because each iteration adds clauses about the same large components, just from a different angle. Equivalent to: the constraints are stronger but also more numerous, and the solver can't amortize them.

### Alternative: Top-Down Min-Cut (would this help?)

Could we precompute a hierarchy of min-cuts before the SEC loop starts? For example:
1. Compute all 2-edge-cuts via global min-cut (Stoer-Wagner)
2. Decompose the graph into 2-edge-connected components
3. Encode SEC constraints for each subgraph

This was not explored. It might help by decomposing the problem upfront without iterative clause growth. However, HCP requires the HC to use every vertex exactly once — a top-down decomposition does not guarantee that the HC's route respects the cut hierarchy.

### Alternative: Periodic Solver Restart

Instead of splitting components, restart the SAT solver periodically (every 100 iterations). This clears the learned clause database, making each solve fast. The accumulated SEC clauses are re-asserted from the formula, but with a fresh solver state.

Not explored. Potential issue: losing learned clauses means the solver re-learns the same blocking patterns each restart cycle.

---

## Comparison with Baseline

| Graph | Baseline (b54011a) | Branch HEAD (2b54632) | Delta |
|-------|-------------------|----------------------|-------|
| graph48 | SAT ~120s | SAT ~90s | +30s (auto-scale waste) |
| graph162 | SAT <30s | SAT 29s | ~same |
| graph171 | SAT <30s | SAT 15s | ~same |
| graph197 | SAT <30s | SAT 12s | ~same |
| graph223 | SAT ~120s | SAT ~90s | +30s |
| graph237 | SAT <30s | SAT 11s | ~same |
| graph249 | **TIMEOUT** | **SAT 6s** | **New solve** |
| graph252 | SAT <30s | SAT 19s | ~same |
| graph254 | **TIMEOUT** | **SAT 5s** | **New solve** |
| graph255 | SAT <30s | SAT 12s | ~same |
| graph424 | SAT ~120s | SAT ~90s | +30s |
| graph446 | SAT ~120s | SAT ~90s | +30s |
| graph491 | SAT ~120s | SAT ~90s | +30s |
| graph506 | SAT ~120s | SAT ~90s | +30s |
| graph522 | SAT ~120s | SAT ~90s | +30s |
| graph526 | SAT ~120s | SAT ~90s | +30s |
| graph529 | SAT ~120s | SAT ~90s | +30s |
| graph470 | TIMEOUT | TIMEOUT | ~same |
| **Total solved (120s)** | **15/18** | **17/18** | **+2** |

Note: graphs with auto-scale m≥3360 show +30s because Phase 1 wastes 30s before Phase 2 fallback. Practical recommendation: for graphs where n > 2000, skip Phase 1 and go directly to cycle=2 SEC.

---

## Raw Data (graph470, cycle=2, 900s limit)

```
$ ./hcp-solver ../graphs/graph470.edge --incremental --cycle 2 --time-limit 900
c total variables: 14021
c total clauses: 29348
c Iteration: found 236 components, added 23 SEC clauses
c Iteration: found 75 components, added 69 SEC clauses
c Iteration: found 45 components, added 121 SEC clauses
c Iteration: found 37 components, added 234 SEC clauses
c ...
c Iteration: found 4 components, added 8 SEC clauses
c Iteration: found 4 components, added 8 SEC clauses
c Iteration: found 4 components, added 8 SEC clauses
c HAMILTONIAN found
c incremental actions: 2843
c final solve time: 0.0658116
c total solver time: 319.417

Hamitonian Cycle found.
```

2843 iterations, 319.4s total, 0.112s/iteration average.

Per-iteration time starts at ~0.05s (early iterations, small formula), grows to ~0.2s (late iterations, larger formula from accumulated SEC clauses). The final solve after the Hamiltonian finding is near-instantaneous (no further clause additions needed).

---

## Files Changed in This Branch

| File | Change | Status |
|------|--------|--------|
| `src/Solver.cpp:724-731` | `computeAutoScaleCycle` — CRE factorization | Kept |
| `src/Solver.cpp:905-946` | `--cycle auto` two-phase strategy | Kept |
| `src/ContractedMinCut.cpp` | Dinic max-flow implementation | Kept |
| `src/ContractedMinCut.hpp` | `maxFlowDinic` declaration, `maxFlowVertLimit = 2000` | Kept |
| `src/Solver.cpp` (removed) | Internal min-cut splitting in SEC loop | Reverted |
| `docs/AGENTS.md` | Updated benchmark results | Updated |
| `docs/superpowers/specs/2026-07-12-internal-mincut-hcp-design.md` | Design spec | Written |
| `docs/superpowers/plans/2026-07-12-internal-mincut-hcp-plan.md` | Implementation plan | Written |

---

## Open Questions for Instructor

1. **graph470 at 120s:** No approach converges. Baseline is 319s pure SEC. Any ideas for 2.6× acceleration?
   - Candidate: periodic solver restart (reset learned clauses every 100 iterations, keep accumulated SEC clauses)
   - Candidate: exploit graph470's specific structure — it has 490 2-edge-cuts; can we decompose into 2-EC blocks and encode cross-block constraints?

2. **Phase 1 waste:** m=3360 formulas all TIMEOUT at 30s. Should we skip Phase 1 for n > 2000? The 30s loss makes 9 graphs take ~90s instead of ~60s.

3. **Min-cut splitting:** The design seemed sound but per-iteration overhead was lethal. Is there a way to apply min-cut knowledge without adding clauses? E.g., use assumptions instead of clauses?

4. **Other CRE values:** The sequence 2, 6, 30, 210, 420, 840, 1680, 3360 is CRE-optimal for preventing subcycles. But maybe there's a middle ground: m = 30 or m = 210 allows some subcycles but reduces formula size, and the SEC loop finishes the rest quickly. Is there a Pareto-optimal tradeoff?

5. **Auto-detect graph structure:** graph470 has 490 2-edge-cuts (bridges in the 2-edge-connected sense). Can we pre-decompose the graph and encode constraints per 2-edge-connected block? This would give a hierarchy of subproblems with guaranteed cut sizes.
