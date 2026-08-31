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

        // Search for Hamiltonian paths using bounded DFS
        // Candidate start vertices: any vertex in the module
        let mut all_paths: Vec<Vec<i32>> = Vec::new();
        let mut steps = 0;

        for &start_v in mod_vertices {
            let mut path = Vec::with_capacity(n);
            let mut visited = HashSet::with_capacity(n);
            path.push(start_v);
            visited.insert(start_v);

            Self::dfs_alternating(
                start_v,
                true, // next step is virtual
                n,
                &virtual_partner,
                &real_adj,
                &mut path,
                &mut visited,
                &mut all_paths,
                &mut steps,
            );

            if all_paths.len() >= 4 || steps > 5000 {
                break;
            }
        }

        if all_paths.len() >= 2 {
            // Pick two distinct paths as True and False
            let t_path = all_paths[0].clone();
            // Find a path that is not simply the reversal of t_path
            let t_rev: Vec<i32> = t_path.iter().rev().copied().collect();
            let f_path = all_paths
                .iter()
                .find(|p| *p != &t_path && *p != &t_rev)
                .cloned()
                .unwrap_or_else(|| all_paths[1].clone());

            Some((t_path, f_path))
        } else if all_paths.len() == 1 {
            let t_path = all_paths[0].clone();
            let f_path = t_path.iter().rev().copied().collect();
            Some((t_path, f_path))
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
        if *steps > 5000 || results.len() >= 10 {
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
                        false, // next step must be real edge
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
            // Must take real edge
            if let Some(nbrs) = real_adj.get(&curr) {
                for &nxt in nbrs {
                    if !visited.contains(&nxt) {
                        visited.insert(nxt);
                        path.push(nxt);
                        Self::dfs_alternating(
                            nxt,
                            true, // next step must be virtual edge
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
            }
        }
    }
}
