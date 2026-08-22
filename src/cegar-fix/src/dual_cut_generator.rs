use std::collections::HashSet;
use crate::graph::Graph;
use crate::encoder::Encoder;
use crate::staged_subcycle_filter::Subcycle;
use rustsat::types::Clause;

pub struct DualCutGenerator;

impl DualCutGenerator {
    /// Generates direct exclusion clause: \bigvee_{e \in C} \neg x_e
    pub fn generate_direct_exclusion_clause(
        cycle: &Subcycle,
        encoder: &Encoder,
    ) -> Option<Clause> {
        let edges = if !cycle.edges.is_empty() {
            cycle.edges.clone()
        } else if cycle.vertices.len() >= 2 {
            let n = cycle.vertices.len();
            (0..n).map(|i| (cycle.vertices[i], cycle.vertices[(i + 1) % n])).collect()
        } else {
            return None;
        };

        let mut lits = Vec::with_capacity(edges.len());
        for edge in edges {
            if let Some(&lit) = encoder.graph_lit_map.get(&edge) {
                lits.push(!lit);
            } else {
                return None;
            }
        }

        if lits.is_empty() {
            return None;
        }

        lits.sort_unstable();
        lits.dedup();
        Some(Clause::from_iter(lits))
    }

    /// Generates boundary cut clause: \bigvee_{u \in C, v \notin C, (u, v) \in E} x_{u \to v}
    pub fn generate_boundary_cut_clause(
        cycle: &Subcycle,
        g: &Graph,
        encoder: &Encoder,
    ) -> Option<Clause> {
        let vertices: HashSet<i32> = if !cycle.vertices.is_empty() {
            cycle.vertices.iter().copied().collect()
        } else if !cycle.edges.is_empty() {
            cycle.edges.iter().flat_map(|&(u, v)| [u, v]).collect()
        } else {
            return None;
        };

        if vertices.is_empty() {
            return None;
        }

        let mut lits = Vec::new();
        for &u in &vertices {
            if let Some(neighbors) = g.adjacency_list.get(&u) {
                for &v in neighbors {
                    if !vertices.contains(&v) {
                        if let Some(&lit) = encoder.graph_lit_map.get(&(u, v)) {
                            lits.push(lit);
                        }
                    }
                }
            }
        }

        if lits.is_empty() {
            return None;
        }

        lits.sort_unstable();
        lits.dedup();
        Some(Clause::from_iter(lits))
    }

    /// Generates dual cuts combining direct exclusion and boundary cut clauses.
    pub fn generate_dual_cuts(
        cycle: &Subcycle,
        g: &Graph,
        encoder: &Encoder,
    ) -> Vec<Clause> {
        let mut cuts = Vec::with_capacity(2);
        if let Some(direct) = Self::generate_direct_exclusion_clause(cycle, encoder) {
            cuts.push(direct);
        }
        if let Some(boundary) = Self::generate_boundary_cut_clause(cycle, g, encoder) {
            cuts.push(boundary);
        }
        cuts
    }
}
