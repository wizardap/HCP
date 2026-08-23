use std::collections::HashSet;
use rustsat::types::Clause;
use crate::graph::Graph;
use crate::encoder::Encoder;

pub struct BridgeCutGenerator;

impl BridgeCutGenerator {
    /// Generates multi-cycle bridge clauses connecting small subcycles to the dominant giant cycle.
    /// For each small subcycle C_s and the giant cycle C_giant:
    /// 1. Identifies all directed boundary arcs E_in = { u -> v | u in C_giant, v in C_s }
    /// 2. Identifies all directed boundary arcs E_out = { v -> u | v in C_s, u in C_giant }
    /// 3. Emits SMT bridge clauses requiring at least one entry arc and one exit arc.
    pub fn generate_bridge_cuts(
        cycles: &[Vec<i32>],
        g: &Graph,
        encoder: &Encoder,
    ) -> Vec<Clause> {
        let mut clauses = Vec::new();
        if cycles.len() < 2 {
            return clauses;
        }

        // Find the giant cycle (largest cycle)
        let mut max_idx = 0;
        let mut max_len = 0;
        for (i, c) in cycles.iter().enumerate() {
            if c.len() > max_len {
                max_len = c.len();
                max_idx = i;
            }
        }

        let giant_cycle = &cycles[max_idx];
        let giant_set: HashSet<i32> = giant_cycle.iter().cloned().collect();

        // For each small cycle, generate bridge connection cuts to the giant cycle
        for (i, small_cycle) in cycles.iter().enumerate() {
            if i == max_idx || small_cycle.is_empty() {
                continue;
            }

            let small_set: HashSet<i32> = small_cycle.iter().cloned().collect();

            let mut in_lits = Vec::new();
            let mut out_lits = Vec::new();

            for &v in &small_set {
                if let Some(neighbors) = g.adjacency_list.get(&v) {
                    for &u in neighbors {
                        if giant_set.contains(&u) {
                            // Arc u (giant) -> v (small)
                            if let Some(&lit_in) = encoder.graph_lit_map.get(&(u, v)) {
                                in_lits.push(lit_in);
                            }
                            // Arc v (small) -> u (giant)
                            if let Some(&lit_out) = encoder.graph_lit_map.get(&(v, u)) {
                                out_lits.push(lit_out);
                            }
                        }
                    }
                }
            }

            // If direct bridge edges exist between C_giant and C_s:
            if !in_lits.is_empty() {
                in_lits.sort_unstable();
                in_lits.dedup();
                clauses.push(Clause::from_iter(in_lits));
            }

            if !out_lits.is_empty() {
                out_lits.sort_unstable();
                out_lits.dedup();
                clauses.push(Clause::from_iter(out_lits));
            }
        }

        clauses
    }
}
