use crate::core::encoder::add_at_most_2;
use crate::core::graph::Graph;
use crate::core::solver_utils::create_solver_with_deadline;
use crate::core::tour_verifier::TourVerifier;
use crate::fallback::cycle_merge::safe_2opt_merge;
use rustsat::clause;
use rustsat::instances::{BasicVarManager, ManageVars};
use rustsat::solvers::{Solve, SolverResult};
use rustsat::types::{Clause, Lit, TernaryVal};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

/// Extracts disjoint Eulerian cycles from an active adjacency map where every vertex has degree 2.
fn extract_subcycles(nodes: &[i32], active_adj: &HashMap<i32, Vec<i32>>) -> Vec<Vec<i32>> {
    let mut visited = HashSet::new();
    let mut cycles = Vec::new();

    for &start_u in nodes {
        if visited.contains(&start_u) {
            continue;
        }
        let mut cyc = Vec::new();
        let mut curr = start_u;
        let mut prev: Option<i32> = None;

        while !visited.contains(&curr) {
            visited.insert(curr);
            cyc.push(curr);
            let nbrs = match active_adj.get(&curr) {
                Some(n) => n,
                None => break,
            };
            let next = if let Some(p) = prev {
                if nbrs.len() > 1 && nbrs[0] == p {
                    nbrs[1]
                } else if !nbrs.is_empty() {
                    nbrs[0]
                } else {
                    break;
                }
            } else if !nbrs.is_empty() {
                nbrs[0]
            } else {
                break;
            };
            prev = Some(curr);
            curr = next;
        }
        if !cyc.is_empty() {
            cycles.push(cyc);
        }
    }

    cycles
}

/// Solves Hamiltonian cycle on `g` using deterministic SAT-CEGAR with 2-opt merging and DFJ cuts.
pub fn solve_cycle(g: &Graph, timeout_secs: f64) -> Result<Vec<i32>, String> {
    solve_cycle_with_forced_edges(g, timeout_secs, &HashSet::new())
}

/// Solves Hamiltonian cycle on `g` requiring all edges in `forced_edges` to be selected.
pub fn solve_cycle_with_forced_edges(
    g: &Graph,
    timeout_secs: f64,
    forced_edges: &HashSet<(i32, i32)>,
) -> Result<Vec<i32>, String> {
    let deadline = Instant::now() + Duration::from_secs_f64(timeout_secs);
    let mut nodes: Vec<i32> = g.adjacency_list.keys().copied().collect();
    nodes.sort_unstable();
    let n = nodes.len();
    if n < 3 {
        return Err("Graph has fewer than 3 vertices".to_string());
    }

    // Collect deduplicated undirected edges and sorted adjacency sets
    let mut edges: Vec<(i32, i32)> = Vec::new();
    let mut adj_sorted: HashMap<i32, Vec<i32>> = HashMap::new();
    let mut adj_sets: HashMap<i32, HashSet<i32>> = HashMap::new();

    for &u in &nodes {
        let mut neighbors: Vec<i32> = g.adjacency_list[&u].clone();
        neighbors.sort_unstable();
        neighbors.dedup();
        for &w in &neighbors {
            if u < w {
                edges.push((u, w));
            }
        }
        adj_sets.insert(u, neighbors.iter().copied().collect());
        adj_sorted.insert(u, neighbors);
    }
    edges.sort_unstable();

    // Map each undirected edge to a SAT literal
    let mut var_mgr = BasicVarManager::default();
    let mut edge_vars: HashMap<(i32, i32), Lit> = HashMap::new();

    for &(u, v) in &edges {
        let lit = var_mgr.new_var().pos_lit();
        edge_vars.insert((u, v), lit);
        edge_vars.insert((v, u), lit);
    }

    let mut solver = create_solver_with_deadline(deadline);
    let mut base_clauses: Vec<Clause> = Vec::new();

    // Add exact degree-2 constraints for each vertex
    for &u in &nodes {
        let mut inc: Vec<Lit> = Vec::new();
        if let Some(nbrs) = adj_sorted.get(&u) {
            for &v in nbrs {
                if let Some(&lit) = edge_vars.get(&(u, v)) {
                    inc.push(lit);
                }
            }
        }

        let deg = inc.len();
        if deg < 2 {
            return Err("UNSAT".to_string());
        } else if deg == 2 {
            base_clauses.push(clause![inc[0]]);
            base_clauses.push(clause![inc[1]]);
        } else {
            // At-least-1
            base_clauses.push(Clause::from_iter(inc.iter().copied()));

            // At-least-2: for each edge i, clause containing all other incident edges
            for i in 0..deg {
                let mut cl = Vec::with_capacity(deg - 1);
                for j in 0..deg {
                    if i != j {
                        cl.push(inc[j]);
                    }
                }
                base_clauses.push(Clause::from_iter(cl));
            }

            // At-most-2
            if deg <= 8 {
                for i in 0..deg {
                    for j in (i + 1)..deg {
                        for k in (j + 1)..deg {
                            let mut cl = Clause::new();
                            cl.add(!inc[i]);
                            cl.add(!inc[j]);
                            cl.add(!inc[k]);
                            base_clauses.push(cl);
                        }
                    }
                }
            } else {
                add_at_most_2(&mut solver, &mut var_mgr, &inc);
            }
        }
    }

    // Force each specified edge as a unit clause
    for &fe in forced_edges {
        let e = (fe.0.min(fe.1), fe.0.max(fe.1));
        if let Some(&lit) = edge_vars.get(&e) {
            base_clauses.push(clause![lit]);
        }
    }

    for cl in &base_clauses {
        let _ = solver.add_clause_ref(cl);
    }

    let mut forbidden_edges: HashSet<(i32, i32)> = HashSet::new();
    for &fe in forced_edges {
        forbidden_edges.insert((fe.0.min(fe.1), fe.0.max(fe.1)));
    }

    // CEGAR loop with DFJ subcycle cuts and 2-opt merge acceleration
    loop {
        if Instant::now() >= deadline {
            return Err("TIMEOUT".to_string());
        }

        let res = match solver.solve() {
            Ok(r) => r,
            Err(e) => return Err(format!("CaDiCaL error: {:?}", e)),
        };

        match res {
            SolverResult::Unsat => return Err("UNSAT".to_string()),
            SolverResult::Interrupted => return Err("TIMEOUT".to_string()),
            SolverResult::Sat => {
                let sol = solver
                    .full_solution()
                    .map_err(|e| format!("Failed to get full solution: {:?}", e))?;

                let mut active_adj: HashMap<i32, Vec<i32>> = HashMap::new();
                for &v in &nodes {
                    active_adj.insert(v, Vec::new());
                }

                for &(u, v) in &edges {
                    if let Some(&lit) = edge_vars.get(&(u, v)) {
                        if sol.lit_value(lit) == TernaryVal::True {
                            active_adj.entry(u).or_default().push(v);
                            active_adj.entry(v).or_default().push(u);
                        }
                    }
                }

                let cycles = extract_subcycles(&nodes, &active_adj);

                if cycles.len() == 1 && cycles[0].len() == n {
                    let (ok, err) = TourVerifier::verify(g, &cycles[0]);
                    if ok {
                        return Ok(cycles[0].clone());
                    } else {
                        return Err(format!("Tour verification failed: {}", err));
                    }
                }

                // Heuristic 2-opt merge acceleration for 2..=4 cycles
                if cycles.len() >= 2 && cycles.len() <= 4 {
                    if let Some(merged) = safe_2opt_merge(&cycles, &adj_sets, &forbidden_edges) {
                        if merged.len() == n {
                            let (ok, _) = TourVerifier::verify(g, &merged);
                            if ok {
                                return Ok(merged);
                            }
                        }
                    }
                }

                // Add DFJ cuts and cycle blocking clauses
                for cyc in &cycles {
                    if cyc.len() < n {
                        let cyc_set: HashSet<i32> = cyc.iter().copied().collect();
                        let mut cut_lits = Vec::new();
                        for &u in cyc {
                            if let Some(nbrs) = adj_sorted.get(&u) {
                                for &v in nbrs {
                                    if !cyc_set.contains(&v) {
                                        if let Some(&lit) = edge_vars.get(&(u, v)) {
                                            cut_lits.push(lit);
                                        }
                                    }
                                }
                            }
                        }
                        cut_lits.sort_unstable();
                        cut_lits.dedup();
                        if cut_lits.is_empty() {
                            return Err("UNSAT".to_string());
                        }
                        let cl = Clause::from_iter(cut_lits);
                        let _ = solver.add_clause(cl);

                        // Cycle edge blocking clause: \bigvee_{e in C} \neg e
                        let mut block_lits = Vec::new();
                        for i in 0..cyc.len() {
                            let u = cyc[i];
                            let v = cyc[(i + 1) % cyc.len()];
                            if let Some(&lit) = edge_vars.get(&(u, v)) {
                                block_lits.push(!lit);
                            }
                        }
                        if !block_lits.is_empty() {
                            let cl = Clause::from_iter(block_lits);
                            let _ = solver.add_clause(cl);
                        }
                    }
                }
            }
        }
    }
}

/// Solves Hamiltonian path between `port_u` and `port_v` on `g`.
/// Reduces the path problem to Hamiltonian cycle by adding a dummy vertex `w`
/// connected strictly to `port_u` and `port_v`.
pub fn solve_path(
    g: &Graph,
    port_u: i32,
    port_v: i32,
    timeout_secs: f64,
) -> Result<Vec<i32>, String> {
    if port_u == port_v {
        return Err("Path ports must be distinct".to_string());
    }

    // Allocate dummy vertex ID higher than any existing vertex ID
    let max_id = g.adjacency_list.keys().copied().max().unwrap_or(0);
    let dummy_w = max_id + 1;

    let mut augmented_g = g.clone();
    augmented_g.add_edge(dummy_w, port_u);
    augmented_g.add_edge(dummy_w, port_v);

    let cycle = solve_cycle(&augmented_g, timeout_secs)?;

    // Extract path between port_u and port_v excluding dummy_w
    let dummy_pos = cycle
        .iter()
        .position(|&x| x == dummy_w)
        .ok_or_else(|| "Dummy vertex missing from cycle".to_string())?;

    let cycle_len = cycle.len();
    let mut raw_path = Vec::with_capacity(cycle_len - 1);
    for i in 1..cycle_len {
        raw_path.push(cycle[(dummy_pos + i) % cycle_len]);
    }

    // Ensure path starts at port_u and ends at port_v
    if raw_path.first() == Some(&port_u) && raw_path.last() == Some(&port_v) {
        Ok(raw_path)
    } else if raw_path.first() == Some(&port_v) && raw_path.last() == Some(&port_u) {
        raw_path.reverse();
        Ok(raw_path)
    } else {
        Err("Cycle orientation mismatch with specified ports".to_string())
    }
}
