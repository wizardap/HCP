use std::collections::{BTreeMap, HashMap, HashSet};
use crate::graph::Graph;
use rustsat::types::{Clause, Lit};

pub trait LitMap {
    fn get_lit(&self, u: i32, v: i32) -> Option<Lit>;
}

impl LitMap for HashMap<(i32, i32), Lit> {
    fn get_lit(&self, u: i32, v: i32) -> Option<Lit> {
        self.get(&(u, v)).copied()
    }
}

impl LitMap for BTreeMap<(i32, i32), Lit> {
    fn get_lit(&self, u: i32, v: i32) -> Option<Lit> {
        self.get(&(u, v)).copied()
    }
}

impl<T: LitMap + ?Sized> LitMap for &T {
    fn get_lit(&self, u: i32, v: i32) -> Option<Lit> {
        (**self).get_lit(u, v)
    }
}

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
    g.adjacency_list
        .get(&u)
        .map_or(false, |nbrs| nbrs.contains(&v))
}

#[derive(Debug, Clone)]
struct Bridge2Opt {
    e_a: (i32, i32),
    e_b: (i32, i32),
    x1: (i32, i32),
    x2: (i32, i32),
}

pub struct TwinGiantSplicer;

impl TwinGiantSplicer {
    /// Attempts direct 2-opt or 3-opt intermediate splicing between two giant cycles.
    /// Returns Some(merged_cycles) if a merge was achieved, or None if no valid merge exists.
    pub fn try_splice_twin_giants(
        cycles: &[Vec<i32>],
        g: &Graph,
        total_v: usize,
    ) -> Option<Vec<Vec<i32>>> {
        if cycles.len() < 2 {
            return None;
        }

        // Validate minimum cycle lengths
        for c in cycles {
            if c.len() < 3 {
                return None;
            }
        }

        // Find the two largest cycles C1 and C2
        let mut indexed: Vec<(usize, &Vec<i32>)> = cycles.iter().enumerate().collect();
        indexed.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

        let idx1 = indexed[0].0;
        let c1 = indexed[0].1;
        let idx2 = indexed[1].0;
        let c2 = indexed[1].1;

        // Threshold check: both |C1|, |C2| >= max(10, total_v / 5)
        let threshold = 10.max(total_v / 5);
        if c1.len() < threshold || c2.len() < threshold {
            return None;
        }

        // 1. Direct 2-Opt between C1 and C2
        let direct_bridges = find_bridges_between_cycles(c1, c2, g);
        for b in &direct_bridges {
            if let Some(merged) = reconstruct_merged_cycle(&[c1, c2], std::slice::from_ref(b), g) {
                let mut new_cycles = Vec::with_capacity(cycles.len() - 1);
                for (idx, c) in cycles.iter().enumerate() {
                    if idx != idx1 && idx != idx2 {
                        new_cycles.push(c.clone());
                    }
                }
                new_cycles.push(merged);
                return Some(new_cycles);
            }
        }

        // 2. Intermediate 3-Way Bridge via Ck
        for (k_idx, ck) in cycles.iter().enumerate() {
            if k_idx == idx1 || k_idx == idx2 || ck.len() < 3 {
                continue;
            }

            let bridges_1k = find_bridges_between_cycles(c1, ck, g);
            if bridges_1k.is_empty() {
                continue;
            }

            let bridges_k2 = find_bridges_between_cycles(ck, c2, g);
            if bridges_k2.is_empty() {
                continue;
            }

            for b1 in &bridges_1k {
                let e_k1 = b1.e_b;
                for b2 in &bridges_k2 {
                    let e_k2 = b2.e_a;
                    // Disjoint bridge check on Ck
                    if e_k1 == e_k2 {
                        continue;
                    }

                    if let Some(merged) =
                        reconstruct_merged_cycle(&[c1, ck, c2], &[b1.clone(), b2.clone()], g)
                    {
                        let mut new_cycles = Vec::with_capacity(cycles.len() - 2);
                        for (idx, c) in cycles.iter().enumerate() {
                            if idx != idx1 && idx != idx2 && idx != k_idx {
                                new_cycles.push(c.clone());
                            }
                        }
                        new_cycles.push(merged);
                        return Some(new_cycles);
                    }
                }
            }
        }

        None
    }

    /// Generates exact bicomponent cut clauses between two giant cycles C1 and C2.
    pub fn generate_bicomponent_cut_clauses<M: LitMap + ?Sized>(
        c1: &[i32],
        c2: &[i32],
        g: &Graph,
        graph_lit_map: &M,
    ) -> Vec<Clause> {
        if c1.is_empty() || c2.is_empty() {
            return Vec::new();
        }

        let s1: HashSet<i32> = c1.iter().copied().collect();
        let s2: HashSet<i32> = c2.iter().copied().collect();

        let mut lits_1_to_2 = Vec::new();
        let mut lits_2_to_1 = Vec::new();

        for &u in &s1 {
            if let Some(neighbors) = g.adjacency_list.get(&u) {
                for &v in neighbors {
                    if s2.contains(&v) {
                        if let Some(lit) = graph_lit_map.get_lit(u, v) {
                            lits_1_to_2.push(lit);
                        }
                    }
                }
            }
        }

        for &u in &s2 {
            if let Some(neighbors) = g.adjacency_list.get(&u) {
                for &v in neighbors {
                    if s1.contains(&v) {
                        if let Some(lit) = graph_lit_map.get_lit(u, v) {
                            lits_2_to_1.push(lit);
                        }
                    }
                }
            }
        }

        let mut clauses = Vec::new();

        if !lits_1_to_2.is_empty() {
            lits_1_to_2.sort_unstable();
            lits_1_to_2.dedup();
            clauses.push(Clause::from_iter(lits_1_to_2));
        }

        if !lits_2_to_1.is_empty() {
            lits_2_to_1.sort_unstable();
            lits_2_to_1.dedup();
            clauses.push(Clause::from_iter(lits_2_to_1));
        }

        clauses
    }
}

/// Finds all candidate 2-opt bridges between cycle Ca and cycle Cb.
fn find_bridges_between_cycles(ca: &[i32], cb: &[i32], g: &Graph) -> Vec<Bridge2Opt> {
    let n = ca.len();
    let m = cb.len();
    if n < 3 || m < 3 {
        return Vec::new();
    }

    let mut pos_in_b: HashMap<i32, usize> = HashMap::with_capacity(m);
    for (idx, &v) in cb.iter().enumerate() {
        pos_in_b.insert(v, idx);
    }

    let mut bridges = Vec::new();
    let mut seen = HashSet::new();

    for i in 0..n {
        let u1 = ca[i];
        let u2 = ca[(i + 1) % n];
        let e_a = min_max(u1, u2);

        if let Some(nbrs) = g.adjacency_list.get(&u1) {
            for &v1 in nbrs {
                if let Some(&j) = pos_in_b.get(&v1) {
                    let v_next = cb[(j + 1) % m];
                    let v_prev = cb[(j + m - 1) % m];

                    // Candidate 1: Case A (u1 -> v1, u2 -> v_next)
                    if is_edge_in_graph(g, u2, v_next) {
                        let e_b = min_max(v1, v_next);
                        let x1 = min_max(u1, v1);
                        let x2 = min_max(u2, v_next);
                        let key = (e_a, e_b, x1, x2);
                        if seen.insert(key) {
                            bridges.push(Bridge2Opt { e_a, e_b, x1, x2 });
                        }
                    }

                    // Candidate 2: Case B (u1 -> v1, u2 -> v_prev)
                    if is_edge_in_graph(g, u2, v_prev) {
                        let e_b = min_max(v1, v_prev);
                        let x1 = min_max(u1, v1);
                        let x2 = min_max(u2, v_prev);
                        let key = (e_a, e_b, x1, x2);
                        if seen.insert(key) {
                            bridges.push(Bridge2Opt { e_a, e_b, x1, x2 });
                        }
                    }
                }
            }
        }
    }

    bridges
}

/// Reconstructs a single merged cycle from candidate cycles and selected bridges.
/// Validates 2-regularity, graph edge validity, and single-cycle connectivity.
fn reconstruct_merged_cycle(
    splice_cycles: &[&Vec<i32>],
    bridges: &[Bridge2Opt],
    g: &Graph,
) -> Option<Vec<i32>> {
    let total_merged_v: usize = splice_cycles.iter().map(|c| c.len()).sum();
    if total_merged_v < 3 {
        return None;
    }

    let mut removed_edges: HashSet<(i32, i32)> = HashSet::new();
    let mut added_edges: Vec<(i32, i32)> = Vec::new();

    for b in bridges {
        removed_edges.insert(b.e_a);
        removed_edges.insert(b.e_b);
        added_edges.push(b.x1);
        added_edges.push(b.x2);
    }

    let mut adj: HashMap<i32, Vec<i32>> = HashMap::with_capacity(total_merged_v);

    for cycle in splice_cycles {
        let n = cycle.len();
        if n < 3 {
            return None;
        }
        for i in 0..n {
            let u = cycle[i];
            let v = cycle[(i + 1) % n];
            let e = min_max(u, v);
            if !removed_edges.contains(&e) {
                adj.entry(u).or_default().push(v);
                adj.entry(v).or_default().push(u);
            }
        }
    }

    for &(u, v) in &added_edges {
        adj.entry(u).or_default().push(v);
        adj.entry(v).or_default().push(u);
    }

    if adj.len() != total_merged_v {
        return None;
    }

    for (&u, nbrs) in &adj {
        if nbrs.len() != 2 || nbrs[0] == nbrs[1] || nbrs[0] == u || nbrs[1] == u {
            return None;
        }
        if !is_edge_in_graph(g, u, nbrs[0]) || !is_edge_in_graph(g, u, nbrs[1]) {
            return None;
        }
    }

    let start_v = splice_cycles[0][0];
    let mut current_cycle = Vec::with_capacity(total_merged_v);
    let mut visited: HashSet<i32> = HashSet::with_capacity(total_merged_v);
    let mut curr = start_v;
    let mut prev: Option<i32> = None;

    loop {
        visited.insert(curr);
        current_cycle.push(curr);

        let nbrs = &adj[&curr];
        let next = if Some(nbrs[0]) == prev {
            nbrs[1]
        } else {
            nbrs[0]
        };

        if next == start_v {
            break;
        }
        if visited.contains(&next) {
            return None;
        }

        prev = Some(curr);
        curr = next;
    }

    if current_cycle.len() != total_merged_v {
        return None;
    }

    Some(current_cycle)
}
