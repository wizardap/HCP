use std::collections::{HashMap, HashSet};
use crate::graph::Graph;

#[inline]
fn min_max(u: i32, v: i32) -> (i32, i32) {
    if u < v {
        (u, v)
    } else {
        (v, u)
    }
}

pub struct MacroCycleStitcher;

impl MacroCycleStitcher {
    /// Attempts exact multi-cycle alternating patch merging on current 2-factor cycles.
    /// Returns Some(merged_cycles) if cycle count strictly decreased, or None.
    pub fn stitch_cycles(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
        max_swaps: usize,
    ) -> Option<Vec<Vec<i32>>> {
        if cycles.len() < 2 || max_swaps < 2 {
            return None;
        }

        // Validate cycle lengths
        for c in cycles {
            if c.len() < 3 {
                return None;
            }
        }

        let total_v: usize = cycles.iter().map(|c| c.len()).sum();
        let mut vertex_to_cycle: HashMap<i32, usize> = HashMap::with_capacity(total_v);
        let mut f_neighbors: HashMap<i32, [i32; 2]> = HashMap::with_capacity(total_v);
        let mut f_edges: HashSet<(i32, i32)> = HashSet::with_capacity(total_v);

        for (c_idx, cycle) in cycles.iter().enumerate() {
            let n = cycle.len();
            for pos in 0..n {
                let u = cycle[pos];
                let prev = cycle[(pos + n - 1) % n];
                let next = cycle[(pos + 1) % n];
                vertex_to_cycle.insert(u, c_idx);
                f_neighbors.insert(u, [prev, next]);
                f_edges.insert(min_max(u, prev));
                f_edges.insert(min_max(u, next));
            }
        }

        let canonical_protected: HashSet<(i32, i32)> = protected_edges
            .iter()
            .map(|&(u, v)| min_max(u, v))
            .collect();

        // Collect candidate cross edges between distinct cycles
        let mut cross_edges: Vec<(i32, i32)> = Vec::new();
        for (&u, &c_u) in &vertex_to_cycle {
            if let Some(nbrs) = g.adjacency_list.get(&u) {
                for &v in nbrs {
                    if u < v {
                        if let Some(&c_v) = vertex_to_cycle.get(&v) {
                            if c_u != c_v {
                                let e = (u, v);
                                if !f_edges.contains(&e) {
                                    cross_edges.push(e);
                                }
                            }
                        }
                    }
                }
            }
        }

        if cross_edges.is_empty() {
            return None;
        }

        // Search for alternating cycles of size m in 2..=max_swaps
        for target_m in 2..=max_swaps {
            for &(u, v) in &cross_edges {
                // Try starting in both directions: u -> v and v -> u
                for &(w0, w1) in &[(u, v), (v, u)] {
                    let edge_y1 = min_max(w0, w1);
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
                        &canonical_protected,
                    ) {
                        return Some(merged);
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
        vertex_to_cycle: &HashMap<i32, usize>,
        f_neighbors: &HashMap<i32, [i32; 2]>,
        f_edges: &HashSet<(i32, i32)>,
        canonical_protected: &HashSet<(i32, i32)>,
    ) -> Option<Vec<Vec<i32>>> {
        let current_y_count = used_y_edges.len();
        let w0 = path[0];
        let curr = *path.last().unwrap();

        let f_nbrs = match f_neighbors.get(&curr) {
            Some(nbrs) => *nbrs,
            None => return None,
        };

        for &next_x in &f_nbrs {
            let edge_x = min_max(curr, next_x);
            if canonical_protected.contains(&edge_x) {
                continue;
            }
            if used_x_edges.contains(&edge_x) || used_y_edges.contains(&edge_x) {
                continue;
            }

            // Case 1: Can we close the cycle back to w0?
            if next_x == w0 {
                if current_y_count == target_m {
                    used_x_edges.insert(edge_x);
                    if let Some(new_cycles) = Self::evaluate_and_reconstruct_cycles(
                        cycles,
                        vertex_to_cycle,
                        f_neighbors,
                        used_x_edges,
                        used_y_edges,
                    ) {
                        if new_cycles.len() < cycles.len() {
                            used_x_edges.remove(&edge_x);
                            return Some(new_cycles);
                        }
                    }
                    used_x_edges.remove(&edge_x);
                }
                continue;
            }

            // Case 2: Extend search if current_y_count < target_m
            if current_y_count < target_m && !visited_verts.contains(&next_x) {
                visited_verts.insert(next_x);
                used_x_edges.insert(edge_x);
                path.push(next_x);

                if let Some(nbrs_in_g) = g.adjacency_list.get(&next_x) {
                    for &next_y in nbrs_in_g {
                        if !vertex_to_cycle.contains_key(&next_y) {
                            continue;
                        }
                        let edge_y = min_max(next_x, next_y);
                        if f_edges.contains(&edge_y) {
                            continue;
                        }
                        if used_y_edges.contains(&edge_y) || used_x_edges.contains(&edge_y) {
                            continue;
                        }
                        if visited_verts.contains(&next_y) {
                            continue;
                        }

                        visited_verts.insert(next_y);
                        used_y_edges.insert(edge_y);
                        path.push(next_y);

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
                            canonical_protected,
                        ) {
                            return Some(merged);
                        }

                        path.pop();
                        used_y_edges.remove(&edge_y);
                        visited_verts.remove(&next_y);
                    }
                }

                path.pop();
                used_x_edges.remove(&edge_x);
                visited_verts.remove(&next_x);
            }
        }

        None
    }

    fn evaluate_and_reconstruct_cycles(
        cycles: &[Vec<i32>],
        vertex_to_cycle: &HashMap<i32, usize>,
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
        for nbrs in adj.values() {
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
                    let mut prev: Option<i32> = None;

                    while !visited.contains(&curr) {
                        visited.insert(curr);
                        current_cycle.push(curr);
                        let nbrs = &adj[&curr];
                        let next = match prev {
                            Some(p) => {
                                if nbrs[0] == p {
                                    nbrs[1]
                                } else {
                                    nbrs[0]
                                }
                            }
                            None => nbrs[0],
                        };
                        prev = Some(curr);
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

    /// Iteratively stitches cycles until a single tour is obtained or no further merge is possible.
    pub fn stitch_until_fixed_point(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
    ) -> Vec<Vec<i32>> {
        if cycles.len() <= 1 {
            return cycles.to_vec();
        }

        let mut current_cycles = cycles.to_vec();
        let max_passes = 20;

        for _ in 0..max_passes {
            if current_cycles.len() <= 1 {
                break;
            }
            if let Some(next_cycles) = Self::stitch_cycles(&current_cycles, g, protected_edges, 4) {
                current_cycles = next_cycles;
            } else if let Some(next_cycles) = Self::stitch_cycles(&current_cycles, g, protected_edges, 6) {
                current_cycles = next_cycles;
            } else {
                break;
            }
        }

        current_cycles
    }
}
