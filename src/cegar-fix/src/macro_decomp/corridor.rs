use crate::core::graph::Graph;
use crate::core::tour_verifier::TourVerifier;
use crate::fallback::contraction::Degree2Contractor;
use crate::fallback::cycle_merge::safe_2opt_merge;
use rustsat::clause;
use rustsat::instances::{BasicVarManager, ManageVars};
use rustsat::solvers::{Solve, SolverResult};
use rustsat::types::{Clause, Lit};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::time::Instant;


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

    let mut solver = crate::core::solver_utils::create_solver_with_deadline(deadline);
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
            crate::core::encoder::add_at_most_2(&mut solver, &mut var_mgr, &lits);
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
            Ok(SolverResult::Interrupted) => return Err("Block A timeout".to_string()),
            Err(e) => return Err(format!("Block A solver error: {:?}", e)),
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
    contractor: &Degree2Contractor,
    v_b: &HashSet<i32>,
    port_u: i32,
    port_v: i32,
) -> Result<Vec<i32>, String> {
    let mut expanded = contractor.uncontract_cycle(cyc);

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
    let mut adj_b_map: HashMap<i32, HashSet<i32>> = HashMap::new();
    for &u in v_b {
        adj_b_map.insert(u, HashSet::new());
        if let Some(nbrs) = adj_map.get(&u) {
            for &v in nbrs {
                if v_b.contains(&v) {
                    adj_b_map.get_mut(&u).unwrap().insert(v);
                }
            }
        }
    }
    // Add virtual edge
    adj_b_map.get_mut(&port_u).unwrap().insert(port_v);
    adj_b_map.get_mut(&port_v).unwrap().insert(port_u);

    let mut btree_adj = BTreeMap::new();
    let mut hash_adj = HashMap::new();
    let mut arcs = Vec::new();
    for (&u, nbrs) in &adj_b_map {
        let mut sorted: Vec<i32> = nbrs.iter().copied().collect();
        sorted.sort_unstable();
        hash_adj.insert(u, sorted.clone());
        btree_adj.insert(u, sorted.clone());
        for &v in &sorted {
            arcs.push((u, v));
        }
    }
    let graph_b = Graph {
        adjacency_list: hash_adj,
        adjacency_list_btree: btree_adj,
        arcs,
    };

    let mut protected_ports = HashSet::new();
    protected_ports.insert(port_u);
    protected_ports.insert(port_v);

    let (contracted_gb, contractor) = Degree2Contractor::contract_with_protected(&graph_b, &protected_ports);
    if contractor.is_infeasible {
        return Err("Block B degree-2 contraction is infeasible".to_string());
    }
    if let Some(ref direct_cycle) = contractor.is_direct_cycle {
        return format_path_b(direct_cycle, &contractor, v_b, port_u, port_v);
    }

    let rem: HashSet<i32> = contracted_gb.adjacency_list.keys().copied().collect();
    println!("[macro_corridor] Block B contracted: {} -> {} vertices", v_b.len(), rem.len());

    assert!(rem.contains(&port_u), "port_u must be uncontracted");
    assert!(rem.contains(&port_v), "port_v must be uncontracted");

    if rem.len() == 2 {
        let mut path = Vec::with_capacity(v_b.len());
        path.push(port_v);
        if let Some(intermediates) = contractor.chain_map.get(&(port_v, port_u)) {
            path.extend(intermediates);
        }
        path.push(port_u);
        if path.len() == v_b.len() {
            println!("[macro_corridor] Block B solved via degree-2 chain: path len = {}", path.len());
            return Ok(path);
        } else {
            return Err(format!("Block B chain length {} != expected {}", path.len(), v_b.len()));
        }
    }


    let virt_edge = (port_u.min(port_v), port_u.max(port_v));
    let mut forbidden_delete = contractor.forced_edges.clone();
    forbidden_delete.insert(virt_edge);

    let mut edges = HashSet::new();
    let mut adj_b: HashMap<i32, HashSet<i32>> = HashMap::new();
    for (&u, nbrs) in &contracted_gb.adjacency_list {
        adj_b.insert(u, nbrs.iter().copied().collect());
        for &v in nbrs {
            if u < v {
                edges.insert((u, v));
            }
        }
    }
    let mut edge_list: Vec<(i32, i32)> = edges.into_iter().collect();
    edge_list.sort_unstable();

    let mut solver = crate::core::solver_utils::create_solver_with_deadline(deadline);
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
            crate::core::encoder::add_at_most_2(&mut solver, &mut var_mgr, &lits);
        }
    }

    // Force virtual edge to True
    let _ = solver.add_clause(clause![edge_to_var[&virt_edge]]);

    // Force all contracted shortcut edges to True
    for ce in &contractor.forced_edges {
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

    // Static chordless squares (only valid when block size > 4; on 4-vertex blocks,
    // the chordless square IS the Hamiltonian cycle itself).
    if rem_list.len() > 4 {
        if Instant::now() >= deadline {
            return Err("Block B timed out during preprocessing".to_string());
        }
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
    }


    // CEGAR loop with 2-opt absorption
    let mut it = 0;
    while Instant::now() < deadline {
        it += 1;
        match solver.solve() {
            Ok(SolverResult::Sat) => {}
            Ok(SolverResult::Unsat) => return Err("Block B UNSAT".to_string()),
            Ok(SolverResult::Interrupted) => return Err("Block B timeout".to_string()),
            Err(e) => return Err(format!("Block B solver error: {:?}", e)),
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
            return format_path_b(&cycles[0], &contractor, v_b, port_u, port_v);
        }

        // Try 2-opt cycle merge
        if let Some(merged_cyc) = safe_2opt_merge(&cycles, &adj_b, &forbidden_delete) {
            println!("[macro_corridor] 2-opt absorption succeeded at iter {}!", it);
            return format_path_b(&merged_cyc, &contractor, v_b, port_u, port_v);
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
/// exactly 2 non-empty components.
fn check_2cut_split(raw_g: &Graph, port_u: i32, port_v: i32) -> bool {
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

    // MATHEMATICAL REQUIREMENT: By the Chvátal-Erdős toughness theorem,
    // a Hamiltonian graph satisfies c(G \ S) <= |S|. For |S| = 2,
    // exactly 2 components is the only valid case for decomposition.
    comp_sizes.len() == 2 && comp_sizes.iter().all(|&sz| sz >= 1)
}

/// Dynamically discovers a 2-vertex separator {port_u, port_v} whose
/// removal splits the graph into exactly two non-empty components,
/// using Tarjan's linear-time articulation point algorithm on G \ {u}.
///
/// Candidates are sorted by degree ascending for efficiency. The
/// algorithm selects the most balanced 2-cut found (largest minimum
/// component).
pub fn find_2cut_ports(raw_g: &Graph) -> Option<(i32, i32)> {
    let n = raw_g.adjacency_list.len();
    if n < 4 {
        // MATHEMATICAL REQUIREMENT: A 2-vertex separator requires >= 4 vertices.
        return None;
    }

    let mut nodes: Vec<i32> = raw_g.adjacency_list.keys().copied().collect();
    nodes.sort_unstable();

    let node_to_idx: HashMap<i32, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, &node)| (node, i))
        .collect();

    let adj: Vec<Vec<usize>> = nodes
        .iter()
        .map(|&u| {
            raw_g
                .adjacency_list
                .get(&u)
                .map(|nbrs| {
                    nbrs.iter()
                        .filter_map(|nbr| node_to_idx.get(nbr).copied())
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect();

    // Sort candidate indices by degree ascending — low-degree vertices
    // are cheaper to probe and more likely to yield 2-cuts.
    let mut candidates: Vec<usize> = (0..n).collect();
    candidates.sort_by_key(|&u| adj[u].len());

    /// PERFORMANCE KNOB: Minimum corridor size for decomposition to
    /// likely outperform monolithic solving. Does NOT filter candidates
    /// — only controls early exit from the search.
    const MIN_PROFITABLE_CORRIDOR: usize = 10;

    find_2cut_ports_with_deadline(raw_g, None, MIN_PROFITABLE_CORRIDOR)
}

/// Discovers a 2-vertex separator respecting an optional deadline.
pub fn find_2cut_ports_with_deadline(
    raw_g: &Graph,
    deadline: Option<Instant>,
    _min_profitable: usize,
) -> Option<(i32, i32)> {
    let n = raw_g.adjacency_list.len();
    if n < 4 {
        return None;
    }

    let mut nodes: Vec<i32> = raw_g.adjacency_list.keys().copied().collect();
    nodes.sort_unstable();

    let node_to_idx: HashMap<i32, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, &node)| (node, i))
        .collect();

    let adj: Vec<Vec<usize>> = nodes
        .iter()
        .map(|&u| {
            raw_g
                .adjacency_list
                .get(&u)
                .map(|nbrs| {
                    nbrs.iter()
                        .filter_map(|nbr| node_to_idx.get(nbr).copied())
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect();

    let mut candidates: Vec<usize> = (0..n).collect();
    candidates.sort_by_key(|&u| adj[u].len());

    let mut best: Option<(i32, i32, usize)> = None;

    // Reuse DFS scratch buffers across candidates to avoid allocation overhead
    let mut tin = vec![-1i32; n];
    let mut low = vec![-1i32; n];
    let mut sz = vec![0usize; n];
    let mut stack = Vec::with_capacity(n);

    for &u in &candidates {
        if let Some(dl) = deadline {
            if Instant::now() >= dl {
                return best.map(|(u, v, _)| (u, v));
            }
        }

        let deg_u = adj[u].len();
        // In a 2-connected graph, any vertex in a 2-vertex separator has degree >= 2
        if deg_u < 2 {
            continue;
        }

        let root = if u != 0 { 0 } else { 1 };
        tin.fill(-1);
        low.fill(-1);
        sz.fill(0);
        stack.clear();

        tin[u] = 0;
        let mut timer = 1i32;
        tin[root] = timer;
        low[root] = timer;
        sz[root] = 1;

        stack.push((root, usize::MAX, 0usize));

        while let Some(&mut (curr, p, ref mut nbr_idx)) = stack.last_mut() {
            let nbrs = &adj[curr];
            if *nbr_idx < nbrs.len() {
                let to = nbrs[*nbr_idx];
                *nbr_idx += 1;
                if to == p || to == u {
                    continue;
                }
                if tin[to] != -1 {
                    low[curr] = low[curr].min(tin[to]);
                } else {
                    timer += 1;
                    tin[to] = timer;
                    low[to] = timer;
                    sz[to] = 1;
                    stack.push((to, curr, 0));
                }
            } else {
                let (curr, _p, _) = stack.pop().unwrap();
                if let Some(&(parent, _, _)) = stack.last() {
                    low[parent] = low[parent].min(low[curr]);
                    sz[parent] += sz[curr];
                    if low[curr] >= tin[parent] {
                        let comp1 = sz[curr];
                        let comp2 = (n - 1).saturating_sub(comp1);
                        if comp1 >= 1 && comp2 >= 1 {
                            let u_orig = nodes[u];
                            let v_orig = nodes[parent];
                            if check_2cut_split(raw_g, u_orig, v_orig) {
                                let min_comp = comp1.min(comp2);
                                let is_better = best
                                    .map_or(true, |(_, _, prev_min)| min_comp > prev_min);
                                if is_better {
                                    best = Some((
                                        u_orig.min(v_orig),
                                        u_orig.max(v_orig),
                                        min_comp,
                                    ));
                                    // A 50-50 split is theoretically optimal and cannot be improved
                                    if min_comp >= n / 2 {
                                        return best.map(|(u, v, _)| (u, v));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    best.map(|(u, v, _)| (u, v))
}

/// Checks if the graph has a 2-vertex separator splitting it into two large components.
pub fn can_solve_2cut(raw_g: &Graph) -> Option<(i32, i32)> {
    find_2cut_ports(raw_g)
}

/// Solves any 2-connected graph admitting a 2-vertex separator splitting the graph
/// into two components of at least 500 vertices each.
pub fn solve_2cut_corridor(raw_g: &Graph, timeout_secs: f64) -> Option<Vec<i32>> {
    let t_start = Instant::now();
    let deadline = t_start + std::time::Duration::from_secs_f64(timeout_secs);

    let (port_u, port_v) = match find_2cut_ports_with_deadline(raw_g, Some(deadline), 10) {
        Some(ports) => ports,
        None => {
            return None;
        }
    };
    if Instant::now() >= deadline {
        eprintln!("[macro_corridor] Timed out during separator search");
        return None;
    }
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
        "[macro_corridor] Decomposed into Block A ({}v) and Block B ({}v)",
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
            "[macro_corridor] 2-cut corridor tour certified in {:.2}s!",
            t_start.elapsed().as_secs_f64()
        );
        Some(tour)
    } else {
        eprintln!("[macro_corridor] Tour verification failed: {}", err);
        None
    }
}

/// Backward-compatible alias for solve_2cut_corridor.
pub fn solve_710(raw_g: &Graph, timeout_secs: f64) -> Option<Vec<i32>> {
    solve_2cut_corridor(raw_g, timeout_secs)
}
