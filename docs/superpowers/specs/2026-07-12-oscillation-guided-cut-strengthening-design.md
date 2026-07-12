# Oscillation-Guided Cut Strengthening for HCP SEC Loop

**Date:** 2026-07-12
**Status:** Design

## Motivation

graph470 (n=2740, e=4509) times out at 120s, converging in 319s. The SEC loop
spends iterations 500-2800 oscillating among 4-8 near-identical partitions:
the weak OR-clause per component (¬e₁ ∨ ... ∨ ¬eₖ, k > 1000 for giant
components) is trivially satisfied by flipping one arbitrary edge, producing a
99.9% identical partition in the next iteration.

Previous attempt (internal min-cut splitting, Approach 3, reverted) applied
a stronger cut every iteration unconditionally. This dropped iterations from
2843 → 263 but increased per-iteration overhead 12-18× via formula bloat,
making total time worse.

This design applies the stronger cut **only when empirically needed**:
when the same partition repeats (oscillation detected), and from **structural
cuts computed once upfront** (zero iteration cost).

## Approach

Two independent pieces compose naturally:

### C: Precomputed 2-Edge-Connected Block Clauses (upfront, one-time)

Run bridge-finding (Tarjan) on the static graph once, before the SEC loop.
Identify 2-edge-connected components (blocks). For each block B that is a
proper subset of vertices, compute the set of outgoing directed edges from
B to the rest of the graph. Add one permanent DFJ clause:

    ¬e₁ ∨ ¬e₂ ∨ ... ∨ ¬eₜ   for each block's outgoing edges

**Soundness:** In any Hamiltonian cycle, for any proper subset S of vertices,
at least one outgoing edge of S must be deselected (the HC returns to S after
leaving, and the same edge cannot be traversed in both directions).

For graph470 with 490 2-edge-cuts, expected 500-1000 blocks, each with
2-4 outgoing edges → 500-1000 binary/ternary clauses. Negligible formula
growth (vs 29K clauses in the base formula). These are structural constraints
independent of the current solution — they seed the formula with the
"expensive" information that the SEC loop would eventually discover over
hundreds of iterations.

**Cost:** O(V+E) for bridge-finding + one DFS for block assignment. Called
once, before any `solve()`.

### B: Oscillation Fingerprinting (per-iteration, targeted)

After each `solve()`, hash each component's vertex set (sorted list → 64-bit
FNV-1a or similar). Maintain a ring buffer mapping hash → `last_seen_iteration`.

If a component's hash repeats within a window of N iterations (default 10),
flag it as oscillating. On oscillation detection:
1. Compute internal min-cut on that component's current vertex set
   (using existing `computeInternalMinCut` from ContractedMinCut.cpp)
2. If a valid cut found (split A + B with 2+ edges), add one permanent DFJ
   clause on A's boundary: ¬a₁ ∨ ... ∨ ¬a_s where a_i are A's outgoing edges
   to outside-C (including to B and to rest of graph)
3. Reset oscillation flag when the component's fingerprint changes

**Why oscillation detection solves the bloat problem:** The cut clause is
added only for components empirically proven stuck, not speculatively every
round. graph470 has exactly 2 giant components oscillating — most iterations
add 0 cut clauses, only the ~50-100 iterations where fingerprint repeats.
Compare to Approach 3 which added 2 cut clauses per iteration unconditionally
for 263 iterations (526 total clauses). This approach adds <100 clauses total
for graph470.

## Algorithm

```
Phase 0 — Precomputed structural cuts:
  bridges = findBridges(graph)
  blocks = find2EdgeConnectedComponents(graph, bridges)
  for each block B in blocks:
    if |B| == graph.nNode: continue (not a proper subset)
    outgoing = findOutgoingEdges(B, graph)
    if outgoing.size() < 2: continue
    addClause([-e for e in outgoing])

Phase 1 — SEC loop (existing, with oscillation extension):
  hashHistory = empty_map  // component_hash → last_seen_iteration
  
  while solving:
    solve()
    components = detectSubtours(model)
    
    for each component C:
      hash = fingerprint(C.vertices)
      lastSeen = hashHistory[hash]
      isOscillating = (lastSeen != 0) and (iteration - lastSeen < OSC_WINDOW)
      
      if isOscillating and |C.vertices| > MIN_CUT_THRESHOLD:
        mcr = computeInternalMinCut(C, graph, maxFlowVertLimit)
        if mcr.valid and cut_size between 2 and MAX_CUT_SIZE:
          clause = buildBoundaryClause(mcr.sideA_vertices, C, graph)
          addClause(clause)    // one permanent DFJ clause
      
      hashHistory[hash] = iteration
    
    addNormalWeakSECClauses(components)
```

## Data Structures

```cpp
// In Solver.cpp (add to existing members):
struct OscillationTracker {
    static constexpr int WINDOW = 10;
    static constexpr int MIN_CUT_THRESHOLD = 100;    // min vertices for cut
    static constexpr int MAX_CUT_SIZE = 10;           // cap cut clause size
    
    // hash → last iteration seen
    std::unordered_map<uint64_t, int> history;
    
    bool isOscillating(uint64_t hash, int currentIter) const;
    void record(uint64_t hash, int currentIter);
    void prune();  // optional: remove entries older than currentIter - WINDOW*2
};
```

## Implementation Changes

### New: `SecEncoder::encodeBoundaryClause` (or helper in Solver.cpp)

```
buildBoundaryClause(sideAVertices, fullComponent, graph):
    collect all outgoing edges from sideAVertices to outside fullComponent
    return vector of negated literals (one clause)
```

### New: `find2EdgeConnectedBlocks` (in GraphPreprocessor or Solver.cpp)

```
find2EdgeConnectedBlocks(graph):
    bridges = findBridges(graph)   // already in GraphPreprocessor
    // Tarjan bridge-finding + DFS for block assignment
    // Returns vector<vector<int>> — each block is a list of vertex IDs
```

### Modified: `Solver::runIncremental` SEC loop

Add oscillation tracking before the normal `encodeSecs` call. ~30 lines.

### Existing (no changes): `ContractedMinCut.cpp`

`computeInternalMinCut` stays as-is. Dinic at k > 500, BFS at k ≤ 500.
`maxFlowVertLimit = 2000` (from previous session).

## CLI Options

New flags:
- `--oscillation-window N` (default 10): how many iterations to track
- `--precompute-blocks` (default: on): enable Phase 0 structural cuts
- `--cut-threshold N` (default 100): minimum component size for cut escalation

## Success Criteria

1. graph470 solves within 120s with `--cycle auto` (was TIMEOUT at 319s)
2. No regression on other 17 FHCPP graphs at 120s
3. Total SEC iterations for graph470 drops from ~2843 to <1000 (target: <500)
4. Per-iteration time increases by <10% vs baseline (target: <5%)

## Risks and Mitigations

| Risk | Mitigation |
|------|-----------|
| Precomputed block clauses are UNSAT (solver can't satisfy all structural cuts + HC encoding) | Soundness proof: for any HC, for any proper subset S, at least one outgoing edge is false. The DFJ clause ¬e₁∨...∨¬eₜ is always satisfied by any HC. Only risk is if block boundary has <2 edges (trivial: skip) |
| Oscillation fingerprinting doesn't fire (no fingerprints repeat) | graph470 data shows clear oscillation pattern (2843 iterations, 4-8 components repeating). If false, at worst we add 0 cut clauses, same as baseline |
| Precomputed blocks slow (graph470 has 490 2-edge-cuts) | O(V+E) bridge-finding. For graph470 (2740v, 4509e), DFS completes in <1ms |
| Component sets identical across iterations produce false oscillation (e.g., 4 components converging naturally between structural rearrangements) | Window N=10 prevents false positives from transient repeats. Only sustained repetition triggers escalation |
| Cut clause adds formula bloat (same failure as Approach 3) | Capped at ~100 clauses total (only oscillating components, not all components every iteration). Compare to Approach 3's 526 clauses over 263 iterations, each with sequential counter encoding. This is 1 trivial clause per escalation |

## Testing

- Unit test: `find2EdgeConnectedBlocks` on graphs with known block structure
- Unit test: oscillation fingerprinting with synthetic component sequences
- Integration: graph470 with `--cycle auto --time-limit 120` solves <120s
- Regression: all 18 FHCPP graphs at 120s with no regression

## Open Questions

1. **Window size**: 10 iterations is a guess. graph470's typical oscillation
   cycle appears to be 2-4 iterations. Should window be adaptive based on
   observed cycle frequency? Or just pick N=5 and observe empirically?

2. **Cut clause size cap**: MAX_CUT_SIZE=10 prevents huge clauses from very
   small cuts. Could a tiny side-A (2-3 vertices) with 10+ boundary edges
   actually be useful? Probably not — the clause is too weak. Cap at 10.

3. **Interaction with --stagnation-strategy**: The oscillation cut clause
   serves a similar purpose to the DFJ stagnation strategy (adds ¬e₁∨...∨¬eₖ
   for each component). Should oscillation escalation replace DFJ stagnation,
   or complement it? Design assumes complement: oscillation catches mid-loop
   stagnation (4-8 components), DFJ catches near-convergence (2-4 components).
