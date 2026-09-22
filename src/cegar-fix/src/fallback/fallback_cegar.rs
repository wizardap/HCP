use crate::core::graph::Graph;
use crate::core::tour_verifier::TourVerifier;
use crate::fallback::contraction::Degree2Contractor;
use crate::fallback::cycle_merge::safe_2opt_merge;
use rustsat::clause;
use rustsat::instances::{BasicVarManager, ManageVars};
use rustsat::solvers::{Solve, SolverResult};
use rustsat::types::{Clause, Lit, TernaryVal};
use rustsat_cadical::CaDiCaL;
use std::collections::{HashMap, HashSet};
use std::time::Instant;


/// Pure Rust CDCL CEGAR fallback solver with degree-2 contraction,
/// forced shortcuts, and forbidden-delete 2-opt cycle merger.
pub fn solve_with_contraction(g: &Graph, timeout_secs: f64) -> Result<Vec<i32>, String> {
    let t_start = Instant::now();

    // 1. Contract degree-2 chains
    let (contracted_g, contractor) = Degree2Contractor::contract(g);

    if contractor.is_infeasible {
        return Err("Infeasible".to_string());
    }

    if let Some(cycle) = contractor.is_direct_cycle {
        let (ok, err) = TourVerifier::verify(g, &cycle);
        if ok {
            return Ok(cycle);
        } else {
            return Err(format!("Direct cycle verification failed: {}", err));
        }
    }

    let contracted_v = contracted_g.adjacency_list.len();
    if contracted_v < 3 {
        if contracted_v == 0 {
            return Ok(Vec::new());
        }
        return Err("Infeasible: contracted graph has fewer than 3 vertices".to_string());
    }

    // 2. Build unique undirected edges and adjacency sets
    let mut edges: Vec<(i32, i32)> = Vec::new();
    let mut contracted_adj_set: HashMap<i32, HashSet<i32>> = HashMap::new();

    for (&u, neighbors) in &contracted_g.adjacency_list {
        let set: HashSet<i32> = neighbors.iter().copied().collect();
        for &w in &set {
            if u < w {
                edges.push((u, w));
            }
        }
        contracted_adj_set.insert(u, set);
    }
    edges.sort_unstable();

    // 3. Assign boolean variable to each undirected edge
    let mut solver = CaDiCaL::default();
    let mut var_mgr = BasicVarManager::default();
    let mut edge_vars: HashMap<(i32, i32), Lit> = HashMap::new();

    for &e in &edges {
        let lit = var_mgr.new_var().pos_lit();
        edge_vars.insert(e, lit);
    }

    // 4. Add degree-2 constraints for every vertex in the contracted graph
    for (&u, nbrs) in &contracted_g.adjacency_list {
        let mut inc_lits: Vec<Lit> = Vec::with_capacity(nbrs.len());
        for &w in nbrs {
            let e = (u.min(w), u.max(w));
            if let Some(&lit) = edge_vars.get(&e) {
                inc_lits.push(lit);
            }
        }

        let deg = inc_lits.len();
        if deg < 2 {
            return Err(format!("Infeasible: vertex {} degree < 2", u));
        } else if deg == 2 {
            let _ = solver.add_clause(clause![inc_lits[0]]);
            let _ = solver.add_clause(clause![inc_lits[1]]);
        } else {
            // At-least-1
            let _ = solver.add_clause(Clause::from_iter(inc_lits.iter().copied()));

            // At-least-2: for each neighbor i, clause containing all other incident edges
            for i in 0..deg {
                let mut cl = Vec::with_capacity(deg - 1);
                for j in 0..deg {
                    if i != j {
                        cl.push(inc_lits[j]);
                    }
                }
                let _ = solver.add_clause(Clause::from_iter(cl));
            }

            // At-most-2
            crate::core::encoder::add_at_most_2(&mut solver, &mut var_mgr, &inc_lits);
        }
    }

    // 5. Force each shortcut edge as a unit clause in CaDiCaL
    for &fe in &contractor.forced_edges {
        let e = (fe.0.min(fe.1), fe.0.max(fe.1));
        if let Some(&lit) = edge_vars.get(&e) {
            let _ = solver.add_clause(clause![lit]);
        }
    }

    // 6. CEGAR loop with DFJ subcycle cuts and safe 2-opt cycle merger
    let contracted_vertices: Vec<i32> = contracted_g.adjacency_list.keys().copied().collect();

    while t_start.elapsed().as_secs_f64() < timeout_secs {
        let res = match solver.solve() {
            Ok(r) => r,
            Err(e) => return Err(format!("CaDiCaL error: {:?}", e)),
        };

        match res {
            SolverResult::Unsat => return Err("UNSAT".to_string()),
            SolverResult::Interrupted => return Err("Solver interrupted".to_string()),
            SolverResult::Sat => {
                let sol = solver
                    .full_solution()
                    .map_err(|e| format!("Failed to get full solution: {:?}", e))?;

                let mut active_adj: HashMap<i32, Vec<i32>> = HashMap::new();
                for &v in &contracted_vertices {
                    active_adj.insert(v, Vec::new());
                }

                for (&(u, w), &lit) in &edge_vars {
                    if sol.lit_value(lit) == TernaryVal::True {
                        active_adj.entry(u).or_default().push(w);
                        active_adj.entry(w).or_default().push(u);
                    }
                }

                // Extract disjoint Eulerian cycles
                let mut visited = HashSet::new();
                let mut cycles = Vec::new();

                for &start_u in &contracted_vertices {
                    if visited.contains(&start_u) {
                        continue;
                    }
                    let mut cyc = Vec::new();
                    let mut curr = start_u;
                    let mut prev = -1;

                    while !visited.contains(&curr) {
                        visited.insert(curr);
                        cyc.push(curr);
                        let nbrs = match active_adj.get(&curr) {
                            Some(n) => n,
                            None => break,
                        };
                        let next = if !nbrs.is_empty() && nbrs[0] == prev {
                            if nbrs.len() > 1 {
                                nbrs[1]
                            } else {
                                nbrs[0]
                            }
                        } else if !nbrs.is_empty() {
                            nbrs[0]
                        } else {
                            break;
                        };
                        prev = curr;
                        curr = next;
                    }
                    if !cyc.is_empty() {
                        cycles.push(cyc);
                    }
                }

                // Single full tour found
                if cycles.len() == 1 && cycles[0].len() == contracted_v {
                    let expanded_tour = contractor.expand_tour(&cycles[0]);
                    let (ok, err) = TourVerifier::verify(g, &expanded_tour);
                    if ok {
                        return Ok(expanded_tour);
                    } else {
                        return Err(format!("Tour verification failed: {}", err));
                    }
                }

                // Attempt 2-opt merge when 2 to 4 cycles exist.
                // Merging disjoint cycles via 2-opt edge swaps is combinatorial in the number of cycles;
                // bounding the attempt to 2..=4 cycles ensures near-instant heuristic patching when close to a full
                // Hamiltonian tour, while deferring higher cycle counts (>= 5) to CDCL/DFJ cut separation.
                if cycles.len() >= 2 && cycles.len() <= 4 {
                    if let Some(merged) =
                        safe_2opt_merge(&cycles, &contracted_adj_set, &contractor.forced_edges)
                    {
                        if merged.len() == contracted_v {
                            let expanded_tour = contractor.expand_tour(&merged);
                            let (ok, _) = TourVerifier::verify(g, &expanded_tour);
                            if ok {
                                return Ok(expanded_tour);
                            }
                        }
                    }
                }

                // Add DFJ subcycle cuts for all disconnected components
                for cyc in &cycles {
                    if cyc.len() < contracted_v {
                        let cyc_set: HashSet<i32> = cyc.iter().copied().collect();

                        // Cut edges delta(C) >= 1
                        let mut cut_lits = Vec::new();
                        for &u in cyc {
                            if let Some(nbrs) = contracted_adj_set.get(&u) {
                                for &w in nbrs {
                                    if !cyc_set.contains(&w) {
                                        let e = (u.min(w), u.max(w));
                                        if let Some(&lit) = edge_vars.get(&e) {
                                            cut_lits.push(lit);
                                        }
                                    }
                                }
                            }
                        }
                        cut_lits.sort_unstable();
                        cut_lits.dedup();
                        if !cut_lits.is_empty() {
                            let _ = solver.add_clause(Clause::from_iter(cut_lits));
                        }

                        // Cycle edge blocking clause: \bigvee_{e in C} \neg e
                        let mut block_lits = Vec::new();
                        for i in 0..cyc.len() {
                            let u = cyc[i];
                            let w = cyc[(i + 1) % cyc.len()];
                            let e = (u.min(w), u.max(w));
                            if let Some(&lit) = edge_vars.get(&e) {
                                block_lits.push(!lit);
                            }
                        }
                        if !block_lits.is_empty() {
                            let _ = solver.add_clause(Clause::from_iter(block_lits));
                        }
                    }
                }
            }
        }
    }

    Err("Timeout".to_string())
}
