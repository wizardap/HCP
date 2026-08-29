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
        for &u in &vertices {
            if let Some(u_nbrs) = g.adjacency_list.get(&u) {
                let mut sorted_nbrs = u_nbrs.clone();
                sorted_nbrs.sort_unstable();
                sorted_nbrs.dedup();

                for i in 0..sorted_nbrs.len() {
                    let v = sorted_nbrs[i];
                    if v <= u { continue; }
                    let sorted_v_nbrs = if let Some(v_nbrs) = g.adjacency_list.get(&v) {
                        let mut s = v_nbrs.clone();
                        s.sort_unstable();
                        s.dedup();
                        s
                    } else {
                        Vec::new()
                    };

                    for j in (i + 1)..sorted_nbrs.len() {
                        let w = sorted_nbrs[j];
                        if w <= u { continue; }
                        // Look for common neighbors x of v and w (where x > u)
                        for &x in &sorted_v_nbrs {
                            if x <= u { continue; }
                            if adj_sets.get(&w).is_some_and(|s| s.contains(&x)) {
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

        // 3. Find all 6-cycles (hexagons)
        if total_v > 6 {
            let mut six_cycle_clauses = 0;
            const MAX_6_CYCLE_CLAUSES: usize = 4000;

            'outer_6: for &u1 in &vertices {
                if let Some(u1_nbrs) = g.adjacency_list.get(&u1) {
                    let mut sorted_nbrs: Vec<i32> = u1_nbrs.iter().copied().filter(|&x| x > u1).collect();
                    sorted_nbrs.sort_unstable();
                    sorted_nbrs.dedup();

                    for i in 0..sorted_nbrs.len() {
                        let u2 = sorted_nbrs[i];
                        let u2_nbrs = if let Some(nbrs) = g.adjacency_list.get(&u2) {
                            let mut s: Vec<i32> = nbrs.iter().copied().filter(|&x| x > u1 && x != u2).collect();
                            s.sort_unstable();
                            s.dedup();
                            s
                        } else {
                            Vec::new()
                        };

                        for j in (i + 1)..sorted_nbrs.len() {
                            let u6 = sorted_nbrs[j]; // u2 < u6
                            let u6_nbrs = if let Some(nbrs) = g.adjacency_list.get(&u6) {
                                let mut s: Vec<i32> = nbrs.iter().copied().filter(|&x| x > u1 && x != u6).collect();
                                s.sort_unstable();
                                s.dedup();
                                s
                            } else {
                                Vec::new()
                            };

                            for &u3 in &u2_nbrs {
                                if u3 == u6 { continue; }
                                let u3_nbrs = if let Some(nbrs) = g.adjacency_list.get(&u3) {
                                    let mut s: Vec<i32> = nbrs.iter().copied().filter(|&x| x > u1 && x != u3).collect();
                                    s.sort_unstable();
                                    s.dedup();
                                    s
                                } else {
                                    Vec::new()
                                };

                                for &u5 in &u6_nbrs {
                                    if u5 == u2 || u5 == u3 { continue; }
                                    let u5_set = match adj_sets.get(&u5) {
                                        Some(s) => s,
                                        None => continue,
                                    };

                                    for &u4 in &u3_nbrs {
                                        if u4 == u2 || u4 == u6 || u4 == u5 { continue; }
                                        if u5_set.contains(&u4) {
                                            // Ensure cycle is strictly induced (no chords)
                                            let u1_set = match adj_sets.get(&u1) { Some(s) => s, None => continue };
                                            if u1_set.contains(&u3) || u1_set.contains(&u4) || u1_set.contains(&u5) { continue; }
                                            let u2_set = match adj_sets.get(&u2) { Some(s) => s, None => continue };
                                            if u2_set.contains(&u4) || u2_set.contains(&u5) || u2_set.contains(&u6) { continue; }
                                            let u3_set = match adj_sets.get(&u3) { Some(s) => s, None => continue };
                                            if u3_set.contains(&u5) || u3_set.contains(&u6) { continue; }
                                            let u4_set = match adj_sets.get(&u4) { Some(s) => s, None => continue };
                                            if u4_set.contains(&u6) { continue; }

                                            // Found strictly induced 6-cycle: u1 - u2 - u3 - u4 - u5 - u6 - u1
                                            // Direction 1: u1 -> u2 -> u3 -> u4 -> u5 -> u6 -> u1
                                            if let (
                                                Some(&l_12),
                                                Some(&l_23),
                                                Some(&l_34),
                                                Some(&l_45),
                                                Some(&l_56),
                                                Some(&l_61),
                                            ) = (
                                                encoder.graph_lit_map.get(&(u1, u2)),
                                                encoder.graph_lit_map.get(&(u2, u3)),
                                                encoder.graph_lit_map.get(&(u3, u4)),
                                                encoder.graph_lit_map.get(&(u4, u5)),
                                                encoder.graph_lit_map.get(&(u5, u6)),
                                                encoder.graph_lit_map.get(&(u6, u1)),
                                            ) {
                                                cnf.add_clause(clause!(!l_12, !l_23, !l_34, !l_45, !l_56, !l_61));
                                                six_cycle_clauses += 1;
                                            }

                                            // Direction 2: u1 -> u6 -> u5 -> u4 -> u3 -> u2 -> u1
                                            if let (
                                                Some(&l_16),
                                                Some(&l_65),
                                                Some(&l_54),
                                                Some(&l_43),
                                                Some(&l_32),
                                                Some(&l_21),
                                            ) = (
                                                encoder.graph_lit_map.get(&(u1, u6)),
                                                encoder.graph_lit_map.get(&(u6, u5)),
                                                encoder.graph_lit_map.get(&(u5, u4)),
                                                encoder.graph_lit_map.get(&(u4, u3)),
                                                encoder.graph_lit_map.get(&(u3, u2)),
                                                encoder.graph_lit_map.get(&(u2, u1)),
                                            ) {
                                                cnf.add_clause(clause!(!l_16, !l_65, !l_54, !l_43, !l_32, !l_21));
                                                six_cycle_clauses += 1;
                                            }

                                            if six_cycle_clauses >= MAX_6_CYCLE_CLAUSES {
                                                break 'outer_6;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 4. Statically extract induced 7-cycles (heptagons)
        const MAX_7_CYCLE_CLAUSES: usize = 4000;
        let mut seven_cycle_clauses = 0;
        if total_v > 7 {
            'outer_7: for &u1 in &vertices {
                let u1_nbrs = if let Some(nbrs) = g.adjacency_list.get(&u1) {
                    let mut s: Vec<i32> = nbrs.iter().copied().filter(|&x| x > u1).collect();
                    s.sort_unstable();
                    s.dedup();
                    s
                } else {
                    Vec::new()
                };

                for i in 0..u1_nbrs.len() {
                    let u2 = u1_nbrs[i];
                    let u2_nbrs = if let Some(nbrs) = g.adjacency_list.get(&u2) {
                        let mut s: Vec<i32> = nbrs.iter().copied().filter(|&x| x > u1 && x != u2).collect();
                        s.sort_unstable();
                        s.dedup();
                        s
                    } else {
                        Vec::new()
                    };

                    for j in (i + 1)..u1_nbrs.len() {
                        let u7 = u1_nbrs[j];
                        let u7_nbrs = if let Some(nbrs) = g.adjacency_list.get(&u7) {
                            let mut s: Vec<i32> = nbrs.iter().copied().filter(|&x| x > u1 && x != u7 && x != u2).collect();
                            s.sort_unstable();
                            s.dedup();
                            s
                        } else {
                            Vec::new()
                        };

                        for &u3 in &u2_nbrs {
                            if u3 == u7 { continue; }
                            let u3_nbrs = if let Some(nbrs) = g.adjacency_list.get(&u3) {
                                let mut s: Vec<i32> = nbrs.iter().copied().filter(|&x| x > u1 && x != u3 && x != u2 && x != u7).collect();
                                s.sort_unstable();
                                s.dedup();
                                s
                            } else {
                                Vec::new()
                            };

                            for &u6 in &u7_nbrs {
                                if u6 == u2 || u6 == u3 { continue; }
                                let u6_nbrs = if let Some(nbrs) = g.adjacency_list.get(&u6) {
                                    let mut s: Vec<i32> = nbrs.iter().copied().filter(|&x| x > u1 && x != u6 && x != u7 && x != u2 && x != u3).collect();
                                    s.sort_unstable();
                                    s.dedup();
                                    s
                                } else {
                                    Vec::new()
                                };

                                for &u4 in &u3_nbrs {
                                    if u4 == u7 || u4 == u6 { continue; }
                                    for &u5 in &u6_nbrs {
                                        if u5 == u2 || u5 == u3 || u5 == u4 { continue; }
                                        let u5_set = match adj_sets.get(&u5) {
                                            Some(s) => s,
                                            None => continue,
                                        };

                                        if u5_set.contains(&u4) {
                                            // Check induced chords for 7-cycle: u1 - u2 - u3 - u4 - u5 - u6 - u7 - u1
                                            let u1_set = match adj_sets.get(&u1) { Some(s) => s, None => continue };
                                            if u1_set.contains(&u3) || u1_set.contains(&u4) || u1_set.contains(&u5) || u1_set.contains(&u6) { continue; }
                                            let u2_set = match adj_sets.get(&u2) { Some(s) => s, None => continue };
                                            if u2_set.contains(&u4) || u2_set.contains(&u5) || u2_set.contains(&u6) || u2_set.contains(&u7) { continue; }
                                            let u3_set = match adj_sets.get(&u3) { Some(s) => s, None => continue };
                                            if u3_set.contains(&u5) || u3_set.contains(&u6) || u3_set.contains(&u7) { continue; }
                                            let u4_set = match adj_sets.get(&u4) { Some(s) => s, None => continue };
                                            if u4_set.contains(&u6) || u4_set.contains(&u7) { continue; }
                                            if u5_set.contains(&u7) { continue; }

                                            // Direction 1: u1 -> u2 -> u3 -> u4 -> u5 -> u6 -> u7 -> u1
                                            if let (
                                                Some(&l_12),
                                                Some(&l_23),
                                                Some(&l_34),
                                                Some(&l_45),
                                                Some(&l_56),
                                                Some(&l_67),
                                                Some(&l_71),
                                            ) = (
                                                encoder.graph_lit_map.get(&(u1, u2)),
                                                encoder.graph_lit_map.get(&(u2, u3)),
                                                encoder.graph_lit_map.get(&(u3, u4)),
                                                encoder.graph_lit_map.get(&(u4, u5)),
                                                encoder.graph_lit_map.get(&(u5, u6)),
                                                encoder.graph_lit_map.get(&(u6, u7)),
                                                encoder.graph_lit_map.get(&(u7, u1)),
                                            ) {
                                                cnf.add_clause(clause!(!l_12, !l_23, !l_34, !l_45, !l_56, !l_67, !l_71));
                                                seven_cycle_clauses += 1;
                                            }

                                            // Direction 2: u1 -> u7 -> u6 -> u5 -> u4 -> u3 -> u2 -> u1
                                            if let (
                                                Some(&l_17),
                                                Some(&l_76),
                                                Some(&l_65),
                                                Some(&l_54),
                                                Some(&l_43),
                                                Some(&l_32),
                                                Some(&l_21),
                                            ) = (
                                                encoder.graph_lit_map.get(&(u1, u7)),
                                                encoder.graph_lit_map.get(&(u7, u6)),
                                                encoder.graph_lit_map.get(&(u6, u5)),
                                                encoder.graph_lit_map.get(&(u5, u4)),
                                                encoder.graph_lit_map.get(&(u4, u3)),
                                                encoder.graph_lit_map.get(&(u3, u2)),
                                                encoder.graph_lit_map.get(&(u2, u1)),
                                            ) {
                                                cnf.add_clause(clause!(!l_17, !l_76, !l_65, !l_54, !l_43, !l_32, !l_21));
                                                seven_cycle_clauses += 1;
                                            }

                                            if seven_cycle_clauses >= MAX_7_CYCLE_CLAUSES {
                                                break 'outer_7;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 5. Statically extract induced 8-cycles (octagons)
        const MAX_8_CYCLE_CLAUSES: usize = 4000;
        let mut eight_cycle_clauses = 0;
        if total_v > 8 {
            'outer_8: for &u1 in &vertices {
                let u1_nbrs = if let Some(nbrs) = g.adjacency_list.get(&u1) {
                    let mut s: Vec<i32> = nbrs.iter().copied().filter(|&x| x > u1).collect();
                    s.sort_unstable();
                    s.dedup();
                    s
                } else {
                    Vec::new()
                };

                for i in 0..u1_nbrs.len() {
                    let u2 = u1_nbrs[i];
                    let u2_nbrs = if let Some(nbrs) = g.adjacency_list.get(&u2) {
                        let mut s: Vec<i32> = nbrs.iter().copied().filter(|&x| x > u1 && x != u2).collect();
                        s.sort_unstable();
                        s.dedup();
                        s
                    } else {
                        Vec::new()
                    };

                    for j in (i + 1)..u1_nbrs.len() {
                        let u8 = u1_nbrs[j];
                        let u8_nbrs = if let Some(nbrs) = g.adjacency_list.get(&u8) {
                            let mut s: Vec<i32> = nbrs.iter().copied().filter(|&x| x > u1 && x != u8 && x != u2).collect();
                            s.sort_unstable();
                            s.dedup();
                            s
                        } else {
                            Vec::new()
                        };

                        for &u3 in &u2_nbrs {
                            if u3 == u8 { continue; }
                            let u3_nbrs = if let Some(nbrs) = g.adjacency_list.get(&u3) {
                                let mut s: Vec<i32> = nbrs.iter().copied().filter(|&x| x > u1 && x != u3 && x != u2 && x != u8).collect();
                                s.sort_unstable();
                                s.dedup();
                                s
                            } else {
                                Vec::new()
                            };

                            for &u7 in &u8_nbrs {
                                if u7 == u2 || u7 == u3 { continue; }
                                let u7_nbrs = if let Some(nbrs) = g.adjacency_list.get(&u7) {
                                    let mut s: Vec<i32> = nbrs.iter().copied().filter(|&x| x > u1 && x != u7 && x != u8 && x != u2 && x != u3).collect();
                                    s.sort_unstable();
                                    s.dedup();
                                    s
                                } else {
                                    Vec::new()
                                };

                                for &u4 in &u3_nbrs {
                                    if u4 == u8 || u4 == u7 { continue; }
                                    let u4_nbrs = if let Some(nbrs) = g.adjacency_list.get(&u4) {
                                        let mut s: Vec<i32> = nbrs.iter().copied().filter(|&x| x > u1 && x != u4 && x != u3 && x != u2 && x != u8 && x != u7).collect();
                                        s.sort_unstable();
                                        s.dedup();
                                        s
                                    } else {
                                        Vec::new()
                                    };

                                    for &u6 in &u7_nbrs {
                                        if u6 == u2 || u6 == u3 || u6 == u4 { continue; }
                                        let u6_set = match adj_sets.get(&u6) {
                                            Some(s) => s,
                                            None => continue,
                                        };

                                        for &u5 in &u4_nbrs {
                                            if u5 == u8 || u5 == u7 || u5 == u6 { continue; }
                                            if u6_set.contains(&u5) {
                                                // Check induced chords for 8-cycle: u1 - u2 - u3 - u4 - u5 - u6 - u7 - u8 - u1
                                                let u1_set = match adj_sets.get(&u1) { Some(s) => s, None => continue };
                                                if u1_set.contains(&u3) || u1_set.contains(&u4) || u1_set.contains(&u5) || u1_set.contains(&u6) || u1_set.contains(&u7) { continue; }
                                                let u2_set = match adj_sets.get(&u2) { Some(s) => s, None => continue };
                                                if u2_set.contains(&u4) || u2_set.contains(&u5) || u2_set.contains(&u6) || u2_set.contains(&u7) || u2_set.contains(&u8) { continue; }
                                                let u3_set = match adj_sets.get(&u3) { Some(s) => s, None => continue };
                                                if u3_set.contains(&u5) || u3_set.contains(&u6) || u3_set.contains(&u7) || u3_set.contains(&u8) { continue; }
                                                let u4_set = match adj_sets.get(&u4) { Some(s) => s, None => continue };
                                                if u4_set.contains(&u6) || u4_set.contains(&u7) || u4_set.contains(&u8) { continue; }
                                                let u5_set = match adj_sets.get(&u5) { Some(s) => s, None => continue };
                                                if u5_set.contains(&u7) || u5_set.contains(&u8) { continue; }
                                                if u6_set.contains(&u8) { continue; }

                                                // Direction 1: u1 -> u2 -> u3 -> u4 -> u5 -> u6 -> u7 -> u8 -> u1
                                                if let (
                                                    Some(&l_12),
                                                    Some(&l_23),
                                                    Some(&l_34),
                                                    Some(&l_45),
                                                    Some(&l_56),
                                                    Some(&l_67),
                                                    Some(&l_78),
                                                    Some(&l_81),
                                                ) = (
                                                    encoder.graph_lit_map.get(&(u1, u2)),
                                                    encoder.graph_lit_map.get(&(u2, u3)),
                                                    encoder.graph_lit_map.get(&(u3, u4)),
                                                    encoder.graph_lit_map.get(&(u4, u5)),
                                                    encoder.graph_lit_map.get(&(u5, u6)),
                                                    encoder.graph_lit_map.get(&(u6, u7)),
                                                    encoder.graph_lit_map.get(&(u7, u8)),
                                                    encoder.graph_lit_map.get(&(u8, u1)),
                                                ) {
                                                    cnf.add_clause(clause!(!l_12, !l_23, !l_34, !l_45, !l_56, !l_67, !l_78, !l_81));
                                                    eight_cycle_clauses += 1;
                                                }

                                                // Direction 2: u1 -> u8 -> u7 -> u6 -> u5 -> u4 -> u3 -> u2 -> u1
                                                if let (
                                                    Some(&l_18),
                                                    Some(&l_87),
                                                    Some(&l_76),
                                                    Some(&l_65),
                                                    Some(&l_54),
                                                    Some(&l_43),
                                                    Some(&l_32),
                                                    Some(&l_21),
                                                ) = (
                                                    encoder.graph_lit_map.get(&(u1, u8)),
                                                    encoder.graph_lit_map.get(&(u8, u7)),
                                                    encoder.graph_lit_map.get(&(u7, u6)),
                                                    encoder.graph_lit_map.get(&(u6, u5)),
                                                    encoder.graph_lit_map.get(&(u5, u4)),
                                                    encoder.graph_lit_map.get(&(u4, u3)),
                                                    encoder.graph_lit_map.get(&(u3, u2)),
                                                    encoder.graph_lit_map.get(&(u2, u1)),
                                                ) {
                                                    cnf.add_clause(clause!(!l_18, !l_87, !l_76, !l_65, !l_54, !l_43, !l_32, !l_21));
                                                    eight_cycle_clauses += 1;
                                                }

                                                if eight_cycle_clauses >= MAX_8_CYCLE_CLAUSES {
                                                    break 'outer_8;
                                                }
                                            }
                                        }
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
