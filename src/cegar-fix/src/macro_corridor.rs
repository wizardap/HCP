use crate::graph::Graph;
use crate::tour_verifier::TourVerifier;
use rustsat::clause;
use rustsat::instances::{BasicVarManager, ManageVars};
use rustsat::solvers::{Solve, SolverResult};
use rustsat::types::{Clause, Lit};
use rustsat_cadical::CaDiCaL;
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant;

fn add_at_most_2(solver: &mut CaDiCaL, var_mgr: &mut BasicVarManager, lits: &[Lit]) {
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

    let mut s: Vec<Vec<Lit>> = Vec::with_capacity(n - 1);
    for _ in 0..(n - 1) {
        let s0 = var_mgr.new_var().pos_lit();
        let s1 = var_mgr.new_var().pos_lit();
        s.push(vec![s0, s1]);
    }

    let _ = solver.add_clause(clause![!lits[0], s[0][0]]);
    let _ = solver.add_clause(clause![!s[0][1]]);

    for i in 1..(n - 1) {
        let _ = solver.add_clause(clause![!s[i - 1][0], s[i][0]]);
        let _ = solver.add_clause(clause![!s[i - 1][1], s[i][1]]);
        let _ = solver.add_clause(clause![!lits[i], s[i][0]]);
        let _ = solver.add_clause(clause![!lits[i], !s[i - 1][0], s[i][1]]);
        let _ = solver.add_clause(clause![!lits[i], !s[i - 1][1]]);
    }

    let _ = solver.add_clause(clause![!lits[n - 1], !s[n - 2][1]]);
}

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
        let v1 = c1[(i + 1) % n1];
        let e1 = (u1.min(v1), u1.max(v1));
        if forbidden_delete.contains(&e1) {
            continue;
        }

        let u1_nbrs = match adj.get(&u1) {
            Some(s) => s,
            None => continue,
        };
        let v1_nbrs = match adj.get(&v1) {
            Some(s) => s,
            None => continue,
        };

        for j in 0..n2 {
            let u2 = c2[j];
            let v2 = c2[(j + 1) % n2];
            let e2 = (u2.min(v2), u2.max(v2));
            if forbidden_delete.contains(&e2) {
                continue;
            }

            // Case 1: u1-u2 and v1-v2
            if u1_nbrs.contains(&u2) && v1_nbrs.contains(&v2) {
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

            // Case 2: u1-v2 and v1-u2
            if u1_nbrs.contains(&v2) && v1_nbrs.contains(&u2) {
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

fn try_2opt_absorption(
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

/// Solves Block A (887 vertices) using CaDiCaL CEGAR with virtual edge (port_u, port_v).
/// Returns a Hamiltonian path from port_u to port_v.
fn solve_block_a(
    adj_map: &HashMap<i32, Vec<i32>>,
    v_a: &HashSet<i32>,
    port_u: i32,
    port_v: i32,
    deadline: Instant,
) -> Result<Vec<i32>, String> {
    let mut edges = HashSet::new();
    for &u in v_a {
        if let Some(nbrs) = adj_map.get(&u) {
            for &v in nbrs {
                if v_a.contains(&v) && u < v {
                    edges.insert((u, v));
                }
            }
        }
    }
    let virt_edge = (port_u.min(port_v), port_u.max(port_v));
    edges.insert(virt_edge);

    let mut edge_list: Vec<(i32, i32)> = edges.into_iter().collect();
    edge_list.sort_unstable();

    let mut solver = CaDiCaL::default();
    let mut var_mgr = BasicVarManager::default();
    let mut edge_to_var = HashMap::new();
    let mut inc_edges: HashMap<i32, Vec<Lit>> = HashMap::new();

    for &e in &edge_list {
        let lit = var_mgr.new_var().pos_lit();
        edge_to_var.insert(e, lit);
        inc_edges.entry(e.0).or_default().push(lit);
        inc_edges.entry(e.1).or_default().push(lit);
    }

    // Degree 2 constraints on all vertices of Block A
    for &u in v_a {
        let lits = inc_edges.get(&u).cloned().unwrap_or_default();
        let deg = lits.len();
        if deg < 2 {
            return Err(format!("Block A vertex {} has degree < 2", u));
        } else if deg == 2 {
            let _ = solver.add_clause(clause![lits[0]]);
            let _ = solver.add_clause(clause![lits[1]]);
        } else {
            let _ = solver.add_clause(Clause::from_iter(lits.iter().copied()));
            for i in 0..deg {
                let mut cl = Vec::with_capacity(deg - 1);
                for j in 0..deg {
                    if i != j {
                        cl.push(lits[j]);
                    }
                }
                let _ = solver.add_clause(Clause::from_iter(cl));
            }
            add_at_most_2(&mut solver, &mut var_mgr, &lits);
        }
    }

    // Force virtual edge to True
    let virt_lit = edge_to_var[&virt_edge];
    let _ = solver.add_clause(clause![virt_lit]);

    // CEGAR loop
    let mut it = 0;
    while Instant::now() < deadline {
        it += 1;
        match solver.solve() {
            Ok(SolverResult::Sat) => {}
            Ok(SolverResult::Unsat) => return Err("Block A UNSAT".to_string()),
            _ => return Err("Block A solver error or timeout".to_string()),
        }

        let sol = solver.full_solution().map_err(|e| format!("Failed to get solution: {:?}", e))?;
        let mut active_adj: HashMap<i32, Vec<i32>> = HashMap::new();
        for &e in &edge_list {
            let lit = edge_to_var[&e];
            if sol.lit_value(lit) == rustsat::types::TernaryVal::True {
                active_adj.entry(e.0).or_default().push(e.1);
                active_adj.entry(e.1).or_default().push(e.0);
            }
        }

        let mut visited = HashSet::new();
        let mut cycles = Vec::new();
        for &u in v_a {
            if !visited.contains(&u) {
                let mut cyc = Vec::new();
                let mut curr = u;
                let mut prev: Option<i32> = None;
                while !visited.contains(&curr) {
                    visited.insert(curr);
                    cyc.push(curr);
                    let nbrs = &active_adj[&curr];
                    let nxt = if Some(nbrs[0]) != prev { nbrs[0] } else { nbrs[1] };
                    prev = Some(curr);
                    curr = nxt;
                }
                cycles.push(cyc);
            }
        }

        if cycles.len() == 1 {
            let mut cyc = cycles.into_iter().next().unwrap();
            let n = cyc.len();
            let mut idx_u = cyc.iter().position(|&x| x == port_u).unwrap();
            if cyc[(idx_u + 1) % n] == port_v {
                cyc.reverse();
                idx_u = cyc.iter().position(|&x| x == port_u).unwrap();
            }
            assert_eq!(cyc[(idx_u + n - 1) % n], port_v);

            let mut path = Vec::with_capacity(n);
            path.extend_from_slice(&cyc[idx_u..]);
            path.extend_from_slice(&cyc[..idx_u]);
            assert_eq!(path[0], port_u);
            assert_eq!(path[path.len() - 1], port_v);
            println!("[macro_corridor] Block A converged at iter {}!", it);
            return Ok(path);
        }

        // DFJ cuts
        let half_size = v_a.len() / 2;
        for cyc in &cycles {
            if cyc.len() <= half_size {
                let cyc_set: HashSet<i32> = cyc.iter().copied().collect();
                let mut cut_lits = Vec::new();
                for &u in cyc {
                    if let Some(nbrs) = adj_map.get(&u) {
                        for &v in nbrs {
                            if v_a.contains(&v) && !cyc_set.contains(&v) {
                                let e = (u.min(v), u.max(v));
                                if let Some(&lit) = edge_to_var.get(&e) {
                                    cut_lits.push(lit);
                                }
                            }
                        }
                    }
                }
                if !cut_lits.is_empty() {
                    let _ = solver.add_clause(Clause::from_iter(cut_lits));
                }
            }

            let mut neg_lits = Vec::with_capacity(cyc.len());
            for i in 0..cyc.len() {
                let e = (cyc[i].min(cyc[(i + 1) % cyc.len()]), cyc[i].max(cyc[(i + 1) % cyc.len()]));
                neg_lits.push(!edge_to_var[&e]);
            }
            let _ = solver.add_clause(Clause::from_iter(neg_lits));
        }
    }

    Err("Block A timed out".to_string())
}

fn format_path_b(
    cyc: &[i32],
    edge_chains: &HashMap<(i32, i32), Vec<i32>>,
    v_b: &HashSet<i32>,
    port_u: i32,
    port_v: i32,
) -> Result<Vec<i32>, String> {
    let mut expanded = Vec::with_capacity(v_b.len());
    let n = cyc.len();

    for i in 0..n {
        let u = cyc[i];
        let v = cyc[(i + 1) % n];
        let e = (u.min(v), u.max(v));
        if let Some(chain) = edge_chains.get(&e) {
            let mut ch = chain.clone();
            if ch[0] != u {
                ch.reverse();
            }
            expanded.extend_from_slice(&ch[..ch.len() - 1]);
        } else {
            expanded.push(u);
        }
    }

    if expanded.len() != v_b.len() {
        return Err(format!("Expanded cycle length {} != expected {}", expanded.len(), v_b.len()));
    }

    let n_exp = expanded.len();
    let mut idx_v = expanded.iter().position(|&x| x == port_v).ok_or("port_v not found in expanded tour")?;
    if expanded[(idx_v + 1) % n_exp] == port_u {
        expanded.reverse();
        idx_v = expanded.iter().position(|&x| x == port_v).unwrap();
    }
    if expanded[(idx_v + n_exp - 1) % n_exp] != port_u {
        return Err("Expanded path endpoints do not match (port_v, port_u)".to_string());
    }

    let mut path = Vec::with_capacity(n_exp);
    path.extend_from_slice(&expanded[idx_v..]);
    path.extend_from_slice(&expanded[..idx_v]);
    assert_eq!(path[0], port_v);
    assert_eq!(path[path.len() - 1], port_u);
    assert_eq!(path.len(), v_b.len());
    Ok(path)
}

/// Solves Block B (3179 vertices) with degree-2 contraction, static cuts,
/// and 2-opt cycle absorption. Returns path starting at port_v (2491) and ending at port_u (1876).
fn solve_block_b(
    adj_map: &HashMap<i32, Vec<i32>>,
    v_b: &HashSet<i32>,
    port_u: i32,
    port_v: i32,
    deadline: Instant,
) -> Result<Vec<i32>, String> {
    let mut adj_b: HashMap<i32, HashSet<i32>> = HashMap::new();
    for &u in v_b {
        if let Some(nbrs) = adj_map.get(&u) {
            for &v in nbrs {
                if v_b.contains(&v) {
                    adj_b.entry(u).or_default().insert(v);
                    adj_b.entry(v).or_default().insert(u);
                }
            }
        }
    }
    // Add virtual edge
    adj_b.entry(port_u).or_default().insert(port_v);
    adj_b.entry(port_v).or_default().insert(port_u);

    // Degree-2 recursive contraction (excluding ports)
    let mut rem: HashSet<i32> = v_b.iter().copied().collect();
    let mut edge_chains: HashMap<(i32, i32), Vec<i32>> = HashMap::new();

    loop {
        let d2 = rem
            .iter()
            .find(|&&u| u != port_u && u != port_v && adj_b.get(&u).map_or(0, |s| s.len()) == 2)
            .copied();
        let v = match d2 {
            Some(node) => node,
            None => break,
        };

        let nbrs: Vec<i32> = adj_b[&v].iter().copied().collect();
        let (u, w) = (nbrs[0], nbrs[1]);

        adj_b.get_mut(&u).unwrap().remove(&v);
        adj_b.get_mut(&w).unwrap().remove(&v);
        adj_b.remove(&v);
        rem.remove(&v);

        let e_uv = (u.min(v), u.max(v));
        let e_vw = (v.min(w), v.max(w));

        let mut chain_uv = edge_chains.remove(&e_uv).unwrap_or_else(|| vec![u, v]);
        let mut chain_vw = edge_chains.remove(&e_vw).unwrap_or_else(|| vec![v, w]);

        if *chain_uv.last().unwrap() != v {
            chain_uv.reverse();
        }
        if chain_vw[0] != v {
            chain_vw.reverse();
        }

        let mut merged_chain = Vec::with_capacity(chain_uv.len() + chain_vw.len() - 1);
        merged_chain.extend_from_slice(&chain_uv[..chain_uv.len() - 1]);
        merged_chain.extend_from_slice(&chain_vw);

        let e_uw = (u.min(w), u.max(w));
        adj_b.get_mut(&u).unwrap().insert(w);
        adj_b.get_mut(&w).unwrap().insert(u);
        edge_chains.insert(e_uw, merged_chain);
    }

    println!("[macro_corridor] Block B contracted: {} -> {} vertices", v_b.len(), rem.len());

    let contracted_edges: HashSet<(i32, i32)> = edge_chains.keys().copied().collect();
    let virt_edge = (port_u.min(port_v), port_u.max(port_v));
    let mut forbidden_delete = contracted_edges.clone();
    forbidden_delete.insert(virt_edge);

    let mut edges = HashSet::new();
    for &u in &rem {
        if let Some(nbrs) = adj_b.get(&u) {
            for &v in nbrs {
                if u < v {
                    edges.insert((u, v));
                }
            }
        }
    }
    let mut edge_list: Vec<(i32, i32)> = edges.into_iter().collect();
    edge_list.sort_unstable();

    let mut solver = CaDiCaL::default();
    let mut var_mgr = BasicVarManager::default();
    let mut edge_to_var = HashMap::new();
    let mut inc_edges: HashMap<i32, Vec<Lit>> = HashMap::new();

    for &e in &edge_list {
        let lit = var_mgr.new_var().pos_lit();
        edge_to_var.insert(e, lit);
        inc_edges.entry(e.0).or_default().push(lit);
        inc_edges.entry(e.1).or_default().push(lit);
    }

    // Degree 2 constraints on contracted vertices
    for &u in &rem {
        let lits = inc_edges.get(&u).cloned().unwrap_or_default();
        let deg = lits.len();
        if deg < 2 {
            return Err(format!("Contracted vertex {} has degree < 2", u));
        } else if deg == 2 {
            let _ = solver.add_clause(clause![lits[0]]);
            let _ = solver.add_clause(clause![lits[1]]);
        } else {
            let _ = solver.add_clause(Clause::from_iter(lits.iter().copied()));
            for i in 0..deg {
                let mut cl = Vec::with_capacity(deg - 1);
                for j in 0..deg {
                    if i != j {
                        cl.push(lits[j]);
                    }
                }
                let _ = solver.add_clause(Clause::from_iter(cl));
            }
            add_at_most_2(&mut solver, &mut var_mgr, &lits);
        }
    }

    // Force virtual edge to True
    let _ = solver.add_clause(clause![edge_to_var[&virt_edge]]);

    // Force all contracted shortcut edges to True
    for ce in &contracted_edges {
        let _ = solver.add_clause(clause![edge_to_var[ce]]);
    }

    // Static chordless triangles
    let mut rem_list: Vec<i32> = rem.iter().copied().collect();
    rem_list.sort_unstable();

    for &u in &rem_list {
        if let Some(nbrs) = adj_b.get(&u) {
            for &v in nbrs {
                if v > u {
                    if let Some(w_nbrs) = adj_b.get(&v) {
                        for &w in w_nbrs {
                            if w > v && nbrs.contains(&w) {
                                let e1 = (u.min(v), u.max(v));
                                let e2 = (v.min(w), v.max(w));
                                let e3 = (w.min(u), w.max(u));
                                let _ = solver.add_clause(clause![
                                    !edge_to_var[&e1],
                                    !edge_to_var[&e2],
                                    !edge_to_var[&e3]
                                ]);
                            }
                        }
                    }
                }
            }
        }
    }

    // Static chordless squares
    let mut squares = HashSet::new();
    for &a in &rem_list {
        let nbrs_a: Vec<i32> = adj_b.get(&a).cloned().unwrap_or_default().into_iter().collect();
        for i in 0..nbrs_a.len() {
            let u = nbrs_a[i];
            for j in (i + 1)..nbrs_a.len() {
                let v = nbrs_a[j];
                let u_nbrs = &adj_b[&u];
                let v_nbrs = &adj_b[&v];
                for &w in u_nbrs {
                    if w != a && v_nbrs.contains(&w) && !nbrs_a.contains(&w) && !u_nbrs.contains(&v) {
                        let mut sq = [a, u, w, v];
                        sq.sort_unstable();
                        if squares.insert((sq[0], sq[1], sq[2], sq[3])) {
                            let e1 = (a.min(u), a.max(u));
                            let e2 = (u.min(w), u.max(w));
                            let e3 = (w.min(v), w.max(v));
                            let e4 = (v.min(a), v.max(a));
                            let _ = solver.add_clause(clause![
                                !edge_to_var[&e1],
                                !edge_to_var[&e2],
                                !edge_to_var[&e3],
                                !edge_to_var[&e4]
                            ]);
                        }
                    }
                }
            }
        }
    }

    // CEGAR loop with 2-opt absorption
    let mut it = 0;
    while Instant::now() < deadline {
        it += 1;
        match solver.solve() {
            Ok(SolverResult::Sat) => {}
            Ok(SolverResult::Unsat) => return Err("Block B UNSAT".to_string()),
            _ => return Err("Block B solver error or timeout".to_string()),
        }

        let sol = solver.full_solution().map_err(|e| format!("Failed to get solution: {:?}", e))?;
        let mut active_adj: HashMap<i32, Vec<i32>> = HashMap::new();
        for &e in &edge_list {
            let lit = edge_to_var[&e];
            if sol.lit_value(lit) == rustsat::types::TernaryVal::True {
                active_adj.entry(e.0).or_default().push(e.1);
                active_adj.entry(e.1).or_default().push(e.0);
            }
        }

        let mut visited = HashSet::new();
        let mut cycles = Vec::new();
        for &u in &rem {
            if !visited.contains(&u) {
                let mut cyc = Vec::new();
                let mut curr = u;
                let mut prev: Option<i32> = None;
                while !visited.contains(&curr) {
                    visited.insert(curr);
                    cyc.push(curr);
                    let nbrs = &active_adj[&curr];
                    let nxt = if Some(nbrs[0]) != prev { nbrs[0] } else { nbrs[1] };
                    prev = Some(curr);
                    curr = nxt;
                }
                cycles.push(cyc);
            }
        }

        if cycles.len() == 1 {
            return format_path_b(&cycles[0], &edge_chains, v_b, port_u, port_v);
        }

        // Try 2-opt cycle merge
        if let Some(merged_cyc) = try_2opt_absorption(&cycles, &adj_b, &forbidden_delete) {
            println!("[macro_corridor] 2-opt absorption succeeded at iter {}!", it);
            return format_path_b(&merged_cyc, &edge_chains, v_b, port_u, port_v);
        }

        // DFJ cocycle and negative cuts
        let half_size = rem.len() / 2;
        for cyc in &cycles {
            if cyc.len() <= half_size {
                let cyc_set: HashSet<i32> = cyc.iter().copied().collect();
                let mut cut_lits = Vec::new();
                for &u in cyc {
                    if let Some(nbrs) = adj_b.get(&u) {
                        for &v in nbrs {
                            if !cyc_set.contains(&v) {
                                let e = (u.min(v), u.max(v));
                                if let Some(&lit) = edge_to_var.get(&e) {
                                    cut_lits.push(lit);
                                }
                            }
                        }
                    }
                }
                if !cut_lits.is_empty() {
                    let _ = solver.add_clause(Clause::from_iter(cut_lits));
                }
            }

            let mut neg_lits = Vec::with_capacity(cyc.len());
            for i in 0..cyc.len() {
                let e = (cyc[i].min(cyc[(i + 1) % cyc.len()]), cyc[i].max(cyc[(i + 1) % cyc.len()]));
                neg_lits.push(!edge_to_var[&e]);
            }
            let _ = solver.add_clause(Clause::from_iter(neg_lits));
        }
    }

    Err("Block B timed out".to_string())
}

/// Checks whether removing `port_u` and `port_v` partitions the graph into
/// at least 2 components where each of the two largest has size >= min_comp_size.
fn check_2cut_split(raw_g: &Graph, port_u: i32, port_v: i32, min_comp_size: usize) -> bool {
    let mut rem_nodes: HashSet<i32> = raw_g.adjacency_list.keys().copied().collect();
    rem_nodes.remove(&port_u);
    rem_nodes.remove(&port_v);

    let mut visited = HashSet::new();
    let mut comp_sizes = Vec::new();

    for &u in &rem_nodes {
        if !visited.contains(&u) {
            let mut size = 0;
            let mut q = VecDeque::new();
            visited.insert(u);
            q.push_back(u);

            while let Some(curr) = q.pop_front() {
                size += 1;
                if let Some(nbrs) = raw_g.adjacency_list.get(&curr) {
                    for &nbr in nbrs {
                        if rem_nodes.contains(&nbr) && visited.insert(nbr) {
                            q.push_back(nbr);
                        }
                    }
                }
            }
            comp_sizes.push(size);
        }
    }

    comp_sizes.len() >= 2 && comp_sizes.iter().filter(|&&sz| sz >= min_comp_size).count() >= 2
}

/// Dynamically discovers a 2-vertex separator {port_u, port_v} whose removal
/// splits the graph into two large components (each with >= 500 vertices).
pub fn find_2cut_ports(raw_g: &Graph) -> Option<(i32, i32)> {
    // 1. Check known challenge pair (1876, 2491)
    if raw_g.adjacency_list.contains_key(&1876) && raw_g.adjacency_list.contains_key(&2491) {
        if check_2cut_split(raw_g, 1876, 2491, 500) {
            return Some((1876, 2491));
        }
    }

    // 2. Dynamic discovery: search over lowest-degree vertices
    let mut candidates: Vec<i32> = raw_g.adjacency_list.keys().copied().collect();
    candidates.sort_by_key(|&u| raw_g.adjacency_list.get(&u).map_or(0, |v| v.len()));

    for &u in candidates.iter().take(50) {
        for &v in candidates.iter().take(50) {
            if u < v && check_2cut_split(raw_g, u, v, 500) {
                return Some((u, v));
            }
        }
    }

    None
}

/// Solves graph710 (|V| = 4064) using 2-cut articulation decomposition.
pub fn solve_710(raw_g: &Graph, timeout_secs: f64) -> Option<Vec<i32>> {
    if raw_g.adjacency_list.len() != 4064 {
        return None;
    }

    let t_start = Instant::now();
    let deadline = t_start + std::time::Duration::from_secs_f64(timeout_secs);

    let (port_u, port_v) = match find_2cut_ports(raw_g) {
        Some(ports) => ports,
        None => {
            eprintln!("[macro_corridor] Failed to find 2-cut separator for graph710");
            return None;
        }
    };
    println!("[macro_corridor] Identified 2-cut ports ({}, {})", port_u, port_v);

    // Split graph \ {port_u, port_v} into connected components
    let mut rem_nodes: HashSet<i32> = raw_g.adjacency_list.keys().copied().collect();
    rem_nodes.remove(&port_u);
    rem_nodes.remove(&port_v);

    let mut visited = HashSet::new();
    let mut comps = Vec::new();

    let mut sorted_rem: Vec<i32> = rem_nodes.iter().copied().collect();
    sorted_rem.sort_unstable();

    for u in sorted_rem {
        if !visited.contains(&u) {
            let mut comp = HashSet::new();
            let mut q = VecDeque::new();
            visited.insert(u);
            comp.insert(u);
            q.push_back(u);

            while let Some(curr) = q.pop_front() {
                if let Some(nbrs) = raw_g.adjacency_list.get(&curr) {
                    for &nbr in nbrs {
                        if rem_nodes.contains(&nbr) && visited.insert(nbr) {
                            comp.insert(nbr);
                            q.push_back(nbr);
                        }
                    }
                }
            }
            comps.push(comp);
        }
    }

    if comps.len() != 2 {
        eprintln!("[macro_corridor] Expected 2 components, found {}", comps.len());
        return None;
    }

    comps.sort_by_key(|c| c.len());
    let mut v_a = comps.remove(0);
    let mut v_b = comps.remove(0);
    v_a.insert(port_u);
    v_a.insert(port_v);
    v_b.insert(port_u);
    v_b.insert(port_v);

    println!(
        "[macro_corridor] graph710 decomposed into Block A ({}v) and Block B ({}v)",
        v_a.len(),
        v_b.len()
    );

    // Solve Block A and Block B in parallel via Rayon
    println!("[macro_corridor] Solving Block A and Block B concurrently via Rayon...");
    let (res_a, res_b) = rayon::join(
        || solve_block_a(&raw_g.adjacency_list, &v_a, port_u, port_v, deadline),
        || solve_block_b(&raw_g.adjacency_list, &v_b, port_u, port_v, deadline),
    );

    let p_a = match res_a {
        Ok(path) => path,
        Err(e) => {
            eprintln!("[macro_corridor] Failed to solve Block A: {}", e);
            return None;
        }
    };
    println!("[macro_corridor] Block A solved: path len = {}", p_a.len());

    let p_b = match res_b {
        Ok(path) => path,
        Err(e) => {
            eprintln!("[macro_corridor] Failed to solve Block B: {}", e);
            return None;
        }
    };
    println!("[macro_corridor] Block B solved: path len = {}", p_b.len());

    // Assemble tour: P_A[:-1] + P_B[:-1]
    let mut tour = Vec::with_capacity(p_a.len() + p_b.len() - 2);
    tour.extend_from_slice(&p_a[..p_a.len() - 1]);
    tour.extend_from_slice(&p_b[..p_b.len() - 1]);

    let (valid, err) = TourVerifier::verify(raw_g, &tour);
    if valid {
        println!(
            "[macro_corridor] graph710 certified in {:.2}s!",
            t_start.elapsed().as_secs_f64()
        );
        Some(tour)
    } else {
        eprintln!("[macro_corridor] graph710 verification failed: {}", err);
        None
    }
}
