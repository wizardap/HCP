use std::collections::HashSet;
use rustsat::types::Clause;
use crate::graph::Graph;
use crate::encoder::Encoder;
use crate::contraction::Degree2Contractor;

pub struct HemisphereSplicer;

impl HemisphereSplicer {
    /// Attempts pairwise 2-opt cross-splicing between macro-cycles (k in 2..=4).
    /// Returns Some(merged_cycles) if any pair was merged.
    pub fn try_direct_splice_all(
        cycles: &[Vec<i32>],
        g: &Graph,
        contractor: &Degree2Contractor,
    ) -> Option<Vec<Vec<i32>>> {
        if cycles.len() < 2 || cycles.len() > 4 {
            return None;
        }

        for a in 0..cycles.len() {
            let ca = &cycles[a];
            let n = ca.len();
            if n < 2 {
                continue;
            }

            for b in (a + 1)..cycles.len() {
                let cb = &cycles[b];
                let m = cb.len();
                if m < 2 {
                    continue;
                }

                for i in 0..n {
                    let u_i = ca[i];
                    let u_next = ca[(i + 1) % n];

                    if contractor.chain_map.contains_key(&(u_i, u_next))
                        || contractor.chain_map.contains_key(&(u_next, u_i))
                    {
                        continue;
                    }

                    for j in 0..m {
                        let v_j = cb[j];
                        let v_next = cb[(j + 1) % m];

                        if contractor.chain_map.contains_key(&(v_j, v_next))
                            || contractor.chain_map.contains_key(&(v_next, v_j))
                        {
                            continue;
                        }

                        // Case A: (u_i, v_j) \in E(G) and (u_{i+1}, v_{j+1}) \in E(G)
                        if is_edge_in_graph(g, u_i, v_j) && is_edge_in_graph(g, u_next, v_next) {
                            let mut merged = Vec::with_capacity(n + m);
                            // Traverse u_{i+1} ... u_i (length n)
                            for k in 1..=n {
                                merged.push(ca[(i + k) % n]);
                            }
                            // Traverse v_j ... v_{j+1} reversed (length m)
                            for k in 0..m {
                                merged.push(cb[(j + m - (k % m)) % m]);
                            }

                            let mut new_cycles = Vec::with_capacity(cycles.len() - 1);
                            for (idx, c) in cycles.iter().enumerate() {
                                if idx != a && idx != b {
                                    new_cycles.push(c.clone());
                                }
                            }
                            new_cycles.push(merged);
                            return Some(new_cycles);
                        }

                        // Case B: (u_i, v_{j+1}) \in E(G) and (u_{i+1}, v_j) \in E(G)
                        if is_edge_in_graph(g, u_i, v_next) && is_edge_in_graph(g, u_next, v_j) {
                            let mut merged = Vec::with_capacity(n + m);
                            // Traverse u_{i+1} ... u_i (length n)
                            for k in 1..=n {
                                merged.push(ca[(i + k) % n]);
                            }
                            // Traverse v_{j+1} ... v_j forward (length m)
                            for k in 1..=m {
                                merged.push(cb[(j + k) % m]);
                            }

                            let mut new_cycles = Vec::with_capacity(cycles.len() - 1);
                            for (idx, c) in cycles.iter().enumerate() {
                                if idx != a && idx != b {
                                    new_cycles.push(c.clone());
                                }
                            }
                            new_cycles.push(merged);
                            return Some(new_cycles);
                        }
                    }
                }
            }
        }

        None
    }

    /// Generates directional bi-partition crossing cuts for each macro-cycle (k in 2..=4):
    /// for each cycle C_i, asserts \/_e\in\delta^+(C_i) x_e and \/_e\in\delta^-(C_i) x_e.
    pub fn generate_hemisphere_crossing_cuts(
        cycles: &[Vec<i32>],
        g: &Graph,
        encoder: &Encoder,
    ) -> Vec<Clause> {
        let mut clauses = Vec::new();
        if cycles.len() < 2 || cycles.len() > 4 {
            return clauses;
        }

        for cycle in cycles {
            if cycle.is_empty() {
                continue;
            }

            let s: HashSet<i32> = cycle.iter().copied().collect();
            let mut out_lits = Vec::new();
            let mut in_lits = Vec::new();

            for &u in &s {
                if let Some(neighbors) = g.adjacency_list.get(&u) {
                    for &v in neighbors {
                        if !s.contains(&v) {
                            // Crossing arc u -> v (outgoing from S)
                            if let Some(&lit) = encoder.graph_lit_map.get(&(u, v)) {
                                out_lits.push(lit);
                            }
                            // Crossing arc v -> u (incoming to S)
                            if let Some(&lit) = encoder.graph_lit_map.get(&(v, u)) {
                                in_lits.push(lit);
                            }
                        }
                    }
                }
            }

            if !out_lits.is_empty() {
                out_lits.sort_unstable();
                out_lits.dedup();
                clauses.push(Clause::from_iter(out_lits));
            }

            if !in_lits.is_empty() {
                in_lits.sort_unstable();
                in_lits.dedup();
                clauses.push(Clause::from_iter(in_lits));
            }
        }

        clauses
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
