use std::collections::{HashMap, HashSet};
use crate::graph::Graph;
use crate::contraction::Degree2Contractor;
use rustsat::clause;
use rustsat::instances::Cnf;
use rustsat::solvers::{LimitConflicts, Solve, SolverResult};
use rustsat::types::{Clause, Lit, TernaryVal, Var};
use rustsat_cadical::CaDiCaL;

#[inline]
fn min_max(u: i32, v: i32) -> (i32, i32) {
    if u < v {
        (u, v)
    } else {
        (v, u)
    }
}

pub struct MacroCrossoverSplicer;

impl MacroCrossoverSplicer {
    /// Attempts to solve an auxiliary local SAT subproblem on the boundary cross-edges
    /// between 2 <= k <= 6 macro-cycles to find a parity-breaking k-opt swap (k >= 4)
    /// that merges all macro-cycles into a single Hamiltonian cycle, preserving 100% of protected edges.
    pub fn try_crossover_splice(
        cycles: &[Vec<i32>],
        g: &Graph,
        contractor: &Degree2Contractor,
    ) -> Option<Vec<i32>> {
        if cycles.len() < 2 || cycles.len() > 6 {
            return None;
        }

        let k = cycles.len();
        let total_v: usize = cycles.iter().map(|c| c.len()).sum();
        if total_v != g.adjacency_list.len() {
            return None;
        }

        // Canonical set of protected edges
        let canonical_protected: HashSet<(i32, i32)> = contractor.chain_map
            .keys()
            .map(|&(u, v)| min_max(u, v))
            .collect();

        // Map each vertex to its cycle index
        let mut vertex_to_cycle: HashMap<i32, usize> = HashMap::with_capacity(total_v);
        let mut cycle_incident_edges: HashMap<i32, [i32; 2]> = HashMap::with_capacity(total_v);

        for (c_idx, cycle) in cycles.iter().enumerate() {
            let n = cycle.len();
            for pos in 0..n {
                let u = cycle[pos];
                let prev = cycle[(pos + n - 1) % n];
                let next = cycle[(pos + 1) % n];
                vertex_to_cycle.insert(u, c_idx);
                cycle_incident_edges.insert(u, [prev, next]);
            }
        }

        // Identify boundary vertices B: vertices having neighbors in a different cycle
        let mut boundary_vertices: HashSet<i32> = HashSet::new();
        let mut candidate_cross_edges: HashSet<(i32, i32)> = HashSet::new();

        for (&u, nbrs) in &g.adjacency_list {
            let c_u = match vertex_to_cycle.get(&u) {
                Some(&c) => c,
                None => continue,
            };
            for &v in nbrs {
                if let Some(&c_v) = vertex_to_cycle.get(&v) {
                    if c_u != c_v {
                        boundary_vertices.insert(u);
                        boundary_vertices.insert(v);
                        candidate_cross_edges.insert(min_max(u, v));
                    }
                }
            }
        }

        if candidate_cross_edges.is_empty() {
            return None;
        }

        // Collect candidate replaceable cycle edges:
        // Any edge (u, v) in the current cycles connecting boundary vertices that is NOT protected
        let mut candidate_cycle_edges: HashSet<(i32, i32)> = HashSet::new();
        for &u in &boundary_vertices {
            if let Some(&[prev, next]) = cycle_incident_edges.get(&u) {
                for &v in &[prev, next] {
                    let e = min_max(u, v);
                    if !canonical_protected.contains(&e) {
                        candidate_cycle_edges.insert(e);
                    }
                }
            }
        }

        // All active subproblem edges: candidate_cross_edges U candidate_cycle_edges
        let mut subproblem_edges: Vec<(i32, i32)> = Vec::new();
        for &e in &candidate_cross_edges {
            subproblem_edges.push(e);
        }
        for &e in &candidate_cycle_edges {
            subproblem_edges.push(e);
        }
        subproblem_edges.sort_unstable();
        subproblem_edges.dedup();

        // Relevant vertices in the subproblem
        let mut subproblem_vertices: HashSet<i32> = HashSet::new();
        for &(u, v) in &subproblem_edges {
            subproblem_vertices.insert(u);
            subproblem_vertices.insert(v);
        }

        // If subproblem is too large, budget it
        if subproblem_edges.len() > 3000 {
            return None;
        }

        // Build SAT variable mapping for each undirected edge: Var::new(i).pos_lit()
        let mut edge_to_lit: HashMap<(i32, i32), Lit> = HashMap::with_capacity(subproblem_edges.len());
        let mut lit_to_edge: HashMap<Lit, (i32, i32)> = HashMap::with_capacity(subproblem_edges.len());
        for (idx, &e) in subproblem_edges.iter().enumerate() {
            let lit = Var::new(idx as u32).pos_lit();
            edge_to_lit.insert(e, lit);
            lit_to_edge.insert(lit, e);
        }

        let mut cnf = Cnf::new();

        // For each vertex v in subproblem_vertices:
        // Count how many fixed protected / fixed internal edges v has in the 2-factor.
        // In the original cycle, v has 2 incident edges.
        // If an incident edge is not in candidate_cycle_edges, it is FIXED ACTIVE in the tour.
        // Thus, active_incident_subproblem_edges(v) must equal 2 - fixed_count.
        for &v in &subproblem_vertices {
            let [p, n] = cycle_incident_edges[&v];
            let ep = min_max(v, p);
            let en = min_max(v, n);

            let p_fixed = !candidate_cycle_edges.contains(&ep);
            let n_fixed = !candidate_cycle_edges.contains(&en);
            let fixed_count = (p_fixed as usize) + (n_fixed as usize);

            let target_sub_active = match 2usize.checked_sub(fixed_count) {
                Some(t) => t,
                None => continue,
            };

            // Collect all subproblem edges incident to v
            let incident_lits: Vec<Lit> = subproblem_edges
                .iter()
                .filter(|&&(x, y)| x == v || y == v)
                .map(|&e| edge_to_lit[&e])
                .collect();

            if target_sub_active == 0 {
                // All incident subproblem edges must be false
                for &lit in &incident_lits {
                    cnf.add_clause(clause![!lit]);
                }
            } else if target_sub_active == 1 {
                // Exactly one incident subproblem edge must be true:
                // At Least One
                cnf.add_clause(Clause::from_iter(incident_lits.clone()));
                // At Most One
                for i in 0..incident_lits.len() {
                    for j in (i + 1)..incident_lits.len() {
                        cnf.add_clause(clause![!incident_lits[i], !incident_lits[j]]);
                    }
                }
            } else if target_sub_active == 2 {
                // Exactly two incident subproblem edges must be true
                let n_vars = incident_lits.len();
                if n_vars < 2 {
                    return None; // Unsatisfiable degree requirement
                }
                // At most 2: for every triple, not all 3 can be true
                for i in 0..n_vars {
                    for j in (i + 1)..n_vars {
                        for m in (j + 1)..n_vars {
                            cnf.add_clause(clause![!incident_lits[i], !incident_lits[j], !incident_lits[m]]);
                        }
                    }
                }
                // At least 2: for every (n-1) subset, not all can be false
                if n_vars == 2 {
                    cnf.add_clause(clause![incident_lits[0]]);
                    cnf.add_clause(clause![incident_lits[1]]);
                } else {
                    for i in 0..n_vars {
                        let mut cl = Vec::with_capacity(n_vars - 1);
                        for j in 0..n_vars {
                            if j != i {
                                cl.push(incident_lits[j]);
                            }
                        }
                        cnf.add_clause(Clause::from_iter(cl));
                    }
                }
            }
        }

        // Connectivity cuts across macro-cycles:
        // Must choose at least 2 cross-edges for each cycle to ensure it connects and exits
        for c_idx in 0..k {
            let cross_lits: Vec<Lit> = candidate_cross_edges
                .iter()
                .filter(|&&(u, v)| vertex_to_cycle[&u] == c_idx || vertex_to_cycle[&v] == c_idx)
                .map(|&e| edge_to_lit[&e])
                .collect();
            if cross_lits.is_empty() {
                return None;
            }
            // ALO 1 cross edge incident to this cycle
            cnf.add_clause(Clause::from_iter(cross_lits));
        }

        // Must change at least something (cannot pick the original identity)
        let orig_cycle_neg_lits: Vec<Lit> = candidate_cycle_edges
            .iter()
            .map(|&e| !edge_to_lit[&e])
            .collect();
        if !orig_cycle_neg_lits.is_empty() {
            cnf.add_clause(Clause::from_iter(orig_cycle_neg_lits));
        }

        let mut solver = CaDiCaL::default();
        let _ = solver.limit_conflicts(Some(10000));
        if solver.add_cnf_ref(&cnf).is_err() {
            return None;
        }

        let mut round = 0;
        while round < 50 {
            round += 1;
            let res = solver.solve();
            if let Ok(SolverResult::Sat) = res {
                let sol = match solver.full_solution() {
                    Ok(s) => s,
                    Err(_) => return None,
                };

                // Extract active edges from model
                let mut active_graph_adj: HashMap<i32, Vec<i32>> = HashMap::with_capacity(total_v);
                
                // Add fixed internal cycle edges
                for (&u, &[p, n]) in &cycle_incident_edges {
                    let ep = min_max(u, p);
                    let en = min_max(u, n);
                    if !candidate_cycle_edges.contains(&ep) && u < p {
                        active_graph_adj.entry(u).or_default().push(p);
                        active_graph_adj.entry(p).or_default().push(u);
                    }
                    if !candidate_cycle_edges.contains(&en) && u < n {
                        active_graph_adj.entry(u).or_default().push(n);
                        active_graph_adj.entry(n).or_default().push(u);
                    }
                }

                // Add active subproblem edges
                let mut active_sub_edges: Vec<(i32, i32)> = Vec::new();
                for (&lit, &(u, v)) in &lit_to_edge {
                    if sol.lit_value(lit) == TernaryVal::True {
                        active_graph_adj.entry(u).or_default().push(v);
                        active_graph_adj.entry(v).or_default().push(u);
                        active_sub_edges.push((u, v));
                    }
                }

                // Verify 2-regularity and extract components
                let mut visited = HashSet::with_capacity(total_v);
                let mut extracted_cycles = Vec::new();

                for (&start_v, _) in &g.adjacency_list {
                    if !visited.contains(&start_v) {
                        let mut comp = Vec::new();
                        let mut curr = start_v;
                        let mut prev = -1;
                        let mut is_cycle = true;

                        while !visited.contains(&curr) {
                            visited.insert(curr);
                            comp.push(curr);
                            let nbrs = match active_graph_adj.get(&curr) {
                                Some(list) if list.len() == 2 => list,
                                _ => {
                                    is_cycle = false;
                                    break;
                                }
                            };
                            let next = if nbrs[0] == prev { nbrs[1] } else { nbrs[0] };
                            prev = curr;
                            curr = next;
                        }

                        if is_cycle && curr == start_v && comp.len() >= 3 {
                            extracted_cycles.push(comp);
                        } else {
                            // Invalid degree configuration, block this state
                            let block_lits: Vec<Lit> = active_sub_edges
                                .iter()
                                .map(|&e| !edge_to_lit[&e])
                                .collect();
                            solver.add_clause(Clause::from_iter(block_lits)).ok();
                            break;
                        }
                    }
                }

                if extracted_cycles.len() == 1 && extracted_cycles[0].len() == total_v {
                    // Found single Hamiltonian cycle!
                    let tour = extracted_cycles.remove(0);
                    return Some(tour);
                } else if extracted_cycles.len() > 1 && visited.len() == total_v {
                    // 2-factor with multiple cycles: add subtour elimination clause for each subcycle
                    for subc in &extracted_cycles {
                        let sub_edges_in_cycle: Vec<Lit> = subproblem_edges
                            .iter()
                            .filter(|&&(u, v)| {
                                let n = subc.len();
                                (0..n).any(|i| {
                                    let x = subc[i];
                                    let y = subc[(i + 1) % n];
                                    min_max(x, y) == min_max(u, v)
                                })
                            })
                            .map(|&e| !edge_to_lit[&e])
                            .collect();
                        if !sub_edges_in_cycle.is_empty() {
                            solver.add_clause(Clause::from_iter(sub_edges_in_cycle)).ok();
                        }
                    }
                } else {
                    // Block current combination
                    let block_lits: Vec<Lit> = active_sub_edges
                        .iter()
                        .map(|&e| !edge_to_lit[&e])
                        .collect();
                    if block_lits.is_empty() {
                        break;
                    }
                    solver.add_clause(Clause::from_iter(block_lits)).ok();
                }
            } else {
                return None;
            }
        }

        None
    }
}
