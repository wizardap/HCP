# Spec: Parallel Cluster Sub-HCP Solving for CEGAR HCP Solver

**Date**: 2026-08-09  
**Status**: Approved  

---

## 1. Goal

When the CEGAR loop stalls with multiple unmergeable subcycles, instead of re-solving the entire graph SAT problem (which takes 475s+ on large graphs), decompose the remaining subcycles into clusters and solve smaller, independent Sub-HCP problems in parallel. Each sub-problem creates a fresh SAT encoding on a much smaller induced subgraph, avoiding clause bloat and dramatically reducing SAT solving time.

## 2. Integration: Adaptive Escalation Level 3

Extends the existing Adaptive Escalation framework:

| Level | Trigger | Components |
|---|---|---|
| Level 0 | Initial | 2-opt + ASP Cut-Set |
| Level 1 | `stall_count >= 3` | + Restricted 3-opt |
| Level 2 | `stall_count >= 6` | + Hard Fallback + Partial MTZ |
| **Level 3** | **`stall_count >= 9`** | **Parallel Cluster Sub-HCP Solving** |

When Level 3 triggers, the solver bypasses normal blocking clause generation and instead:
1. Partitions remaining subcycles into clusters
2. Spawns parallel sub-HCP solvers
3. Applies results back to the global cycle cover
4. Returns to the main CEGAR loop with fewer subcycles

## 3. Clustering Algorithm

### 3.1 Subcycle Adjacency Graph

Build a weighted graph $H$ where:
- Each node = one subcycle $C_i$
- Edge weight $w(C_i, C_j)$ = number of edges in $G$ connecting $V(C_i)$ to $V(C_j)$
- No edge if $w = 0$

### 3.2 Greedy Union-Find Clustering

1. Sort edges of $H$ by weight descending (prioritize subcycles sharing most cross-edges)
2. Initialize each subcycle as its own cluster (Union-Find)
3. Iterate edges: merge clusters if combined vertex count $\le$ `MAX_CLUSTER_SIZE` (default: 500)
4. Skip isolated subcycles (no cross-edges) — they cannot benefit from sub-HCP solving

### 3.3 Size Control

- Target cluster size: 50–500 vertices
- `MAX_CLUSTER_SIZE`: 500 (configurable)
- Clusters exceeding 1000 vertices after merging: split using the same algorithm recursively
- Clusters with only 1 subcycle: skip (no merging possible)

## 4. Sub-HCP Problem Creation

For each cluster with subcycles $\{C_1, C_2, \ldots, C_n\}$:

1. **Vertex set**: $V' = \bigcup_{i=1}^{n} V(C_i)$
2. **Edge set**: All edges in $G$ with both endpoints in $V'$ (induced subgraph $G[V']$)
3. **Fresh encoding**: New `Encoder` instance, new `CaDiCaL` instance
4. **Standard HCP encoding**: Same encoding method as the main solver (`-e 1`)
5. **Mini-CEGAR loop**: Run standard CEGAR with 2-opt (and optionally 3-opt) on the sub-problem
6. **Sub-problem timeout**: 60 seconds per cluster (configurable via `--sub-hcp-timeout`)

## 5. Threading Model

### 5.1 Thread Spawning

```
Main thread:
  1. Partition subcycles into m clusters
  2. For each cluster i:
     - Build induced subgraph G[V'_i]
     - Spawn thread_i with (subgraph_i, timeout)
  3. Join all threads, collect results
  4. Apply successful merges to global cycle cover
```

Each thread is fully independent:
- Own `Encoder` (created inside thread)
- Own `CaDiCaL` instance (created inside thread)
- Own copy of induced subgraph
- No shared mutable state → no synchronization needed

### 5.2 Result Type

```rust
enum SubHcpResult {
    Solved(Vec<i32>),  // Single cycle found on cluster's vertex set
    Unsolved,          // UNSAT, timeout, or error
}
```

### 5.3 Result Application

- `Solved`: Replace all subcycles in that cluster with the single merged cycle
- `Unsolved`: Keep original subcycles unchanged
- Safety: Cluster vertex sets are disjoint → all results can be applied simultaneously without conflicts

## 6. Fallback & Error Handling

| Scenario | Action |
|---|---|
| All clusters Solved | Best case. Subcycle count drops dramatically. Return to main CEGAR. |
| Some Solved, some Unsolved | Partial progress. Return to main CEGAR with fewer subcycles. |
| All Unsolved | No progress. Reset `stall_count`, fall back to Level 2. |
| Thread panic | Caught by `join()`. Treated as `Unsolved`. |

## 7. CLI Interface

New flags:
- `--sub-hcp-timeout <seconds>`: Timeout per cluster sub-HCP solve (default: 60)
- `--max-cluster-size <n>`: Maximum vertices per cluster (default: 500)

Existing flag interaction:
- `--adaptive-escalation 1`: Level 3 auto-triggers at `stall_count >= 9`
- `--adaptive-escalation 0`: Level 3 disabled (manual flags only)

## 8. Expected Performance Impact

### graph998.col (5000+ vertices, 163 subcycles at stall point)

| Metric | Current (Level 2) | With Level 3 (Cluster Sub-HCP) |
|---|---|---|
| SAT problem size | 134,898 clauses (full graph) | ~3,000-5,000 clauses per cluster |
| SAT solving time per iteration | 475 seconds | ~2-10 seconds per cluster |
| Total time per Level 3 round | N/A | ~10-15 seconds (8 clusters parallel) |
| Blocking clause accumulation | Millions (clause bloat) | Zero (fresh encoding each time) |

### Easy graphs (graph12.col, etc.)

No impact — Level 3 never triggers (solver finishes at Level 0 in < 0.2s).

## 9. Implementation Scope

Files to modify:
- `src/cegar-fix/src/options.rs`: Add `--sub-hcp-timeout` and `--max-cluster-size` flags
- `src/cegar-fix/src/main.rs`: Parse new flags, pass to solver
- `src/cegar-fix/src/hcp_solver.rs`: Add Level 3 logic, clustering, thread spawning, sub-HCP solving
- `src/cegar-fix/src/graph.rs`: Add `induced_subgraph(vertices: &HashSet<i32>) -> Graph` method

New module:
- `src/cegar-fix/src/parallel_sub_hcp.rs`: Clustering algorithm, sub-HCP solver, result collection
