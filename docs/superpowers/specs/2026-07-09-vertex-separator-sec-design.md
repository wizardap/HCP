# Vertex-Separator SEC Encoding Design

## Problem

Large sparse graphs (e.g., graph470: 2740 nodes, avg deg ~3.3) timeout
because the incremental SAT loop stagnates: the solver repeatedly finds
similar subtour components with large vertex sets but small boundary
sets (1-3 boundary vertices). Standard SEC generates long clauses that
propagate poorly in CDCL, wasting solve calls on near-identical partitions.

## Root Cause

For each SAT-model component C with boundary vertex set
S = {v ∉ C | ∃u ∈ C: (u,v) ∈ E}, the crossing structure constrains
the Hamiltonian cycle:

|S| size | Required crossing pattern
|--------|--------------------------
| 1      | Cycle must enter & exit C through same vertex v → visits v twice → impossible unless C=∅
| 2      | Cycle must enter through v1, exit through v2 (or vice versa) — must use both
| ≥3     | Any 2 distinct boundary vertices suffice

Standard SEC (`E_out ≥ 1, E_in ≥ 1`) ignores this structure. For |S| ≤ 2,
it produces 2 clauses that propagate independently instead of one
stronger constraint.

## Design

### New CLI flags

```
--vertex-sep            Enable vertex-separator SEC (all 3 improvements)
--vtx-sep-threshold <int>  |S| threshold for cardinality encoding (default: 4)
--vtx-sep-card-only     Only enable cardinality (skip vertex-disjoint clauses)
```

### Three independent improvements

#### A. Cardinality ≥ 2 for |S| ≤ threshold

When |S| ≤ threshold (default 4), encode a single cardinality constraint
`sum(E_out ∪ E_in) ≥ 2` using sequential counter instead of 2 separate
clauses `E_out ≥ 1, E_in ≥ 1`.

**Why stronger:** CDCL propagates cardinality constraints via watched
literals on the sequential counter chain — unit propagation fires when
k-1 literals are false, which doesn't happen with 2 separate clauses.

**Threshold rationale:** |S| small → boundary edges concentrated on few
vertices → cardinality encoding has few aux vars. For |S| > 4, boundary
edges are spread enough that 2 separate clauses are fine.

#### B. Vertex-disjoint crossing for |S| = 2

When exactly 2 boundary vertices exist, add binary clauses forbidding
both crossing edges from using the SAME vertex:

```
For each v ∈ S:
  For each pair (e1, e2) ∈ (E_out ∪ E_in) both incident to v:
    add ¬e1 ∨ ¬e2
```

These are NOT redundant with the base encoding: base encoding prevents
2 edges entering v or 2 edges leaving v — it does NOT prevent 2 edges
from C both going to v (one incoming, one outgoing through v).  The
vertex-disjoint clause ensures the cycle must use both boundary vertices.

**Skip when |E_out ∪ E_in| is small (< 4):** not enough literals for the
solver to matter.

#### C. Articulation-point-aware strengthening

Precompute cut vertices of the original graph ONCE before the SAT loop
(1 DFS pass, O(V+E)). During the loop, for each component C:

1. Compute S = boundary vertices
2. If any s ∈ S is a precomputed articulation point:
   - Add extra clause `sum(E_out ∪ E_in) ≥ 3` via sequential counter
     (reasoning: the cycle must enter region through s, traverse C,
     and exit — but with only 2 boundary edges, at least one of which
     goes through s, the cycle would visit s twice if it both enters
     and exits through s)
   
   Actually: if |S| ≥ 3 and s is an articulation point, the cycle can
   enter through s and exit through a different vertex — standard SEC
   suffices.  This clause is only beneficial when |S| = 1 or 2 AND s
   is an articulation point.

   For |S| = 1 and s articulation: already handled by (A) cardinality.
   
   **Decision:** skip this sub-feature for now — (A) and (B) cover the
   impactful cases.  If benchmarks show stagnation persists on |S|=1
   components despite cardinality, add it later.

### Integration with existing code

No dependency on unmerged code from 2026-07-09 spec.  Files
`SequentialCounter.hpp/.cpp` and `SecEncoderOptimized.hpp/.cpp`
DO NOT EXIST on current branch — implement vertex-separator as a
standalone addition to the existing `SecEncoder`.

Two integration strategies (choose 1):

**Strategy 1 — Modified SecEncoder (simpler, recommended):**
Add vertex-separator logic directly into `SecEncoder::encodeSecs`:

```cpp
// SecEncoder::encodeSecs gets a new flag parameter
vector<vector<int>> encodeSecs(
    const vector<Component>& components,
    bool useVertexSep = false,
    int vtxSepThreshold = 4
);
```

Inside, for each component:
1. Compute vertex boundary S
2. If |S| ≤ threshold: use sequential counter (inline encoding) 
3. If |S| == 2: add vertex-disjoint clauses
4. Else: emit 2 separate clauses (current behavior)

Sequential counter logic inlined into SecEncoder (no separate
SequentialCounter class).  Algorithm is simple (≤ 50 LOC).

**Strategy 2 — New VertexSecEncoder (cleaner, more code):**
New file `src/VertexSecEncoder.hpp/.cpp` wrapping SecEncoder:

```cpp
class VertexSecEncoder {
    SecEncoder baseEncoder;
    int vtxSepThreshold;
    vector<bool> articulationPoints;
public:
    VertexSecEncoder(const Graph& g, int threshold);
    void setArticulationPoints(const vector<bool>& ap);
    vector<vector<int>> encodeSecs(
        const vector<Component>& components);
};
```

**Recommendation: Strategy 1 for first iteration** — less code, same
effect.  Extract to standalone class only if SecEncoder grows unwieldy.

### Per-component flow (Strategy 1)

```
1. Compute E_out (outgoing edges), E_in (incoming edges) — existing code
2. Compute vertex boundary S:
   For each edge e ∈ E_out ∪ E_in:
     find endpoint not in C → add to S
3. If |S| ≤ vtxSepThreshold:
   a. Merge E_out and E_in into allBoundary
   b. Emit sequential counter: sum(allBoundary) ≥ 2
   c. If |S| == 2 AND |allBoundary| ≥ 4:
      emit vertex-disjoint ¬e1 ∨ ¬e2 clauses
4. Else:
   a. Emit E_out clause (existing)
   b. Emit E_in clause (existing)
```

### Changes to Solver.cpp

In `runIncremental`, at the SEC encoding block:

```cpp
// Current code (line ~647):
SecEncoder secEncoder(g);
auto secClauses = secEncoder.encodeSecs(components);

// Change to:
SecEncoder secEncoder(g);
auto secClauses = secEncoder.encodeSecs(components, useVertexSep, vtxSepThreshold);
```

### When vertex-sep helps most

- |S| = 1: rare in practice (component has single boundary vertex).
  When it occurs, cardinality ≥ 2 is equivalent to standard SEC but
  propagates better.
- |S| = 2: common for large sparse graphs with narrow "necks".
  Vertex-disjoint clauses add real power: solver cannot use same
  boundary vertex for both crossing edges.
- |S| = 3-4: cardinality ≥ 2 is beneficial; vertex-disjoint not needed.

## Testing

### Unit tests (new file: test_vertex_separator.cpp)
- `testBoundarySingle()`: component with 1 external neighbor → |S| = 1
- `testBoundaryTwo()`: component with 2 external neighbors → |S| = 2
- `testBoundaryMany()`: component with many external neighbors → |S| > threshold
- `testVertexDisjointEncoding()`: verify ¬e1 ∨ ¬e2 generated for same-vertex edges
- `testArticulationPoints()`: verify correct detection on known graph

### Integration
- Run `--incremental --vertex-sep` on all 36 test graphs
- Compare against baseline: same SAT/UNSAT, fewer or equal iterations
- Specifically benchmark graph424 (2466 nodes) and graph470 (2740 nodes)

## Risk / Mitigation

| Risk | Mitigation |
|------|------------|
| Vertex-disjoint clauses redundant with base encoding | Verify by counting unit propagations with/without flag |
| |S| computation adds per-iteration overhead | O(deg(C)) per component — negligible vs solve time |
| Threshold wrong for some graphs | Independent flag + adjustable threshold for benchmarking |
| No benefit for dense graphs | Skip when |S| > threshold (default 4), fall through to standard SEC |

## Success Criteria

- graph424 (2466 nodes): solve ≤ 60s with --vertex-sep (vs 84-91s baseline)
- graph470 (2740 nodes): solve ≤ 120s with --vertex-sep (vs timeout)
- No regression on small graphs (< 200 nodes): within 5% of baseline
- Component count per iteration decreases or stays same on all instances
