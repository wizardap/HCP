use crate::graph::Graph;
use std::collections::{HashMap, HashSet};

/// GadgetPathAbsorber
/// Attempts to absorb small satellite subcycles (|C| <= 16) into larger cycles by discovering
/// Hamiltonian paths in the induced subgraphs of the small cycles.
pub struct GadgetPathAbsorber;

impl GadgetPathAbsorber {
    /// Attempts to absorb small satellite subcycles (|C| <= 16) into larger cycles by discovering
    /// Hamiltonian paths in the induced subgraphs of the small cycles.
    pub fn try_absorb_gadgets(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
    ) -> Vec<Vec<i32>> {
        if cycles.len() <= 1 {
            return cycles.to_vec();
        }

        // 1. Partition cycles into large_cycles (|C| > 16) and small_cycles (|C| <= 16)
        let mut large_cycles: Vec<Vec<i32>> =
            cycles.iter().filter(|c| c.len() > 16).cloned().collect();
        let mut small_cycles: Vec<Vec<i32>> =
            cycles.iter().filter(|c| c.len() <= 16).cloned().collect();

        if large_cycles.is_empty() || small_cycles.is_empty() {
            return cycles.to_vec();
        }

        // 2. Iteratively absorb small cycles into large cycles
        let mut progress = true;
        while progress && !small_cycles.is_empty() {
            progress = false;
            let mut unabsorbed = Vec::new();

            for cs in small_cycles {
                if cs.is_empty() {
                    continue;
                }

                // Enumerate Hamiltonian paths in G[V(C_s)]
                let hpaths = Self::find_hamiltonian_paths(&cs, g);
                let mut absorbed = false;

                'path_loop: for path in &hpaths {
                    let p_first = path[0];
                    let p_last = *path.last().unwrap();

                    for cl in &mut large_cycles {
                        let n_cl = cl.len();
                        if n_cl < 2 {
                            continue;
                        }

                        // Search for an edge (u, v) in cl to splice into
                        for i in 0..n_cl {
                            let u = cl[i];
                            let v = cl[(i + 1) % n_cl];

                            if !Self::is_protected(u, v, protected_edges)
                                && !Self::is_protected(u, p_first, protected_edges)
                                && !Self::is_protected(p_last, v, protected_edges)
                                && Self::has_edge(g, u, p_first)
                                && Self::has_edge(g, p_last, v)
                            {
                                // Splicing: replace edge (u, v) with u -> p_1 -> ... -> p_k -> v
                                let mut new_cl = Vec::with_capacity(n_cl + path.len());
                                new_cl.extend_from_slice(&cl[0..=i]);
                                new_cl.extend_from_slice(path);
                                new_cl.extend_from_slice(&cl[(i + 1)..n_cl]);

                                if Self::validate_cycle(&new_cl, g) {
                                    *cl = new_cl;
                                    absorbed = true;
                                    progress = true;
                                    break 'path_loop;
                                }
                            }
                        }
                    }
                }

                if !absorbed {
                    unabsorbed.push(cs);
                }
            }

            small_cycles = unabsorbed;
        }

        // 3. Return consolidated cycles
        let mut result = large_cycles;
        result.extend(small_cycles);
        result
    }

    /// Enumerates all Hamiltonian paths in the induced subgraph G[V(C_s)].
    fn find_hamiltonian_paths(cs: &[i32], g: &Graph) -> Vec<Vec<i32>> {
        let k = cs.len();
        if k == 0 {
            return Vec::new();
        }
        if k == 1 {
            return vec![cs.to_vec()];
        }

        let node_set: HashSet<i32> = cs.iter().cloned().collect();
        let mut induced_adj: HashMap<i32, Vec<i32>> = HashMap::new();

        for &u in cs {
            let mut neighbors = Vec::new();
            if let Some(nbrs) = g.adjacency_list.get(&u) {
                for &v in nbrs {
                    if node_set.contains(&v) {
                        neighbors.push(v);
                    }
                }
            }
            induced_adj.insert(u, neighbors);
        }

        let mut paths = Vec::new();
        let mut current_path = Vec::with_capacity(k);
        let mut visited: HashSet<i32> = HashSet::with_capacity(k);

        const MAX_PATHS: usize = 50_000;

        for &start_node in cs {
            current_path.push(start_node);
            visited.insert(start_node);
            Self::dfs_hamiltonian_paths(
                start_node,
                k,
                &induced_adj,
                &mut visited,
                &mut current_path,
                &mut paths,
                MAX_PATHS,
            );
            visited.remove(&start_node);
            current_path.pop();

            if paths.len() >= MAX_PATHS {
                break;
            }
        }

        paths
    }

    fn dfs_hamiltonian_paths(
        current: i32,
        target_len: usize,
        adj: &HashMap<i32, Vec<i32>>,
        visited: &mut HashSet<i32>,
        current_path: &mut Vec<i32>,
        results: &mut Vec<Vec<i32>>,
        max_paths: usize,
    ) {
        if results.len() >= max_paths {
            return;
        }

        if current_path.len() == target_len {
            results.push(current_path.clone());
            return;
        }

        if let Some(neighbors) = adj.get(&current) {
            for &next in neighbors {
                if !visited.contains(&next) {
                    visited.insert(next);
                    current_path.push(next);
                    Self::dfs_hamiltonian_paths(
                        next,
                        target_len,
                        adj,
                        visited,
                        current_path,
                        results,
                        max_paths,
                    );
                    current_path.pop();
                    visited.remove(&next);

                    if results.len() >= max_paths {
                        return;
                    }
                }
            }
        }
    }

    #[inline]
    fn is_protected(u: i32, v: i32, protected_edges: &HashSet<(i32, i32)>) -> bool {
        protected_edges.contains(&(u, v)) || protected_edges.contains(&(v, u))
    }

    #[inline]
    fn has_edge(g: &Graph, u: i32, v: i32) -> bool {
        if let Some(nbrs) = g.adjacency_list.get(&u) {
            nbrs.contains(&v)
        } else {
            false
        }
    }

    /// Validates 2-regularity and simplicity of the enlarged cycle.
    fn validate_cycle(cycle: &[i32], g: &Graph) -> bool {
        let n = cycle.len();
        if n < 3 {
            return false;
        }
        let mut seen = HashSet::with_capacity(n);
        for i in 0..n {
            let u = cycle[i];
            if !seen.insert(u) {
                return false;
            }
            let v = cycle[(i + 1) % n];
            if !Self::has_edge(g, u, v) {
                return false;
            }
        }
        true
    }
}
