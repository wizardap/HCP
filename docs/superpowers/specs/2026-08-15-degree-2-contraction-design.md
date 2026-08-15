# Design Spec: Degree-2 Path Contraction & Invariant Preprocessing

**Date:** 2026-08-15  
**Topic:** Degree-2 Path Contraction & Invariant Preprocessing for Hamiltonian Cycle CEGAR Solver  
**Status:** Approved by User  

---

## 1. Overview & Problem Statement

In the FHCP Challenge Set (FHCPCS), a substantial subset of graphs (specifically in the 700–998 series, such as `graph710.col` to `graph998.col`) exhibit large sparse path structures containing **900 to 1,840 degree-2 vertices** ($deg(v) = 2$).

In the Hamiltonian Cycle Problem (HCP):
- Every vertex $v$ with $deg(v) = 2$ and neighbors $u, w$ ($u \ne w$) **must** use both incident edges $(u, v)$ and $(v, w)$ in any valid Hamiltonian cycle.
- Current SAT-based CEGAR encoding generates individual Boolean edge variables and At-Most-One / At-Least-One cardinality constraints for all intermediate degree-2 vertices, doubling the size of the CNF formula and prolonging solving time unnecessarily.

**Goal:** Implement a clean, exact **Degree-2 Path Contraction Preprocessing** module that reduces graph size by contracting chains of degree-2 vertices before SAT encoding, and reconstructs the full Hamiltonian cycle upon solution discovery.

---

## 2. Mathematical Principles & Contraction Rules

### 2.1 Degree-2 Chain Definition
Let $G = (V, E)$ be a simple undirected graph. A **maximal degree-2 path** is a sequence of vertices:
$$P = (u, v_1, v_2, \dots, v_k, w)$$
such that:
1. $k \ge 1$.
2. For all $1 \le i \le k$, $deg(v_i) = 2$ in $G$.
3. $u \ne w$, with $deg(u) \ge 3$ and $deg(w) \ge 3$ (the endpoints of the path are non-degree-2 vertices, or $u=w$ for isolated cycles).
4. For all $1 \le i < k$, $(v_i, v_{i+1}) \in E$, $(u, v_1) \in E$, and $(v_k, w) \in E$.

### 2.2 Contraction Transformation
The path $P$ is replaced in the contracted graph $G' = (V', E')$ by a single virtual edge $e' = (u, w)$:
- $V' = V \setminus \{v_1, v_2, \dots, v_k\}$
- $E' = (E \setminus \text{edges}(P)) \cup \{(u, w)\}$
- The ordered sequence of intermediate vertices $[v_1, \dots, v_k]$ is recorded in a bidirectional chain map:
  $$\text{chain\_map}[(u, w)] = [v_1, v_2, \dots, v_k]$$
  $$\text{chain\_map}[(w, u)] = [v_k, \dots, v_2, v_1]$$

### 2.3 Edge Cases & Invariant Decisions
1. **Isolated 2-Regular Cycle ($u = w$):**
   - If the connected component consists entirely of degree-2 vertices:
     - If $|P| = |V|$, the graph itself is a single Hamiltonian Cycle. Return `s SATISFIABLE` immediately.
     - If $|P| < |V|$, the graph is disconnected. Return `s UNSATISFIABLE` immediately.
2. **Parallel Degree-2 Chains between $(u, w)$:**
   - If there exist $\ge 2$ distinct degree-2 paths connecting the same endpoint pair $(u, w)$:
     - Any Hamiltonian cycle can traverse $(u, w)$ at most once. If $|V| > |P_1 \cup P_2|$, the other path's vertices cannot be visited $\implies$ Return `s UNSATISFIABLE`.
3. **Shortcut Edge Pruning:**
   - If direct edge $(u, w) \in E$ already exists and there is a degree-2 path between $(u, w)$:
     - Using direct edge $(u, w)$ alongside path $P$ forms an isolated cycle of length $k+2$.
     - When $|V| > k+2$, direct edge $(u, w)$ cannot be part of any Hamiltonian cycle and is pruned.

---

## 3. Data Structures & Software Architecture

### 3.1 `Degree2Contractor` Struct
In `src/cegar-fix/src/contraction.rs` (or `src/cegar-fix/src/graph.rs`):

```rust
use std::collections::HashMap;
use crate::graph::Graph;

pub struct Degree2Contractor {
    pub chain_map: HashMap<(i32, i32), Vec<i32>>,
    pub original_vertices_count: usize,
    pub contracted_vertices_count: usize,
    pub is_direct_cycle: Option<Vec<i32>>,
    pub is_infeasible: bool,
}

impl Degree2Contractor {
    pub fn contract(g: &Graph) -> (Graph, Degree2Contractor);
    pub fn uncontract_cycle(&self, contracted_cycle: &[i32]) -> Vec<i32>;
}
```

### 3.2 Solution Reconstruction Algorithm (`uncontract_cycle`)
Given a directed cycle $C' = [x_1, x_2, \dots, x_m]$ found on $G'$:
1. Initialize empty vector `full_cycle`.
2. For $i = 0 \dots m-1$:
   - Let $u = x_i$, $v = x_{(i+1) \% m}$.
   - Append $u$ to `full_cycle`.
   - If `chain_map.contains_key(&(u, v))`:
     - Let `intermediates = &chain_map[&(u, v)]`.
     - Append all vertices in `intermediates` to `full_cycle`.
3. Assert `full_cycle.len() == original_vertices_count`.
4. Return `full_cycle`.

---

## 4. Integration into Solver Pipeline

In `src/cegar-fix/src/main.rs`:

```rust
// 1. Initial graph parsing
let mut g = Graph::new(&args.input);

// 2. Structural invariant check
if g.has_articulation_points() {
    println!("Graph has cut-vertex or is disconnected.");
    println!("s UNSATISFIABLE");
    return;
}

// 3. Degree-2 triangle pruning
let pruned = g.prune_degree2_triangles();
if pruned > 0 {
    println!("Pruned {} degree-2 triangle shortcut edges", pruned);
}

// 4. Degree-2 path contraction
let (contracted_g, contractor) = Degree2Contractor::contract(&g);

if let Some(cycle) = contractor.is_direct_cycle {
    println!("Graph is a single 2-regular Hamiltonian cycle.");
    print!("solution: ");
    for v in &cycle { print!("{} ", v); }
    println!();
    println!("s SATISFIABLE");
    return;
}

if contractor.is_infeasible {
    println!("Infeasible parallel degree-2 chains detected.");
    println!("s UNSATISFIABLE");
    return;
}

if contractor.contracted_vertices_count < contractor.original_vertices_count {
    println!(
        "Degree-2 contraction: compressed graph from {} to {} vertices (reduced by {}%)",
        contractor.original_vertices_count,
        contractor.contracted_vertices_count,
        (contractor.original_vertices_count - contractor.contracted_vertices_count) * 100 / contractor.original_vertices_count
    );
}

// 5. Run CEGAR on contracted_g
let (sol_result, contracted_cycle) = run_cegar(&contracted_g, ...);

// 6. Uncontract solution if SATISFIABLE
if sol_result == "SATISFIABLE" {
    let full_cycle = contractor.uncontract_cycle(&contracted_cycle);
    assert_eq!(full_cycle.len(), g.adjacency_list.len());
    // Print solution
}
```

---

## 5. Verification & Testing Strategy

1. **Unit Tests:**
   - `test_contract_single_degree2_chain`: Verify $u - v - w$ contracts to $(u, w)$ and uncontracts to $[u, v, w]$.
   - `test_contract_multi_step_degree2_chain`: Verify chain of length 4 ($u - v_1 - v_2 - v_3 - w$) contracts and uncontracts correctly.
   - `test_contract_pure_cycle`: Verify 2-regular cycle graph is detected and solved directly.
   - `test_contract_infeasible_parallel_chains`: Verify detection of parallel degree-2 chains leading to unsatisfiability.

2. **Regression Benchmarks:**
   - Verify 10 key regression benchmarks (`graph45`, `graph132`, `graph161`, `graph178`, `graph183`, `graph230`, `graph248`, `graph313`, `graph339`, `graph346`) produce 100% `s SATISFIABLE` solutions.

3. **Performance Profiling on Target Graphs:**
   - Measure vertex count reduction and solving time on `graph710.col`, `graph717.col`, `graph725.col`, `graph998.col`.
