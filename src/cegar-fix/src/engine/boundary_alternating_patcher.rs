use std::collections::{HashMap, HashSet};
use crate::contraction::Degree2Contractor;
use crate::graph::Graph;

#[inline]
fn min_max(u: i32, v: i32) -> (i32, i32) {
    if u < v {
        (u, v)
    } else {
        (v, u)
    }
}

pub struct BoundaryAlternatingPatcher;

impl BoundaryAlternatingPatcher {
    /// Searches for multi-hop alternating augmenting cycles between macro-cycles (k in 2..=4)
    /// and merges them. Returns Some(merged_cycles) if cycle count is reduced.
    pub fn try_patch_macro_hemispheres(
        cycles: &[Vec<i32>],
        g: &Graph,
        contractor: &Degree2Contractor,
        max_search_depth: usize,
    ) -> Option<Vec<Vec<i32>>> {
        if cycles.len() < 2 || cycles.len() > 4 {
            return None;
        }

        let max_depth = max_search_depth.clamp(2, 6);
        let mut current_cycles = cycles.to_vec();
        let mut any_merged = false;

        while current_cycles.len() >= 2 && current_cycles.len() <= 4 {
            if let Some(next_cycles) = Self::find_single_alternating_patch(
                &current_cycles,
                g,
                contractor,
                max_depth,
            ) {
                current_cycles = next_cycles;
                any_merged = true;
                if current_cycles.len() == 1 {
                    break;
                }
            } else {
                break;
            }
        }

        if any_merged {
            Some(current_cycles)
        } else {
            None
        }
    }

    fn find_single_alternating_patch(
        cycles: &[Vec<i32>],
        g: &Graph,
        contractor: &Degree2Contractor,
        max_depth: usize,
    ) -> Option<Vec<Vec<i32>>> {
        let total_v: usize = cycles.iter().map(|c| c.len()).sum();
        let mut vertex_to_cycle: HashMap<i32, (usize, usize)> = HashMap::with_capacity(total_v);
        let mut f_neighbors: HashMap<i32, [i32; 2]> = HashMap::with_capacity(total_v);
        let mut f_edges: HashSet<(i32, i32)> = HashSet::with_capacity(total_v);

        for (c_idx, cycle) in cycles.iter().enumerate() {
            let n = cycle.len();
            if n < 2 {
                return None;
            }
            for (pos, &u) in cycle.iter().enumerate() {
                let prev = cycle[(pos + n - 1) % n];
                let next = cycle[(pos + 1) % n];
                vertex_to_cycle.insert(u, (c_idx, pos));
                f_neighbors.insert(u, [prev, next]);
                f_edges.insert(min_max(u, prev));
                f_edges.insert(min_max(u, next));
            }
        }

        let mut protected_edges: HashSet<(i32, i32)> = HashSet::new();
        for (&(u, v), _) in &contractor.chain_map {
            protected_edges.insert(min_max(u, v));
        }

        // Iterative deepening on alternating cycle size m in 2..=max_depth
        for target_m in 2..=max_depth {
            for (c_idx, cycle) in cycles.iter().enumerate() {
                for &w0 in cycle {
                    if let Some(neighbors) = g.adjacency_list.get(&w0) {
                        for &w1 in neighbors {
                            if !vertex_to_cycle.contains_key(&w1) {
                                continue;
                            }
                            let edge_y1 = min_max(w0, w1);
                            if f_edges.contains(&edge_y1) {
                                continue;
                            }
                            let w1_c_idx = vertex_to_cycle[&w1].0;
                            if w1_c_idx == c_idx {
                                // Must start with a cross-edge between distinct cycles
                                continue;
                            }

                            let mut path = vec![w0, w1];
                            let mut visited_verts = HashSet::new();
                            visited_verts.insert(w0);
                            visited_verts.insert(w1);

                            let mut used_y_edges = HashSet::new();
                            used_y_edges.insert(edge_y1);
                            let mut used_x_edges = HashSet::new();

                            if let Some(merged) = Self::dfs_alternating(
                                &mut path,
                                &mut visited_verts,
                                &mut used_y_edges,
                                &mut used_x_edges,
                                target_m,
                                g,
                                cycles,
                                &vertex_to_cycle,
                                &f_neighbors,
                                &f_edges,
                                &protected_edges,
                            ) {
                                return Some(merged);
                            }
                        }
                    }
                }
            }
        }

        None
    }

    fn dfs_alternating(
        path: &mut Vec<i32>,
        visited_verts: &mut HashSet<i32>,
        used_y_edges: &mut HashSet<(i32, i32)>,
        used_x_edges: &mut HashSet<(i32, i32)>,
        target_m: usize,
        g: &Graph,
        cycles: &[Vec<i32>],
        vertex_to_cycle: &HashMap<i32, (usize, usize)>,
        f_neighbors: &HashMap<i32, [i32; 2]>,
        f_edges: &HashSet<(i32, i32)>,
        protected_edges: &HashSet<(i32, i32)>,
    ) -> Option<Vec<Vec<i32>>> {
        let current_y_count = used_y_edges.len();
        let w0 = path[0];
        let curr = *path.last().unwrap();

        // Currently at vertex curr where we just arrived via a Y-edge.
        // We now need to take an X-edge in F \ protected_edges.
        let f_nbrs = match f_neighbors.get(&curr) {
            Some(nbrs) => *nbrs,
            None => return None,
        };

        for &next in &f_nbrs {
            let edge_x = min_max(curr, next);
            if protected_edges.contains(&edge_x) {
                continue;
            }
            if used_x_edges.contains(&edge_x) || used_y_edges.contains(&edge_x) {
                continue;
            }

            // Case 1: Can we close the cycle back to w0?
            if next == w0 {
                if current_y_count == target_m {
                    used_x_edges.insert(edge_x);
                    if let Some(new_cycles) = Self::evaluate_symmetric_difference(
                        cycles,
                        vertex_to_cycle,
                        f_neighbors,
                        used_x_edges,
                        used_y_edges,
                    ) {
                        used_x_edges.remove(&edge_x);
                        return Some(new_cycles);
                    }
                    used_x_edges.remove(&edge_x);
                }
                continue;
            }

            // Case 2: Continue search if not yet at target_m
            if current_y_count < target_m && !visited_verts.contains(&next) {
                visited_verts.insert(next);
                used_x_edges.insert(edge_x);
                path.push(next);

                // From `next`, we now need to take a Y-edge (in E(G) \ F)
                if let Some(nbrs_in_g) = g.adjacency_list.get(&next) {
                    for &y_next in nbrs_in_g {
                        if !vertex_to_cycle.contains_key(&y_next) {
                            continue;
                        }
                        let edge_y = min_max(next, y_next);
                        if f_edges.contains(&edge_y) {
                            continue;
                        }
                        if used_y_edges.contains(&edge_y) || used_x_edges.contains(&edge_y) {
                            continue;
                        }
                        if visited_verts.contains(&y_next) {
                            continue;
                        }

                        visited_verts.insert(y_next);
                        used_y_edges.insert(edge_y);
                        path.push(y_next);

                        if let Some(merged) = Self::dfs_alternating(
                            path,
                            visited_verts,
                            used_y_edges,
                            used_x_edges,
                            target_m,
                            g,
                            cycles,
                            vertex_to_cycle,
                            f_neighbors,
                            f_edges,
                            protected_edges,
                        ) {
                            return Some(merged);
                        }

                        path.pop();
                        used_y_edges.remove(&edge_y);
                        visited_verts.remove(&y_next);
                    }
                }

                path.pop();
                used_x_edges.remove(&edge_x);
                visited_verts.remove(&next);
            }
        }

        None
    }

    fn evaluate_symmetric_difference(
        cycles: &[Vec<i32>],
        vertex_to_cycle: &HashMap<i32, (usize, usize)>,
        f_neighbors: &HashMap<i32, [i32; 2]>,
        used_x_edges: &HashSet<(i32, i32)>,
        used_y_edges: &HashSet<(i32, i32)>,
    ) -> Option<Vec<Vec<i32>>> {
        let total_v = vertex_to_cycle.len();
        let mut adj: HashMap<i32, Vec<i32>> = HashMap::with_capacity(total_v);

        for (&u, &f_nbrs) in f_neighbors {
            let mut u_adj = Vec::with_capacity(2);
            for &nbr in &f_nbrs {
                let e = min_max(u, nbr);
                if !used_x_edges.contains(&e) {
                    u_adj.push(nbr);
                }
            }
            adj.insert(u, u_adj);
        }

        for &(u, v) in used_y_edges {
            if let Some(u_list) = adj.get_mut(&u) {
                u_list.push(v);
            } else {
                return None;
            }
            if let Some(v_list) = adj.get_mut(&v) {
                v_list.push(u);
            } else {
                return None;
            }
        }

        // Verify 2-regularity for all vertices
        for (&_u, nbrs) in &adj {
            if nbrs.len() != 2 {
                return None;
            }
        }

        // Extract connected cycles
        let mut visited = HashSet::with_capacity(total_v);
        let mut new_cycles = Vec::new();

        for cycle in cycles {
            for &start in cycle {
                if !visited.contains(&start) {
                    let mut current_cycle = Vec::new();
                    let mut curr = start;
                    let mut prev = -1;

                    while !visited.contains(&curr) {
                        visited.insert(curr);
                        current_cycle.push(curr);
                        let nbrs = &adj[&curr];
                        let next = if nbrs[0] == prev {
                            nbrs[1]
                        } else {
                            nbrs[0]
                        };
                        prev = curr;
                        curr = next;
                    }

                    if curr != start || current_cycle.len() < 3 {
                        return None;
                    }
                    new_cycles.push(current_cycle);
                }
            }
        }

        if visited.len() != total_v {
            return None;
        }

        if new_cycles.len() < cycles.len() {
            Some(new_cycles)
        } else {
            None
        }
    }
}
