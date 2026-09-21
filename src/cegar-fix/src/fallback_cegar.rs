use crate::contraction::Degree2Contractor;
use crate::graph::Graph;
use crate::tour_verifier::TourVerifier;
use rustsat::clause;
use rustsat::instances::{BasicVarManager, ManageVars};
use rustsat::solvers::{Solve, SolverResult};
use rustsat::types::{Clause, Lit, TernaryVal};
use rustsat_cadical::CaDiCaL;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

/// Encodes at-most-2 constraint: sum(lits) <= 2
fn add_at_most_2(
    solver: &mut CaDiCaL,
    var_mgr: &mut BasicVarManager,
    lits: &[Lit],
) {
    let n = lits.len();
    if n <= 2 {
        return;
    }
    if n <= 8 {
        for i in 0..n {
            for j in (i + 1)..n {
                for k in (j + 1)..n {
                    let _ = solver.add_clause(clause![!lits[i], !lits[j], !lits[k]]);
                }
            }
        }
        return;
    }

    // Sinz sequential counter for bound k = 2
    let mut s: Vec<Vec<Lit>> = Vec::with_capacity(n - 1);
    for _ in 0..(n - 1) {
        let s0 = var_mgr.new_var().pos_lit();
        let s1 = var_mgr.new_var().pos_lit();
        s.push(vec![s0, s1]);
    }

    // Base clauses (i = 0)
    let _ = solver.add_clause(clause![!lits[0], s[0][0]]);
    let _ = solver.add_clause(clause![!s[0][1]]);

    // Step clauses (i = 1..n-1)
    for i in 1..(n - 1) {
        let _ = solver.add_clause(clause![!s[i - 1][0], s[i][0]]);
        let _ = solver.add_clause(clause![!s[i - 1][1], s[i][1]]);

        let _ = solver.add_clause(clause![!lits[i], s[i][0]]);
        let _ = solver.add_clause(clause![!lits[i], !s[i - 1][0], s[i][1]]);
        let _ = solver.add_clause(clause![!lits[i], !s[i - 1][1]]);
    }

    // Final clause for last literal
    let _ = solver.add_clause(clause![!lits[n - 1], !s[n - 2][1]]);
}

/// Merges two cycles if there exist vertices u1, u2 on c1 and v1, v2 on c2
/// such that deleting (u1, u2) and (v1, v2) and reconnecting forms a single cycle.
/// Never deletes edges present in `forbidden_delete`.
fn merge_two_cycles(
    c1: &[i32],
    c2: &[i32],
    adj: &HashMap<i32, HashSet<i32>>,
    forbidden_delete: &HashSet<(i32, i32)>,
) -> Option<Vec<i32>> {
    let n1 = c1.len();
    let n2 = c2.len();
    if n1 == 0 || n2 == 0 {
        return None;
    }

    for i in 0..n1 {
        let u1 = c1[i];
        let u2 = c1[(i + 1) % n1];
        let e1 = (u1.min(u2), u1.max(u2));
        if forbidden_delete.contains(&e1) {
            continue;
        }

        let u1_nbrs = match adj.get(&u1) {
            Some(s) => s,
            None => continue,
        };
        let u2_nbrs = match adj.get(&u2) {
            Some(s) => s,
            None => continue,
        };

        for j in 0..n2 {
            let v1 = c2[j];
            let v2 = c2[(j + 1) % n2];
            let e2 = (v1.min(v2), v1.max(v2));
            if forbidden_delete.contains(&e2) {
                continue;
            }

            // Case 1: (u1, v1) and (u2, v2)
            if u1_nbrs.contains(&v1) && u2_nbrs.contains(&v2) {
                let mut tour = Vec::with_capacity(n1 + n2);
                for k in 1..=n1 {
                    tour.push(c1[(i + k) % n1]);
                }
                for k in 0..n2 {
                    let idx = (j + n2 - (k % n2)) % n2;
                    tour.push(c2[idx]);
                }
                return Some(tour);
            }

            // Case 2: (u1, v2) and (u2, v1)
            if u1_nbrs.contains(&v2) && u2_nbrs.contains(&v1) {
                let mut tour = Vec::with_capacity(n1 + n2);
                for k in 1..=n1 {
                    tour.push(c1[(i + k) % n1]);
                }
                for k in 0..n2 {
                    tour.push(c2[(j + 1 + k) % n2]);
                }
                return Some(tour);
            }
        }
    }

    None
}

/// Attempts greedy 2-opt pairwise merge on disjoint cycles.
/// Never deletes any edge in `forbidden_delete`.
fn try_fast_2opt_merge(
    cycles: &[Vec<i32>],
    adj: &HashMap<i32, HashSet<i32>>,
    forbidden_delete: &HashSet<(i32, i32)>,
) -> Option<Vec<i32>> {
    let mut curr = cycles.to_vec();
    let mut merged_any = true;

    while merged_any && curr.len() > 1 {
        merged_any = false;
        curr.sort_by(|a, b| b.len().cmp(&a.len()));

        let num_cycles = curr.len();
        let mut merge_step = None;

        'search: for i in 0..num_cycles {
            for j in (i + 1)..num_cycles {
                if let Some(res) = merge_two_cycles(&curr[i], &curr[j], adj, forbidden_delete) {
                    merge_step = Some((i, j, res));
                    break 'search;
                }
            }
        }

        if let Some((i, j, res)) = merge_step {
            curr.remove(j);
            curr[i] = res;
            merged_any = true;
        }
    }

    if curr.len() == 1 {
        Some(curr.into_iter().next().unwrap())
    } else {
        None
    }
}

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
            add_at_most_2(&mut solver, &mut var_mgr, &inc_lits);
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

                // Attempt 2-opt merge when 2 to 4 cycles exist
                if cycles.len() >= 2 && cycles.len() <= 4 {
                    if let Some(merged) =
                        try_fast_2opt_merge(&cycles, &contracted_adj_set, &contractor.forced_edges)
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
