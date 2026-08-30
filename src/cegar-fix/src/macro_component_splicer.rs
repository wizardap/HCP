use std::collections::{HashMap, HashSet, VecDeque};
use crate::graph::Graph;

#[inline]
fn min_max(u: i32, v: i32) -> (i32, i32) {
    if u < v {
        (u, v)
    } else {
        (v, u)
    }
}

#[inline]
fn is_edge_in_graph(g: &Graph, u: i32, v: i32) -> bool {
    if let Some(nbrs) = g.adjacency_list.get(&u) {
        nbrs.contains(&v)
    } else {
        false
    }
}

pub struct MacroComponentSplicer;

impl MacroComponentSplicer {
    /// Discovers the macro-adjacency graph of 2-opt bridges and merges entire connected spanning trees.
    pub fn splice_spanning_components(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
    ) -> Vec<Vec<i32>> {
        if cycles.len() <= 1 {
            return cycles.to_vec();
        }

        // Validate cycle lengths
        for c in cycles {
            if c.len() < 3 {
                return cycles.to_vec();
            }
        }

        let canonical_protected: HashSet<(i32, i32)> = protected_edges
            .iter()
            .map(|&(u, v)| min_max(u, v))
            .collect();

        let mut current_cycles = cycles.to_vec();
        let max_passes = 20;

        for _ in 0..max_passes {
            if current_cycles.len() <= 1 {
                break;
            }
            let next_cycles = Self::splice_one_pass(&current_cycles, g, &canonical_protected);
            if next_cycles.len() < current_cycles.len() {
                current_cycles = next_cycles;
            } else {
                break;
            }
        }

        // Sort cycles deterministically: descending length, then min vertex
        current_cycles.sort_by(|a, b| {
            b.len()
                .cmp(&a.len())
                .then_with(|| a.iter().min().cmp(&b.iter().min()))
        });

        current_cycles
    }

    /// Single pass: constructs macro-adjacency graph of 2-opt bridges, extracts connected components
    /// and spanning trees, and merges cycles sequentially along the spanning trees.
    fn splice_one_pass(
        cycles: &[Vec<i32>],
        g: &Graph,
        canonical_protected: &HashSet<(i32, i32)>,
    ) -> Vec<Vec<i32>> {
        let m = cycles.len();
        if m <= 1 {
            return cycles.to_vec();
        }

        let total_v: usize = cycles.iter().map(|c| c.len()).sum();
        let mut vertex_to_cycle: HashMap<i32, usize> = HashMap::with_capacity(total_v);
        let mut cycle_neighbors: HashMap<i32, [i32; 2]> = HashMap::with_capacity(total_v);

        for (c_idx, cycle) in cycles.iter().enumerate() {
            let n = cycle.len();
            for pos in 0..n {
                let u = cycle[pos];
                let prev = cycle[(pos + n - 1) % n];
                let next = cycle[(pos + 1) % n];
                vertex_to_cycle.insert(u, c_idx);
                cycle_neighbors.insert(u, [prev, next]);
            }
        }

        // 1. Build Macro-Adjacency Graph of 2-opt bridges
        let mut macro_adj: Vec<HashSet<usize>> = vec![HashSet::new(); m];

        for i in 0..m {
            let cycle = &cycles[i];
            let n = cycle.len();
            for pos in 0..n {
                let u1 = cycle[pos];
                let u2 = cycle[(pos + 1) % n];
                let e_i = min_max(u1, u2);
                if canonical_protected.contains(&e_i) {
                    continue;
                }

                if let Some(nbrs) = g.adjacency_list.get(&u1) {
                    for &v1 in nbrs {
                        if let Some(&j) = vertex_to_cycle.get(&v1) {
                            if j == i {
                                continue;
                            }

                            let [v_prev, v_next] = cycle_neighbors[&v1];

                            // Candidate 1: v2 = v_next
                            let e_j_next = min_max(v1, v_next);
                            if !canonical_protected.contains(&e_j_next) && is_edge_in_graph(g, u2, v_next) {
                                macro_adj[i].insert(j);
                                macro_adj[j].insert(i);
                            }

                            // Candidate 2: v2 = v_prev
                            let e_j_prev = min_max(v1, v_prev);
                            if !canonical_protected.contains(&e_j_prev) && is_edge_in_graph(g, u2, v_prev) {
                                macro_adj[i].insert(j);
                                macro_adj[j].insert(i);
                            }
                        }
                    }
                }
            }
        }

        // 2. Discover Connected Components in Macro-Graph
        let mut comp_visited = vec![false; m];
        let mut components: Vec<Vec<usize>> = Vec::new();

        for i in 0..m {
            if !comp_visited[i] {
                let mut comp = Vec::new();
                let mut queue = VecDeque::new();
                comp_visited[i] = true;
                queue.push_back(i);

                while let Some(curr) = queue.pop_front() {
                    comp.push(curr);
                    for &nbr in &macro_adj[curr] {
                        if !comp_visited[nbr] {
                            comp_visited[nbr] = true;
                            queue.push_back(nbr);
                        }
                    }
                }
                comp.sort_unstable();
                components.push(comp);
            }
        }

        // 3. Extract Spanning Tree & Merge Cycles Sequentially per Component
        let mut result_cycles: Vec<Vec<i32>> = Vec::new();

        for comp in components {
            if comp.len() == 1 {
                result_cycles.push(cycles[comp[0]].clone());
                continue;
            }

            // Extract BFS spanning tree order starting at comp[0]
            let comp_set: HashSet<usize> = comp.iter().copied().collect();
            let root = comp[0];
            let mut tree_order: Vec<usize> = Vec::new();
            let mut tree_visited: HashSet<usize> = HashSet::new();
            let mut queue = VecDeque::new();

            tree_visited.insert(root);
            queue.push_back(root);

            while let Some(curr) = queue.pop_front() {
                tree_order.push(curr);
                for &nbr in &macro_adj[curr] {
                    if comp_set.contains(&nbr) && tree_visited.insert(nbr) {
                        queue.push_back(nbr);
                    }
                }
            }

            // Sequential Tree Merging
            let mut merged = cycles[root].clone();
            let mut unmerged: VecDeque<Vec<i32>> = VecDeque::new();
            for &idx in &tree_order[1..] {
                unmerged.push_back(cycles[idx].clone());
            }

            let mut progress = true;
            while progress && !unmerged.is_empty() {
                progress = false;
                let n_unmerged = unmerged.len();
                for _ in 0..n_unmerged {
                    let next_c = unmerged.pop_front().unwrap();
                    if let Some(new_merged) = Self::try_merge_two_cycles(&merged, &next_c, g, canonical_protected) {
                        merged = new_merged;
                        progress = true;
                    } else {
                        unmerged.push_back(next_c);
                    }
                }
            }

            result_cycles.push(merged);
            for remaining in unmerged {
                result_cycles.push(remaining);
            }
        }

        result_cycles
    }

    /// Attempts a valid 2-opt merge between two disjoint cycles `c1` and `c2`.
    /// Preserves protected edges and validates 2-regularity and Hamiltonian connectivity.
    fn try_merge_two_cycles(
        c1: &[i32],
        c2: &[i32],
        g: &Graph,
        canonical_protected: &HashSet<(i32, i32)>,
    ) -> Option<Vec<i32>> {
        let p = c1.len();
        let q = c2.len();
        if p < 3 || q < 3 {
            return None;
        }

        let mut c2_map: HashMap<i32, usize> = HashMap::with_capacity(q);
        for (pos, &v) in c2.iter().enumerate() {
            c2_map.insert(v, pos);
        }

        for i in 0..p {
            let u1 = c1[i];
            let u2 = c1[(i + 1) % p];
            let e1 = min_max(u1, u2);
            if canonical_protected.contains(&e1) {
                continue;
            }

            if let Some(nbrs) = g.adjacency_list.get(&u1) {
                for &v1 in nbrs {
                    if let Some(&j) = c2_map.get(&v1) {
                        // Candidate 1: Edge (c2[j], c2[(j+1)%q]) in c2
                        let next_j = (j + 1) % q;
                        let v2_next = c2[next_j];
                        let e2_next = min_max(v1, v2_next);
                        if !canonical_protected.contains(&e2_next) && is_edge_in_graph(g, u2, v2_next) {
                            // Case 1: Cross edges (u1, v1) and (u2, v2_next)
                            let mut merged = Vec::with_capacity(p + q);
                            // c1 forward from u2 to u1:
                            for k in 1..=p {
                                merged.push(c1[(i + k) % p]);
                            }
                            // c2 reverse from v1 to v2_next:
                            for k in 0..q {
                                merged.push(c2[(j + q - (k % q)) % q]);
                            }
                            if Self::validate_cycle(&merged, p + q, g) {
                                return Some(merged);
                            }
                        }

                        // Candidate 2: Edge (c2[(j+q-1)%q], c2[j]) in c2
                        let prev_j = (j + q - 1) % q;
                        let v2_prev = c2[prev_j];
                        let e2_prev = min_max(v2_prev, v1);
                        if !canonical_protected.contains(&e2_prev) && is_edge_in_graph(g, u2, v2_prev) {
                            // Case 2: Cross edges (u1, v1) and (u2, v2_prev)
                            let mut merged = Vec::with_capacity(p + q);
                            // c1 forward from u2 to u1:
                            for k in 1..=p {
                                merged.push(c1[(i + k) % p]);
                            }
                            // c2 forward from v1 to v2_prev:
                            for k in 1..=q {
                                merged.push(c2[(prev_j + k) % q]);
                            }
                            if Self::validate_cycle(&merged, p + q, g) {
                                return Some(merged);
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Validates that `cycle` is a single simple cycle of exact length `expected_len`
    /// where every consecutive pair is an edge in graph `g`.
    fn validate_cycle(cycle: &[i32], expected_len: usize, g: &Graph) -> bool {
        if cycle.len() != expected_len || cycle.len() < 3 {
            return false;
        }

        let mut seen = HashSet::with_capacity(cycle.len());
        for &v in cycle {
            if !seen.insert(v) {
                return false; // duplicate vertex
            }
        }

        let n = cycle.len();
        for i in 0..n {
            let u = cycle[i];
            let v = cycle[(i + 1) % n];
            if !is_edge_in_graph(g, u, v) {
                return false;
            }
        }

        true
    }
}
