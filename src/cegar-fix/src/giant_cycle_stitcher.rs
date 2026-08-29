use std::collections::{HashMap, HashSet};
use crate::graph::Graph;
use crate::macro_cycle_stitcher::MacroCycleStitcher;
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

pub struct GiantCycleStitcher;

impl GiantCycleStitcher {
    /// Attempts greedy sequential absorption of all candidate subcycles into the dominant giant cycle.
    pub fn absorb_into_giant_cycle(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
        max_swaps: usize,
    ) -> Vec<Vec<i32>> {
        if cycles.len() <= 1 || max_swaps < 2 {
            return cycles.to_vec();
        }

        // Validate cycle lengths
        for c in cycles {
            if c.len() < 3 {
                return cycles.to_vec();
            }
        }

        // 1. Giant Cycle Identification
        let mut giant_idx = 0;
        let mut max_len = 0;
        for (i, c) in cycles.iter().enumerate() {
            if c.len() > max_len {
                max_len = c.len();
                giant_idx = i;
            }
        }

        // If dominant cycle is too small (< 20), fall back to standard MacroCycleStitcher
        if max_len < 20 {
            return MacroCycleStitcher::stitch_until_fixed_point(cycles, g, protected_edges);
        }

        let canonical_protected: HashSet<(i32, i32)> = protected_edges
            .iter()
            .map(|&(u, v)| min_max(u, v))
            .collect();

        let mut current_giant = cycles[giant_idx].clone();
        let mut remaining_cycles: Vec<Vec<i32>> = cycles
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != giant_idx)
            .map(|(_, c)| c.clone())
            .collect();

        // 2. Sequential Targeted Absorption Loop
        let mut made_progress = true;
        while made_progress && !remaining_cycles.is_empty() {
            made_progress = false;
            let mut next_remaining = Vec::new();

            for subcycle in remaining_cycles {
                if let Some(merged_giant) = Self::try_absorb_subcycle(
                    &current_giant,
                    &subcycle,
                    g,
                    &canonical_protected,
                    max_swaps,
                ) {
                    current_giant = merged_giant;
                    made_progress = true;
                } else {
                    next_remaining.push(subcycle);
                }
            }
            remaining_cycles = next_remaining;
        }

        let mut result = Vec::with_capacity(1 + remaining_cycles.len());
        result.push(current_giant);
        result.extend(remaining_cycles);
        result
    }

    /// Formulate and solve exact 2-cycle SAT symmetric difference problem to absorb subcycle into giant.
    fn try_absorb_subcycle(
        giant: &[i32],
        subcycle: &[i32],
        g: &Graph,
        canonical_protected: &HashSet<(i32, i32)>,
        max_swaps: usize,
    ) -> Option<Vec<i32>> {
        let n_giant = giant.len();
        let n_sub = subcycle.len();
        if n_giant < 3 || n_sub < 3 {
            return None;
        }

        let giant_set: HashSet<i32> = giant.iter().copied().collect();
        let sub_set: HashSet<i32> = subcycle.iter().copied().collect();
        let total_v = n_giant + n_sub;

        let mut f_neighbors: HashMap<i32, [i32; 2]> = HashMap::with_capacity(total_v);
        let mut f_edges: HashSet<(i32, i32)> = HashSet::with_capacity(total_v);

        // Giant cycle edges and neighbors
        for pos in 0..n_giant {
            let u = giant[pos];
            let prev = giant[(pos + n_giant - 1) % n_giant];
            let next = giant[(pos + 1) % n_giant];
            f_neighbors.insert(u, [prev, next]);
            f_edges.insert(min_max(u, prev));
            f_edges.insert(min_max(u, next));
        }

        // Subcycle edges and neighbors
        for pos in 0..n_sub {
            let u = subcycle[pos];
            let prev = subcycle[(pos + n_sub - 1) % n_sub];
            let next = subcycle[(pos + 1) % n_sub];
            f_neighbors.insert(u, [prev, next]);
            f_edges.insert(min_max(u, prev));
            f_edges.insert(min_max(u, next));
        }

        // Collect candidate cross-edges between giant and subcycle
        let mut cross_edges: Vec<(i32, i32)> = Vec::new();
        let mut seen_cross: HashSet<(i32, i32)> = HashSet::new();

        for &u in &giant_set {
            if let Some(nbrs) = g.adjacency_list.get(&u) {
                for &v in nbrs {
                    if sub_set.contains(&v) {
                        let e = min_max(u, v);
                        if seen_cross.insert(e) {
                            cross_edges.push(e);
                        }
                    }
                }
            }
        }

        if cross_edges.len() < 2 {
            return None;
        }

        // Collect removable cycle edges (non-protected)
        let mut removable_cycle_edges: Vec<(i32, i32)> = Vec::new();
        for &e in &f_edges {
            if !canonical_protected.contains(&e) {
                removable_cycle_edges.push(e);
            }
        }

        if removable_cycle_edges.is_empty() {
            return None;
        }

        // Map removable cycle edges and cross edges to SAT boolean variables
        let mut y_map: HashMap<(i32, i32), Lit> = HashMap::new();
        let mut z_map: HashMap<(i32, i32), Lit> = HashMap::new();
        let mut var_idx: u32 = 0;

        for &e in &removable_cycle_edges {
            y_map.insert(e, Lit::positive(var_idx));
            var_idx += 1;
        }

        for &e in &cross_edges {
            z_map.insert(e, Lit::positive(var_idx));
            var_idx += 1;
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

        // Construct SAT CNF for 2-cycle alternating symmetric difference
        let mut cnf = Cnf::new();

        // 1. Vertex Parity: sum(z) == sum(y) for each vertex
        let mut all_vertices: Vec<i32> = Vec::with_capacity(total_v);
        all_vertices.extend_from_slice(giant);
        all_vertices.extend_from_slice(subcycle);

        for &u in &all_vertices {
            let c_edges = v_cycle_edges.get(&u).cloned().unwrap_or_default();
            let x_edges = v_cross_edges.get(&u).cloned().unwrap_or_default();

            if x_edges.is_empty() {
                for &(_, y_lit) in &c_edges {
                    cnf.add_clause(Clause::from_iter([!y_lit]));
                }
            } else if c_edges.is_empty() {
                for &(_, z_lit) in &x_edges {
                    cnf.add_clause(Clause::from_iter([!z_lit]));
                }
            } else {
                // At most 1 cross-edge added
                for i in 0..x_edges.len() {
                    for j in (i + 1)..x_edges.len() {
                        cnf.add_clause(Clause::from_iter([!x_edges[i].1, !x_edges[j].1]));
                    }
                }
                // At most 1 cycle-edge removed
                for i in 0..c_edges.len() {
                    for j in (i + 1)..c_edges.len() {
                        cnf.add_clause(Clause::from_iter([!c_edges[i].1, !c_edges[j].1]));
                    }
                }
                // Cross implies removed cycle edge: !z \/ \/ y
                for &(_, z_lit) in &x_edges {
                    let mut cl = vec![!z_lit];
                    for &(_, y_lit) in &c_edges {
                        cl.push(y_lit);
                    }
                    cnf.add_clause(Clause::from_iter(cl));
                }
                // Removed cycle edge implies added cross edge: !y \/ \/ z
                for &(_, y_lit) in &c_edges {
                    let mut cl = vec![!y_lit];
                    for &(_, z_lit) in &x_edges {
                        cl.push(z_lit);
                    }
                    cnf.add_clause(Clause::from_iter(cl));
                }
            }
        }

        // Require at least one cross edge
        let mut cross_lits = Vec::with_capacity(cross_edges.len());
        for &e in &cross_edges {
            if let Some(&lit) = z_map.get(&e) {
                cross_lits.push(lit);
            }
        }
        cnf.add_clause(Clause::from_iter(cross_lits));

        // Solve SAT subproblem with CaDiCaL
        let mut solver = CaDiCaL::default();
        if solver.add_cnf_ref(&cnf).is_err() {
            return None;
        }

        let mut attempts = 0;
        while attempts < 20 {
            attempts += 1;
            match solver.solve() {
                Ok(SolverResult::Sat) => {
                    if let Ok(sol) = solver.full_solution() {
                        let model_set: HashSet<Lit> = sol.into_iter().collect();
                        let mut used_y = HashSet::new();
                        let mut used_z = HashSet::new();

                        for (&e, &lit) in &y_map {
                            if model_set.contains(&lit) {
                                used_y.insert(e);
                            }
                        }
                        for (&e, &lit) in &z_map {
                            if model_set.contains(&lit) {
                                used_z.insert(e);
                            }
                        }

                        if used_z.len() >= 2 && used_z.len() <= max_swaps && used_z.len() == used_y.len() {
                            if let Some(new_cycle) = Self::reconstruct_and_verify_single_cycle(
                                giant,
                                subcycle,
                                &f_neighbors,
                                &used_y,
                                &used_z,
                            ) {
                                return Some(new_cycle);
                            }
                        }

                        // Block this specific combination of cross-edges
                        let block_clause: Vec<Lit> = used_z
                            .iter()
                            .filter_map(|e| z_map.get(e).map(|&l| !l))
                            .collect();
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

        None
    }

    /// Reconstructs unified cycle and verifies single 2-regular connectivity across all vertices.
    fn reconstruct_and_verify_single_cycle(
        giant: &[i32],
        subcycle: &[i32],
        f_neighbors: &HashMap<i32, [i32; 2]>,
        used_y: &HashSet<(i32, i32)>,
        used_z: &HashSet<(i32, i32)>,
    ) -> Option<Vec<i32>> {
        let total_v = giant.len() + subcycle.len();
        let mut adj: HashMap<i32, Vec<i32>> = HashMap::with_capacity(total_v);

        for (&u, &nbrs) in f_neighbors {
            let mut u_adj = Vec::with_capacity(2);
            for &nbr in &nbrs {
                let e = min_max(u, nbr);
                if !used_y.contains(&e) {
                    u_adj.push(nbr);
                }
            }
            adj.insert(u, u_adj);
        }

        for &(u, v) in used_z {
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

        // Traverse single cycle from giant[0]
        let start_v = giant[0];
        let mut visited: HashSet<i32> = HashSet::with_capacity(total_v);
        let mut merged = Vec::with_capacity(total_v);
        let mut curr = start_v;
        let mut prev: Option<i32> = None;

        loop {
            visited.insert(curr);
            merged.push(curr);

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

        if merged.len() == total_v {
            Some(merged)
        } else {
            None
        }
    }

    /// Iterates absorption and multi-cycle stitching until fixed point.
    pub fn repair_until_fixed_point(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
    ) -> Vec<Vec<i32>> {
        if cycles.len() <= 1 {
            return cycles.to_vec();
        }

        let max_passes = 20;
        let mut current_cycles = cycles.to_vec();

        for _ in 0..max_passes {
            if current_cycles.len() <= 1 {
                break;
            }

            let prev_count = current_cycles.len();

            // 1. First attempt targeted giant cycle absorption with max_swaps = 4
            let absorbed = Self::absorb_into_giant_cycle(&current_cycles, g, protected_edges, 4);
            if absorbed.len() < current_cycles.len() {
                current_cycles = absorbed;
                if current_cycles.len() <= 1 {
                    break;
                }
                continue;
            }

            // 2. Try with max_swaps = 6
            let absorbed_6 = Self::absorb_into_giant_cycle(&current_cycles, g, protected_edges, 6);
            if absorbed_6.len() < current_cycles.len() {
                current_cycles = absorbed_6;
                if current_cycles.len() <= 1 {
                    break;
                }
                continue;
            }

            // 3. Multi-Cycle Sweep: stitch remaining subcycles with MacroCycleStitcher
            let stitched = MacroCycleStitcher::stitch_until_fixed_point(&current_cycles, g, protected_edges);
            if stitched.len() < current_cycles.len() {
                current_cycles = stitched;
                if current_cycles.len() <= 1 {
                    break;
                }
                continue;
            }

            // If no strategy decreased cycle count, fixed point reached
            if current_cycles.len() == prev_count {
                break;
            }
        }

        current_cycles
    }
}
