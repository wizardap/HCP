use std::collections::{HashMap, HashSet};
use rustsat::instances::Cnf;
use rustsat::clause;
use crate::graph::Graph;
use crate::encoder::Encoder;

pub struct StaticCycleCutter;

impl StaticCycleCutter {
    /// Statically finds all induced 3-cycles (triangles) and 4-cycles (squares) in graph G
    /// and generates directional subtour elimination clauses.
    pub fn generate_static_small_cycle_cuts(
        g: &Graph,
        encoder: &Encoder,
    ) -> Cnf {
        let mut cnf = Cnf::new();
        let total_v = g.adjacency_list.len();
        if total_v <= 4 {
            return cnf; // Do not block full tour if graph itself is <= 4 vertices
        }

        // Build adjacency sets
        let adj_sets: HashMap<i32, HashSet<i32>> = g.adjacency_list
            .iter()
            .map(|(&u, nbrs)| (u, nbrs.iter().copied().collect()))
            .collect();

        // 1. Find all 3-cycles (triangles)
        let mut vertices: Vec<i32> = g.adjacency_list.keys().copied().collect();
        vertices.sort_unstable();

        for &u in &vertices {
            if let Some(u_nbrs) = g.adjacency_list.get(&u) {
                let mut sorted_nbrs = u_nbrs.clone();
                sorted_nbrs.sort_unstable();
                sorted_nbrs.dedup();

                for &v in &sorted_nbrs {
                    if v <= u { continue; }
                    for &w in &sorted_nbrs {
                        if w <= v { continue; }
                        if adj_sets.get(&v).map_or(false, |s| s.contains(&w)) {
                            // Found triangle (u, v, w)
                            // Direction 1: u -> v -> w -> u
                            if let (Some(&l_uv), Some(&l_vw), Some(&l_wu)) = (
                                encoder.graph_lit_map.get(&(u, v)),
                                encoder.graph_lit_map.get(&(v, w)),
                                encoder.graph_lit_map.get(&(w, u)),
                            ) {
                                cnf.add_clause(clause!(!l_uv, !l_vw, !l_wu));
                            }
                            // Direction 2: u -> w -> v -> u
                            if let (Some(&l_uw), Some(&l_wv), Some(&l_vu)) = (
                                encoder.graph_lit_map.get(&(u, w)),
                                encoder.graph_lit_map.get(&(w, v)),
                                encoder.graph_lit_map.get(&(v, u)),
                            ) {
                                cnf.add_clause(clause!(!l_uw, !l_wv, !l_vu));
                            }
                        }
                    }
                }
            }
        }

        // 2. Find all 4-cycles (squares)
        let mut seen_4cycles = HashSet::new();
        for &u in &vertices {
            if let Some(u_nbrs) = g.adjacency_list.get(&u) {
                let mut sorted_nbrs = u_nbrs.clone();
                sorted_nbrs.sort_unstable();
                sorted_nbrs.dedup();

                for i in 0..sorted_nbrs.len() {
                    let v = sorted_nbrs[i];
                    if v <= u { continue; }
                    for j in (i + 1)..sorted_nbrs.len() {
                        let w = sorted_nbrs[j];
                        if w <= u { continue; }
                        // Look for common neighbors x of v and w (where x != u)
                        if let Some(v_nbrs) = g.adjacency_list.get(&v) {
                            let mut sorted_v_nbrs = v_nbrs.clone();
                            sorted_v_nbrs.sort_unstable();
                            sorted_v_nbrs.dedup();

                            for &x in &sorted_v_nbrs {
                                if x == u || x <= u { continue; }
                                if adj_sets.get(&w).map_or(false, |s| s.contains(&x)) {
                                    // Found 4-cycle: (u, v, x, w)
                                    let mut canonical = [u, v, x, w];
                                    canonical.sort_unstable();
                                    if !seen_4cycles.insert(canonical) {
                                        continue;
                                    }

                                    // Direction 1: u -> v -> x -> w -> u
                                    if let (Some(&l_uv), Some(&l_vx), Some(&l_xw), Some(&l_wu)) = (
                                        encoder.graph_lit_map.get(&(u, v)),
                                        encoder.graph_lit_map.get(&(v, x)),
                                        encoder.graph_lit_map.get(&(x, w)),
                                        encoder.graph_lit_map.get(&(w, u)),
                                    ) {
                                        cnf.add_clause(clause!(!l_uv, !l_vx, !l_xw, !l_wu));
                                    }
                                    // Direction 2: u -> w -> x -> v -> u
                                    if let (Some(&l_uw), Some(&l_wx), Some(&l_xv), Some(&l_vu)) = (
                                        encoder.graph_lit_map.get(&(u, w)),
                                        encoder.graph_lit_map.get(&(w, x)),
                                        encoder.graph_lit_map.get(&(x, v)),
                                        encoder.graph_lit_map.get(&(v, u)),
                                    ) {
                                        cnf.add_clause(clause!(!l_uw, !l_wx, !l_xv, !l_vu));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        cnf
    }
}
