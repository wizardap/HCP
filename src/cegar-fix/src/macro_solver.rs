use std::collections::{HashMap, HashSet};
use crate::contraction::Degree2Contractor;
use crate::graph::Graph;
use crate::hub_registry::HubRegistry;
use rustsat::clause;
use rustsat::instances::{BasicVarManager, Cnf, ManageVars};
use rustsat::solvers::{Solve, SolverResult};
use rustsat::types::{Clause, Lit, TernaryVal};
use rustsat_cadical::CaDiCaL;

#[derive(Clone, Debug)]
pub struct MacroGraph {
    pub num_macro_nodes: usize,
    pub macro_adj: Vec<Vec<usize>>,
    pub connectors: HashMap<(usize, usize), Vec<(i32, i32)>>,
    pub vertex_to_cycle: HashMap<i32, (usize, usize)>, // vertex -> (cycle_idx, pos_in_cycle)
}

pub struct MacroGraphSolver;

impl MacroGraphSolver {
    /// Solves for a single Hamiltonian cycle by contracting subcycles into a macro-graph,
    /// finding a macro-tour via Mini-SAT, and expanding the macro-tour into a full cycle.
    pub fn solve_via_macro_graph(
        cycles: &[Vec<i32>],
        g: &Graph,
        contractor: &Degree2Contractor,
        _hub_registry: &HubRegistry,
    ) -> Option<Vec<i32>> {
        if cycles.len() <= 1 {
            return None;
        }

        let macro_graph = build_macro_graph(cycles, g, contractor);
        let tour_opt = solve_macro_sat(&macro_graph, cycles, g, contractor);

        if let Some(tour) = tour_opt {
            if is_valid_cycle(&tour, g) && tour.len() == g.adjacency_list.len() {
                return Some(tour);
            }
        }

        None
    }
}

/// Builds the macro-graph where each subcycle is a macro-node, extracting all cross-edges
/// that satisfy degree-2 break safety invariants.
pub fn build_macro_graph(
    cycles: &[Vec<i32>],
    g: &Graph,
    contractor: &Degree2Contractor,
) -> MacroGraph {
    let k = cycles.len();
    let mut vertex_to_cycle = HashMap::new();

    for (c_idx, cycle) in cycles.iter().enumerate() {
        for (pos, &v) in cycle.iter().enumerate() {
            vertex_to_cycle.insert(v, (c_idx, pos));
        }
    }

    let mut connectors: HashMap<(usize, usize), Vec<(i32, i32)>> = HashMap::new();
    let mut macro_adj_sets: Vec<HashSet<usize>> = vec![HashSet::new(); k];

    for (c_idx, cycle) in cycles.iter().enumerate() {
        let r = cycle.len();
        for (pos, &u) in cycle.iter().enumerate() {
            // Check if vertex u is a safe port in cycle c_idx
            let pred = cycle[(pos + r - 1) % r];
            let succ = cycle[(pos + 1) % r];
            let u_safe = r <= 1
                || is_safe_to_break(pred, u, contractor)
                || is_safe_to_break(u, succ, contractor);
            if !u_safe {
                continue;
            }

            if let Some(neighbors) = g.adjacency_list.get(&u) {
                for &v in neighbors {
                    if let Some(&(other_c_idx, other_pos)) = vertex_to_cycle.get(&v) {
                        if c_idx != other_c_idx {
                            let other_cycle = &cycles[other_c_idx];
                            let other_r = other_cycle.len();
                            let other_pred = other_cycle[(other_pos + other_r - 1) % other_r];
                            let other_succ = other_cycle[(other_pos + 1) % other_r];
                            let v_safe = other_r <= 1
                                || is_safe_to_break(other_pred, v, contractor)
                                || is_safe_to_break(v, other_succ, contractor);

                            if v_safe {
                                connectors
                                    .entry((c_idx, other_c_idx))
                                    .or_default()
                                    .push((u, v));
                                macro_adj_sets[c_idx].insert(other_c_idx);
                            }
                        }
                    }
                }
            }
        }
    }

    let mut macro_adj = Vec::with_capacity(k);
    for adj_set in macro_adj_sets {
        let mut adj_vec: Vec<usize> = adj_set.into_iter().collect();
        adj_vec.sort_unstable();
        macro_adj.push(adj_vec);
    }

    for edges in connectors.values_mut() {
        edges.sort_unstable();
        edges.dedup();
    }

    MacroGraph {
        num_macro_nodes: k,
        macro_adj,
        connectors,
        vertex_to_cycle,
    }
}

/// Solves Hamiltonian Cycle on the macro-graph using CaDiCaL SAT solver with CEGAR subtour cuts,
/// and splices the subcycles along the chosen connector ports into a single valid tour.
pub fn solve_macro_sat(
    macro_graph: &MacroGraph,
    cycles: &[Vec<i32>],
    g: &Graph,
    contractor: &Degree2Contractor,
) -> Option<Vec<i32>> {
    let k = macro_graph.num_macro_nodes;
    if k <= 1 {
        return None;
    }

    let mut var_manager = BasicVarManager::default();
    let mut arc_lit_map: HashMap<(usize, usize), Lit> = HashMap::new();

    for u in 0..k {
        for &v in &macro_graph.macro_adj[u] {
            let lit = var_manager.new_lit();
            arc_lit_map.insert((u, v), lit);
        }
    }

    let mut cnf = Cnf::new();

    // 1. Degree-1 outgoing constraint for each macro-node
    for u in 0..k {
        let out_lits: Vec<Lit> = macro_graph.macro_adj[u]
            .iter()
            .map(|&v| arc_lit_map[&(u, v)])
            .collect();
        if out_lits.is_empty() {
            return None; // Isolated macro-node
        }

        let mut cl = Clause::new();
        cl.extend(out_lits.clone());
        cnf.add_clause(cl);

        for i in 0..out_lits.len() {
            for j in i + 1..out_lits.len() {
                cnf.add_clause(clause!(!out_lits[i], !out_lits[j]));
            }
        }
    }

    // 2. Degree-1 incoming constraint for each macro-node
    for v in 0..k {
        let mut in_lits = Vec::new();
        for u in 0..k {
            if let Some(&lit) = arc_lit_map.get(&(u, v)) {
                in_lits.push(lit);
            }
        }
        if in_lits.is_empty() {
            return None;
        }

        let mut cl = Clause::new();
        cl.extend(in_lits.clone());
        cnf.add_clause(cl);

        for i in 0..in_lits.len() {
            for j in i + 1..in_lits.len() {
                cnf.add_clause(clause!(!in_lits[i], !in_lits[j]));
            }
        }
    }

    // 3. 2-cycle prohibition for k > 2
    if k > 2 {
        for u in 0..k {
            for &v in &macro_graph.macro_adj[u] {
                if u < v {
                    if let Some(&lit_vu) = arc_lit_map.get(&(v, u)) {
                        let lit_uv = arc_lit_map[&(u, v)];
                        cnf.add_clause(clause!(!lit_uv, !lit_vu));
                    }
                }
            }
        }
    }

    let mut solver = CaDiCaL::default();
    let _ = solver.add_cnf(cnf);

    // CEGAR loop for subtour elimination on macro-graph
    let max_macro_iterations = 200;
    for _ in 0..max_macro_iterations {
        match solver.solve() {
            Ok(SolverResult::Sat) => {
                let sol = solver.full_solution().unwrap();
                let mut succ_map: HashMap<usize, usize> = HashMap::new();
                for (&(u, v), &lit) in &arc_lit_map {
                    if sol.lit_value(lit) == TernaryVal::True {
                        succ_map.insert(u, v);
                    }
                }

                // Extract all cycles in the macro-graph solution
                let mut visited = vec![false; k];
                let mut macro_cycles = Vec::new();

                for start in 0..k {
                    if visited[start] {
                        continue;
                    }
                    let mut cycle = Vec::new();
                    let mut curr = start;
                    while !visited[curr] {
                        visited[curr] = true;
                        cycle.push(curr);
                        if let Some(&next) = succ_map.get(&curr) {
                            curr = next;
                        } else {
                            break;
                        }
                    }
                    if !cycle.is_empty() {
                        macro_cycles.push(cycle);
                    }
                }

                // Check if we found a single macro-tour covering all k nodes
                if macro_cycles.len() == 1 && macro_cycles[0].len() == k {
                    let macro_tour = &macro_cycles[0];
                    if let Some(full_tour) =
                        try_splice_macro_tour(macro_tour, macro_graph, cycles, contractor, g)
                    {
                        return Some(full_tour);
                    }

                    // Splicing failed for this specific macro-tour; block it and search for another.
                    let mut block_clause = Clause::new();
                    for t in 0..k {
                        let u = macro_tour[t];
                        let v = macro_tour[(t + 1) % k];
                        let lit = arc_lit_map[&(u, v)];
                        block_clause.add(!lit);
                    }
                    let _ = solver.add_clause(block_clause);
                } else {
                    // Subtours found: add subtour elimination clauses and cut constraints
                    for subtour in &macro_cycles {
                        if subtour.len() < k {
                            let mut subtour_block = Clause::new();
                            let m = subtour.len();
                            for t in 0..m {
                                let u = subtour[t];
                                let v = subtour[(t + 1) % m];
                                if let Some(&lit) = arc_lit_map.get(&(u, v)) {
                                    subtour_block.add(!lit);
                                }
                            }
                            let _ = solver.add_clause(subtour_block);

                            let subtour_set: HashSet<usize> = subtour.iter().cloned().collect();
                            let mut cut_lits = Vec::new();
                            for &u in subtour {
                                for &v in &macro_graph.macro_adj[u] {
                                    if !subtour_set.contains(&v) {
                                        if let Some(&lit) = arc_lit_map.get(&(u, v)) {
                                            cut_lits.push(lit);
                                        }
                                    }
                                }
                            }
                            if !cut_lits.is_empty() {
                                let mut cut_clause = Clause::new();
                                cut_clause.extend(cut_lits);
                                let _ = solver.add_clause(cut_clause);
                            }
                        }
                    }
                }
            }
            _ => {
                return None;
            }
        }
    }

    None
}

/// Backtracking search to find a compatible connector assignment for `macro_tour`,
/// and splice all subcycles into a single Hamiltonian cycle.
fn try_splice_macro_tour(
    macro_tour: &[usize],
    macro_graph: &MacroGraph,
    cycles: &[Vec<i32>],
    contractor: &Degree2Contractor,
    g: &Graph,
) -> Option<Vec<i32>> {
    let k = macro_tour.len();
    if k == 0 {
        return None;
    }

    let mut step_edges: Vec<&Vec<(i32, i32)>> = Vec::with_capacity(k);
    for t in 0..k {
        let from_node = macro_tour[t];
        let to_node = macro_tour[(t + 1) % k];
        let edges = macro_graph.connectors.get(&(from_node, to_node))?;
        if edges.is_empty() {
            return None;
        }
        step_edges.push(edges);
    }

    let mut chosen_edges = vec![(0, 0); k];

    fn backtrack(
        step: usize,
        k: usize,
        macro_tour: &[usize],
        step_edges: &[&Vec<(i32, i32)>],
        chosen_edges: &mut Vec<(i32, i32)>,
        cycles: &[Vec<i32>],
        contractor: &Degree2Contractor,
    ) -> bool {
        if step == k {
            let in_port = chosen_edges[k - 1].1;
            let out_port = chosen_edges[0].0;
            return get_subcycle_path(&cycles[macro_tour[0]], in_port, out_port, contractor)
                .is_some();
        }

        for &edge in step_edges[step] {
            chosen_edges[step] = edge;

            if step > 0 {
                let in_port = chosen_edges[step - 1].1;
                let out_port = chosen_edges[step].0;
                if get_subcycle_path(&cycles[macro_tour[step]], in_port, out_port, contractor)
                    .is_none()
                {
                    continue;
                }
            }

            if backtrack(
                step + 1,
                k,
                macro_tour,
                step_edges,
                chosen_edges,
                cycles,
                contractor,
            ) {
                return true;
            }
        }
        false
    }

    if !backtrack(
        0,
        k,
        macro_tour,
        &step_edges,
        &mut chosen_edges,
        cycles,
        contractor,
    ) {
        return None;
    }

    let total_v: usize = cycles.iter().map(|c| c.len()).sum();
    let mut full_tour = Vec::with_capacity(total_v);

    for t in 0..k {
        let in_port = if t == 0 {
            chosen_edges[k - 1].1
        } else {
            chosen_edges[t - 1].1
        };
        let out_port = chosen_edges[t].0;
        let sub_path = get_subcycle_path(&cycles[macro_tour[t]], in_port, out_port, contractor)?;
        full_tour.extend(sub_path);
    }

    if is_valid_cycle(&full_tour, g) && full_tour.len() == g.adjacency_list.len() {
        Some(full_tour)
    } else {
        None
    }
}

/// Returns the sequence of vertices in `cycle` traversing from `in_port` to `out_port`,
/// visiting all vertices in `cycle` exactly once, if valid and safe.
fn get_subcycle_path(
    cycle: &[i32],
    in_port: i32,
    out_port: i32,
    contractor: &Degree2Contractor,
) -> Option<Vec<i32>> {
    let r = cycle.len();
    if r == 0 {
        return None;
    }
    if r == 1 {
        if in_port == cycle[0] && out_port == cycle[0] {
            return Some(vec![cycle[0]]);
        }
        return None;
    }
    if r == 2 {
        if in_port == cycle[0] && out_port == cycle[1] {
            if is_safe_to_break(cycle[0], cycle[1], contractor) {
                return Some(vec![cycle[0], cycle[1]]);
            }
        } else if in_port == cycle[1] && out_port == cycle[0] {
            if is_safe_to_break(cycle[1], cycle[0], contractor) {
                return Some(vec![cycle[1], cycle[0]]);
            }
        }
        return None;
    }

    let p = cycle.iter().position(|&v| v == in_port)?;
    let q = cycle.iter().position(|&v| v == out_port)?;

    // Case 1: out_port is the successor of in_port (q == (p + 1) % r)
    // Traversal: reverse direction from in_port to out_port, breaking edge (in_port, out_port)
    if q == (p + 1) % r {
        if is_safe_to_break(in_port, out_port, contractor) {
            let mut path = Vec::with_capacity(r);
            for s in 0..r {
                path.push(cycle[(p + r - (s % r)) % r]);
            }
            return Some(path);
        }
    }

    // Case 2: out_port is the predecessor of in_port (q == (p + r - 1) % r)
    // Traversal: forward direction from in_port to out_port, breaking edge (out_port, in_port)
    if q == (p + r - 1) % r {
        if is_safe_to_break(out_port, in_port, contractor) {
            let mut path = Vec::with_capacity(r);
            for s in 0..r {
                path.push(cycle[(p + s) % r]);
            }
            return Some(path);
        }
    }

    None
}

#[inline]
fn is_safe_to_break(u: i32, v: i32, contractor: &Degree2Contractor) -> bool {
    !contractor.chain_map.contains_key(&(u, v)) && !contractor.chain_map.contains_key(&(v, u))
}

fn is_valid_cycle(cycle: &[i32], g: &Graph) -> bool {
    let len = cycle.len();
    if len < 3 {
        return false;
    }
    for i in 0..len {
        let u = cycle[i];
        let v = cycle[(i + 1) % len];
        if !g.adjacency_list.get(&u).map_or(false, |adj| adj.contains(&v)) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contraction::Degree2Contractor;
    use crate::graph::Graph;
    use crate::hub_registry::HubRegistry;
    use std::collections::HashMap;
    use std::time::Instant;

    fn build_test_graph(edges: &[(i32, i32)]) -> Graph {
        let mut g = Graph::new();
        for &(u, v) in edges {
            g.add_edge(u, v);
        }
        g
    }

    fn empty_contractor() -> Degree2Contractor {
        Degree2Contractor {
            chain_map: HashMap::new(),
            original_vertices_count: 0,
            contracted_vertices_count: 0,
            is_direct_cycle: None,
            is_infeasible: false,
        }
    }

    #[test]
    fn test_macro_graph_construction() {
        let cycles = vec![
            vec![1, 2, 3],
            vec![4, 5, 6],
            vec![7, 8, 9],
        ];

        let mut edges = Vec::new();
        // Cycle edges
        edges.push((1, 2)); edges.push((2, 3)); edges.push((3, 1));
        edges.push((4, 5)); edges.push((5, 6)); edges.push((6, 4));
        edges.push((7, 8)); edges.push((8, 9)); edges.push((9, 7));
        // Cross edges
        edges.push((2, 4));
        edges.push((5, 7));
        edges.push((8, 1));

        let g = build_test_graph(&edges);
        let contractor = empty_contractor();

        let mg = build_macro_graph(&cycles, &g, &contractor);

        assert_eq!(mg.num_macro_nodes, 3);
        assert_eq!(mg.vertex_to_cycle.get(&5), Some(&(1, 1)));
        assert_eq!(mg.vertex_to_cycle.get(&9), Some(&(2, 2)));

        assert!(mg.macro_adj[0].contains(&1));
        assert!(mg.macro_adj[1].contains(&2));
        assert!(mg.macro_adj[2].contains(&0));

        assert_eq!(mg.connectors.get(&(0, 1)), Some(&vec![(2, 4)]));
        assert_eq!(mg.connectors.get(&(1, 2)), Some(&vec![(5, 7)]));
        assert_eq!(mg.connectors.get(&(2, 0)), Some(&vec![(8, 1)]));
    }

    #[test]
    fn test_macro_solver_synthetic_grid() {
        // 6-subcycle grid graph (each subcycle of size 4, total 24 vertices)
        let cycles = vec![
            vec![1, 2, 3, 4],
            vec![5, 6, 7, 8],
            vec![9, 10, 11, 12],
            vec![13, 14, 15, 16],
            vec![17, 18, 19, 20],
            vec![21, 22, 23, 24],
        ];

        let mut edges = Vec::new();
        for c in &cycles {
            let r = c.len();
            for i in 0..r {
                edges.push((c[i], c[(i + 1) % r]));
            }
        }

        // Add cross connections connecting the 6 cycles in a ring:
        // C0 <-> C1 via (2, 5) and (3, 8)
        edges.push((2, 5)); edges.push((3, 8));
        // C1 <-> C2 via (6, 9) and (7, 12)
        edges.push((6, 9)); edges.push((7, 12));
        // C2 <-> C3 via (10, 13) and (11, 16)
        edges.push((10, 13)); edges.push((11, 16));
        // C3 <-> C4 via (14, 17) and (15, 20)
        edges.push((14, 17)); edges.push((15, 20));
        // C4 <-> C5 via (18, 21) and (19, 24)
        edges.push((18, 21)); edges.push((19, 24));
        // C5 <-> C0 via (22, 1) and (23, 4)
        edges.push((22, 1)); edges.push((23, 4));

        let g = build_test_graph(&edges);
        let contractor = empty_contractor();
        let hub_registry = HubRegistry::new(&g);

        let start = Instant::now();
        let result = MacroGraphSolver::solve_via_macro_graph(&cycles, &g, &contractor, &hub_registry);
        let elapsed = start.elapsed();

        assert!(result.is_some(), "MacroGraphSolver should solve 6-subcycle grid");
        let tour = result.unwrap();
        assert_eq!(tour.len(), 24);
        assert!(is_valid_cycle(&tour, &g));
        println!("MacroGraphSolver 6-subcycle solve time: {:?}", elapsed);
        assert!(elapsed.as_millis() < 50, "Should solve in under 50ms, took {:?}", elapsed);
    }

    #[test]
    fn test_macro_solver_degree2_safety() {
        // 2 subcycles with two possible merge options:
        // Option A: break (1, 2) and (5, 6) with cross-edges (1, 5) and (2, 6)
        // Option B: break (3, 4) and (7, 8) with cross-edges (3, 7) and (4, 8)
        let cycles = vec![
            vec![1, 2, 3, 4],
            vec![5, 6, 7, 8],
        ];

        let mut edges = Vec::new();
        for c in &cycles {
            let r = c.len();
            for i in 0..r {
                edges.push((c[i], c[(i + 1) % r]));
            }
        }
        // Option A cross edges
        edges.push((1, 5)); edges.push((2, 6));
        // Option B cross edges
        edges.push((3, 7)); edges.push((4, 8));

        let g = build_test_graph(&edges);
        let mut contractor = empty_contractor();
        // Protect edge (1, 2) as a contracted degree-2 chain
        contractor.chain_map.insert((1, 2), vec![100]);
        contractor.chain_map.insert((2, 1), vec![100]);

        let hub_registry = HubRegistry::new(&g);
        let result = MacroGraphSolver::solve_via_macro_graph(&cycles, &g, &contractor, &hub_registry);

        assert!(result.is_some(), "MacroGraphSolver should solve via Option B");
        let tour = result.unwrap();
        assert_eq!(tour.len(), 8);
        assert!(is_valid_cycle(&tour, &g));

        // Verify that protected edge (1, 2) is strictly preserved (intact in the tour)
        let mut preserved_1_2 = false;
        for i in 0..tour.len() {
            let u = tour[i];
            let v = tour[(i + 1) % tour.len()];
            if (u == 1 && v == 2) || (u == 2 && v == 1) {
                preserved_1_2 = true;
                break;
            }
        }
        assert!(preserved_1_2, "Protected degree-2 edge (1, 2) must be preserved in the cycle");

        // Now protect all remaining edges of cycle C0 so no safe break can occur
        contractor.chain_map.insert((2, 3), vec![101]);
        contractor.chain_map.insert((3, 2), vec![101]);
        contractor.chain_map.insert((3, 4), vec![102]);
        contractor.chain_map.insert((4, 3), vec![102]);
        contractor.chain_map.insert((4, 1), vec![103]);
        contractor.chain_map.insert((1, 4), vec![103]);

        let result_blocked = MacroGraphSolver::solve_via_macro_graph(&cycles, &g, &contractor, &hub_registry);
        assert!(result_blocked.is_none(), "When all cycle edges are protected, should return None safely");
    }
}
