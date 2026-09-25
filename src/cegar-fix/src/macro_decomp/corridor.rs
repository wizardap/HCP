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


fn format_corridor_path(
    cyc: &[i32],
    contractor: &Degree2Contractor,
    expected_len: usize,
    start_port: i32,
    end_port: i32,
) -> Result<Vec<i32>, String> {
    let mut expanded = contractor.uncontract_cycle(cyc);

    if expanded.len() != expected_len {
        return Err(format!("Expanded cycle length {} != expected {}", expanded.len(), expected_len));
    }

    let n_exp = expanded.len();
    let mut idx_start = expanded
        .iter()
        .position(|&x| x == start_port)
        .ok_or_else(|| "start_port not found in expanded tour".to_string())?;

    if expanded[(idx_start + 1) % n_exp] == end_port {
        expanded.reverse();
        idx_start = expanded.iter().position(|&x| x == start_port).unwrap();
    }
    if expanded[(idx_start + n_exp - 1) % n_exp] != end_port {
        return Err("Expanded path endpoints do not match (start_port, end_port)".to_string());
    }

    let mut path = Vec::with_capacity(n_exp);
    path.extend_from_slice(&expanded[idx_start..]);
    path.extend_from_slice(&expanded[..idx_start]);
    assert_eq!(path[0], start_port);
    assert_eq!(path[path.len() - 1], end_port);
    assert_eq!(path.len(), expected_len);
    Ok(path)
}

fn solve_corridor_block(
    adj_map: &HashMap<i32, Vec<i32>>,
    block_nodes: &HashSet<i32>,
    start_port: i32,
    end_port: i32,
    block_name: &str,
    deadline: Instant,
) -> Result<Vec<i32>, String> {
    let mut block_adj_map: HashMap<i32, HashSet<i32>> = HashMap::new();
    for &u in block_nodes {
        block_adj_map.insert(u, HashSet::new());
        if let Some(nbrs) = adj_map.get(&u) {
            for &v in nbrs {
                if block_nodes.contains(&v) {
                    block_adj_map.get_mut(&u).unwrap().insert(v);
                }
            }
        }
    }
    // Add virtual edge between start_port and end_port
    block_adj_map.get_mut(&start_port).unwrap().insert(end_port);
    block_adj_map.get_mut(&end_port).unwrap().insert(start_port);

    let mut btree_adj = BTreeMap::new();
    let mut hash_adj = HashMap::new();
    let mut arcs = Vec::new();
    for (&u, nbrs) in &block_adj_map {
        let mut sorted: Vec<i32> = nbrs.iter().copied().collect();
        sorted.sort_unstable();
        hash_adj.insert(u, sorted.clone());
        btree_adj.insert(u, sorted.clone());
        for &v in &sorted {
            arcs.push((u, v));
        }
    }
    let graph_block = Graph {
        adjacency_list: hash_adj,
        adjacency_list_btree: btree_adj,
        arcs,
    };

    let mut protected_ports = HashSet::new();
    protected_ports.insert(start_port);
    protected_ports.insert(end_port);

    let (contracted_gb, contractor) = Degree2Contractor::contract_with_protected(&graph_block, &protected_ports);
    if contractor.is_infeasible {
        return Err(format!("{} degree-2 contraction is infeasible", block_name));
    }
    if let Some(ref direct_cycle) = contractor.is_direct_cycle {
        return format_corridor_path(direct_cycle, &contractor, block_nodes.len(), start_port, end_port);
    }

    let rem: HashSet<i32> = contracted_gb.adjacency_list.keys().copied().collect();
    let mut rem_sorted: Vec<i32> = rem.iter().copied().collect();
    rem_sorted.sort_unstable();
    println!("[macro_corridor] {} contracted: {} -> {} vertices", block_name, block_nodes.len(), rem.len());

    assert!(rem.contains(&start_port), "start_port must be uncontracted");
    assert!(rem.contains(&end_port), "end_port must be uncontracted");

    if rem.len() == 2 {
        let mut path = Vec::with_capacity(block_nodes.len());
        path.push(start_port);
        let key = if start_port < end_port {
            (start_port, end_port)
        } else {
            (end_port, start_port)
        };
        if let Some(intermediates) = contractor.chain_map.get(&key) {
            if contractor.chain_map.contains_key(&(start_port, end_port)) {
                path.extend(intermediates);
            } else {
                path.extend(intermediates.iter().rev());
            }
        }
        path.push(end_port);
        if path.len() == block_nodes.len() {
            println!("[macro_corridor] {} solved via degree-2 chain: path len = {}", block_name, path.len());
            return Ok(path);
        } else {
            return Err(format!("{} chain length {} != expected {}", block_name, path.len(), block_nodes.len()));
        }
    }

    let virt_edge = (start_port.min(end_port), start_port.max(end_port));
    let mut forbidden_delete = contractor.forced_edges.clone();
    forbidden_delete.insert(virt_edge);

    let mut edges = HashSet::new();
    let mut adj_contracted: HashMap<i32, HashSet<i32>> = HashMap::new();
    for (&u, nbrs) in &contracted_gb.adjacency_list {
        adj_contracted.insert(u, nbrs.iter().copied().collect());
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
    for &u in &rem_sorted {
        let lits = inc_edges.get(&u).cloned().unwrap_or_default();
        let deg = lits.len();
        if deg < 2 {
            return Err(format!("Contracted vertex {} in {} has degree < 2", u, block_name));
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

    // Force virtual edge and shortcut edges
    let _ = solver.add_clause(clause![edge_to_var[&virt_edge]]);
    for ce in &contractor.forced_edges {
        let _ = solver.add_clause(clause![edge_to_var[ce]]);
    }
    // Preprocessing complete. Enter CEGAR loop with 2-opt absorption

    // CEGAR loop with 2-opt absorption
    let mut it = 0;
    while Instant::now() < deadline {
        it += 1;
        match solver.solve() {
            Ok(SolverResult::Sat) => {}
            Ok(SolverResult::Unsat) => return Err(format!("{} UNSAT", block_name)),
            Ok(SolverResult::Interrupted) => return Err(format!("{} timeout", block_name)),
            Err(e) => return Err(format!("{} solver error: {:?}", block_name, e)),
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
        for &u in &rem_sorted {
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
            println!("[macro_corridor] {} converged at iter {}!", block_name, it);
            return format_corridor_path(&cycles[0], &contractor, block_nodes.len(), start_port, end_port);
        }

        if it % 25 == 0 || cycles.len() <= 5 {
            println!("[macro_corridor] {} iter {}: cycles.len={}", block_name, it, cycles.len());
        }

        // Try 2-opt cycle merge
        if let Some(merged_cyc) = safe_2opt_merge(&cycles, &adj_contracted, &forbidden_delete) {
            println!("[macro_corridor] {} 2-opt absorption succeeded at iter {}!", block_name, it);
            return format_corridor_path(&merged_cyc, &contractor, block_nodes.len(), start_port, end_port);
        }

        // DFJ cocycle and negative cuts
        let half_size = rem.len() / 2;
        for cyc in &cycles {
            if cyc.len() <= half_size {
                let cyc_set: HashSet<i32> = cyc.iter().copied().collect();
                let mut cut_lits = Vec::new();
                for &u in cyc {
                    if let Some(nbrs) = adj_contracted.get(&u) {
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

    Err(format!("{} timed out", block_name))
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

    // Solve Block A and Block B sequentially
    println!("[macro_corridor] Solving Block A and Block B sequentially...");
    let res_a = solve_corridor_block(&raw_g.adjacency_list, &v_a, port_u, port_v, "Block A", deadline);
    let p_a = match res_a {
        Ok(path) => path,
        Err(e) => {
            eprintln!("[macro_corridor] Failed to solve Block A: {}", e);
            return None;
        }
    };
    println!("[macro_corridor] Block A solved: path len = {}", p_a.len());

    let res_b = solve_corridor_block(&raw_g.adjacency_list, &v_b, port_v, port_u, "Block B", deadline);
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
