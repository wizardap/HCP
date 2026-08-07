# Design: Candidate Graph Optimization for Restricted 3-Opt

**Date**: 2026-08-07  
**Status**: Approved  
**Prerequisite**: Restricted 3-opt (spec 2026-08-06) already implemented and working.

---

## 1. Problem

The current `merge_three_cycles` brute-force iterates $\binom{N}{3}$ cycle triplets. On `graph470.col` (391 active cycles after 2-opt), this means ~9.88 million triplets × $O(|C_1| \cdot |C_2| \cdot |C_3|)$ inner edge checks — causing a multi-minute hang on a graph that should solve in ~10 seconds.

## 2. Solution: Inter-cycle Candidate Graph

Replace brute-force triplet enumeration with a precomputed adjacency structure between cycles. Only triplets where all three cycles share inter-cycle edges in $G$ are examined.

### 2.1 Data Structures (built once per `merge_three_cycles` call)

1. **`vertex_to_cycle: HashMap<i32, usize>`** — maps each vertex to its active cycle index.
2. **`cycle_neighbors: Vec<HashSet<usize>>`** — for each active cycle $i$, the set of other active cycle indices $j$ such that at least one edge $(u, v) \in G$ exists with $u \in C_i, v \in C_j$.

### 2.2 Construction (inside `merge_three_cycles`, before triplet loop)

```
for each active cycle index i:
    for each vertex u in cycles[i]:
        for each neighbor v of u in G.adjacency_list:
            j = vertex_to_cycle[v]
            if j != i:
                cycle_neighbors[i].insert(j)
```

Cost: $O(\sum_i |C_i| \cdot d)$ where $d$ is the average vertex degree in $G$. For `graph470`: $O(2740 \cdot 3.3) \approx 9000$ operations — negligible.

### 2.3 Filtered Triplet Search

```
for a in 0..n:
    for b in cycle_neighbors[a]:  // only adjacent cycles
        if b <= a: continue       // avoid duplicates
        for c in cycle_neighbors[a] ∩ cycle_neighbors[b]:
            if c <= b: continue   // avoid duplicates
            swap_three_nodes(cycles[a], cycles[b], cycles[c], g)
```

Cost: $O(N \cdot d^2)$ triplets instead of $O(N^3)$. On sparse graphs ($d \ll N$), this is orders of magnitude faster.

## 3. Scope

- **Modified function**: `merge_three_cycles` in `hcp_solver.rs` only.
- **No CLI changes**: Applied automatically when `--three-opt 1` is used.
- **No other functions affected**: `swap_three_nodes`, `cycle_join_three`, `two_opt` loop logic unchanged.

## 4. Verification

1. `cargo check && cargo build --release` in `src/cegar-ffi`.
2. Run `graph470.col` with `-t 1 --three-opt 1`: must complete within reasonable time (target: under 30 seconds for the local-search step).
3. Run `graph12.col`, `graph14.col`, `graph16.col` with `-t 1 --three-opt 1`: results must match or improve upon previous benchmark (no regression).
