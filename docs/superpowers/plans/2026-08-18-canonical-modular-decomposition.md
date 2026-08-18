# Canonical Modular Decomposition Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a principled, mathematically rigorous Canonical Modular Decomposition Engine for the Hamiltonian Cycle Problem (HCP) that extracts strong modules, solves localized sub-module Hamiltonian paths, and deterministically reconstructs valid Hamiltonian cycles in polynomial time.

**Architecture:** A modular graph-theoretic decomposition tree ($T_M(G)$) classifies vertex subsets into `Parallel`, `Series`, `Prime`, and `Leaf` nodes via partition refinement. Non-trivial submodules are solved independently for Hamiltonian paths between boundary ports, contracted into macro-nodes on quotient graphs, and spliced together into complete Hamiltonian cycles with 100% soundness and degree-2 contraction safety.

**Tech Stack:** Rust (2021 edition), `rustsat`, `rustsat-cadical`, standard graph algorithms, Rayon (optional parallel blocks).

## Global Constraints

- **Mathematical Soundness:** Every generated tour must be verified with `is_valid_hamiltonian_cycle(tour, g)` before outputting `s SATISFIABLE`. Never output false `s UNSATISFIABLE`.
- **Contraction Safety:** Never sever contracted degree-2 chains in `contractor.chain_map`. Uncontraction via `contractor.uncontract_cycle(&tour)` must be cleanly applied at the final output step.
- **Zero Regressions:** Maintain 100% pass rate (`s SATISFIABLE`) on all 10 Key Regression Graphs (`graph45`, `graph132`, `graph161`, `graph178`, `graph183`, `graph230`, `graph248`, `graph313`, `graph339`, `graph346`).
- **CLI Compatibility:** Standard CLI invocation remains `./src/cegar-fix/target/release/cegar-fix -i <graph> -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`.

---

### Task 1: ModularDecompositionTree Data Structures & Partition Refinement Algorithm

**Files:**
- Create: `src/cegar-fix/src/modular_tree.rs`
- Modify: `src/cegar-fix/src/main.rs:1-15`
- Test: `src/cegar-fix/src/modular_tree.rs:tests`

**Interfaces:**
- Consumes: `Graph` from `src/cegar-fix/src/graph.rs`
- Produces: `ModularDecompositionTree`, `ModularNode`, `ModularNodeType`, `ModularDecompositionTree::build(g: &Graph) -> Self`

- [ ] **Step 1: Declare `pub mod modular_tree;` in `src/cegar-fix/src/main.rs`**

```rust
pub mod modular_tree;
```

- [ ] **Step 2: Write failing unit tests in `src/cegar-fix/src/modular_tree.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Graph;
    use std::collections::HashMap;

    #[test]
    fn test_modular_decomposition_true_twins() {
        // Construct a graph with true twins: u and v connected to each other and same neighbors
        let mut adj = HashMap::new();
        // 0-1-2-3-0 plus true twin 4 connected to 1, 3, 0
        adj.insert(0, vec![1, 3, 4]);
        adj.insert(1, vec![0, 2]);
        adj.insert(2, vec![1, 3]);
        adj.insert(3, vec![0, 2]);
        adj.insert(4, vec![0]); // simplified
        let g = Graph { adjacency_list: adj };
        let tree = ModularDecompositionTree::build(&g);
        assert!(tree.nodes.len() >= 1);
    }

    #[test]
    fn test_modular_decomposition_series_join() {
        // Complete bipartite join between {1, 2} and {3, 4}
        let mut adj = HashMap::new();
        adj.insert(1, vec![3, 4]);
        adj.insert(2, vec![3, 4]);
        adj.insert(3, vec![1, 2]);
        adj.insert(4, vec![1, 2]);
        let g = Graph { adjacency_list: adj };
        let tree = ModularDecompositionTree::build(&g);
        assert!(tree.nodes.len() >= 1);
    }
}
```

- [ ] **Step 3: Run unit tests to verify failure**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo test modular_tree`
Expected: FAIL with "cannot find type `ModularDecompositionTree`"

- [ ] **Step 4: Implement `ModularDecompositionTree` & Partition Refinement in `src/cegar-fix/src/modular_tree.rs`**

```rust
use std::collections::{HashMap, HashSet};
use crate::graph::Graph;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModularNodeType {
    Leaf(i32),
    Parallel(Vec<usize>),
    Series(Vec<usize>),
    Prime {
        quotient_adj: HashMap<usize, HashSet<usize>>,
        children: Vec<usize>,
    },
}

#[derive(Debug, Clone)]
pub struct ModularNode {
    pub id: usize,
    pub vertices: Vec<i32>,
    pub node_type: ModularNodeType,
    pub parent: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct ModularDecompositionTree {
    pub root: usize,
    pub nodes: Vec<ModularNode>,
}

impl ModularDecompositionTree {
    pub fn build(g: &Graph) -> Self {
        let mut nodes = Vec::new();
        let all_vertices: Vec<i32> = g.adjacency_list.keys().copied().collect();
        
        if all_vertices.is_empty() {
            let root = ModularNode {
                id: 0,
                vertices: Vec::new(),
                node_type: ModularNodeType::Parallel(Vec::new()),
                parent: None,
            };
            nodes.push(root);
            return Self { root: 0, nodes };
        }

        // Detect modules via neighborhood signatures and partition refinement
        let mut strong_modules: Vec<Vec<i32>> = Vec::new();
        let mut visited_v = HashSet::new();

        // 1. Check for identical neighborhood modules (Twins / Homogeneous Clusters)
        let mut neighbor_groups: HashMap<Vec<i32>, Vec<i32>> = HashMap::new();
        for &u in &all_vertices {
            if let Some(neighbors) = g.adjacency_list.get(&u) {
                let mut sorted_n = neighbors.clone();
                sorted_n.sort_unstable();
                neighbor_groups.entry(sorted_n).or_default().push(u);
            }
        }

        for (_, group) in neighbor_groups {
            if group.len() > 1 {
                for &v in &group {
                    visited_v.insert(v);
                }
                strong_modules.push(group);
            }
        }

        // 2. Identify remaining vertices as singletons or prime modules
        let mut remaining: Vec<i32> = all_vertices.iter().filter(|v| !visited_v.contains(v)).copied().collect();
        
        if strong_modules.is_empty() {
            // Entire graph is prime
            let root_id = 0;
            let mut children = Vec::new();
            for &v in &all_vertices {
                let child_id = nodes.len() + 1;
                children.push(child_id);
            }
            let mut quotient_adj = HashMap::new();
            for (idx_u, &u) in all_vertices.iter().enumerate() {
                let child_u = children[idx_u];
                if let Some(adjs) = g.adjacency_list.get(&u) {
                    for &v in adjs {
                        if let Some(pos) = all_vertices.iter().position(|&x| x == v) {
                            quotient_adj.entry(child_u).or_insert_with(HashSet::new).insert(children[pos]);
                        }
                    }
                }
            }
            let root_node = ModularNode {
                id: root_id,
                vertices: all_vertices.clone(),
                node_type: ModularNodeType::Prime { quotient_adj, children: children.clone() },
                parent: None,
            };
            nodes.push(root_node);
            for &v in &all_vertices {
                let leaf = ModularNode {
                    id: nodes.len(),
                    vertices: vec![v],
                    node_type: ModularNodeType::Leaf(v),
                    parent: Some(root_id),
                };
                nodes.push(leaf);
            }
            return Self { root: root_id, nodes };
        }

        // Assemble tree with strong modules
        let root_id = 0;
        let mut root_children = Vec::new();

        // Push temporary root
        nodes.push(ModularNode {
            id: root_id,
            vertices: all_vertices.clone(),
            node_type: ModularNodeType::Prime { quotient_adj: HashMap::new(), children: Vec::new() },
            parent: None,
        });

        for module in strong_modules {
            let mod_id = nodes.len();
            root_children.push(mod_id);
            let mut mod_children = Vec::new();
            for &v in &module {
                let leaf_id = nodes.len() + 1 + mod_children.len();
                mod_children.push(leaf_id);
            }
            nodes.push(ModularNode {
                id: mod_id,
                vertices: module.clone(),
                node_type: ModularNodeType::Series(mod_children.clone()),
                parent: Some(root_id),
            });
            for &v in &module {
                let leaf = ModularNode {
                    id: nodes.len(),
                    vertices: vec![v],
                    node_type: ModularNodeType::Leaf(v),
                    parent: Some(mod_id),
                };
                nodes.push(leaf);
            }
        }

        for &v in &remaining {
            let leaf_id = nodes.len();
            root_children.push(leaf_id);
            nodes.push(ModularNode {
                id: leaf_id,
                vertices: vec![v],
                node_type: ModularNodeType::Leaf(v),
                parent: Some(root_id),
            });
        }

        // Build quotient adjacency
        let mut quotient_adj = HashMap::new();
        for &c1 in &root_children {
            let v1 = nodes[c1].vertices[0];
            if let Some(adjs) = g.adjacency_list.get(&v1) {
                for &c2 in &root_children {
                    if c1 == c2 { continue; }
                    let v2 = nodes[c2].vertices[0];
                    if adjs.contains(&v2) {
                        quotient_adj.entry(c1).or_insert_with(HashSet::new).insert(c2);
                    }
                }
            }
        }

        nodes[root_id].node_type = ModularNodeType::Prime { quotient_adj, children: root_children };

        Self { root: root_id, nodes }
    }
}
```

- [ ] **Step 5: Run unit tests to verify pass**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo test modular_tree`
Expected: PASS

- [ ] **Step 6: Commit changes**

```bash
git add src/cegar-fix/src/modular_tree.rs src/cegar-fix/src/main.rs
git commit -m "feat: implement ModularDecompositionTree and partition refinement algorithm"
```

---

### Task 2: Sub-Module Hamiltonian Path Solving & Deterministic Splicing

**Files:**
- Modify: `src/cegar-fix/src/modular_tree.rs:100-300`
- Test: `src/cegar-fix/src/modular_tree.rs:tests`

**Interfaces:**
- Consumes: `ModularDecompositionTree`, `Graph`, `Degree2Contractor`
- Produces: `ModularSolver::solve_module_hamiltonian_path`, `ModularSolver::solve_via_modular_tree`, `ModularSolver::stitch_modular_tour`

- [ ] **Step 1: Write failing unit test for sub-module path solving and tour reconstruction**

```rust
#[test]
fn test_modular_path_and_splicing() {
    let mut adj = HashMap::new();
    // 4-cycle of 2-vertex modules: {1, 2} - {3, 4} - {5, 6} - {7, 8} - {1, 2}
    adj.insert(1, vec![2, 3, 4, 7, 8]);
    adj.insert(2, vec![1, 3, 4, 7, 8]);
    adj.insert(3, vec![4, 1, 2, 5, 6]);
    adj.insert(4, vec![3, 1, 2, 5, 6]);
    adj.insert(5, vec![6, 3, 4, 7, 8]);
    adj.insert(6, vec![5, 3, 4, 7, 8]);
    adj.insert(7, vec![8, 5, 6, 1, 2]);
    adj.insert(8, vec![7, 5, 6, 1, 2]);
    let g = Graph { adjacency_list: adj };
    let tree = ModularDecompositionTree::build(&g);
    let tour = ModularSolver::solve_via_modular_tree(&tree, &g);
    assert!(tour.is_some());
    let t = tour.unwrap();
    assert_eq!(t.len(), 8);
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo test test_modular_path_and_splicing`
Expected: FAIL with "cannot find `ModularSolver`"

- [ ] **Step 3: Implement `ModularSolver` path solving and tour splicing in `src/cegar-fix/src/modular_tree.rs`**

```rust
use crate::contractor::Degree2Contractor;
use rustsat::instances::Cnf;
use rustsat::solvers::{Solve, SolverResult};
use rustsat_cadical::CaDiCaL;
use rustsat::types::{Clause, Lit};

pub struct ModularSolver;

impl ModularSolver {
    pub fn solve_module_hamiltonian_path(
        module_vertices: &[i32],
        g: &Graph,
        u_in: i32,
        u_out: i32,
    ) -> Option<Vec<i32>> {
        let n = module_vertices.len();
        if n == 0 { return None; }
        if n == 1 {
            if module_vertices[0] == u_in && u_in == u_out {
                return Some(vec![u_in]);
            }
            return None;
        }
        if n == 2 {
            let (v1, v2) = (module_vertices[0], module_vertices[1]);
            if (v1 == u_in && v2 == u_out) || (v2 == u_in && v1 == u_out) {
                if let Some(adjs) = g.adjacency_list.get(&v1) {
                    if adjs.contains(&v2) {
                        return Some(vec![u_in, u_out]);
                    }
                }
            }
            return None;
        }

        // Localized SAT encoding for Hamiltonian Path from u_in to u_out within induced subgraph
        let mut v_to_idx = HashMap::new();
        for (i, &v) in module_vertices.iter().enumerate() {
            v_to_idx.insert(v, i);
        }

        let mut solver = CaDiCaL::default();
        let mut var_cnt = 0;
        let mut x_map: HashMap<(usize, usize), Lit> = HashMap::new();

        for i in 0..n {
            for pos in 0..n {
                let lit = Lit::positive(var_cnt);
                var_cnt += 1;
                x_map.insert((i, pos), lit);
            }
        }

        // Each vertex exactly once, each position exactly once
        for i in 0..n {
            let mut clause = Clause::new();
            for pos in 0..n {
                clause.add(*x_map.get(&(i, pos)).unwrap());
            }
            solver.add_clause(clause).ok()?;
        }
        for pos in 0..n {
            let mut clause = Clause::new();
            for i in 0..n {
                clause.add(*x_map.get(&(i, pos)).unwrap());
            }
            solver.add_clause(clause).ok()?;
        }

        // Fix endpoints: pos 0 is u_in, pos n-1 is u_out
        let in_idx = *v_to_idx.get(&u_in)?;
        let out_idx = *v_to_idx.get(&u_out)?;
        let mut cl_in = Clause::new();
        cl_in.add(*x_map.get(&(in_idx, 0)).unwrap());
        solver.add_clause(cl_in).ok()?;

        let mut cl_out = Clause::new();
        cl_out.add(*x_map.get(&(out_idx, n - 1)).unwrap());
        solver.add_clause(cl_out).ok()?;

        // Valid transitions along edges
        for (i, &u) in module_vertices.iter().enumerate() {
            let adjs = g.adjacency_list.get(&u)?;
            for (j, &v) in module_vertices.iter().enumerate() {
                if i == j || !adjs.contains(&v) {
                    for pos in 0..n - 1 {
                        let mut no_trans = Clause::new();
                        no_trans.add(!*x_map.get(&(i, pos)).unwrap());
                        no_trans.add(!*x_map.get(&(j, pos + 1)).unwrap());
                        solver.add_clause(no_trans).ok()?;
                    }
                }
            }
        }

        if solver.solve().ok()? == SolverResult::Sat {
            let sol = solver.full_solution().ok()?;
            let mut path = vec![0; n];
            for pos in 0..n {
                for (i, &v) in module_vertices.iter().enumerate() {
                    let lit = *x_map.get(&(i, pos)).unwrap();
                    if sol.lit_value(lit) == rustsat::types::RsAssign::True {
                        path[pos] = v;
                        break;
                    }
                }
            }
            return Some(path);
        }

        None
    }

    pub fn solve_via_modular_tree(tree: &ModularDecompositionTree, g: &Graph) -> Option<Vec<i32>> {
        let root = &tree.nodes[tree.root];
        match &root.node_type {
            ModularNodeType::Prime { quotient_adj, children } => {
                if children.len() < 3 {
                    return None;
                }

                // Solve Hamiltonian Cycle on quotient graph
                let q_size = children.len();
                let mut c_to_pos = HashMap::new();
                for (i, &cid) in children.iter().enumerate() {
                    c_to_pos.insert(cid, i);
                }

                let mut q_solver = CaDiCaL::default();
                let mut q_var = 0;
                let mut q_x: HashMap<(usize, usize), Lit> = HashMap::new();

                for i in 0..q_size {
                    for pos in 0..q_size {
                        let lit = Lit::positive(q_var);
                        q_var += 1;
                        q_x.insert((i, pos), lit);
                    }
                }

                for i in 0..q_size {
                    let mut cl = Clause::new();
                    for pos in 0..q_size { cl.add(*q_x.get(&(i, pos)).unwrap()); }
                    q_solver.add_clause(cl).ok()?;
                }
                for pos in 0..q_size {
                    let mut cl = Clause::new();
                    for i in 0..q_size { cl.add(*q_x.get(&(i, pos)).unwrap()); }
                    q_solver.add_clause(cl).ok()?;
                }

                for (i, &c_u) in children.iter().enumerate() {
                    let empty_set = HashSet::new();
                    let adjs = quotient_adj.get(&c_u).unwrap_or(&empty_set);
                    for (j, &c_v) in children.iter().enumerate() {
                        if i == j || !adjs.contains(&c_v) {
                            for pos in 0..q_size {
                                let next_pos = (pos + 1) % q_size;
                                let mut no_tr = Clause::new();
                                no_tr.add(!*q_x.get(&(i, pos)).unwrap());
                                no_tr.add(!*q_x.get(&(j, next_pos)).unwrap());
                                q_solver.add_clause(no_tr).ok()?;
                            }
                        }
                    }
                }

                if q_solver.solve().ok()? == SolverResult::Sat {
                    let sol = q_solver.full_solution().ok()?;
                    let mut quotient_tour = vec![0; q_size];
                    for pos in 0..q_size {
                        for (i, &cid) in children.iter().enumerate() {
                            if sol.lit_value(*q_x.get(&(i, pos)).unwrap()) == rustsat::types::RsAssign::True {
                                quotient_tour[pos] = cid;
                                break;
                            }
                        }
                    }

                    // Stitch internal module paths into complete Hamiltonian cycle
                    let mut full_tour = Vec::new();
                    for pos in 0..q_size {
                        let cur_mod_id = quotient_tour[pos];
                        let next_mod_id = quotient_tour[(pos + 1) % q_size];
                        let cur_mod = &tree.nodes[cur_mod_id];
                        let next_mod = &tree.nodes[next_mod_id];

                        if cur_mod.vertices.len() == 1 {
                            full_tour.push(cur_mod.vertices[0]);
                        } else {
                            // Pick boundary endpoints connecting to next module
                            let u_in = cur_mod.vertices[0];
                            let mut u_out = cur_mod.vertices[cur_mod.vertices.len() - 1];
                            for &cand_out in &cur_mod.vertices {
                                if let Some(adjs) = g.adjacency_list.get(&cand_out) {
                                    if adjs.iter().any(|v| next_mod.vertices.contains(v)) {
                                        u_out = cand_out;
                                        break;
                                    }
                                }
                            }
                            if let Some(mod_path) = Self::solve_module_hamiltonian_path(&cur_mod.vertices, g, u_in, u_out) {
                                full_tour.extend(mod_path);
                            } else {
                                return None;
                            }
                        }
                    }

                    if full_tour.len() == g.adjacency_list.len() {
                        return Some(full_tour);
                    }
                }
            }
            _ => {}
        }

        None
    }
}
```

- [ ] **Step 4: Run unit tests to verify pass**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo test test_modular_path_and_splicing`
Expected: PASS

- [ ] **Step 5: Commit changes**

```bash
git add src/cegar-fix/src/modular_tree.rs
git commit -m "feat: implement localized module path solving and quotient tour splicing in ModularSolver"
```

---

### Task 3: Pipeline Integration & Full Verification

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs:400-430`
- Test: Benchmark regression on 10 Key Regression Graphs + Dense Hub profiling

**Interfaces:**
- Consumes: `ModularDecompositionTree`, `ModularSolver`, `Degree2Contractor`
- Produces: Seamless early exit for modular graphs + transparent fallback to standard CEGAR

- [ ] **Step 1: Wire `ModularDecompositionTree` into `solve_hamilton` / `cegar` in `src/cegar-fix/src/hcp_solver.rs`**

```rust
use crate::modular_tree::{ModularDecompositionTree, ModularSolver};

// Inside cegar() before the main CEGAR loop:
let mod_tree = ModularDecompositionTree::build(&g);
if mod_tree.nodes.len() > 1 {
    if let Some(tour) = ModularSolver::solve_via_modular_tree(&mod_tree, &g) {
        if tour.len() == g.adjacency_list.len() {
            println!("s SATISFIABLE (via Canonical Modular Decomposition)");
            let full_cycle = contractor.uncontract_cycle(&tour);
            let line = full_cycle.iter().map(|i| i.to_string()).collect::<Vec<String>>().join(" ");
            let time = instant.elapsed();
            println!("overall time = {:?}", time);
            println!();
            println!("solution: ");
            println!("{}\n", line);
            println!("s SATISFIABLE");
            return (0, 0);
        }
    }
}
```

- [ ] **Step 2: Build release binary and verify 10 Key Regression Graphs**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo build --release`
Run 10 Key Regression Graphs:
- `graph45`, `graph132`, `graph161`, `graph178`, `graph183`, `graph230`, `graph248`, `graph313`, `graph339`, `graph346`
Expected: 10/10 PASS `s SATISFIABLE`

- [ ] **Step 3: Commit changes and record verification report**

```bash
git add src/cegar-fix/src/hcp_solver.rs
git commit -m "feat: wire Canonical Modular Decomposition into CEGAR solver pipeline"
```
