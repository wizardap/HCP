use std::collections::{HashMap, HashSet};
use crate::graph::Graph;
use crate::metagraph_router::MetagraphRouter;
use crate::tour_verifier::TourVerifier;
use rustsat::instances::Cnf;
use rustsat::solvers::{Solve, SolverResult};
use rustsat::types::{Clause, Lit, TernaryVal, Var};
use rustsat_cadical::CaDiCaL;

#[derive(Debug, Clone)]
pub struct DeReducedVariable {
    pub var_id: usize,
    pub vertices: Vec<i32>,
    pub port_in: i32,
    pub port_out: i32,
    pub true_path: Vec<i32>,
    pub false_path: Vec<i32>,
}

#[derive(Debug, Clone)]
pub struct DeReducedClause {
    pub clause_id: usize,
    pub clause_vertices: Vec<i32>,
    pub literal_hooks: Vec<(usize, bool, i32, i32)>, // (var_id, is_positive, enter_rung, exit_rung)
}

pub struct Inverse3SatSynthesizer;

impl Inverse3SatSynthesizer {
    /// Attempts to de-reduce graph G into a 3-SAT instance, solve it in CaDiCaL,
    /// and synthesize the exact Hamiltonian Tour.
    /// Returns Some(tour) if successful, or None if the graph is not a standard 3-SAT reduction.
    pub fn try_solve_via_inverse_3sat(g: &Graph) -> Option<Vec<i32>> {
        if g.adjacency_list.len() < 4 {
            return None;
        }

        // Step 1: Detect clause nodes and variable gadgets
        let (variables, clauses) = Self::de_reduce_gadgets(g)?;
        if variables.is_empty() || clauses.is_empty() {
            return None;
        }

        // Step 2: Build CNF formula
        let mut cnf = Cnf::new();
        for clause in &clauses {
            if clause.literal_hooks.is_empty() {
                return None;
            }
            let mut lits = Vec::new();
            for &(var_id, is_positive, _, _) in &clause.literal_hooks {
                let lit = if is_positive {
                    Lit::positive(var_id as u32)
                } else {
                    Lit::negative(var_id as u32)
                };
                lits.push(lit);
            }
            cnf.add_clause(Clause::from_iter(lits));
        }

        // Step 3: Exact SAT Solve via CaDiCaL
        let mut solver = CaDiCaL::default();
        let _ = solver.add_cnf(cnf);

        let num_vars = variables.len();
        let sat_assignment: Vec<bool> = match solver.solve() {
            Ok(SolverResult::Sat) => {
                let sol = solver.full_solution().unwrap();
                let mut assignment = Vec::with_capacity(num_vars);
                for var_idx in 0..num_vars {
                    let var = Var::new(var_idx as u32);
                    assignment.push(sol.var_value(var) == TernaryVal::True);
                }
                assignment
            }
            _ => return None, // UNSAT or error
        };

        // Step 4: Tour Synthesis & Splicing
        let tour = Self::synthesize_tour(g, &variables, &clauses, &sat_assignment)?;

        // Verify with TourVerifier
        if TourVerifier::verify_raw_tour(&tour, g).is_ok() {
            Some(tour)
        } else {
            None
        }
    }

    /// De-reduces graph G into variables and clauses.
    fn de_reduce_gadgets(
        g: &Graph,
    ) -> Option<(Vec<DeReducedVariable>, Vec<DeReducedClause>)> {
        // Step 1a: Identify clause vertices
        // A clause vertex c has even degree 2k >= 2, and its neighbors N(c) partition into k disjoint edges in G.
        let mut clause_nodes: Vec<i32> = Vec::new();
        let mut clause_rungs_map: HashMap<i32, Vec<(i32, i32)>> = HashMap::new();

        for (&v, neighbors) in &g.adjacency_list {
            let deg = neighbors.len();
            if deg >= 2 && deg % 2 == 0 && deg <= 16 {
                let nbr_set: HashSet<i32> = neighbors.iter().copied().collect();
                let mut is_clause = true;
                let mut matched = HashSet::new();
                let mut rungs = Vec::new();

                for &u in neighbors {
                    if matched.contains(&u) {
                        continue;
                    }
                    if let Some(u_nbrs) = g.adjacency_list.get(&u) {
                        let common: Vec<i32> = u_nbrs
                            .iter()
                            .copied()
                            .filter(|n| nbr_set.contains(n) && *n != u)
                            .collect();
                        if common.len() == 1 {
                            let partner = common[0];
                            if !matched.contains(&partner) {
                                matched.insert(u);
                                matched.insert(partner);
                                rungs.push((u, partner));
                            } else {
                                is_clause = false;
                                break;
                            }
                        } else {
                            is_clause = false;
                            break;
                        }
                    } else {
                        is_clause = false;
                        break;
                    }
                }

                if is_clause && matched.len() == deg {
                    clause_nodes.push(v);
                    clause_rungs_map.insert(v, rungs);
                }
            }
        }

        if clause_nodes.is_empty() {
            return None;
        }

        clause_nodes.sort_unstable();
        let clause_set: HashSet<i32> = clause_nodes.iter().copied().collect();
        let non_clause_vertices: Vec<i32> = g
            .adjacency_list
            .keys()
            .copied()
            .filter(|v| !clause_set.contains(v))
            .collect();

        if non_clause_vertices.is_empty() {
            return None;
        }

        // Call MetagraphRouter to assist in modular partition
        let _modules = MetagraphRouter::detect_gadget_modules_with_size(g, 32);

        // Group non-clause vertices into variable modules
        let mut non_clause_adj: HashMap<i32, Vec<i32>> = HashMap::new();
        for &u in &non_clause_vertices {
            if let Some(nbrs) = g.adjacency_list.get(&u) {
                let filtered: Vec<i32> = nbrs
                    .iter()
                    .copied()
                    .filter(|v| !clause_set.contains(v))
                    .collect();
                non_clause_adj.insert(u, filtered);
            }
        }

        // Collect all clause rungs
        let mut all_rungs: Vec<(i32, i32)> = Vec::new();
        for rungs in clause_rungs_map.values() {
            for &(u, v) in rungs {
                all_rungs.push((u, v));
            }
        }

        // 1. Form initial rung clusters
        let mut rung_clusters: Vec<HashSet<i32>> = Vec::new();
        for &(u, v) in &all_rungs {
            let mut found_cluster = None;
            for (c_idx, cl) in rung_clusters.iter().enumerate() {
                if cl.contains(&u) || cl.contains(&v) {
                    found_cluster = Some(c_idx);
                    break;
                }
            }
            if let Some(c_idx) = found_cluster {
                rung_clusters[c_idx].insert(u);
                rung_clusters[c_idx].insert(v);
            } else {
                let mut new_cl = HashSet::new();
                new_cl.insert(u);
                new_cl.insert(v);
                rung_clusters.push(new_cl);
            }
        }

        // 2. Merge rung clusters that are connected by direct ladder edges
        let mut merged = true;
        while merged {
            merged = false;
            let mut merge_pair = None;
            'outer: for i in 0..rung_clusters.len() {
                for j in (i + 1)..rung_clusters.len() {
                    let mut is_connected = false;
                    for &u in &rung_clusters[i] {
                        if let Some(nbrs) = non_clause_adj.get(&u) {
                            for &v in nbrs {
                                if rung_clusters[j].contains(&v) {
                                    is_connected = true;
                                    break;
                                }
                            }
                        }
                        if is_connected {
                            break;
                        }
                    }
                    if is_connected {
                        merge_pair = Some((i, j));
                        break 'outer;
                    }
                }
            }
            if let Some((i, j)) = merge_pair {
                let nodes_j = rung_clusters.remove(j);
                rung_clusters[i].extend(nodes_j);
                merged = true;
            }
        }

        if rung_clusters.is_empty() {
            return None;
        }

        // 3. Assign every non-clause vertex to its connected variable cluster
        // Core rung vertices:
        let mut var_modules: Vec<HashSet<i32>> = rung_clusters.clone();
        let mut all_core_rungs: HashSet<i32> = HashSet::new();
        for cl in &rung_clusters {
            for &v in cl {
                all_core_rungs.insert(v);
            }
        }

        // For each unassigned vertex, find which rung cluster contains a neighbor in all_core_rungs
        for &v in &non_clause_vertices {
            if all_core_rungs.contains(&v) {
                continue;
            }
            if let Some(nbrs) = non_clause_adj.get(&v) {
                for &nbr in nbrs {
                    for (c_idx, cl) in rung_clusters.iter().enumerate() {
                        if cl.contains(&nbr) {
                            var_modules[c_idx].insert(v);
                            break;
                        }
                    }
                }
            }
        }

        let mut final_var_modules: Vec<Vec<i32>> = Vec::with_capacity(var_modules.len());
        for cl in var_modules {
            let mut mod_vec: Vec<i32> = cl.into_iter().collect();
            mod_vec.sort_unstable();
            final_var_modules.push(mod_vec);
        }

        // Step 1b: Extract interface ports and dual paths (true_path, false_path) for each variable
        let mut de_reduced_vars: Vec<DeReducedVariable> = Vec::new();
        let mut vertex_to_var: HashMap<i32, usize> = HashMap::new();

        for (var_id, mod_vertices) in final_var_modules.iter().enumerate() {
            for &v in mod_vertices {
                vertex_to_var.insert(v, var_id);
            }
        }

        for (var_id, mod_vertices) in final_var_modules.iter().enumerate() {
            let mod_set: HashSet<i32> = mod_vertices.iter().copied().collect();
            // Find port vertices: vertices in this module connected to other variable modules
            let mut external_ports: Vec<i32> = Vec::new();
            for &u in mod_vertices {
                if let Some(nbrs) = non_clause_adj.get(&u) {
                    if nbrs.iter().any(|v| !mod_set.contains(v)) {
                        external_ports.push(u);
                    }
                }
            }
            external_ports.sort_unstable();
            external_ports.dedup();

            let (port_a, port_b) = if external_ports.len() >= 2 {
                (external_ports[0], external_ports[external_ports.len() - 1])
            } else if external_ports.len() == 1 {
                let p_a = external_ports[0];
                let p_b = mod_vertices
                    .iter()
                    .copied()
                    .filter(|&v| v != p_a)
                    .max_by_key(|&v| {
                        let deg = non_clause_adj
                            .get(&v)
                            .map(|n| n.iter().filter(|x| mod_set.contains(x)).count())
                            .unwrap_or(0);
                        if deg == 1 {
                            100
                        } else {
                            0
                        }
                    })
                    .unwrap_or(p_a);
                (p_a, p_b)
            } else {
                (mod_vertices[0], mod_vertices[mod_vertices.len() - 1])
            };

            // Find Hamiltonian paths in G[mod_vertices]
            let (true_path_opt, false_path_opt) =
                Self::find_dual_paths(mod_vertices, port_a, port_b, g);

            let true_path = true_path_opt?;
            let false_path = false_path_opt?;

            de_reduced_vars.push(DeReducedVariable {
                var_id,
                vertices: mod_vertices.clone(),
                port_in: port_a,
                port_out: port_b,
                true_path,
                false_path,
            });
        }

        // Step 1c: Construct DeReducedClause structs with literal hooks
        let mut de_reduced_clauses: Vec<DeReducedClause> = Vec::new();

        // Build directed arc set from g.arcs to preserve hook entry/exit orientation
        let arc_set: HashSet<(i32, i32)> = g.arcs.iter().copied().collect();

        for (clause_id, &c_node) in clause_nodes.iter().enumerate() {
            let rungs = clause_rungs_map.get(&c_node)?;
            let mut literal_hooks: Vec<(usize, bool, i32, i32)> = Vec::new();

            for &(u, v) in rungs {
                let var_id = match (vertex_to_var.get(&u), vertex_to_var.get(&v)) {
                    (Some(&v1), Some(&v2)) if v1 == v2 => v1,
                    _ => return None, // Rung must be inside same variable
                };

                let var_gadget = &de_reduced_vars[var_id];
                // Check hook orientation (u -> c -> v or v -> c -> u)
                let (enter, exit) = if arc_set.contains(&(u, c_node)) && arc_set.contains(&(c_node, v)) {
                    (u, v)
                } else if arc_set.contains(&(v, c_node)) && arc_set.contains(&(c_node, u)) {
                    (v, u)
                } else {
                    (u, v)
                };

                let mut matched = false;

                // Check True path for exact directed match (enter -> exit)
                for i in 0..(var_gadget.true_path.len().saturating_sub(1)) {
                    if var_gadget.true_path[i] == enter && var_gadget.true_path[i + 1] == exit {
                        literal_hooks.push((var_id, true, enter, exit));
                        matched = true;
                        break;
                    }
                }

                if !matched {
                    // Check False path for exact directed match (enter -> exit)
                    for i in 0..(var_gadget.false_path.len().saturating_sub(1)) {
                        if var_gadget.false_path[i] == enter && var_gadget.false_path[i + 1] == exit {
                            literal_hooks.push((var_id, false, enter, exit));
                            matched = true;
                            break;
                        }
                    }
                }

                if !matched {
                    return None;
                }
            }

            de_reduced_clauses.push(DeReducedClause {
                clause_id,
                clause_vertices: vec![c_node],
                literal_hooks,
            });
        }

        Some((de_reduced_vars, de_reduced_clauses))
    }

    /// Finds dual Hamiltonian paths (T_i and F_i) within a variable gadget module.
    fn find_dual_paths(
        mod_vertices: &[i32],
        port_a: i32,
        port_b: i32,
        g: &Graph,
    ) -> (Option<Vec<i32>>, Option<Vec<i32>>) {
        let n = mod_vertices.len();
        if n < 2 || n > 32 {
            return (None, None);
        }
        let mod_set: HashSet<i32> = mod_vertices.iter().copied().collect();

        let mut all_paths_a_to_b: Vec<Vec<i32>> = Vec::new();
        let mut all_paths_b_to_a: Vec<Vec<i32>> = Vec::new();

        // Search paths port_a -> port_b
        let mut path = Vec::with_capacity(n);
        let mut visited = HashSet::with_capacity(n);
        path.push(port_a);
        visited.insert(port_a);
        let mut steps = 0;
        Self::dfs_hamiltonian_paths(
            port_a,
            port_b,
            n,
            &mod_set,
            g,
            &mut path,
            &mut visited,
            &mut all_paths_a_to_b,
            &mut steps,
        );

        // Search paths port_b -> port_a
        path.clear();
        visited.clear();
        path.push(port_b);
        visited.insert(port_b);
        let mut steps = 0;
        Self::dfs_hamiltonian_paths(
            port_b,
            port_a,
            n,
            &mod_set,
            g,
            &mut path,
            &mut visited,
            &mut all_paths_b_to_a,
            &mut steps,
        );

        if !all_paths_a_to_b.is_empty() && !all_paths_b_to_a.is_empty() {
            (
                Some(all_paths_a_to_b[0].clone()),
                Some(all_paths_b_to_a[0].clone()),
            )
        } else if all_paths_a_to_b.len() >= 2 {
            (
                Some(all_paths_a_to_b[0].clone()),
                Some(all_paths_a_to_b[1].clone()),
            )
        } else if !all_paths_a_to_b.is_empty() {
            let t = all_paths_a_to_b[0].clone();
            let f = t.iter().rev().copied().collect();
            (Some(t), Some(f))
        } else {
            (None, None)
        }
    }

    fn dfs_hamiltonian_paths(
        curr: i32,
        target: i32,
        target_len: usize,
        mod_set: &HashSet<i32>,
        g: &Graph,
        path: &mut Vec<i32>,
        visited: &mut HashSet<i32>,
        results: &mut Vec<Vec<i32>>,
        steps: &mut usize,
    ) {
        *steps += 1;
        if *steps > 10_000 || results.len() >= 10 {
            return;
        }
        if path.len() == target_len {
            if curr == target {
                results.push(path.clone());
            }
            return;
        }

        if let Some(nbrs) = g.adjacency_list.get(&curr) {
            for &next in nbrs {
                if mod_set.contains(&next) && !visited.contains(&next) {
                    visited.insert(next);
                    path.push(next);
                    Self::dfs_hamiltonian_paths(
                        next,
                        target,
                        target_len,
                        mod_set,
                        g,
                        path,
                        visited,
                        results,
                        steps,
                    );
                    path.pop();
                    visited.remove(&next);
                }
            }
        }
    }

    /// Synthesizes the full Hamiltonian tour from truth assignment and spliced clause detours.
    fn synthesize_tour(
        g: &Graph,
        variables: &[DeReducedVariable],
        clauses: &[DeReducedClause],
        sat_assignment: &[bool],
    ) -> Option<Vec<i32>> {
        let num_vars = variables.len();
        let mut var_paths: Vec<Vec<i32>> = Vec::with_capacity(num_vars);

        for i in 0..num_vars {
            if sat_assignment[i] {
                var_paths.push(variables[i].true_path.clone());
            } else {
                var_paths.push(variables[i].false_path.clone());
            }
        }

        // Splice each clause into the first satisfied literal hook
        for clause in clauses {
            let mut satisfied_hook = None;
            for hook in &clause.literal_hooks {
                let (var_id, is_positive, enter_rung, exit_rung) = *hook;
                if sat_assignment[var_id] == is_positive {
                    satisfied_hook = Some((var_id, enter_rung, exit_rung));
                    break;
                }
            }

            let (var_id, enter_rung, exit_rung) = satisfied_hook?;
            let path = &mut var_paths[var_id];
            let mut splice_pos = None;

            for i in 0..(path.len().saturating_sub(1)) {
                if path[i] == enter_rung && path[i + 1] == exit_rung {
                    splice_pos = Some(i + 1);
                    break;
                }
            }

            if let Some(pos) = splice_pos {
                for (offset, &cv) in clause.clause_vertices.iter().enumerate() {
                    path.insert(pos + offset, cv);
                }
            } else {
                return None;
            }
        }

        // Chain variable modules in cyclic order
        if num_vars == 1 {
            let tour = var_paths[0].clone();
            if TourVerifier::verify_raw_tour(&tour, g).is_ok() {
                return Some(tour);
            }
        }

        // Find valid cyclic permutation of variable modules
        for start_idx in 0..num_vars {
            let mut visited = HashSet::new();
            let mut order = Vec::new();
            visited.insert(start_idx);
            order.push(start_idx);

            if Self::find_variable_chain_order(start_idx, &mut visited, &mut order, &var_paths, g) {
                let mut tour = Vec::new();
                for &v_idx in &order {
                    tour.extend_from_slice(&var_paths[v_idx]);
                }
                if TourVerifier::verify_raw_tour(&tour, g).is_ok() {
                    return Some(tour);
                }
            }
        }

        None
    }

    fn find_variable_chain_order(
        curr_idx: usize,
        visited: &mut HashSet<usize>,
        order: &mut Vec<usize>,
        var_paths: &[Vec<i32>],
        g: &Graph,
    ) -> bool {
        if order.len() == var_paths.len() {
            let last_v = *var_paths[*order.last().unwrap()].last().unwrap();
            let first_v = *var_paths[order[0]].first().unwrap();
            return g
                .adjacency_list
                .get(&last_v)
                .map_or(false, |nbrs| nbrs.contains(&first_v));
        }

        let curr_last = *var_paths[curr_idx].last().unwrap();
        let n = var_paths.len();

        for next_idx in 0..n {
            if !visited.contains(&next_idx) {
                let next_first = *var_paths[next_idx].first().unwrap();
                if let Some(nbrs) = g.adjacency_list.get(&curr_last) {
                    if nbrs.contains(&next_first) {
                        visited.insert(next_idx);
                        order.push(next_idx);
                        if Self::find_variable_chain_order(next_idx, visited, order, var_paths, g) {
                            return true;
                        }
                        order.pop();
                        visited.remove(&next_idx);
                    }
                }
            }
        }
        false
    }
}
