use std::collections::{HashMap, HashSet};
use crate::graph::Graph;
use crate::contraction::Degree2Contractor;

#[inline]
fn min_max(u: i32, v: i32) -> (i32, i32) {
    if u < v {
        (u, v)
    } else {
        (v, u)
    }
}

pub struct ModuleDualPathExtractor;

impl ModuleDualPathExtractor {
    /// Extracts dual Hamiltonian paths (T_i and F_i) spanning all vertices of a module,
    /// where every step alternates between virtual edges and real edges, and 100% of
    /// the module's virtual edges are utilized.
    pub fn extract_dual_paths(
        mod_vertices: &[i32],
        g: &Graph,
        contractor: &Degree2Contractor,
    ) -> Option<(Vec<i32>, Vec<i32>)> {
        let n = mod_vertices.len();
        if n < 2 {
            return None;
        }
        let mod_set: HashSet<i32> = mod_vertices.iter().copied().collect();

        // Build virtual edge mapping for this module: vertex -> virtual partner
        let mut virtual_partner: HashMap<i32, i32> = HashMap::new();
        for &v in mod_vertices {
            for (&(u, w), _) in &contractor.chain_map {
                if u == v && mod_set.contains(&w) {
                    virtual_partner.insert(u, w);
                } else if w == v && mod_set.contains(&u) {
                    virtual_partner.insert(w, u);
                }
            }
        }

        // Induced real adjacency inside the module
        let mut real_adj: HashMap<i32, Vec<i32>> = HashMap::new();
        for &u in mod_vertices {
            if let Some(nbrs) = g.adjacency_list.get(&u) {
                let mut valid_nbrs = Vec::new();
                for &v in nbrs {
                    if mod_set.contains(&v) {
                        // Check it's not the virtual partner
                        if virtual_partner.get(&u) != Some(&v) {
                            valid_nbrs.push(v);
                        }
                    }
                }
                real_adj.insert(u, valid_nbrs);
            }
        }

        // Identify boundary ports (vertices with no internal virtual partner, or lowest internal virtual degree)
        let mut boundary_ports: Vec<i32> = mod_vertices
            .iter()
            .copied()
            .filter(|v| !virtual_partner.contains_key(v))
            .collect();

        if boundary_ports.len() < 2 {
            // Fallback: any vertex in module
            boundary_ports = mod_vertices.to_vec();
        }

        let mut all_paths: Vec<Vec<i32>> = Vec::new();
        let mut steps = 0;

        for &start_v in &boundary_ports {
            for &initial_virtual in &[false, true] {
                let mut path = Vec::with_capacity(n);
                let mut visited = HashSet::with_capacity(n);
                path.push(start_v);
                visited.insert(start_v);

                Self::dfs_alternating(
                    start_v,
                    initial_virtual,
                    n,
                    &virtual_partner,
                    &real_adj,
                    &mut path,
                    &mut visited,
                    &mut all_paths,
                    &mut steps,
                );

                if all_paths.len() >= 4 || steps > 20000 {
                    break;
                }
            }
            if all_paths.len() >= 4 || steps > 20000 {
                break;
            }
        }

        // Filter paths to find at least 2 paths with distinct UNDIRECTED edge sets
        let mut distinct_paths: Vec<Vec<i32>> = Vec::new();
        let mut seen_undirected_edge_sets: Vec<HashSet<(i32, i32)>> = Vec::new();

        for p in all_paths {
            let edge_set: HashSet<(i32, i32)> = p
                .windows(2)
                .map(|w| if w[0] < w[1] { (w[0], w[1]) } else { (w[1], w[0]) })
                .collect();

            if !seen_undirected_edge_sets.contains(&edge_set) {
                seen_undirected_edge_sets.push(edge_set);
                distinct_paths.push(p);
                if distinct_paths.len() >= 2 {
                    break;
                }
            }
        }

        if distinct_paths.len() >= 2 {
            Some((distinct_paths[0].clone(), distinct_paths[1].clone()))
        } else {
            None
        }
    }

    fn dfs_alternating(
        curr: i32,
        must_take_virtual: bool,
        target_len: usize,
        virtual_partner: &HashMap<i32, i32>,
        real_adj: &HashMap<i32, Vec<i32>>,
        path: &mut Vec<i32>,
        visited: &mut HashSet<i32>,
        results: &mut Vec<Vec<i32>>,
        steps: &mut usize,
    ) {
        *steps += 1;
        if *steps > 20000 || results.len() >= 10 {
            return;
        }

        if path.len() == target_len {
            results.push(path.clone());
            return;
        }

        if must_take_virtual {
            // Must take virtual edge partner
            if let Some(&nxt) = virtual_partner.get(&curr) {
                if !visited.contains(&nxt) {
                    visited.insert(nxt);
                    path.push(nxt);
                    Self::dfs_alternating(
                        nxt,
                        false,
                        target_len,
                        virtual_partner,
                        real_adj,
                        path,
                        visited,
                        results,
                        steps,
                    );
                    path.pop();
                    visited.remove(&nxt);
                }
            }
        } else {
            // Must take real edge - Warnsdorff heuristic: sort neighbors by unvisited degree
            if let Some(nbrs) = real_adj.get(&curr) {
                let mut candidates: Vec<(usize, i32)> = Vec::new();
                for &nxt in nbrs {
                    if !visited.contains(&nxt) {
                        let unvisited_deg = real_adj
                            .get(&nxt)
                            .map_or(0, |adj| adj.iter().filter(|x| !visited.contains(x)).count());
                        candidates.push((unvisited_deg, nxt));
                    }
                }
                candidates.sort_by_key(|&(deg, _)| deg);

                for (_, nxt) in candidates {
                    visited.insert(nxt);
                    path.push(nxt);
                    Self::dfs_alternating(
                        nxt,
                        true,
                        target_len,
                        virtual_partner,
                        real_adj,
                        path,
                        visited,
                        results,
                        steps,
                    );
                    path.pop();
                    visited.remove(&nxt);

                    if results.len() >= 10 || *steps > 20000 {
                        return;
                    }
                }
            }
        }
    }
}
