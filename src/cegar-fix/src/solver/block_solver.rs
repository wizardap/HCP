use crate::core::encoder::add_at_most_2;
use crate::core::graph::Graph;
use crate::core::solver_utils::create_solver_with_deadline;
use rustsat::clause;
use rustsat::instances::{BasicVarManager, ManageVars};
use rustsat::solvers::{Solve, SolverResult};
use rustsat::types::{Clause, Lit, TernaryVal};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

/// Extracts disjoint cycles from an active adjacency map where every vertex has degree 2.
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

/// Solves Hamiltonian cycle on `g` using standardized SAT-CEGAR with exact degree-2 clauses
/// and DFJ subcycle elimination cuts.
pub fn solve_hamiltonian_cycle(g: &Graph, timeout_secs: f64) -> Result<Vec<i32>, String> {
    let deadline = Instant::now() + Duration::from_secs_f64(timeout_secs);
    let nodes: Vec<i32> = g.adjacency_list.keys().copied().collect();
    let n = nodes.len();
    if n < 3 {
        return Err("Graph has fewer than 3 vertices".to_string());
    }

    // Collect deduplicated undirected edges and adjacency sets
    let mut edges: Vec<(i32, i32)> = Vec::new();
    let mut adj_set: HashMap<i32, HashSet<i32>> = HashMap::new();

    for (&u, neighbors) in &g.adjacency_list {
        let set: HashSet<i32> = neighbors.iter().copied().collect();
        for &w in &set {
            if u < w {
                edges.push((u, w));
            }
        }
        adj_set.insert(u, set);
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

    // Add degree-2 constraints for each vertex
    for &u in &nodes {
        let mut inc: Vec<Lit> = Vec::new();
        if let Some(nbrs) = adj_set.get(&u) {
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
            let _ = solver.add_clause(clause![inc[0]]);
            let _ = solver.add_clause(clause![inc[1]]);
        } else {
            // At-least-1
            let _ = solver.add_clause(Clause::from_iter(inc.iter().copied()));

            // At-least-2: for each edge i, clause containing all other incident edges
            for i in 0..deg {
                let mut cl = Vec::with_capacity(deg - 1);
                for j in 0..deg {
                    if i != j {
                        cl.push(inc[j]);
                    }
                }
                let _ = solver.add_clause(Clause::from_iter(cl));
            }

            // At-most-2
            add_at_most_2(&mut solver, &mut var_mgr, &inc);
        }
    }

    // CEGAR loop with DFJ subcycle cuts
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
                    return Ok(cycles[0].clone());
                }

                // Add DFJ cuts and cycle blocking clauses
                for cyc in &cycles {
                    if cyc.len() < n {
                        let cyc_set: HashSet<i32> = cyc.iter().copied().collect();
                        let mut cut_lits = Vec::new();
                        for &u in cyc {
                            if let Some(nbrs) = adj_set.get(&u) {
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
                        let _ = solver.add_clause(Clause::from_iter(cut_lits));

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
                            let _ = solver.add_clause(Clause::from_iter(block_lits));
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
pub fn solve_hamiltonian_path(
    g: &Graph,
    port_u: i32,
    port_v: i32,
    timeout_secs: f64,
) -> Result<Vec<i32>, String> {
    if !g.adjacency_list.contains_key(&port_u) || !g.adjacency_list.contains_key(&port_v) {
        return Err("Port not in graph".to_string());
    }

    let n = g.adjacency_list.len();
    if port_u == port_v {
        if n == 1 {
            return Ok(vec![port_u]);
        }
        return Err("Infeasible: port_u == port_v with n > 1".to_string());
    }

    if n == 2 {
        if let Some(nbrs) = g.adjacency_list.get(&port_u) {
            if nbrs.contains(&port_v) {
                return Ok(vec![port_u, port_v]);
            }
        }
        return Err("UNSAT".to_string());
    }

    // For n >= 3, add dummy vertex w connected ONLY to port_u and port_v.
    // Any Hamiltonian cycle in G \cup {w} must use (port_u, w) and (port_v, w)
    // because deg(w) = 2. Deleting w yields a Hamiltonian path between port_u and port_v.
    let mut dummy_w = g.adjacency_list.keys().max().copied().unwrap_or(0).wrapping_add(1);
    while g.adjacency_list.contains_key(&dummy_w) {
        dummy_w = dummy_w.wrapping_add(1);
    }

    let mut aug_g = g.clone();
    aug_g.add_edge(port_u, dummy_w);
    aug_g.add_edge(port_v, dummy_w);

    let cycle = solve_hamiltonian_cycle(&aug_g, timeout_secs)?;
    let w_idx = cycle
        .iter()
        .position(|&x| x == dummy_w)
        .ok_or_else(|| "Dummy vertex missing from cycle".to_string())?;

    let len = cycle.len();
    let mut path = Vec::with_capacity(n);
    for i in 1..len {
        path.push(cycle[(w_idx + i) % len]);
    }

    if path.first() == Some(&port_u) {
        Ok(path)
    } else if path.first() == Some(&port_v) {
        path.reverse();
        Ok(path)
    } else {
        Err("Cycle did not connect ports through dummy vertex".to_string())
    }
}
