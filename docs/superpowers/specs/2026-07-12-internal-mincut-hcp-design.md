# Internal Min-Cut Component Splitting for HCP SEC Loop

**Date:** 2026-07-12
**Status:** Design

## Motivation

graph470 (n=2740, e=4509) times out at 120s — converges in 314s but needs 2843 SEC
iterations. The bottleneck: for giant components (~1370 vertices), the weak outgoing-edge
SEC clause (~¬e₁∨...∨¬eₖ with hundreds of literals) blocks only one partition per iteration.
Exponentially many equivalent partitions re-form, causing ~2600 iterations of oscillation
at 4-8 components.

## Approach

Replace weak outgoing-edge SEC with **internal min-cut splitting** for large components.
Per iteration: compute the minimum edge cut within each giant component, split across that
cut, and encode SEC on the smaller side. This creates a divide-and-conquer convergence
pattern instead of the current "block one partition at a time" approach.

## Algorithm

```
For each component C where |C| > threshold (100v):
  1. Build undirected capacity matrix for edges within C
  2. Find min s-t cut via Dinic max-flow
     - source s = first boundary vertex of C (has neighbor outside C)
     - try each other boundary vertex as sink t
  3. Split C → A ∪ B where A = side of cut containing s
  4. Encode SEC clause on A's sub-boundary:
     - outgoing edges from A to outside-A within C
     - incoming edges from outside-A to A within C
  5. This clause blocks all edges of the cut → forces C to reconfigure
     across at least one cut edge → component splits at the cut

If no useful cut found (|A| == |C| or cut_size == 0), fall back to
current weak outgoing-edge SEC for the whole component.
```

## Key Properties

- **Stronger constraints**: A min-cut with 2-3 crossing edges produces a 2-3 literal
  clause vs hundreds of literals in the current approach. Fewer literals = stronger
  constraint = SAT solver finds fewer violating models.
- **Divide and conquer**: Each iteration splits a 1370v component into ~685+685,
  then 342+342, etc. After ~log₂(1370) ≈ 11 iterations, no component >100v.
- **Sound**: The min-cut edge set, when blocked, forces at least one edge of the
  internal cut to differ from the current component cycle. The Hamiltonian cycle
  through C must cross the cut via different edges.
- **Not guaranteed complete**: If the min-cut splits C into A with |A| = 1 (cut
  edge from a single vertex to the rest), the SEC on that single vertex is correct
  but weak. Fall back to weak SEC for this case.

## Implementation Changes

### 1. Dinic max-flow (replaces Edmonds-Karp in ContractedMinCut.cpp)

- Add `maxFlowDinic(n, cap, s, t, minCutSide)` — O(E·√V) vs O(V·E²)
- Same interface as current `maxFlowBFS`
- Required for 1370v components: Edmonds-Karp would take O(1370·4509) ≈ 6M per
  flow computation, which with ~10 sink tries = 60M operations. Dinic reduces to
  O(4509·√1370) ≈ 167K per computation.
- Keep `maxFlowBFS` as a fallback for small components (< 50v)

### 2. Increase maxFlowVertLimit (ContractedMinCut.hpp)

- Change default from 500 to 2000
- Edmonds-Karp cap at 500; above 500 use Dinic

### 3. Wire into SEC iteration loop (Solver.cpp)

- In the main SEC iteration block (around line 619-648), before calling
  `encodeSecs`, split large components via `computeInternalMinCut`
- For each large component C, if min-cut found, create a synthetic Component
  from `sideA_vertices` and encode SEC on that
- If no min-cut found, use the original component C with weak outgoing-edge SEC

```
For each component C in components:
  if |C| > 100:
    result = computeInternalMinCut(C, graph, 2000)
    if result has valid split:
      encode SEC on result.sideA_vertices
    else:
      encode SEC on C (current weak approach)
  else:
    encode SEC on C (current weak approach)
```

### 4. Remove unused stagnation code

- `mincut` stagnation strategy currently calls `computeInternalMinCut`, but this
  path is now subsumed by the main loop. Remove the mincut branch from
  stagnation escalation to avoid redundant computation.
- Keep `union` and `greedy` stagnation strategies as-is for the near-convergence
  case (≤10 components, where internal min-cut is not needed).

## Performance Impact

| Component size | Dinic time (est.) | Weak SEC time | Net |
|---------------|-------------------|---------------|-----|
| 1370v | ~2ms | <0.1ms | +2ms/iter |
| 685v | ~0.5ms | <0.05ms | +0.5ms/iter |
| 342v | ~0.2ms | <0.02ms | +0.2ms/iter |

If iterations drop from 2843 to ~200 (14x reduction), total time at 2ms/iter =
0.4s overhead vs 314s current. Net saving: ~300s.

## Risk

- **Dinic bug on asymmetric graphs**: Undirected capacity matrix must be symmetric.
  Current code constructs `cap[u][v] += 1` per directed edge; with both directions
  this gives `cap[u][v] = 2`. Dinic works on general capacities; this is fine.
- **Component too small for useful cut**: For a component with 4 vertices or fewer,
  min-cut may isolate a single vertex. Fallback weak SEC handles this.
- **Memory for 2000x2000 matrix**: 4M entries × 4 bytes = 16MB per call. Allocated
  per iteration, freed after. Acceptable.

## Testing

- Unit test for Dinic on known small graphs
- Integration test: graph470 with `--cycle auto --time-limit 360` should converge
  in <120s
- Regression: all other FHCPP 18 graphs must still solve at 120s

## Success Criteria

- graph470 solves within 120s with `--cycle auto` (was TIMEOUT)
- No regression on other 17 FHCPP graphs
- Total SEC iterations for graph470 drops from ~2843 to <500
