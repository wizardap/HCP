use std::collections::HashSet;
use rustsat::clause;
use rustsat::instances::Cnf;
use rustsat::types::Clause;
use crate::graph::Graph;
use crate::encoder::Encoder;
use crate::metagraph_router::MetagraphRouter;

#[derive(Debug, Clone)]
pub struct GadgetDualPath {
    pub module_id: usize,
    pub vertices: Vec<i32>,
    pub ports: [i32; 2],
    pub true_path_edges: Vec<(i32, i32)>,
    pub false_path_edges: Vec<(i32, i32)>,
}

pub struct InterfacePortSynchronizer;

impl InterfacePortSynchronizer {
    /// Detects gadget modules with size 8 <= |V| <= 32 having exactly 2 interface ports,
    /// and extracts 2 distinct internal Hamiltonian paths (True and False paths) between the ports.
    pub fn extract_gadget_dual_paths(g: &Graph, max_module_size: usize) -> Vec<GadgetDualPath> {
        let modules = MetagraphRouter::detect_gadget_modules_with_size(g, max_module_size);
        let mut result = Vec::new();

        for module in modules {
            let n = module.vertices.len();
            if n < 8 || n > 32 {
                continue;
            }

            let mod_set: HashSet<i32> = module.vertices.iter().copied().collect();

            // Identify interface port vertices (vertices in module with neighbors outside module)
            let mut ports: Vec<i32> = Vec::new();
            for &u in &module.vertices {
                if let Some(neighbors) = g.adjacency_list.get(&u) {
                    if neighbors.iter().any(|v| !mod_set.contains(v)) {
                        ports.push(u);
                    }
                }
            }
            ports.sort_unstable();
            ports.dedup();

            if ports.len() != 2 {
                continue;
            }

            let p0 = ports[0];
            let p1 = ports[1];

            // Search for distinct internal Hamiltonian paths between p0 and p1
            let paths = Self::find_distinct_hamiltonian_paths(p0, p1, &module.vertices, g, 2);
            if paths.len() < 2 {
                continue;
            }

            let true_path_edges: Vec<(i32, i32)> = paths[0].windows(2).map(|w| (w[0], w[1])).collect();
            let false_path_edges: Vec<(i32, i32)> = paths[1].windows(2).map(|w| (w[0], w[1])).collect();

            result.push(GadgetDualPath {
                module_id: module.id,
                vertices: module.vertices,
                ports: [p0, p1],
                true_path_edges,
                false_path_edges,
            });
        }

        result
    }

    /// Recursively finds up to `limit` distinct Hamiltonian paths from start to target within module vertices.
    fn find_distinct_hamiltonian_paths(
        start: i32,
        target: i32,
        module_vertices: &[i32],
        g: &Graph,
        limit: usize,
    ) -> Vec<Vec<i32>> {
        let k = module_vertices.len();
        let mod_set: HashSet<i32> = module_vertices.iter().copied().collect();
        let mut visited: HashSet<i32> = HashSet::with_capacity(k);
        let mut path: Vec<i32> = Vec::with_capacity(k);
        let mut results: Vec<Vec<i32>> = Vec::new();

        visited.insert(start);
        path.push(start);

        fn dfs(
            curr: i32,
            target: i32,
            k: usize,
            visited: &mut HashSet<i32>,
            path: &mut Vec<i32>,
            g: &Graph,
            mod_set: &HashSet<i32>,
            results: &mut Vec<Vec<i32>>,
            limit: usize,
        ) {
            if results.len() >= limit {
                return;
            }
            if path.len() == k {
                if curr == target {
                    results.push(path.clone());
                }
                return;
            }

            if let Some(neighbors) = g.adjacency_list.get(&curr) {
                let mut sorted_neighbors: Vec<i32> = neighbors
                    .iter()
                    .copied()
                    .filter(|v| mod_set.contains(v))
                    .collect();
                sorted_neighbors.sort_unstable();

                for next in sorted_neighbors {
                    if !visited.contains(&next) {
                        // Prune: do not visit target vertex before visiting all other vertices
                        if next == target && path.len() + 1 < k {
                            continue;
                        }

                        visited.insert(next);
                        path.push(next);

                        dfs(next, target, k, visited, path, g, mod_set, results, limit);

                        path.pop();
                        visited.remove(&next);

                        if results.len() >= limit {
                            return;
                        }
                    }
                }
            }
        }

        dfs(start, target, k, &mut visited, &mut path, g, &mod_set, &mut results, limit);
        results
    }

    /// Encodes interface port synchronization and channeling clauses for each gadget module.
    /// - Allocates a boolean variable x_k for gadget k.
    /// - Channels internal undirected edges:
    ///   - e in T_k \ F_k: (!x_k \/ l_uv \/ l_vu) and (x_k \/ !l_uv) and (x_k \/ !l_vu)
    ///   - e in F_k \ T_k: (x_k \/ l_uv \/ l_vu) and (!x_k \/ !l_uv) and (!x_k \/ !l_vu)
    ///   - e in T_k \cap F_k: (l_uv \/ l_vu)
    ///   - e \notin T_k \cup F_k: (!l_uv) and (!l_vu)
    pub fn encode_interface_port_synchronization(
        dual_paths: &[GadgetDualPath],
        g: &Graph,
        encoder: &mut Encoder,
        cnf: &mut Cnf,
    ) {
        for dual in dual_paths {
            let mod_set: HashSet<i32> = dual.vertices.iter().copied().collect();

            // 1. Allocate boolean choice literal x_k for this gadget
            let x_k = encoder.instance.new_lit();

            let true_set: HashSet<(i32, i32)> = dual
                .true_path_edges
                .iter()
                .map(|&(u, v)| if u < v { (u, v) } else { (v, u) })
                .collect();
            let false_set: HashSet<(i32, i32)> = dual
                .false_path_edges
                .iter()
                .map(|&(u, v)| if u < v { (u, v) } else { (v, u) })
                .collect();

            // 2. Identify all unique undirected internal edges in G[M_k]
            let mut internal_undirected_edges: HashSet<(i32, i32)> = HashSet::new();
            for &u in &dual.vertices {
                if let Some(neighbors) = g.adjacency_list.get(&u) {
                    for &v in neighbors {
                        if mod_set.contains(&v) && u < v {
                            internal_undirected_edges.insert((u, v));
                        }
                    }
                }
            }

            // 3. Channeling clauses for all undirected internal edges
            for (u, v) in internal_undirected_edges {
                let l_uv = encoder.graph_lit_map.get(&(u, v)).copied();
                let l_vu = encoder.graph_lit_map.get(&(v, u)).copied();

                let in_t = true_set.contains(&(u, v));
                let in_f = false_set.contains(&(u, v));

                match (l_uv, l_vu) {
                    (Some(fwd), Some(rev)) => {
                        if in_t && !in_f {
                            // x_k => (fwd \/ rev)
                            cnf.add_clause(clause![!x_k, fwd, rev]);
                            // !x_k => (!fwd /\ !rev)
                            cnf.add_clause(clause![x_k, !fwd]);
                            cnf.add_clause(clause![x_k, !rev]);
                        } else if in_f && !in_t {
                            // !x_k => (fwd \/ rev)
                            cnf.add_clause(clause![x_k, fwd, rev]);
                            // x_k => (!fwd /\ !rev)
                            cnf.add_clause(clause![!x_k, !fwd]);
                            cnf.add_clause(clause![!x_k, !rev]);
                        } else if in_t && in_f {
                            // Shared backbone edge: must be used in one direction
                            cnf.add_clause(clause![fwd, rev]);
                        } else {
                            // Forbidden non-path edge: neither direction can be used
                            cnf.add_clause(clause![!fwd]);
                            cnf.add_clause(clause![!rev]);
                        }
                    }
                    (Some(lit), None) | (None, Some(lit)) => {
                        if in_t && !in_f {
                            cnf.add_clause(clause![!x_k, lit]);
                            cnf.add_clause(clause![x_k, !lit]);
                        } else if in_f && !in_t {
                            cnf.add_clause(clause![x_k, lit]);
                            cnf.add_clause(clause![!x_k, !lit]);
                        } else if in_t && in_f {
                            cnf.add_clause(clause![lit]);
                        } else {
                            cnf.add_clause(clause![!lit]);
                        }
                    }
                    (None, None) => {}
                }
            }
        }
    }
}
