# Principled SPQR-Tree Decomposition & Zero-Hardcode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate all benchmark-specific overfitting, ad-hoc macro solvers, and precomputed cache files, replacing them with a 100% de novo, mathematically sound, zero-hardcode pipeline based on classical SPQR-tree decomposition and standardized CDCL SAT-CEGAR block solving.

**Architecture:** A 4-stage principled pipeline:
1. Fast Invariant Filter: Checks minimum degree, connectivity, 1-cuts (articulation points), and bipartite parity in $O(V+E)$ time to immediately certify UNSAT.
2. SPQR Decomposition: Discovers degree-2 series chains and 2-cut separation pairs to isolate subcomponents into virtual shortcut edges.
3. Standardized SAT-CEGAR Block Solver: Solves Hamiltonian cycles and Hamiltonian paths between cut-ports via CaDiCaL with preemptive terminators and DFJ cuts.
4. Recursive Tour Assembly: Replaces virtual edges with unrolled chains and stitched subpaths, certified independently via `TourVerifier`.

**Tech Stack:** Rust 1.83+ (`cegar-fix`), `rustsat`, `rustsat_cadical` (CaDiCaL 1.9.5 C++ FFI), DIMACS `.col` standard.

**Spec:** [`docs/superpowers/specs/2026-09-24-principled-spqr-decomposition-design.md`](file:///root/HCP/docs/superpowers/specs/2026-09-24-principled-spqr-decomposition-design.md)

## Global Constraints

- 100% de novo solving: 0 precomputed files, 0 cached tours (`.hcp`, `.json`, `.pkl`), 0 injected subpaths.
- Zero graph-ID routing: no `match graph_id`, no `if N == 4064`, no hardcoded vertex sets.
- Strict Soundness Invariant: every produced tour must pass `TourVerifier::verify` before being returned; zero false SAT.
- Preemptive Deadline: all CaDiCaL solver instances must attach a terminator checking `Instant::now() >= deadline`.
- Backwards Compatibility: CLI options for `cegar-fix` must remain functional (`-i <file>`, `--timeout <secs>`, `-o <tour_file>`).

## Review Focus

1. **Disconnected or 1-Cut Graph:** Must return `UNSAT` immediately without hanging or attempting SAT solving.
2. **Unbalanced Bipartite Graph ($|V_1| \neq |V_2|$):** Must return `UNSAT` immediately based on parity check.
3. **Graph with Degree-2 Chains on 2-Cut Boundary:** Virtual edge $(u, v)$ replacing a subcomponent must not conflict with degree-2 chain contractions touching $u$ or $v$.
4. **Isomorphism / Permutation Invariance:** Re-indexing vertices $V \to \pi(V)$ must produce an isomorphic certified tour, with no dependence on vertex index ordering.
5. **Preemptive Timeout Termination:** A hard subcomponent or cycle search must abort promptly when deadline expires, returning `Timeout` rather than blocking indefinitely in C++ FFI.

---

### Task 1: Purge Legacy Python Ad-hoc Solvers and Precomputed Caches

**Files:**
- Delete: `hcp_solver/families/corridor_solver.py`
- Delete: `data/corridors/`
- Modify: `hcp_solver/core/pipeline.py:299-327`
- Test: `tests/test_purge_clean.py`

**Interfaces:**
- Consumes: `Graph` from `hcp_solver/core/graph.py`
- Produces: `solve_general_hcp` without any imports from `..families`

- [ ] **Step 1: Write test verifying no legacy cache files or family modules exist**

Create `scratch/test_purge.py`:
```python
import os
import unittest

class TestPurge(unittest.TestCase):
    def test_no_precomputed_data_corridors(self):
        self.assertFalse(os.path.exists("data/corridors"), "data/corridors must be purged")

    def test_no_legacy_corridor_solver(self):
        self.assertFalse(os.path.exists("hcp_solver/families/corridor_solver.py"), "corridor_solver.py must be purged")

    def test_pipeline_has_no_family_imports(self):
        with open("hcp_solver/core/pipeline.py", "r") as f:
            content = f.read()
        self.assertNotIn("families.dense_bipartite", content)
        self.assertNotIn("families.corridor_solver", content)
        self.assertNotIn("families.block_splicer", content)

if __name__ == "__main__":
    unittest.main()
```

- [ ] **Step 2: Run test to verify it fails before purge**

Run: `python3 scratch/test_purge.py`  
Expected: FAIL (files still exist)

- [ ] **Step 3: Delete legacy ad-hoc files and remove family imports from `pipeline.py`**

Execute:
```bash
rm -rf data/corridors/
rm -f hcp_solver/families/corridor_solver.py
rm -rf scratch/graph717/found_tour_graph717.hcp scratch/graph882/found_tour_graph882.hcp
```

In `hcp_solver/core/pipeline.py`, remove lines 299–327 (Stage 1 Macro-Decomposition Check that imported from `..families`).

- [ ] **Step 4: Run test to verify it passes**

Run: `python3 scratch/test_purge.py`  
Expected: OK (3 tests passed)

- [ ] **Step 5: Commit changes**

```bash
git add hcp_solver/core/pipeline.py scratch/test_purge.py
git rm -rf data/corridors/ hcp_solver/families/corridor_solver.py 2>/dev/null || true
git commit -m "refactor: purge ad-hoc python corridor solver and precomputed caches"
```

---

### Task 2: Fast Soundness Filters (1-Cut & Parity Checker)

**Files:**
- Create: `src/cegar-fix/src/decomp/mod.rs`
- Create: `src/cegar-fix/src/decomp/fast_filters.rs`
- Modify: `src/cegar-fix/src/lib.rs`
- Test: `src/cegar-fix/tests/test_fast_filters.rs`

**Interfaces:**
- Consumes: `Graph` from `crate::core::graph::Graph`
- Produces:
  ```rust
  pub fn check_fast_invariants(g: &Graph) -> Result<(), &'static str>;
  ```

- [ ] **Step 1: Write failing test for fast soundness filters**

Create `src/cegar-fix/tests/test_fast_filters.rs`:
```rust
use cegar_fix::core::graph::Graph;
use cegar_fix::decomp::fast_filters::check_fast_invariants;

#[test]
fn test_filter_rejects_disconnected() {
    let mut g = Graph::new();
    g.add_edge(1, 2);
    g.add_edge(3, 4);
    assert_eq!(check_fast_invariants(&g), Err("UNSAT: Graph is disconnected"));
}

#[test]
fn test_filter_rejects_degree_one() {
    let mut g = Graph::new();
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 1);
    g.add_edge(3, 4); // node 4 has degree 1
    assert_eq!(check_fast_invariants(&g), Err("UNSAT: Graph contains vertex with degree < 2"));
}

#[test]
fn test_filter_rejects_cut_vertex() {
    let mut g = Graph::new();
    // Two triangles sharing vertex 3 (articulation point)
    g.add_edge(1, 2); g.add_edge(2, 3); g.add_edge(3, 1);
    g.add_edge(3, 4); g.add_edge(4, 5); g.add_edge(5, 3);
    assert_eq!(check_fast_invariants(&g), Err("UNSAT: Graph contains cut-vertex"));
}

#[test]
fn test_filter_rejects_unbalanced_bipartite() {
    let mut g = Graph::new();
    // K_{2,3} complete bipartite graph (unbalanced: 2 vs 3)
    let left = [1, 2];
    let right = [3, 4, 5];
    for &u in &left {
        for &v in &right {
            g.add_edge(u, v);
        }
    }
    assert_eq!(check_fast_invariants(&g), Err("UNSAT: Bipartite graph has unequal partition sizes"));
}

#[test]
fn test_filter_accepts_valid_cycle() {
    let mut g = Graph::new();
    for i in 1..=6 {
        let nxt = if i == 6 { 1 } else { i + 1 };
        g.add_edge(i, nxt);
    }
    assert_eq!(check_fast_invariants(&g), Ok(()));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_fast_filters`  
Expected: FAIL (module `decomp::fast_filters` does not exist)

- [ ] **Step 3: Implement `decomp::fast_filters`**

Create `src/cegar-fix/src/decomp/mod.rs`:
```rust
pub mod fast_filters;
pub mod spqr_series;
pub mod spqr_parallel;
```

Create `src/cegar-fix/src/decomp/fast_filters.rs`:
```rust
use crate::core::graph::Graph;
use std::collections::{HashMap, HashSet, VecDeque};

pub fn check_fast_invariants(g: &Graph) -> Result<(), &'static str> {
    let nv = g.adjacency_list.len();
    if nv < 3 {
        return Err("UNSAT: Graph has fewer than 3 vertices");
    }

    // 1. Minimum degree check
    for (&node, neighbors) in &g.adjacency_list {
        if neighbors.len() < 2 {
            return Err("UNSAT: Graph contains vertex with degree < 2");
        }
    }

    // 2. Connectivity check (BFS)
    let start_node = *g.adjacency_list.keys().next().unwrap();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    visited.insert(start_node);
    queue.push_back(start_node);

    while let Some(u) = queue.pop_front() {
        if let Some(neighbors) = g.adjacency_list.get(&u) {
            for &v in neighbors {
                if visited.insert(v) {
                    queue.push_back(v);
                }
            }
        }
    }
    if visited.len() != nv {
        return Err("UNSAT: Graph is disconnected");
    }

    // 3. Cut-vertex check (Tarjan DFS)
    if g.has_articulation_points() {
        return Err("UNSAT: Graph contains cut-vertex");
    }

    // 4. Bipartite parity check
    let mut color: HashMap<i32, u8> = HashMap::new();
    let mut is_bipartite = true;
    let mut count_color = [0usize, 0usize];

    color.insert(start_node, 0);
    count_color[0] += 1;
    let mut q = VecDeque::new();
    q.push_back(start_node);

    while let Some(u) = q.pop_front() {
        let c_u = color[&u];
        let c_v = 1 - c_u;
        if let Some(neighbors) = g.adjacency_list.get(&u) {
            for &v in neighbors {
                if let Some(&exist_c) = color.get(&v) {
                    if exist_c == c_u {
                        is_bipartite = false;
                        break;
                    }
                } else {
                    color.insert(v, c_v);
                    count_color[c_v as usize] += 1;
                    q.push_back(v);
                }
            }
        }
        if !is_bipartite {
            break;
        }
    }

    if is_bipartite && count_color[0] != count_color[1] {
        return Err("UNSAT: Bipartite graph has unequal partition sizes");
    }

    Ok(())
}
```

Update `src/cegar-fix/src/lib.rs` to expose `pub mod decomp;`.

- [ ] **Step 4: Run test to verify it passes**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_fast_filters`  
Expected: PASS (all 5 tests pass)

- [ ] **Step 5: Commit changes**

```bash
git add src/cegar-fix/src/decomp/ src/cegar-fix/src/lib.rs src/cegar-fix/tests/test_fast_filters.rs
git commit -m "feat(decomp): implement fast topological invariants and unsat filters"
```

---

### Task 3: General Degree-2 Series Contraction (`decomp::spqr_series`)

**Files:**
- Create: `src/cegar-fix/src/decomp/spqr_series.rs`
- Test: `src/cegar-fix/tests/test_spqr_series.rs`

**Interfaces:**
- Consumes: `Graph`
- Produces:
  ```rust
  pub struct SeriesContraction {
      pub contracted_g: Graph,
      pub chain_map: HashMap<(i32, i32), Vec<i32>>,
      pub contracted_count: usize,
  }
  pub fn contract_series_chains(g: &Graph) -> SeriesContraction;
  pub fn expand_series_tour(tour: &[i32], chain_map: &HashMap<(i32, i32), Vec<i32>>) -> Vec<i32>;
  ```

- [ ] **Step 1: Write failing test for series contraction and expansion**

Create `src/cegar-fix/tests/test_spqr_series.rs`:
```rust
use cegar_fix::core::graph::Graph;
use cegar_fix::core::tour_verifier::TourVerifier;
use cegar_fix::decomp::spqr_series::{contract_series_chains, expand_series_tour};

#[test]
fn test_contract_and_expand_cycle_10() {
    let mut g = Graph::new();
    let n = 10;
    for i in 1..=n {
        let nxt = if i == n { 1 } else { i + 1 };
        g.add_edge(i, nxt);
    }

    let sc = contract_series_chains(&g);
    // On pure cycle graph, chains are contracted to minimum triangle/triangle-like skeleton
    assert!(sc.contracted_count > 0);

    // Simulated skeleton tour
    let skeleton_tour = sc.contracted_g.adjacency_list.keys().copied().collect::<Vec<_>>();
    let full_tour = expand_series_tour(&skeleton_tour, &sc.chain_map);

    let (valid, err) = TourVerifier::verify(&g, &full_tour);
    assert!(valid, "Expanded tour must be certified: {}", err);
}

#[test]
fn test_contract_theta_graph() {
    let mut g = Graph::new();
    // Theta graph: poles 1 and 2 connected by 3 disjoint paths:
    // Path 1: 1 - 3 - 4 - 2
    // Path 2: 1 - 5 - 6 - 2
    // Path 3: 1 - 7 - 8 - 2
    g.add_edge(1, 3); g.add_edge(3, 4); g.add_edge(4, 2);
    g.add_edge(1, 5); g.add_edge(5, 6); g.add_edge(6, 2);
    g.add_edge(1, 7); g.add_edge(7, 8); g.add_edge(8, 2);

    let sc = contract_series_chains(&g);
    // Nodes 3,4, 5,6, 7,8 are degree-2 and must be contracted
    assert_eq!(sc.contracted_count, 6);
    assert_eq!(sc.contracted_g.adjacency_list.len(), 2);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_spqr_series`  
Expected: FAIL (module `spqr_series` does not exist)

- [ ] **Step 3: Implement `spqr_series.rs`**

Create `src/cegar-fix/src/decomp/spqr_series.rs`:
```rust
use crate::core::graph::Graph;
use std::collections::{HashMap, HashSet};

pub struct SeriesContraction {
    pub contracted_g: Graph,
    pub chain_map: HashMap<(i32, i32), Vec<i32>>,
    pub contracted_count: usize,
}

pub fn contract_series_chains(g: &Graph) -> SeriesContraction {
    let mut adj: HashMap<i32, HashSet<i32>> = HashMap::new();
    for (&u, nbrs) in &g.adjacency_list {
        adj.insert(u, nbrs.iter().copied().collect());
    }

    let mut chain_map: HashMap<(i32, i32), Vec<i32>> = HashMap::new();
    let mut removed = HashSet::new();

    let mut changed = true;
    while changed {
        changed = false;
        let deg2_candidates: Vec<i32> = adj.iter()
            .filter(|(&v, nbrs)| nbrs.len() == 2 && !removed.contains(&v))
            .map(|(&v, _)| v)
            .collect();

        for v in deg2_candidates {
            if removed.contains(&v) {
                continue;
            }
            let nbrs: Vec<i32> = adj[&v].iter().copied().collect();
            if nbrs.len() != 2 {
                continue;
            }
            let (u, w) = (nbrs[0], nbrs[1]);
            if u == w || u == v || w == v {
                continue; // Prevent self-loops
            }

            // Do not contract if it collapses graph below 3 nodes
            if adj.len() - removed.len() <= 3 {
                break;
            }

            // Check if (u, w) already has an edge
            if adj[&u].contains(&w) && adj.len() - removed.len() > 4 {
                // parallel edge creation guard
                continue;
            }

            // Contract v: u - v - w -> u - w
            adj.get_mut(&u).unwrap().remove(&v);
            adj.get_mut(&w).unwrap().remove(&v);
            adj.get_mut(&u).unwrap().insert(w);
            adj.get_mut(&w).unwrap().insert(u);
            removed.insert(v);

            // Reconstruct chain sequence between u and w
            let mut sub_chain = Vec::new();
            if let Some(inner) = chain_map.remove(&(u.min(v), u.max(v))) {
                sub_chain.extend(inner);
            }
            sub_chain.push(v);
            if let Some(inner) = chain_map.remove(&(v.min(w), v.max(w))) {
                sub_chain.extend(inner);
            }
            chain_map.insert((u.min(w), u.max(w)), sub_chain);
            changed = true;
        }
    }

    let mut contracted_g = Graph::new();
    for (&u, nbrs) in &adj {
        if !removed.contains(&u) {
            for &v in nbrs {
                if !removed.contains(&v) && u < v {
                    contracted_g.add_edge(u, v);
                }
            }
        }
    }

    SeriesContraction {
        contracted_g,
        chain_map,
        contracted_count: removed.len(),
    }
}

pub fn expand_series_tour(tour: &[i32], chain_map: &HashMap<(i32, i32), Vec<i32>>) -> Vec<i32> {
    let mut expanded = Vec::with_capacity(tour.len() * 2);
    let n = tour.len();
    for i in 0..n {
        let u = tour[i];
        let v = tour[(i + 1) % n];
        expanded.push(u);

        let key = (u.min(v), u.max(v));
        if let Some(intermediates) = chain_map.get(&key) {
            if u < v {
                expanded.extend(intermediates.iter().copied());
            } else {
                expanded.extend(intermediates.iter().rev().copied());
            }
        }
    }
    expanded
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_spqr_series`  
Expected: PASS (both tests pass)

- [ ] **Step 5: Commit changes**

```bash
git add src/cegar-fix/src/decomp/spqr_series.rs src/cegar-fix/tests/test_spqr_series.rs
git commit -m "feat(decomp): implement general series contraction and expansion"
```

---

### Task 4: General 2-Cut Separation Pair Extraction (`decomp::spqr_parallel`)

**Files:**
- Create: `src/cegar-fix/src/decomp/spqr_parallel.rs`
- Test: `src/cegar-fix/tests/test_spqr_parallel.rs`

**Interfaces:**
- Consumes: `Graph`
- Produces:
  ```rust
  pub struct SeparationPair {
      pub u: i32,
      pub v: i32,
      pub components: Vec<Vec<i32>>,
  }
  pub fn find_separation_pairs(g: &Graph) -> Vec<SeparationPair>;
  pub fn extract_subcomponent_graph(g: &Graph, comp: &[i32], port_u: i32, port_v: i32) -> Graph;
  ```

- [ ] **Step 1: Write failing test for 2-cut separation pairs**

Create `src/cegar-fix/tests/test_spqr_parallel.rs`:
```rust
use cegar_fix::core::graph::Graph;
use cegar_fix::decomp::spqr_parallel::{find_separation_pairs, extract_subcomponent_graph};

#[test]
fn test_find_separation_pair_two_blocks() {
    let mut g = Graph::new();
    // Block A: vertices {1, 2, 3, 4} with ports 1, 2
    g.add_edge(1, 3); g.add_edge(3, 4); g.add_edge(4, 2); g.add_edge(1, 4);
    // Block B: vertices {1, 2, 5, 6} with ports 1, 2
    g.add_edge(1, 5); g.add_edge(5, 6); g.add_edge(6, 2); g.add_edge(2, 5);

    let pairs = find_separation_pairs(&g);
    assert!(!pairs.is_empty(), "Must find separation pair {1, 2}");
    let sep = pairs.iter().find(|p| (p.u == 1 && p.v == 2) || (p.u == 2 && p.v == 1)).unwrap();
    assert_eq!(sep.components.len(), 2, "Must partition into 2 disjoint components");

    let sub_g = extract_subcomponent_graph(&g, &sep.components[0], sep.u, sep.v);
    assert!(sub_g.adjacency_list.contains_key(&sep.u));
    assert!(sub_g.adjacency_list.contains_key(&sep.v));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_spqr_parallel`  
Expected: FAIL (module `spqr_parallel` does not exist)

- [ ] **Step 3: Implement `spqr_parallel.rs`**

Create `src/cegar-fix/src/decomp/spqr_parallel.rs`:
```rust
use crate::core::graph::Graph;
use std::collections::{HashSet, VecDeque};

pub struct SeparationPair {
    pub u: i32,
    pub v: i32,
    pub components: Vec<Vec<i32>>,
}

pub fn find_separation_pairs(g: &Graph) -> Vec<SeparationPair> {
    let nodes: Vec<i32> = g.adjacency_list.keys().copied().collect();
    let n = nodes.len();
    if n < 4 {
        return Vec::new();
    }

    let mut results = Vec::new();
    let all_nodes: HashSet<i32> = nodes.iter().copied().collect();

    // Iterate over candidate pairs (u, v)
    // Priority: pairs with shared neighbors or high degree
    for i in 0..n {
        let u = nodes[i];
        let u_nbrs: HashSet<i32> = g.adjacency_list[&u].iter().copied().collect();

        for j in (i + 1)..n {
            let v = nodes[j];
            let removed: HashSet<i32> = [u, v].iter().copied().collect();

            // Find connected components in G \ {u, v}
            let mut visited = HashSet::new();
            let mut components = Vec::new();

            for &start in &all_nodes {
                if removed.contains(&start) || visited.contains(&start) {
                    continue;
                }

                let mut comp = Vec::new();
                let mut q = VecDeque::new();
                visited.insert(start);
                q.push_back(start);

                while let Some(curr) = q.pop_front() {
                    comp.push(curr);
                    if let Some(nbrs) = g.adjacency_list.get(&curr) {
                        for &nxt in nbrs {
                            if !removed.contains(&nxt) && visited.insert(nxt) {
                                q.push_back(nxt);
                            }
                        }
                    }
                }
                components.push(comp);
            }

            if components.len() >= 2 {
                // Verify each component has edges to both u and v
                let valid = components.iter().all(|comp| {
                    let comp_set: HashSet<i32> = comp.iter().copied().collect();
                    let connects_u = comp.iter().any(|&c| g.adjacency_list[&c].contains(&u));
                    let connects_v = comp.iter().any(|&c| g.adjacency_list[&c].contains(&v));
                    connects_u && connects_v
                });

                if valid {
                    results.push(SeparationPair {
                        u,
                        v,
                        components,
                    });
                }
            }
        }
    }

    results
}

pub fn extract_subcomponent_graph(g: &Graph, comp: &[i32], port_u: i32, port_v: i32) -> Graph {
    let mut node_set: HashSet<i32> = comp.iter().copied().collect();
    node_set.insert(port_u);
    node_set.insert(port_v);

    let mut sub_g = Graph::new();
    for &u in &node_set {
        if let Some(nbrs) = g.adjacency_list.get(&u) {
            for &v in nbrs {
                if u < v && node_set.contains(&v) {
                    sub_g.add_edge(u, v);
                }
            }
        }
    }
    sub_g
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_spqr_parallel`  
Expected: PASS

- [ ] **Step 5: Commit changes**

```bash
git add src/cegar-fix/src/decomp/spqr_parallel.rs src/cegar-fix/tests/test_spqr_parallel.rs
git commit -m "feat(decomp): implement general 2-cut separation pair decomposition"
```

---

### Task 5: Standardized Block SAT-CEGAR Engine (`solver::block_solver`)

**Files:**
- Create: `src/cegar-fix/src/solver/mod.rs`
- Create: `src/cegar-fix/src/solver/block_solver.rs`
- Modify: `src/cegar-fix/src/lib.rs`
- Test: `src/cegar-fix/tests/test_block_solver.rs`

**Interfaces:**
- Consumes: `Graph`
- Produces:
  ```rust
  pub fn solve_hamiltonian_cycle(g: &Graph, timeout_secs: f64) -> Result<Vec<i32>, String>;
  pub fn solve_hamiltonian_path(g: &Graph, port_u: i32, port_v: i32, timeout_secs: f64) -> Result<Vec<i32>, String>;
  ```

- [ ] **Step 1: Write failing test for Hamiltonian cycle and path block solver**

Create `src/cegar-fix/tests/test_block_solver.rs`:
```rust
use cegar_fix::core::graph::Graph;
use cegar_fix::core::tour_verifier::TourVerifier;
use cegar_fix::solver::block_solver::{solve_hamiltonian_cycle, solve_hamiltonian_path};

#[test]
fn test_solve_cycle_triangle() {
    let mut g = Graph::new();
    g.add_edge(1, 2); g.add_edge(2, 3); g.add_edge(3, 1);
    let tour = solve_hamiltonian_cycle(&g, 5.0).expect("Triangle is Hamiltonian");
    let (valid, err) = TourVerifier::verify(&g, &tour);
    assert!(valid, "Tour must be certified: {}", err);
}

#[test]
fn test_solve_path_between_ports() {
    let mut g = Graph::new();
    // 4-cycle 1-2-3-4-1
    g.add_edge(1, 2); g.add_edge(2, 3); g.add_edge(3, 4); g.add_edge(4, 1);
    // Path from 1 to 4 visiting all vertices: 1 -> 2 -> 3 -> 4
    let path = solve_hamiltonian_path(&g, 1, 4, 5.0).expect("Hamiltonian path must exist");
    assert_eq!(path.len(), 4);
    assert_eq!(*path.first().unwrap(), 1);
    assert_eq!(*path.last().unwrap(), 4);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_block_solver`  
Expected: FAIL (module `solver::block_solver` does not exist)

- [ ] **Step 3: Implement `block_solver.rs`**

Create `src/cegar-fix/src/solver/mod.rs`:
```rust
pub mod block_solver;
```

Create `src/cegar-fix/src/solver/block_solver.rs`:
```rust
use crate::core::graph::Graph;
use crate::core::solver_utils::create_solver_with_deadline;
use rustsat::instances::{BasicVarManager, Cnf, ManageVars};
use rustsat::solvers::{Solve, SolverResult};
use rustsat::types::{Clause, Lit};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

pub fn solve_hamiltonian_cycle(g: &Graph, timeout_secs: f64) -> Result<Vec<i32>, String> {
    let deadline = Instant::now() + Duration::from_secs_f64(timeout_secs);
    let nodes: Vec<i32> = g.adjacency_list.keys().copied().collect();
    let n = nodes.len();
    if n < 3 {
        return Err("Fewer than 3 vertices".to_string());
    }

    let mut edges = Vec::new();
    for &u in &nodes {
        if let Some(nbrs) = g.adjacency_list.get(&u) {
            for &v in nbrs {
                if u < v {
                    edges.push((u, v));
                }
            }
        }
    }

    let mut var_mgr = BasicVarManager::default();
    let mut edge_to_lit = HashMap::new();
    for &(u, v) in &edges {
        let lit = var_mgr.new_var().pos_lit();
        edge_to_lit.insert((u, v), lit);
        edge_to_lit.insert((v, u), lit);
    }

    let mut solver = create_solver_with_deadline(deadline);

    // Degree-2 constraints for all vertices
    for &u in &nodes {
        let inc: Vec<Lit> = g.adjacency_list[&u].iter()
            .map(|&v| edge_to_lit[&(u, v)])
            .collect();
        if inc.len() < 2 {
            return Err("Vertex degree < 2".to_string());
        }
        // At-least-2 and At-most-2 clauses
        add_exact_two_clauses(&mut solver, &mut var_mgr, &inc);
    }

    // CEGAR loop
    loop {
        if Instant::now() >= deadline {
            return Err("TIMEOUT".to_string());
        }

        match solver.solve() {
            Ok(SolverResult::Sat) => {
                let model = solver.solution(var_mgr.new_var()).unwrap();
                let mut active_adj: HashMap<i32, Vec<i32>> = HashMap::new();
                for &(u, v) in &edges {
                    let lit = edge_to_lit[&(u, v)];
                    if model.lit_value(lit) == rustsat::types::LVal::True {
                        active_adj.entry(u).or_default().push(v);
                        active_adj.entry(v).or_default().push(u);
                    }
                }

                let cycles = extract_subcycles(&nodes, &active_adj);
                if cycles.len() == 1 && cycles[0].len() == n {
                    return Ok(cycles[0].clone());
                }

                // Add DFJ cuts
                for cyc in cycles {
                    let cyc_set: HashSet<i32> = cyc.iter().copied().collect();
                    let mut cut_lits = Vec::new();
                    for &u in &cyc {
                        for &v in &g.adjacency_list[&u] {
                            if !cyc_set.contains(&v) {
                                cut_lits.push(edge_to_lit[&(u, v)]);
                            }
                        }
                    }
                    if !cut_lits.is_empty() {
                        let mut clause = Clause::new();
                        for lit in cut_lits {
                            clause.add(lit);
                        }
                        solver.add_clause(clause).map_err(|e| format!("{:?}", e))?;
                    }
                }
            }
            Ok(SolverResult::Unsat) => return Err("UNSAT".to_string()),
            Ok(SolverResult::Interrupted) | Err(_) => return Err("TIMEOUT".to_string()),
        }
    }
}

pub fn solve_hamiltonian_path(
    g: &Graph,
    port_u: i32,
    port_v: i32,
    timeout_secs: f64,
) -> Result<Vec<i32>, String> {
    // Add virtual edge between port_u and port_v, solve cycle, then cut virtual edge
    let mut aug_g = g.clone();
    let virt_exists = aug_g.adjacency_list.get(&port_u).map(|nbrs| nbrs.contains(&port_v)).unwrap_or(false);
    if !virt_exists {
        aug_g.add_edge(port_u, port_v);
    }

    let cycle = solve_hamiltonian_cycle(&aug_g, timeout_secs)?;
    let n = cycle.len();

    // Find position of (port_u, port_v) in cycle and split into path
    for i in 0..n {
        let u1 = cycle[i];
        let u2 = cycle[(i + 1) % n];
        if (u1 == port_u && u2 == port_v) || (u1 == port_v && u2 == port_u) {
            let mut path = Vec::with_capacity(n);
            if u1 == port_u {
                for k in 1..=n {
                    path.push(cycle[(i + k) % n]);
                }
            } else {
                for k in 0..n {
                    path.push(cycle[(i + n - k) % n]);
                }
            }
            return Ok(path);
        }
    }

    Err("Could not extract Hamiltonian path from cycle".to_string())
}

fn add_exact_two_clauses(
    solver: &mut rustsat_cadical::CaDiCaL,
    var_mgr: &mut BasicVarManager,
    lits: &[Lit],
) {
    if lits.len() == 2 {
        let mut c1 = Clause::new(); c1.add(lits[0]);
        let mut c2 = Clause::new(); c2.add(lits[1]);
        let _ = solver.add_clause(c1);
        let _ = solver.add_clause(c2);
        return;
    }
    // At-least-2: not all 0 and not exactly one 1
    // Simplified: quadratic encoding for degrees <= 10
    // At most 2: for every triple, not all true
    for i in 0..lits.len() {
        for j in (i + 1)..lits.len() {
            for k in (j + 1)..lits.len() {
                let mut c = Clause::new();
                c.add(!lits[i]);
                c.add(!lits[j]);
                c.add(!lits[k]);
                let _ = solver.add_clause(c);
            }
        }
    }
    // At least 2: for every (n-1) subset, not all false
    for skip in 0..lits.len() {
        let mut c = Clause::new();
        for (idx, &lit) in lits.iter().enumerate() {
            if idx != skip {
                c.add(lit);
            }
        }
        let _ = solver.add_clause(c);
    }
}

fn extract_subcycles(nodes: &[i32], active_adj: &HashMap<i32, Vec<i32>>) -> Vec<Vec<i32>> {
    let mut visited = HashSet::new();
    let mut cycles = Vec::new();

    for &u in nodes {
        if visited.contains(&u) || !active_adj.contains_key(&u) {
            continue;
        }

        let mut cyc = Vec::new();
        let mut curr = u;
        let mut prev = -1;

        while !visited.contains(&curr) {
            visited.insert(curr);
            cyc.push(curr);

            if let Some(nbrs) = active_adj.get(&curr) {
                let next_opt = nbrs.iter().find(|&&w| w != prev);
                if let Some(&nxt) = next_opt {
                    prev = curr;
                    curr = nxt;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if !cyc.is_empty() {
            cycles.push(cyc);
        }
    }

    cycles
}
```

Update `src/cegar-fix/src/lib.rs` to expose `pub mod solver;`.

- [ ] **Step 4: Run test to verify it passes**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_block_solver`  
Expected: PASS

- [ ] **Step 5: Commit changes**

```bash
git add src/cegar-fix/src/solver/ src/cegar-fix/src/lib.rs src/cegar-fix/tests/test_block_solver.rs
git commit -m "feat(solver): implement standardized block sat-cegar engine for cycles and paths"
```

---

### Task 6: Recursive Tour Assembly (`assembly::tour_stitcher`)

**Files:**
- Create: `src/cegar-fix/src/assembly/mod.rs`
- Create: `src/cegar-fix/src/assembly/tour_stitcher.rs`
- Modify: `src/cegar-fix/src/lib.rs`
- Test: `src/cegar-fix/tests/test_tour_stitcher.rs`

**Interfaces:**
- Consumes: Skeleton tour and subcomponent paths
- Produces:
  ```rust
  pub fn stitch_subpath(
      skeleton_tour: &[i32],
      port_u: i32,
      port_v: i32,
      subpath: &[i32],
  ) -> Result<Vec<i32>, String>;
  ```

- [ ] **Step 1: Write failing test for tour stitcher**

Create `src/cegar-fix/tests/test_tour_stitcher.rs`:
```rust
use cegar_fix::assembly::tour_stitcher::stitch_subpath;

#[test]
fn test_stitch_forward_subpath() {
    // Skeleton: 1 -> 2 -> 3 -> 1, with virtual edge (1, 2)
    let skeleton = vec![1, 2, 3];
    // Subpath from 1 to 2 visiting {4, 5}: 1 -> 4 -> 5 -> 2
    let subpath = vec![1, 4, 5, 2];

    let stitched = stitch_subpath(&skeleton, 1, 2, &subpath).unwrap();
    // Expected stitched tour: 1, 4, 5, 2, 3
    assert_eq!(stitched, vec![1, 4, 5, 2, 3]);
}

#[test]
fn test_stitch_reversed_subpath() {
    // Skeleton: 2 -> 1 -> 3 -> 2 (edge traversed as 2 -> 1)
    let skeleton = vec![2, 1, 3];
    let subpath = vec![1, 4, 5, 2];

    let stitched = stitch_subpath(&skeleton, 1, 2, &subpath).unwrap();
    // Traversed 2 -> 1, so subpath reversed: 2, 5, 4, 1, 3
    assert_eq!(stitched, vec![2, 5, 4, 1, 3]);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_tour_stitcher`  
Expected: FAIL (module `assembly::tour_stitcher` does not exist)

- [ ] **Step 3: Implement `tour_stitcher.rs`**

Create `src/cegar-fix/src/assembly/mod.rs`:
```rust
pub mod tour_stitcher;
```

Create `src/cegar-fix/src/assembly/tour_stitcher.rs`:
```rust
pub fn stitch_subpath(
    skeleton_tour: &[i32],
    port_u: i32,
    port_v: i32,
    subpath: &[i32],
) -> Result<Vec<i32>, String> {
    let n = skeleton_tour.len();
    if n < 3 || subpath.len() < 2 {
        return Err("Invalid dimension for tour stitching".to_string());
    }

    let mut stitched = Vec::with_capacity(n + subpath.len());

    for i in 0..n {
        let u1 = skeleton_tour[i];
        let u2 = skeleton_tour[(i + 1) % n];
        stitched.push(u1);

        if (u1 == port_u && u2 == port_v) || (u1 == port_v && u2 == port_u) {
            // Splicing interior of subpath between ports
            if u1 == *subpath.first().unwrap() {
                // Forward orientation
                stitched.extend(subpath[1..subpath.len() - 1].iter().copied());
            } else {
                // Reverse orientation
                stitched.extend(subpath[1..subpath.len() - 1].iter().rev().copied());
            }
        }
    }

    Ok(stitched)
}
```

Update `src/cegar-fix/src/lib.rs` to expose `pub mod assembly;`.

- [ ] **Step 4: Run test to verify it passes**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_tour_stitcher`  
Expected: PASS

- [ ] **Step 5: Commit changes**

```bash
git add src/cegar-fix/src/assembly/ src/cegar-fix/src/lib.rs src/cegar-fix/tests/test_tour_stitcher.rs
git commit -m "feat(assembly): implement sound subpath stitching into skeleton tour"
```

---

### Task 7: Unified SPQR Solver Pipeline Integration

**Files:**
- Modify: `src/cegar-fix/src/pipeline/solver_pipeline.rs:120-220`
- Test: `src/cegar-fix/tests/test_principled_pipeline.rs`

**Interfaces:**
- Consumes: All `decomp`, `solver`, `assembly`, `core` modules
- Produces: Universal router-free `solve_single_graph`

- [ ] **Step 1: Write integration test for principled SPQR pipeline**

Create `src/cegar-fix/tests/test_principled_pipeline.rs`:
```rust
use cegar_fix::core::graph::Graph;
use cegar_fix::core::tour_verifier::TourVerifier;
use cegar_fix::pipeline::solver_pipeline::solve_single_graph;
use std::fs::File;
use std::io::Write;

#[test]
fn test_pipeline_on_synthetic_2cut_graph() {
    let tmp_path = "scratch/test_pipeline_2cut.col";
    let mut f = File::create(tmp_path).unwrap();
    writeln!(f, "p edge 8 11").unwrap();
    // Block A: 1-3, 3-4, 4-2, 1-4
    writeln!(f, "e 1 3\ne 3 4\ne 4 2\ne 1 4").unwrap();
    // Block B: 1-5, 5-6, 6-7, 7-8, 8-2, 5-8
    writeln!(f, "e 1 5\ne 5 6\ne 6 7\ne 7 8\ne 8 2\ne 5 8\ne 2 1").unwrap();

    let res = solve_single_graph(tmp_path, 10.0, None);
    assert!(res.is_ok(), "Principled pipeline must solve 2-cut graph: {:?}", res.err());
}
```

- [ ] **Step 2: Run test to verify it passes or check failure**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_principled_pipeline`  
Expected: verify behavior with existing pipeline.

- [ ] **Step 3: Refactor `solver_pipeline.rs` to use principled SPQR decomposition**

In `src/cegar-fix/src/pipeline/solver_pipeline.rs`:
1. Call `crate::decomp::fast_filters::check_fast_invariants(&g)` right after loading graph. If error, return immediately as `UNSAT`.
2. Apply `crate::decomp::spqr_series::contract_series_chains(&g)` to eliminate all degree-2 chains.
3. Check `crate::decomp::spqr_parallel::find_separation_pairs(&contracted_g)`. If a 2-cut is found:
   - Extract subcomponent $G_{\text{sub}}$.
   - Solve Hamiltonian path on $G_{\text{sub}}$ using `crate::solver::block_solver::solve_hamiltonian_path`.
   - Contract $G_{\text{sub}}$ into a virtual edge $(u, v)$ in skeleton.
4. Solve skeleton using `solve_hamiltonian_cycle` or `fallback_cegar`.
5. Assemble tour using `tour_stitcher` and `expand_series_tour`.
6. Run `verify_and_export`.

- [ ] **Step 4: Run test to verify it passes**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_principled_pipeline`  
Expected: PASS

- [ ] **Step 5: Commit changes**

```bash
git add src/cegar-fix/src/pipeline/solver_pipeline.rs src/cegar-fix/tests/test_principled_pipeline.rs
git commit -m "feat(pipeline): integrate principled spqr decomposition and fast filters"
```

---

### Task 8: Permutation Invariance & Negative Soundness Tests

**Files:**
- Create: `src/cegar-fix/tests/test_principled_invariance.rs`
- Test: `tests/test_principled_invariance.rs`

**Interfaces:**
- Consumes: `solve_single_graph`
- Produces: Verification that solver is 100% vertex-permutation invariant and reports UNSAT on non-Hamiltonian graphs.

- [ ] **Step 1: Write permutation invariance and negative soundness test**

Create `src/cegar-fix/tests/test_principled_invariance.rs`:
```rust
use cegar_fix::core::graph::Graph;
use cegar_fix::core::tour_verifier::TourVerifier;
use cegar_fix::pipeline::solver_pipeline::solve_single_graph;
use std::fs::File;
use std::io::Write;

#[test]
fn test_petersen_graph_is_unsat() {
    // Petersen graph has 10 vertices, 15 edges, non-Hamiltonian
    let tmp_path = "scratch/test_petersen.col";
    let mut f = File::create(tmp_path).unwrap();
    writeln!(f, "p edge 10 15").unwrap();
    // Outer 5-cycle: 1-2, 2-3, 3-4, 4-5, 5-1
    writeln!(f, "e 1 2\ne 2 3\ne 3 4\ne 4 5\ne 5 1").unwrap();
    // Spokes: 1-6, 2-7, 3-8, 4-9, 5-10
    writeln!(f, "e 1 6\ne 2 7\ne 3 8\ne 4 9\ne 5 10").unwrap();
    // Inner star: 6-8, 8-10, 10-7, 7-9, 9-6
    writeln!(f, "e 6 8\ne 8 10\ne 10 7\ne 7 9\ne 9 6").unwrap();

    let res = solve_single_graph(tmp_path, 10.0, None);
    assert!(res.is_err(), "Petersen graph must be UNSAT");
    assert!(res.unwrap_err().message.contains("UNSAT"));
}

#[test]
fn test_vertex_permutation_invariance() {
    // Generate graph and permute its vertices with bijection: i -> (i * 7) % 8 + 1
    let tmp_orig = "scratch/test_invar_orig.col";
    let tmp_perm = "scratch/test_invar_perm.col";
    let n = 8;
    let pi = |x: i32| -> i32 { (x * 3) % n + 1 };

    let mut f1 = File::create(tmp_orig).unwrap();
    let mut f2 = File::create(tmp_perm).unwrap();
    writeln!(f1, "p edge {} {}", n, n).unwrap();
    writeln!(f2, "p edge {} {}", n, n).unwrap();

    for i in 1..=n {
        let nxt = if i == n { 1 } else { i + 1 };
        writeln!(f1, "e {} {}", i, nxt).unwrap();
        writeln!(f2, "e {} {}", pi(i), pi(nxt)).unwrap();
    }

    let res1 = solve_single_graph(tmp_orig, 5.0, None).expect("Original solves");
    let res2 = solve_single_graph(tmp_perm, 5.0, None).expect("Permuted solves");
    assert_eq!(res1.0.len(), n as usize);
    assert_eq!(res2.0.len(), n as usize);
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_principled_invariance`  
Expected: PASS

- [ ] **Step 3: Commit changes**

```bash
git add src/cegar-fix/tests/test_principled_invariance.rs
git commit -m "test: add permutation invariance and petersen unsat tests"
```

---

### Task 9: Clean Benchmark Verification & Audit Sweep

**Files:**
- Test: Full cargo test sweep + FHCPCS sample verification (`FHCPCS-col/graph1.col` to `graph10.col`)

- [ ] **Step 1: Run all unit and integration tests**

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo test --manifest-path src/cegar-fix/Cargo.toml
```
Expected: all tests pass.

- [ ] **Step 2: Run release build**

```bash
cargo build --manifest-path src/cegar-fix/Cargo.toml --release
```
Expected: successful build of binary `src/cegar-fix/target/release/cegar-fix`.

- [ ] **Step 3: Run verification sweep on sample benchmark graphs**

```bash
python3 -c "
import subprocess
for gid in range(1, 11):
    col = f'FHCPCS-col/graph{gid}.col'
    res = subprocess.run(['src/cegar-fix/target/release/cegar-fix', '-i', col, '--timeout', '10'], capture_output=True, text=True)
    assert 's SATISFIABLE' in res.stdout, f'Failed on {col}: {res.stderr}'
    print(f'[✓] graph{gid} solved & certified')
"
```
Expected: graphs 1 to 10 all solved and certified.

- [ ] **Step 4: Commit and finalize**

```bash
git add docs/superpowers/plans/2026-09-24-principled-spqr-decomposition.md
git commit -m "docs: finalize principled spqr decomposition implementation plan"
```
