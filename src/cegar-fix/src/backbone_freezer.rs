use std::collections::HashSet;
use rustsat::types::Lit;
use crate::graph::Graph;
use crate::encoder::Encoder;
use crate::contraction::Degree2Contractor;

#[derive(Debug, Clone)]
pub struct FreezerOptions {
    pub ratio_threshold: f64,
    pub max_subcycles_trigger: usize,
    pub max_frozen_edges: usize,       // Default: 250
    pub adaptive_relax_time_secs: f64, // Default: 10.0
}

impl Default for FreezerOptions {
    fn default() -> Self {
        Self {
            ratio_threshold: 0.5,
            max_subcycles_trigger: 25,
            max_frozen_edges: 250,
            adaptive_relax_time_secs: 10.0,
        }
    }
}

pub struct BackboneFreezer;

impl BackboneFreezer {
    /// Selects adaptive frozen assumptions with budget capping and dynamic relaxation.
    pub fn select_adaptive_frozen_assumptions(
        cycles: &[Vec<i32>],
        g: &Graph,
        encoder: &Encoder,
        _contractor: &Degree2Contractor,
        opts: &FreezerOptions,
        last_sat_time_secs: f64,
    ) -> Vec<Lit> {
        let total_v = g.adjacency_list.len();
        let min_len = ((total_v as f64) * opts.ratio_threshold).max(3.0) as usize;

        let max_cycle_len = cycles.iter().map(|c| c.len()).max().unwrap_or(0);
        let has_giant = max_cycle_len >= min_len;
        let count_ok = cycles.len() <= opts.max_subcycles_trigger;

        if cycles.len() < 2 || (!has_giant && !count_ok) {
            return Vec::new();
        }

        let effective_max_edges = if last_sat_time_secs >= opts.adaptive_relax_time_secs {
            (opts.max_frozen_edges / 2).max(50)
        } else {
            opts.max_frozen_edges
        };

        if effective_max_edges == 0 {
            return Vec::new();
        }

        let mut candidate_edges = Vec::new();

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
                        candidate_edges.push(lit);
                    }
                }
            }
        }

        let candidate_count = candidate_edges.len();
        if candidate_count <= effective_max_edges {
            candidate_edges
        } else {
            let mut assumptions = Vec::with_capacity(effective_max_edges);
            for i in 0..effective_max_edges {
                let idx = (i * candidate_count) / effective_max_edges;
                assumptions.push(candidate_edges[idx]);
            }
            assumptions
        }
    }

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
        let contractor = Degree2Contractor::new();
        let opts = FreezerOptions {
            ratio_threshold: min_giant_ratio,
            max_subcycles_trigger: max_cycle_count_trigger,
            max_frozen_edges: usize::MAX,
            adaptive_relax_time_secs: f64::MAX,
        };
        Self::select_adaptive_frozen_assumptions(cycles, g, encoder, &contractor, &opts, 0.0)
    }
}
