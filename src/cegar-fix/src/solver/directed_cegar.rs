use crate::core::graph::Graph;
use crate::core::solver_utils::create_solver_with_deadline;
use rustsat::clause;
use rustsat::instances::{BasicVarManager, ManageVars};
use rustsat::solvers::{Solve, SolverResult};
use rustsat::types::{Clause, Lit, TernaryVal};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

/// Checks if graph `g` is suitable for directed 2-loop CEGAR.
/// Covers:
///   - Pure 3-regular (cubic) graphs: all vertices have degree exactly 3.
///   - Near-cubic graphs: min_degree >= 3, max_degree <= 6.
///     No degree-2 vertices means no series contraction needed.
///     Max degree <= 6 keeps AMO clause count manageable (at most 15 per vertex).
pub fn can_solve_directed_cubic(g: &Graph) -> bool {
    let n = g.adjacency_list.len();
    if n < 4 {
        return false;
    }
    let mut min_deg = usize::MAX;
    let mut max_deg = 0usize;
    for nbrs in g.adjacency_list.values() {
        let d = nbrs.len();
        if d < min_deg { min_deg = d; }
        if d > max_deg { max_deg = d; }
    }
    min_deg >= 3 && max_deg <= 6
}

/// Solves Hamiltonian cycle on 3-regular graph `g` using Directed 2-Loop SAT-CEGAR.
pub fn solve_directed_cubic(g: &Graph, timeout_secs: f64) -> Result<Vec<i32>, String> {
    let deadline = Instant::now() + Duration::from_secs_f64(timeout_secs);
    let mut nodes: Vec<i32> = g.adjacency_list.keys().copied().collect();
    nodes.sort_unstable();
    let n = nodes.len();
    if n < 3 {
        return Err("Graph has fewer than 3 vertices".to_string());
    }

    // Map each directed edge (u, v) where {u, v} in E to a SAT literal
    let mut var_mgr = BasicVarManager::default();
    let mut edge_vars: HashMap<(i32, i32), Lit> = HashMap::new();

    for &u in &nodes {
        if let Some(nbrs) = g.adjacency_list.get(&u) {
            for &v in nbrs {
                let lit = var_mgr.new_var().pos_lit();
                edge_vars.insert((u, v), lit);
            }
        }
    }

    let mut solver = create_solver_with_deadline(deadline);

    // 1. Out-degree == 1 for every vertex u:
    //    \sum_{v in N(u)} x_{(u, v)} = 1
    for &u in &nodes {
        let nbrs = match g.adjacency_list.get(&u) {
            Some(n) => n,
            None => return Err("UNSAT".to_string()),
        };
        let out_lits: Vec<Lit> = nbrs.iter().map(|&v| edge_vars[&(u, v)]).collect();
        if out_lits.is_empty() {
            return Err("UNSAT".to_string());
        }
        // At-least-1
        let _ = solver.add_clause(Clause::from_iter(out_lits.iter().copied()));
        // At-most-1
        for i in 0..out_lits.len() {
            for j in (i + 1)..out_lits.len() {
                let _ = solver.add_clause(clause![!out_lits[i], !out_lits[j]]);
            }
        }
    }

    // 2. In-degree == 1 for every vertex v:
    //    \sum_{u in N(v)} x_{(u, v)} = 1
    for &v in &nodes {
        let nbrs = match g.adjacency_list.get(&v) {
            Some(n) => n,
            None => return Err("UNSAT".to_string()),
        };
        let in_lits: Vec<Lit> = nbrs.iter().map(|&u| edge_vars[&(u, v)]).collect();
        if in_lits.is_empty() {
            return Err("UNSAT".to_string());
        }
        // At-least-1
        let _ = solver.add_clause(Clause::from_iter(in_lits.iter().copied()));
        // At-most-1
        for i in 0..in_lits.len() {
            for j in (i + 1)..in_lits.len() {
                let _ = solver.add_clause(clause![!in_lits[i], !in_lits[j]]);
            }
        }
    }

    // 3. 2-loop prohibition:
    //    For every undirected edge {u, v}, \neg x_{(u, v)} \lor \neg x_{(v, u)}
    for &u in &nodes {
        if let Some(nbrs) = g.adjacency_list.get(&u) {
            for &v in nbrs {
                if u < v {
                    let lit_uv = edge_vars[&(u, v)];
                    let lit_vu = edge_vars[&(v, u)];
                    let _ = solver.add_clause(clause![!lit_uv, !lit_vu]);
                }
            }
        }
    }

    // CEGAR loop
    let mut iter_count = 0;
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
                iter_count += 1;
                let sol = solver
                    .full_solution()
                    .map_err(|e| format!("Failed to get full solution: {:?}", e))?;

                // Map each vertex to its directed successor
                let mut succ: HashMap<i32, i32> = HashMap::with_capacity(n);
                for &u in &nodes {
                    if let Some(nbrs) = g.adjacency_list.get(&u) {
                        for &v in nbrs {
                            let lit = edge_vars[&(u, v)];
                            if sol.lit_value(lit) == TernaryVal::True {
                                succ.insert(u, v);
                                break;
                            }
                        }
                    }
                }

                // Extract directed cycles
                let mut visited = HashSet::with_capacity(n);
                let mut cycles: Vec<Vec<i32>> = Vec::new();

                for &start_u in &nodes {
                    if visited.contains(&start_u) {
                        continue;
                    }
                    let mut cyc = Vec::new();
                    let mut curr = start_u;
                    while !visited.contains(&curr) {
                        visited.insert(curr);
                        cyc.push(curr);
                        match succ.get(&curr) {
                            Some(&nxt) => curr = nxt,
                            None => break,
                        }
                    }
                    if !cyc.is_empty() {
                        cycles.push(cyc);
                    }
                }

                // Termination: single cycle containing all vertices
                if cycles.len() == 1 && cycles[0].len() == n {
                    println!("[directed_cegar] Solved graph in {} iterations!", iter_count);
                    return Ok(cycles.pop().unwrap());
                }

                // Add directed cut clauses: for each cycle C, at least one edge must leave C
                // \bigvee_{u in C, v not in C, (u, v) in E} x_{(u, v)}
                for cyc in &cycles {
                    let cyc_set: HashSet<i32> = cyc.iter().copied().collect();
                    let mut cut_lits = Vec::new();

                    for &u in cyc {
                        if let Some(nbrs) = g.adjacency_list.get(&u) {
                            for &v in nbrs {
                                if !cyc_set.contains(&v) {
                                    if let Some(&lit) = edge_vars.get(&(u, v)) {
                                        cut_lits.push(lit);
                                    }
                                }
                            }
                        }
                    }

                    if cut_lits.is_empty() {
                        return Err("UNSAT".to_string());
                    }
                    let _ = solver.add_clause(Clause::from_iter(cut_lits));
                }
            }
        }
    }
}

/// Checks if graph `g` is suitable for directed path solving.
pub fn can_solve_directed_path(g: &Graph) -> bool {
    let max_deg = g.adjacency_list.values().map(|n| n.len()).max().unwrap_or(0);
    max_deg <= 32
}

/// Solves Hamiltonian path between `port_u` and `port_v` on `g` using Directed 2-Loop SAT-CEGAR.
pub fn solve_directed_path_with_forced_edges(
    g: &Graph,
    port_u: i32,
    port_v: i32,
    timeout_secs: f64,
    forced_edges: &HashSet<(i32, i32)>,
) -> Result<Vec<i32>, String> {
    if port_u == port_v {
        return Err("Path ports must be distinct".to_string());
    }
    if !g.adjacency_list.contains_key(&port_u) || !g.adjacency_list.contains_key(&port_v) {
        return Err("Ports not present in graph".to_string());
    }

    let deadline = Instant::now() + Duration::from_secs_f64(timeout_secs);
    let mut nodes: Vec<i32> = g.adjacency_list.keys().copied().collect();
    nodes.sort_unstable();
    let n = nodes.len();
    if n < 2 {
        return Err("Graph has fewer than 2 vertices".to_string());
    }
    if n == 2 {
        if g.adjacency_list.get(&port_u).map_or(false, |nbrs| nbrs.contains(&port_v)) {
            return Ok(vec![port_u, port_v]);
        } else {
            return Err("UNSAT".to_string());
        }
    }

    // Map each directed edge (a, b) where {a, b} in E to a SAT literal
    let mut var_mgr = BasicVarManager::default();
    let mut edge_vars: HashMap<(i32, i32), Lit> = HashMap::new();

    for &a in &nodes {
        if let Some(nbrs) = g.adjacency_list.get(&a) {
            for &b in nbrs {
                let lit = var_mgr.new_var().pos_lit();
                edge_vars.insert((a, b), lit);
            }
        }
    }

    let mut solver = create_solver_with_deadline(deadline);

    // 1. Out-degree constraints:
    //    For port_v: out-degree == 0
    //    For all other vertices a != port_v: out-degree == 1
    for &a in &nodes {
        let nbrs = match g.adjacency_list.get(&a) {
            Some(n) => n,
            None => return Err("UNSAT".to_string()),
        };
        let out_lits: Vec<Lit> = nbrs.iter().map(|&b| edge_vars[&(a, b)]).collect();
        if a == port_v {
            for &lit in &out_lits {
                let _ = solver.add_clause(clause![!lit]);
            }
        } else {
            if out_lits.is_empty() {
                return Err("UNSAT".to_string());
            }
            let _ = solver.add_clause(Clause::from_iter(out_lits.iter().copied()));
            for i in 0..out_lits.len() {
                for j in (i + 1)..out_lits.len() {
                    let _ = solver.add_clause(clause![!out_lits[i], !out_lits[j]]);
                }
            }
        }
    }

    // 2. In-degree constraints:
    //    For port_u: in-degree == 0
    //    For all other vertices b != port_u: in-degree == 1
    for &b in &nodes {
        let nbrs = match g.adjacency_list.get(&b) {
            Some(n) => n,
            None => return Err("UNSAT".to_string()),
        };
        let in_lits: Vec<Lit> = nbrs.iter().map(|&a| edge_vars[&(a, b)]).collect();
        if b == port_u {
            for &lit in &in_lits {
                let _ = solver.add_clause(clause![!lit]);
            }
        } else {
            if in_lits.is_empty() {
                return Err("UNSAT".to_string());
            }
            let _ = solver.add_clause(Clause::from_iter(in_lits.iter().copied()));
            for i in 0..in_lits.len() {
                for j in (i + 1)..in_lits.len() {
                    let _ = solver.add_clause(clause![!in_lits[i], !in_lits[j]]);
                }
            }
        }
    }

    // 3. 2-loop prohibition:
    //    For every undirected edge {a, b}, \neg x_{(a, b)} \lor \neg x_{(b, a)}
    for &a in &nodes {
        if let Some(nbrs) = g.adjacency_list.get(&a) {
            for &b in nbrs {
                if a < b {
                    let lit_ab = edge_vars[&(a, b)];
                    let lit_ba = edge_vars[&(b, a)];
                    let _ = solver.add_clause(clause![!lit_ab, !lit_ba]);
                }
            }
        }
    }

    // 4. Forced edges:
    //    If edge {fa, fb} must be used, at least one of (fa -> fb) or (fb -> fa) must be true.
    for &(fa, fb) in forced_edges {
        if let (Some(&lit_ab), Some(&lit_ba)) = (edge_vars.get(&(fa, fb)), edge_vars.get(&(fb, fa))) {
            let _ = solver.add_clause(clause![lit_ab, lit_ba]);
        }
    }

    // CEGAR loop
    let mut iter_count = 0;
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
                iter_count += 1;
                let sol = solver
                    .full_solution()
                    .map_err(|e| format!("Failed to get full solution: {:?}", e))?;

                // Map each vertex to its directed successor
                let mut succ: HashMap<i32, i32> = HashMap::with_capacity(n);
                for &a in &nodes {
                    if a == port_v {
                        continue;
                    }
                    if let Some(nbrs) = g.adjacency_list.get(&a) {
                        for &b in nbrs {
                            let lit = edge_vars[&(a, b)];
                            if sol.lit_value(lit) == TernaryVal::True {
                                succ.insert(a, b);
                                break;
                            }
                        }
                    }
                }

                // Follow path from port_u
                let mut path = Vec::with_capacity(n);
                let mut curr = port_u;
                let mut on_path = HashSet::with_capacity(n);
                path.push(curr);
                on_path.insert(curr);

                while let Some(&nxt) = succ.get(&curr) {
                    if on_path.contains(&nxt) {
                        break;
                    }
                    curr = nxt;
                    path.push(curr);
                    on_path.insert(curr);
                    if curr == port_v {
                        break;
                    }
                }

                // Termination: single path visiting all vertices and ending at port_v
                if path.len() == n && *path.last().unwrap() == port_v {
                    println!("[directed_cegar] Solved directed path of length {} in {} iterations!", n, iter_count);
                    return Ok(path);
                }

                // Extract disjoint cycles from vertices not on the path
                let mut visited = on_path.clone();
                let mut cycles: Vec<Vec<i32>> = Vec::new();

                for &start_node in &nodes {
                    if visited.contains(&start_node) {
                        continue;
                    }
                    let mut cyc = Vec::new();
                    let mut c = start_node;
                    while !visited.contains(&c) {
                        visited.insert(c);
                        cyc.push(c);
                        match succ.get(&c) {
                            Some(&nxt) => c = nxt,
                            None => break,
                        }
                    }
                    if !cyc.is_empty() {
                        cycles.push(cyc);
                    }
                }

                if cycles.is_empty() {
                    // Path terminated prematurely without visiting all nodes.
                    // Add cut on path vertices (excluding port_v)
                    let mut cut_lits = Vec::new();
                    for &a in &path {
                        if a == port_v {
                            continue;
                        }
                        if let Some(nbrs) = g.adjacency_list.get(&a) {
                            for &b in nbrs {
                                if !on_path.contains(&b) {
                                    if let Some(&lit) = edge_vars.get(&(a, b)) {
                                        cut_lits.push(lit);
                                    }
                                }
                            }
                        }
                    }
                    if cut_lits.is_empty() {
                        return Err("UNSAT".to_string());
                    }
                    let _ = solver.add_clause(Clause::from_iter(cut_lits));
                } else {
                    for cyc in &cycles {
                        let cyc_set: HashSet<i32> = cyc.iter().copied().collect();
                        let mut cut_lits = Vec::new();

                        for &a in cyc {
                            if let Some(nbrs) = g.adjacency_list.get(&a) {
                                for &b in nbrs {
                                    if !cyc_set.contains(&b) {
                                        if let Some(&lit) = edge_vars.get(&(a, b)) {
                                            cut_lits.push(lit);
                                        }
                                    }
                                }
                            }
                        }

                        if cut_lits.is_empty() {
                            return Err("UNSAT".to_string());
                        }
                        let _ = solver.add_clause(Clause::from_iter(cut_lits));
                    }
                }
            }
        }
    }
}

