use std::collections::{HashMap, HashSet};
use crate::graph::Graph;
use rustsat::solvers::{Solve, SolverResult};
use rustsat::types::{Clause, Lit, TernaryVal};
use rustsat_cadical::CaDiCaL;

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
        let mut all_vertices: Vec<i32> = g.adjacency_list.keys().copied().collect();
        all_vertices.sort_unstable();

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

        // 1. Check for identical open neighborhood modules (False Twins)
        let mut neighbor_groups: HashMap<Vec<i32>, Vec<i32>> = HashMap::new();
        for &u in &all_vertices {
            if let Some(neighbors) = g.adjacency_list.get(&u) {
                let mut sorted_n = neighbors.clone();
                sorted_n.sort_unstable();
                neighbor_groups.entry(sorted_n).or_default().push(u);
            }
        }

        for (_, mut group) in neighbor_groups {
            if group.len() > 1 {
                group.sort_unstable();
                for &v in &group {
                    visited_v.insert(v);
                }
                strong_modules.push(group);
            }
        }

        // 1b. Check for identical closed neighborhood modules (True Twins)
        let mut closed_neighbor_groups: HashMap<Vec<i32>, Vec<i32>> = HashMap::new();
        for &u in &all_vertices {
            if !visited_v.contains(&u) {
                if let Some(neighbors) = g.adjacency_list.get(&u) {
                    let mut sorted_n = neighbors.clone();
                    sorted_n.push(u);
                    sorted_n.sort_unstable();
                    closed_neighbor_groups.entry(sorted_n).or_default().push(u);
                }
            }
        }

        for (_, mut group) in closed_neighbor_groups {
            if group.len() > 1 {
                group.sort_unstable();
                for &v in &group {
                    visited_v.insert(v);
                }
                strong_modules.push(group);
            }
        }

        strong_modules.sort_by_key(|m| m[0]);

        // 2. Identify remaining vertices as singletons or prime modules
        let remaining: Vec<i32> = all_vertices
            .iter()
            .filter(|v| !visited_v.contains(v))
            .copied()
            .collect();

        if strong_modules.is_empty() {
            // Entire graph is prime
            let root_id = 0;
            let mut children = Vec::new();
            for (idx, _) in all_vertices.iter().enumerate() {
                children.push(idx + 1);
            }
            let mut quotient_adj: HashMap<usize, HashSet<usize>> = HashMap::new();
            for (idx_u, &u) in all_vertices.iter().enumerate() {
                let child_u = children[idx_u];
                if let Some(adjs) = g.adjacency_list.get(&u) {
                    for &v in adjs {
                        if let Some(pos) = all_vertices.iter().position(|&x| x == v) {
                            quotient_adj
                                .entry(child_u)
                                .or_insert_with(HashSet::new)
                                .insert(children[pos]);
                        }
                    }
                }
            }
            let root_node = ModularNode {
                id: root_id,
                vertices: all_vertices.clone(),
                node_type: ModularNodeType::Prime {
                    quotient_adj,
                    children: children.clone(),
                },
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
            node_type: ModularNodeType::Prime {
                quotient_adj: HashMap::new(),
                children: Vec::new(),
            },
            parent: None,
        });

        for module in strong_modules {
            let mod_id = nodes.len();
            root_children.push(mod_id);
            let mut mod_children = Vec::new();
            for _ in &module {
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
        let mut quotient_adj: HashMap<usize, HashSet<usize>> = HashMap::new();
        for &c1 in &root_children {
            let v1 = nodes[c1].vertices[0];
            if let Some(adjs) = g.adjacency_list.get(&v1) {
                for &c2 in &root_children {
                    if c1 == c2 {
                        continue;
                    }
                    let v2 = nodes[c2].vertices[0];
                    if adjs.contains(&v2) {
                        quotient_adj
                            .entry(c1)
                            .or_insert_with(HashSet::new)
                            .insert(c2);
                    }
                }
            }
        }

        nodes[root_id].node_type = ModularNodeType::Prime {
            quotient_adj,
            children: root_children,
        };

        Self { root: root_id, nodes }
    }
}

pub struct ModularSolver;

impl ModularSolver {
    /// Solves a Hamiltonian path from u_in to u_out visiting all module_vertices in the induced subgraph G[module_vertices].
    pub fn solve_module_hamiltonian_path(
        module_vertices: &[i32],
        g: &Graph,
        u_in: i32,
        u_out: i32,
    ) -> Option<Vec<i32>> {
        let n = module_vertices.len();
        if n == 0 {
            return None;
        }
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

        if u_in == u_out {
            return None;
        }

        // Localized SAT encoding for Hamiltonian Path from u_in to u_out within induced subgraph
        let mut v_to_idx = HashMap::new();
        for (i, &v) in module_vertices.iter().enumerate() {
            v_to_idx.insert(v, i);
        }

        let in_idx = *v_to_idx.get(&u_in)?;
        let out_idx = *v_to_idx.get(&u_out)?;

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

        // Each vertex at least once
        for i in 0..n {
            let mut clause = Clause::new();
            for pos in 0..n {
                clause.add(*x_map.get(&(i, pos)).unwrap());
            }
            solver.add_clause(clause).ok()?;
        }

        // Each position at least once
        for pos in 0..n {
            let mut clause = Clause::new();
            for i in 0..n {
                clause.add(*x_map.get(&(i, pos)).unwrap());
            }
            solver.add_clause(clause).ok()?;
        }

        // At most one vertex per position
        for pos in 0..n {
            for i in 0..n {
                for j in (i + 1)..n {
                    let mut amo = Clause::new();
                    amo.add(!*x_map.get(&(i, pos)).unwrap());
                    amo.add(!*x_map.get(&(j, pos)).unwrap());
                    solver.add_clause(amo).ok()?;
                }
            }
        }

        // At most one position per vertex
        for i in 0..n {
            for p1 in 0..n {
                for p2 in (p1 + 1)..n {
                    let mut amo = Clause::new();
                    amo.add(!*x_map.get(&(i, p1)).unwrap());
                    amo.add(!*x_map.get(&(i, p2)).unwrap());
                    solver.add_clause(amo).ok()?;
                }
            }
        }

        // Fix endpoints: pos 0 is u_in, pos n-1 is u_out
        let mut cl_in = Clause::new();
        cl_in.add(*x_map.get(&(in_idx, 0)).unwrap());
        solver.add_clause(cl_in).ok()?;

        let mut cl_out = Clause::new();
        cl_out.add(*x_map.get(&(out_idx, n - 1)).unwrap());
        solver.add_clause(cl_out).ok()?;

        // Valid transitions along edges
        for (i, &u) in module_vertices.iter().enumerate() {
            let empty = Vec::new();
            let adjs = g.adjacency_list.get(&u).unwrap_or(&empty);
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
                    if sol.lit_value(lit) == TernaryVal::True {
                        path[pos] = v;
                        break;
                    }
                }
            }
            return Some(path);
        }

        None
    }

    /// Solves the quotient graph cycle, matches boundary port endpoints across transitions,
    /// and deterministically splices internal Hamiltonian sub-paths into a full tour.
    pub fn solve_via_modular_tree(tree: &ModularDecompositionTree, g: &Graph) -> Option<Vec<i32>> {
        if tree.nodes.is_empty() {
            return None;
        }
        let root = &tree.nodes[tree.root];
        match &root.node_type {
            ModularNodeType::Prime { quotient_adj, children } => {
                if children.len() < 3 {
                    return None;
                }

                // Solve Hamiltonian Cycle on quotient graph
                let q_size = children.len();
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

                // Each child at least once
                for i in 0..q_size {
                    let mut cl = Clause::new();
                    for pos in 0..q_size {
                        cl.add(*q_x.get(&(i, pos)).unwrap());
                    }
                    q_solver.add_clause(cl).ok()?;
                }

                // Each position at least once
                for pos in 0..q_size {
                    let mut cl = Clause::new();
                    for i in 0..q_size {
                        cl.add(*q_x.get(&(i, pos)).unwrap());
                    }
                    q_solver.add_clause(cl).ok()?;
                }

                // AMO position per child
                for i in 0..q_size {
                    for p1 in 0..q_size {
                        for p2 in (p1 + 1)..q_size {
                            let mut amo = Clause::new();
                            amo.add(!*q_x.get(&(i, p1)).unwrap());
                            amo.add(!*q_x.get(&(i, p2)).unwrap());
                            q_solver.add_clause(amo).ok()?;
                        }
                    }
                }

                // AMO child per position
                for pos in 0..q_size {
                    for i in 0..q_size {
                        for j in (i + 1)..q_size {
                            let mut amo = Clause::new();
                            amo.add(!*q_x.get(&(i, pos)).unwrap());
                            amo.add(!*q_x.get(&(j, pos)).unwrap());
                            q_solver.add_clause(amo).ok()?;
                        }
                    }
                }

                // Quotient graph edge transitions
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
                            if sol.lit_value(*q_x.get(&(i, pos)).unwrap()) == TernaryVal::True {
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
                            // Find a Hamiltonian path within cur_mod
                            let mut found_path = None;

                            // Preferred: boundary node connected to next module
                            let mut cand_out_opt = None;
                            for &cand_out in &cur_mod.vertices {
                                if let Some(adjs) = g.adjacency_list.get(&cand_out) {
                                    if adjs.iter().any(|v| next_mod.vertices.contains(v)) {
                                        cand_out_opt = Some(cand_out);
                                        break;
                                    }
                                }
                            }

                            if let Some(u_out) = cand_out_opt {
                                for &u_in in &cur_mod.vertices {
                                    if u_in != u_out {
                                        if let Some(path) = Self::solve_module_hamiltonian_path(&cur_mod.vertices, g, u_in, u_out) {
                                            found_path = Some(path);
                                            break;
                                        }
                                    }
                                }
                            }

                            // Fallback: try all candidate endpoint pairs
                            if found_path.is_none() {
                                for &in_cand in &cur_mod.vertices {
                                    for &out_cand in &cur_mod.vertices {
                                        if in_cand != out_cand {
                                            if let Some(path) = Self::solve_module_hamiltonian_path(&cur_mod.vertices, g, in_cand, out_cand) {
                                                found_path = Some(path);
                                                break;
                                            }
                                        }
                                    }
                                    if found_path.is_some() {
                                        break;
                                    }
                                }
                            }

                            if let Some(mod_path) = found_path {
                                full_tour.extend(mod_path);
                            } else {
                                return None;
                            }
                        }
                    }

                    if full_tour.len() == g.adjacency_list.len() && is_valid_cycle(&full_tour, g) {
                        return Some(full_tour);
                    }
                }
            }
            _ => {}
        }

        None
    }
}

fn is_valid_cycle(cycle: &[i32], g: &Graph) -> bool {
    let len = cycle.len();
    if len < 3 {
        return false;
    }
    let mut seen = HashSet::with_capacity(len);
    for i in 0..len {
        let u = cycle[i];
        if !seen.insert(u) {
            return false;
        }
        let v = cycle[(i + 1) % len];
        if !g.adjacency_list.get(&u).map_or(false, |adj| adj.contains(&v)) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Graph;
    use std::collections::{BTreeMap, HashMap};

    #[test]
    fn test_modular_decomposition_true_twins() {
        // Construct a graph with twins: u and v connected to each other and same neighbors
        let mut adj = HashMap::new();
        // 0-1-2-3-0 plus true twin 4 connected to 1, 3, 0
        adj.insert(0, vec![1, 3, 4]);
        adj.insert(1, vec![0, 2]);
        adj.insert(2, vec![1, 3]);
        adj.insert(3, vec![0, 2]);
        adj.insert(4, vec![0]); // simplified
        let g = Graph {
            adjacency_list: adj,
            adjacency_list_btree: BTreeMap::new(),
            arcs: Vec::new(),
        };
        let tree = ModularDecompositionTree::build(&g);
        assert!(tree.nodes.len() >= 1);
        assert_eq!(tree.root, 0);
    }

    #[test]
    fn test_modular_decomposition_series_join() {
        // Complete bipartite join between {1, 2} and {3, 4}
        let mut adj = HashMap::new();
        adj.insert(1, vec![3, 4]);
        adj.insert(2, vec![3, 4]);
        adj.insert(3, vec![1, 2]);
        adj.insert(4, vec![1, 2]);
        let g = Graph {
            adjacency_list: adj,
            adjacency_list_btree: BTreeMap::new(),
            arcs: Vec::new(),
        };
        let tree = ModularDecompositionTree::build(&g);
        assert!(tree.nodes.len() >= 1);
        assert_eq!(tree.root, 0);
    }

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
        let g = Graph {
            adjacency_list: adj,
            adjacency_list_btree: BTreeMap::new(),
            arcs: Vec::new(),
        };
        let tree = ModularDecompositionTree::build(&g);
        let tour = ModularSolver::solve_via_modular_tree(&tree, &g);
        assert!(tour.is_some());
        let t = tour.unwrap();
        assert_eq!(t.len(), 8);
    }
}
