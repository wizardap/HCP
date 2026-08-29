use std::collections::{HashMap, HashSet};
use crate::graph::Graph;
use rustsat::instances::Cnf;
use rustsat::solvers::{Solve, SolverResult};
use rustsat::types::{Clause, Lit};
use rustsat_cadical::CaDiCaL;

#[inline]
fn min_max(u: i32, v: i32) -> (i32, i32) {
    if u < v {
        (u, v)
    } else {
        (v, u)
    }
}

pub struct MacroCycleStitcher;

impl MacroCycleStitcher {
    /// Attempts exact multi-cycle alternating patch merging on current 2-factor cycles using a lightweight SAT subproblem.
    /// Returns Some(merged_cycles) if cycle count strictly decreased, or None.
    pub fn stitch_cycles(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
        max_swaps: usize,
    ) -> Option<Vec<Vec<i32>>> {
        if cycles.len() < 2 || max_swaps < 2 {
            return None;
        }

        // Validate cycle lengths
        for c in cycles {
            if c.len() < 3 {
                return None;
            }
        }

        let total_v: usize = cycles.iter().map(|c| c.len()).sum();
        let mut vertex_to_cycle: HashMap<i32, usize> = HashMap::with_capacity(total_v);
        let mut f_neighbors: HashMap<i32, [i32; 2]> = HashMap::with_capacity(total_v);
        let mut f_edges: HashSet<(i32, i32)> = HashSet::with_capacity(total_v);

        for (c_idx, cycle) in cycles.iter().enumerate() {
            let n = cycle.len();
            for pos in 0..n {
                let u = cycle[pos];
                let prev = cycle[(pos + n - 1) % n];
                let next = cycle[(pos + 1) % n];
                vertex_to_cycle.insert(u, c_idx);
                f_neighbors.insert(u, [prev, next]);
                f_edges.insert(min_max(u, prev));
                f_edges.insert(min_max(u, next));
            }
        }

        let canonical_protected: HashSet<(i32, i32)> = protected_edges
            .iter()
            .map(|&(u, v)| min_max(u, v))
            .collect();

        // Collect candidate cross edges between distinct cycles
        let mut cross_edges: Vec<(i32, i32)> = Vec::new();
        let mut cycle_cross_lits: HashMap<usize, Vec<Lit>> = HashMap::new();

        for (&u, &c_u) in &vertex_to_cycle {
            if let Some(nbrs) = g.adjacency_list.get(&u) {
                for &v in nbrs {
                    if u < v {
                        if let Some(&c_v) = vertex_to_cycle.get(&v) {
                            if c_u != c_v {
                                let e = (u, v);
                                if !f_edges.contains(&e) {
                                    cross_edges.push(e);
                                }
                            }
                        }
                    }
                }
            }
        }

        if cross_edges.is_empty() {
            return None;
        }

        // Collect candidate removable cycle edges (non-protected)
        let mut removable_cycle_edges: Vec<(i32, i32)> = Vec::new();
        for &e in &f_edges {
            if !canonical_protected.contains(&e) {
                removable_cycle_edges.push(e);
            }
        }

        // Map removable cycle edges and cross edges to boolean variables
        let mut y_map: HashMap<(i32, i32), Lit> = HashMap::new();
        let mut z_map: HashMap<(i32, i32), Lit> = HashMap::new();
        let mut var_idx: u32 = 0;

        for &e in &removable_cycle_edges {
            y_map.insert(e, Lit::positive(var_idx));
            var_idx += 1;
        }

        for &e in &cross_edges {
            let lit = Lit::positive(var_idx);
            z_map.insert(e, lit);
            var_idx += 1;

            let cu = vertex_to_cycle[&e.0];
            let cv = vertex_to_cycle[&e.1];
            cycle_cross_lits.entry(cu).or_default().push(lit);
            cycle_cross_lits.entry(cv).or_default().push(lit);
        }

        // Build incident edge mappings per vertex
        let mut v_cycle_edges: HashMap<i32, Vec<((i32, i32), Lit)>> = HashMap::new();
        let mut v_cross_edges: HashMap<i32, Vec<((i32, i32), Lit)>> = HashMap::new();

        for (&e, &lit) in &y_map {
            v_cycle_edges.entry(e.0).or_default().push((e, lit));
            v_cycle_edges.entry(e.1).or_default().push((e, lit));
        }

        for (&e, &lit) in &z_map {
            v_cross_edges.entry(e.0).or_default().push((e, lit));
            v_cross_edges.entry(e.1).or_default().push((e, lit));
        }

        // Construct SAT CNF for 2-factor alternating symmetric difference
        let mut cycle_removable_lits: HashMap<usize, Vec<Lit>> = HashMap::new();
        for &e in &removable_cycle_edges {
            let lit = y_map[&e];
            let cu = vertex_to_cycle[&e.0];
            let cv = vertex_to_cycle[&e.1];
            if cu == cv {
                cycle_removable_lits.entry(cu).or_default().push(lit);
            }
        }

        let base_cnf = Self::build_base_cnf(cycles, &v_cycle_edges, &v_cross_edges, &cycle_removable_lits);

        // 2. Solve SAT subproblem per cycle neighborhood
        for c_idx in 0..cycles.len() {
            let active_lits = match cycle_cross_lits.get(&c_idx) {
                Some(lits) if !lits.is_empty() => lits.clone(),
                _ => continue,
            };

            let mut cnf = base_cnf.clone();
            // Require at least one cross-edge incident to cycle c_idx
            cnf.add_clause(Clause::from_iter(active_lits));

            let mut solver = CaDiCaL::default();
            if solver.add_cnf_ref(&cnf).is_ok() {
                let mut attempts = 0;
                while attempts < 15 {
                    attempts += 1;
                    match solver.solve() {
                        Ok(SolverResult::Sat) => {
                            if let Ok(sol) = solver.full_solution() {
                                let model_set: HashSet<Lit> = sol.into_iter().collect();
                                let mut used_x_edges = HashSet::new();
                                let mut used_y_edges = HashSet::new();

                                for (&e, &lit) in &y_map {
                                    if model_set.contains(&lit) {
                                        used_x_edges.insert(e);
                                    }
                                }
                                for (&e, &lit) in &z_map {
                                    if model_set.contains(&lit) {
                                        used_y_edges.insert(e);
                                    }
                                }

                                if used_y_edges.len() >= 2 && used_y_edges.len() <= max_swaps && used_x_edges.len() == used_y_edges.len() {
                                    if let Some(new_cycles) = Self::evaluate_and_reconstruct_cycles(
                                        cycles,
                                        &vertex_to_cycle,
                                        &f_neighbors,
                                        &used_x_edges,
                                        &used_y_edges,
                                    ) {
                                        if new_cycles.len() < cycles.len() {
                                            return Some(new_cycles);
                                        }
                                    }
                                }

                                // Block this specific assignment to explore other symmetric difference combinations
                                let mut block_clause = Vec::new();
                                for &e in &used_y_edges {
                                    if let Some(&lit) = z_map.get(&e) {
                                        block_clause.push(!lit);
                                    }
                                }
                                if block_clause.is_empty() {
                                    break;
                                }
                                let _ = solver.add_clause(Clause::from_iter(block_clause));
                            } else {
                                break;
                            }
                        }
                        _ => break,
                    }
                }
            }
        }

        None
    }

    /// Builds base SAT CNF for 2-factor alternating symmetric difference.
    /// Only enforces At-Most-One (AMO) cycle edge removal on small cycles (|C| < 50).
    /// For giant cycles (|C| >= 50), degree parity (sum y == sum z) guarantees 2-regularity while allowing multi-swap.
    pub fn build_base_cnf(
        cycles: &[Vec<i32>],
        v_cycle_edges: &HashMap<i32, Vec<((i32, i32), Lit)>>,
        v_cross_edges: &HashMap<i32, Vec<((i32, i32), Lit)>>,
        cycle_removable_lits: &HashMap<usize, Vec<Lit>>,
    ) -> Cnf {
        let mut base_cnf = Cnf::new();

        // 1. Vertex Parity: sum(z) == sum(y) for each vertex
        // For vertices with no cross edges, all incident removable cycle edges must NOT be removed
        for (&u, c_edges) in v_cycle_edges {
            let x_edges = v_cross_edges.get(&u).cloned().unwrap_or_default();
            if x_edges.is_empty() {
                for &(_, y_lit) in c_edges {
                    base_cnf.add_clause(Clause::from_iter([!y_lit]));
                }
            } else {
                // At most 1 cross-edge added per vertex
                for i in 0..x_edges.len() {
                    for j in (i + 1)..x_edges.len() {
                        base_cnf.add_clause(Clause::from_iter([!x_edges[i].1, !x_edges[j].1]));
                    }
                }
                // At most 1 cycle-edge removed per vertex
                for i in 0..c_edges.len() {
                    for j in (i + 1)..c_edges.len() {
                        base_cnf.add_clause(Clause::from_iter([!c_edges[i].1, !c_edges[j].1]));
                    }
                }
                // If any cross-edge is added, at least one cycle-edge must be removed: !z_e \/ \/ y_e
                for &(_, z_lit) in &x_edges {
                    let mut cl = vec![!z_lit];
                    for &(_, y_lit) in c_edges {
                        cl.push(y_lit);
                    }
                    base_cnf.add_clause(Clause::from_iter(cl));
                }
                // If any cycle-edge is removed, at least one cross-edge must be added: !y_e \/ \/ z_e
                for &(_, y_lit) in c_edges {
                    let mut cl = vec![!y_lit];
                    for &(_, z_lit) in &x_edges {
                        cl.push(z_lit);
                    }
                    base_cnf.add_clause(Clause::from_iter(cl));
                }
            }
        }

        // For vertices with only cross-edges and no removable cycle edges: cannot add cross-edges
        for (&u, x_edges) in v_cross_edges {
            if !v_cycle_edges.contains_key(&u) {
                for &(_, z_lit) in x_edges {
                    base_cnf.add_clause(Clause::from_iter([!z_lit]));
                }
            }
        }

        // 2. Cycle AMO: only for small cycles (|C| < 50).
        // For giant cycles (|C| >= 50), do NOT add AMO, allowing multiple simultaneous swaps.
        for (c_idx, cycle) in cycles.iter().enumerate() {
            if cycle.len() < 50 {
                if let Some(c_y_lits) = cycle_removable_lits.get(&c_idx) {
                    for i in 0..c_y_lits.len() {
                        for j in (i + 1)..c_y_lits.len() {
                            base_cnf.add_clause(Clause::from_iter([!c_y_lits[i], !c_y_lits[j]]));
                        }
                    }
                }
            }
        }

        base_cnf
    }

    fn evaluate_and_reconstruct_cycles(
        cycles: &[Vec<i32>],
        vertex_to_cycle: &HashMap<i32, usize>,
        f_neighbors: &HashMap<i32, [i32; 2]>,
        used_x_edges: &HashSet<(i32, i32)>,
        used_y_edges: &HashSet<(i32, i32)>,
    ) -> Option<Vec<Vec<i32>>> {
        let total_v = vertex_to_cycle.len();
        let mut adj: HashMap<i32, Vec<i32>> = HashMap::with_capacity(total_v);

        for (&u, &f_nbrs) in f_neighbors {
            let mut u_adj = Vec::with_capacity(2);
            for &nbr in &f_nbrs {
                let e = min_max(u, nbr);
                if !used_x_edges.contains(&e) {
                    u_adj.push(nbr);
                }
            }
            adj.insert(u, u_adj);
        }

        for &(u, v) in used_y_edges {
            if let Some(u_nbrs) = adj.get_mut(&u) {
                u_nbrs.push(v);
            }
            if let Some(v_nbrs) = adj.get_mut(&v) {
                v_nbrs.push(u);
            }
        }

        // Check 2-regularity for all vertices
        for (&u, nbrs) in &adj {
            if nbrs.len() != 2 {
                return None;
            }
            if nbrs[0] == nbrs[1] || nbrs[0] == u || nbrs[1] == u {
                return None;
            }
        }

        // Traverse cycles
        let mut visited: HashSet<i32> = HashSet::with_capacity(total_v);
        let mut new_cycles = Vec::new();

        for &start_v in vertex_to_cycle.keys() {
            if !visited.contains(&start_v) {
                let mut current_cycle = Vec::new();
                let mut curr = start_v;
                let mut prev: Option<i32> = None;

                loop {
                    visited.insert(curr);
                    current_cycle.push(curr);

                    let nbrs = &adj[&curr];
                    let next = if Some(nbrs[0]) == prev {
                        nbrs[1]
                    } else {
                        nbrs[0]
                    };

                    if next == start_v {
                        break;
                    }
                    if visited.contains(&next) {
                        return None;
                    }

                    prev = Some(curr);
                    curr = next;
                }

                if current_cycle.len() < 3 {
                    return None;
                }
                new_cycles.push(current_cycle);
            }
        }

        if visited.len() != total_v {
            return None;
        }

        if new_cycles.len() < cycles.len() {
            Some(new_cycles)
        } else {
            None
        }
    }

    /// Iteratively stitches cycles until a single tour is obtained or no further merge is possible.
    pub fn stitch_until_fixed_point(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
    ) -> Vec<Vec<i32>> {
        if cycles.len() <= 1 {
            return cycles.to_vec();
        }

        let mut current_cycles = cycles.to_vec();
        let max_passes = 20;

        for _ in 0..max_passes {
            if current_cycles.len() <= 1 {
                break;
            }
            if let Some(next_cycles) = Self::stitch_cycles(&current_cycles, g, protected_edges, 4) {
                current_cycles = next_cycles;
            } else if let Some(next_cycles) = Self::stitch_cycles(&current_cycles, g, protected_edges, 6) {
                current_cycles = next_cycles;
            } else if let Some(next_cycles) = Self::stitch_cycles(&current_cycles, g, protected_edges, 16) {
                current_cycles = next_cycles;
            } else if let Some(next_cycles) = Self::stitch_cycles(&current_cycles, g, protected_edges, 32) {
                current_cycles = next_cycles;
            } else {
                break;
            }
        }

        current_cycles
    }
}
