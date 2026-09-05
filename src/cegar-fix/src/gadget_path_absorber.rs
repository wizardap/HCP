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

        // Find the maximum cycle length
        let max_len = cycles.iter().map(|c| c.len()).max().unwrap_or(0);
        let threshold = if max_len >= 100 {
            max_len / 2
        } else if max_len >= 32 {
            32
        } else {
            16
        };

        let mut large_cycles: Vec<Vec<i32>> =
            cycles.iter().filter(|c| c.len() >= threshold).cloned().collect();
        let mut small_cycles: Vec<Vec<i32>> =
            cycles.iter().filter(|c| c.len() < threshold).cloned().collect();

        if large_cycles.is_empty() {
            let mut sorted = cycles.to_vec();
            sorted.sort_by_key(|c| std::cmp::Reverse(c.len()));
            large_cycles.push(sorted.remove(0));
            small_cycles = sorted;
        }

        if small_cycles.is_empty() {
            return cycles.to_vec();
        }

        // Build position maps for all large cycles
        let mut cl_pos_list: Vec<HashMap<i32, usize>> = large_cycles
            .iter()
            .map(|cl| {
                let mut pos = HashMap::with_capacity(cl.len());
                for (idx, &v) in cl.iter().enumerate() {
                    pos.insert(v, idx);
                }
                pos
            })
            .collect();

        // Sort small cycles by length ascending (absorb smaller satellites first)
        small_cycles.sort_by_key(|c| c.len());

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

                    let nbrs_first = match g.adjacency_list.get(&p_first) {
                        Some(nbrs) => nbrs,
                        None => continue,
                    };

                    for &u in nbrs_first {
                        if Self::is_protected(u, p_first, protected_edges) {
                            continue;
                        }

                        for cl_idx in 0..large_cycles.len() {
                            if let Some(&i) = cl_pos_list[cl_idx].get(&u) {
                                let cl = &large_cycles[cl_idx];
                                let n_cl = cl.len();
                                if n_cl < 2 {
                                    continue;
                                }

                                // Check forward splice: (u, v) in cl -> u -> path -> v
                                let v_fwd = cl[(i + 1) % n_cl];
                                if !Self::is_protected(u, v_fwd, protected_edges)
                                    && !Self::is_protected(p_last, v_fwd, protected_edges)
                                    && Self::has_edge(g, p_last, v_fwd)
                                {
                                    let mut new_cl = Vec::with_capacity(n_cl + path.len());
                                    new_cl.extend_from_slice(&cl[0..=i]);
                                    new_cl.extend_from_slice(path);
                                    new_cl.extend_from_slice(&cl[(i + 1)..n_cl]);

                                    let mut new_pos = HashMap::with_capacity(new_cl.len());
                                    for (idx, &v) in new_cl.iter().enumerate() {
                                        new_pos.insert(v, idx);
                                    }
                                    large_cycles[cl_idx] = new_cl;
                                    cl_pos_list[cl_idx] = new_pos;
                                    absorbed = true;
                                    progress = true;
                                    break 'path_loop;
                                }

                                // Check reverse splice: edge (v_rev, u) in cl -> v_rev <- p_last <- ... <- p_first <- u
                                let v_rev = cl[(i + n_cl - 1) % n_cl];
                                if !Self::is_protected(v_rev, u, protected_edges)
                                    && !Self::is_protected(p_last, v_rev, protected_edges)
                                    && Self::has_edge(g, p_last, v_rev)
                                {
                                    let mut new_cl = Vec::with_capacity(n_cl + path.len());
                                    let j = (i + n_cl - 1) % n_cl;
                                    if j < i {
                                        new_cl.extend_from_slice(&cl[0..=j]);
                                        for &v in path.iter().rev() {
                                            new_cl.push(v);
                                        }
                                        new_cl.extend_from_slice(&cl[i..n_cl]);
                                    } else {
                                        for &v in path.iter().rev() {
                                            new_cl.push(v);
                                        }
                                        new_cl.extend_from_slice(&cl[0..n_cl]);
                                    }

                                    let mut new_pos = HashMap::with_capacity(new_cl.len());
                                    for (idx, &v) in new_cl.iter().enumerate() {
                                        new_pos.insert(v, idx);
                                    }
                                    large_cycles[cl_idx] = new_cl;
                                    cl_pos_list[cl_idx] = new_pos;
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

        let mut paths = Vec::new();

        // 1. Natural rotation paths from cycle: removing any edge (cs[i], cs[i+1])
        for i in 0..k {
            let mut p_fwd = Vec::with_capacity(k);
            for offset in 1..=k {
                p_fwd.push(cs[(i + offset) % k]);
            }
            paths.push(p_fwd);

            let mut p_rev = Vec::with_capacity(k);
            for offset in 0..k {
                p_rev.push(cs[(i + k - offset) % k]);
            }
            paths.push(p_rev);
        }

        // 2. If small gadget (k <= 20), also discover alternative paths via internal chords
        if k <= 20 {
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

            let mut current_path = Vec::with_capacity(k);
            let mut visited: HashSet<i32> = HashSet::with_capacity(k);
            const MAX_PATHS: usize = 10_000;

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
