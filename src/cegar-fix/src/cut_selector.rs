use std::collections::HashSet;
use rustsat::types::Clause;
use crate::graph::Graph;
use crate::encoder::Encoder;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CutSelectorOptions {
    pub max_cycle_len_threshold: usize, // Default: 64
    pub base_max_cuts: usize,           // Default: 40
    pub high_volume_max_cuts: usize,    // Default: 100
    pub tiny_cycle_boundary_len: usize, // Default: 8
    pub high_volume_cycle_len: usize,   // Default: 16
    pub high_volume_threshold: usize,   // Default: 30
}

impl Default for CutSelectorOptions {
    fn default() -> Self {
        Self {
            max_cycle_len_threshold: 64,
            base_max_cuts: 40,
            high_volume_max_cuts: 100,
            tiny_cycle_boundary_len: 8,
            high_volume_cycle_len: 16,
            high_volume_threshold: 30,
        }
    }
}

pub struct CutSelector;

impl CutSelector {
    /// Selects, ranks, and dynamically budgets subcycles, then generates direct blocking and boundary cut clauses.
    pub fn select_and_generate_cuts(
        cycles: &[Vec<i32>],
        g: &Graph,
        encoder: &Encoder,
        options: &CutSelectorOptions,
    ) -> (Vec<Clause>, Vec<Vec<i32>>) {
        // 1. Candidate Filtering:
        // Filter out cycles with length > options.max_cycle_len_threshold or length < 3.
        let mut candidates: Vec<Vec<i32>> = cycles
            .iter()
            .filter(|c| c.len() >= 3 && c.len() <= options.max_cycle_len_threshold)
            .cloned()
            .collect();

        // 2. Dynamic Capacity Calculation:
        // Count candidates with len <= options.high_volume_cycle_len.
        // If candidate count > options.high_volume_threshold, effective_max_cuts = options.high_volume_max_cuts.
        // Otherwise, effective_max_cuts = options.base_max_cuts.
        let short_candidate_count = candidates.iter().filter(|c| c.len() <= options.high_volume_cycle_len).count();
        let effective_max_cuts = if short_candidate_count > options.high_volume_threshold {
            options.high_volume_max_cuts
        } else {
            options.base_max_cuts
        };

        // Sort candidate cycles in ascending order of length (c.len())
        candidates.sort_by_key(|c| c.len());

        // 3. Budget Capping & Priority Selection:
        // Take the first min(candidates.len(), effective_max_cuts) cycles.
        // Fallback: if candidates is empty (all cycles > max_cycle_len_threshold), take the shortest cycle to guarantee progress.
        let selected_cycles: Vec<Vec<i32>> = if candidates.is_empty() {
            cycles
                .iter()
                .filter(|c| c.len() >= 3)
                .min_by_key(|c| c.len())
                .cloned()
                .into_iter()
                .collect()
        } else {
            candidates
                .into_iter()
                .take(effective_max_cuts)
                .collect()
        };

        // 4. Clause Generation:
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

            // Boundary cut for tiny cycles: at least one outgoing edge from the cycle must be traversed
            if options.tiny_cycle_boundary_len > 0 && cycle.len() <= options.tiny_cycle_boundary_len {
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

                if !cut_edges.is_empty() {
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

        // 5. Return (clauses, selected_cycles)
        (clauses, selected_cycles)
    }
}
