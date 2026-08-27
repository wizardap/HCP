use std::collections::HashSet;
use rustsat::types::Lit;
use crate::graph::Graph;
use crate::encoder::Encoder;

pub struct BackboneFreezer;

impl BackboneFreezer {
    /// Identifies the internal backbone of all significant subcycles (each with length >= min_giant_ratio * total_v)
    /// and extracts assumption literals for their internal edges.
    /// A vertex u on cycle C is internal if neither u nor its immediate cycle neighbors
    /// have any edges to vertices outside C.
    pub fn extract_backbone_assumptions(
        cycles: &[Vec<i32>],
        g: &Graph,
        encoder: &Encoder,
        min_giant_ratio: f64,
        max_cycle_count_trigger: usize,
    ) -> Vec<Lit> {
        let mut assumptions = Vec::new();
        if cycles.len() < 2 || cycles.len() > max_cycle_count_trigger {
            return assumptions;
        }

        let total_v = g.adjacency_list.len();
        let min_len = ((total_v as f64) * min_giant_ratio).max(3.0) as usize;

        for cycle in cycles.iter() {
            if cycle.len() < min_len {
                continue;
            }

            let n_c = cycle.len();
            let c_vertices: HashSet<i32> = cycle.iter().copied().collect();

            // Identify boundary vertices on this cycle (vertices with neighbors outside this cycle)
            let mut is_boundary = vec![false; n_c];
            for (i, &u) in cycle.iter().enumerate() {
                if let Some(neighbors) = g.adjacency_list.get(&u) {
                    for &v in neighbors {
                        if !c_vertices.contains(&v) {
                            // Mark u and its immediate cycle neighbors as boundary (safety buffer of 1)
                            is_boundary[i] = true;
                            is_boundary[(i + 1) % n_c] = true;
                            is_boundary[(i + n_c - 1) % n_c] = true;
                            break;
                        }
                    }
                }
            }

            // Extract internal backbone directed edges
            for i in 0..n_c {
                let u = cycle[i];
                let v = cycle[(i + 1) % n_c];

                if !is_boundary[i] && !is_boundary[(i + 1) % n_c] {
                    if let Some(&lit) = encoder.graph_lit_map.get(&(u, v)) {
                        assumptions.push(lit);
                    }
                }
            }
        }

        assumptions
    }
}
