use std::collections::HashSet;
use rustsat::types::Clause;
use crate::graph::Graph;
use crate::encoder::Encoder;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CutSelectorOptions {
    pub max_cuts_per_round: usize,    // Default: 40
    pub max_cycle_len_for_cut: usize, // Default: 64
    pub small_cycle_threshold: usize, // Default: 8
    pub enable_boundary_cuts: bool,   // Default: true
}

impl Default for CutSelectorOptions {
    fn default() -> Self {
        Self {
            max_cuts_per_round: 40,
            max_cycle_len_for_cut: 64,
            small_cycle_threshold: 8,
            enable_boundary_cuts: true,
        }
    }
}

pub struct CutSelector;

impl CutSelector {
    /// Selects, ranks, and caps subcycles, then generates direct blocking and boundary cut clauses.
    pub fn select_and_generate_cuts(
        cycles: &[Vec<i32>],
        g: &Graph,
        encoder: &Encoder,
        options: &CutSelectorOptions,
    ) -> (Vec<Clause>, Vec<Vec<i32>>) {
        // 1. Candidate Filtering:
        // Filter out cycles with length > options.max_cycle_len_for_cut or length < 3.
        let mut candidates: Vec<Vec<i32>> = cycles
            .iter()
            .filter(|c| c.len() >= 3 && c.len() <= options.max_cycle_len_for_cut)
            .cloned()
            .collect();

        // Sort candidate cycles in ascending order of length (c.len())
        candidates.sort_by_key(|c| c.len());

        // 2. Budget Capping:
        // Take the first min(candidates.len(), options.max_cuts_per_round) cycles
        let selected_cycles: Vec<Vec<i32>> = candidates
            .into_iter()
            .take(options.max_cuts_per_round)
            .collect();

        // 3. Clause Generation:
        let mut clauses = Vec::new();

        for cycle in &selected_cycles {
            let k = cycle.len();
            if k >= 3 {
                // Direct cycle blocking clause: ¬x_{c0->c1} ∨ ¬x_{c1->c2} ∨ ... ∨ ¬x_{ck-1->c0}
                let mut direct_lits = Vec::with_capacity(k);
                let mut all_exist = true;
                for i in 0..k {
                    let u = cycle[i];
                    let v = cycle[(i + 1) % k];
                    if let Some(&lit) = encoder.graph_lit_map.get(&(u, v)) {
                        direct_lits.push(!lit);
                    } else {
                        all_exist = false;
                        break;
                    }
                }
                if all_exist && !direct_lits.is_empty() {
                    clauses.push(Clause::from_iter(direct_lits));
                }
            }

            // Boundary cut for tiny cycles
            if options.enable_boundary_cuts && cycle.len() <= options.small_cycle_threshold {
                let cycle_set: HashSet<i32> = cycle.iter().copied().collect();
                let mut cut_edges = Vec::new();
                let mut seen_edges = HashSet::new();

                for &u in cycle {
                    if let Some(neighbors) = g.adjacency_list.get(&u) {
                        for &v in neighbors {
                            if !cycle_set.contains(&v) && seen_edges.insert((u, v)) {
                                cut_edges.push((u, v));
                            }
                        }
                    }
                }

                // If |delta(C)| == 2: force both cut edges via unit clauses
                // If |delta(C)| > 2: force at least one cut edge via disjunction
                if cut_edges.len() == 2 {
                    for edge in cut_edges {
                        if let Some(&lit) = encoder.graph_lit_map.get(&edge) {
                            clauses.push(Clause::from_iter([lit]));
                        }
                    }
                } else if cut_edges.len() > 2 {
                    let mut boundary_lits = Vec::new();
                    for edge in cut_edges {
                        if let Some(&lit) = encoder.graph_lit_map.get(&edge) {
                            boundary_lits.push(lit);
                        }
                    }
                    if !boundary_lits.is_empty() {
                        clauses.push(Clause::from_iter(boundary_lits));
                    }
                }
            }
        }

        // 4. Return (clauses, selected_cycles)
        (clauses, selected_cycles)
    }
}
