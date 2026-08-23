use std::collections::HashSet;
use rustsat::types::Lit;
use crate::graph::Graph;
use crate::encoder::Encoder;

pub struct BackboneFreezer;

impl BackboneFreezer {
    /// Identifies the internal backbone of the giant cycle and extracts assumption literals.
    /// A vertex u on C_giant is internal if neither u nor its immediate cycle neighbors
    /// have any edges to other subcycles.
    pub fn extract_backbone_assumptions(
        cycles: &[Vec<i32>],
        g: &Graph,
        encoder: &Encoder,
        min_giant_ratio: f64,
    ) -> Vec<Lit> {
        let mut assumptions = Vec::new();
        if cycles.len() < 2 {
            return assumptions;
        }

        let total_v = g.adjacency_list.len();
        let mut max_idx = 0;
        let mut max_len = 0;
        for (i, c) in cycles.iter().enumerate() {
            if c.len() > max_len {
                max_len = c.len();
                max_idx = i;
            }
        }

        // Only freeze if giant cycle contains at least min_giant_ratio of total vertices
        if (max_len as f64) < (total_v as f64) * min_giant_ratio {
            return assumptions;
        }

        let giant_cycle = &cycles[max_idx];
        let n_giant = giant_cycle.len();

        // Collect all external vertices belonging to other subcycles
        let mut external_vertices = HashSet::new();
        for (i, c) in cycles.iter().enumerate() {
            if i != max_idx {
                for &v in c {
                    external_vertices.insert(v);
                }
            }
        }

        // Identify boundary vertices on C_giant (vertices with neighbors in external_vertices)
        let mut is_boundary = vec![false; n_giant];
        for (i, &u) in giant_cycle.iter().enumerate() {
            if let Some(neighbors) = g.adjacency_list.get(&u) {
                for &v in neighbors {
                    if external_vertices.contains(&v) {
                        // Mark u and its immediate cycle neighbors as boundary (safety buffer of 1)
                        is_boundary[i] = true;
                        is_boundary[(i + 1) % n_giant] = true;
                        is_boundary[(i + n_giant - 1) % n_giant] = true;
                        break;
                    }
                }
            }
        }

        // Extract internal backbone directed edges
        for i in 0..n_giant {
            let u = giant_cycle[i];
            let v = giant_cycle[(i + 1) % n_giant];

            // If neither endpoint is a boundary vertex, freeze the directed arc u -> v
            if !is_boundary[i] && !is_boundary[(i + 1) % n_giant] {
                if let Some(&lit) = encoder.graph_lit_map.get(&(u, v)) {
                    assumptions.push(lit);
                }
            }
        }

        assumptions
    }
}
