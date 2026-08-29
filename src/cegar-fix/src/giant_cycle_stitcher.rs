use std::collections::{HashMap, HashSet};
use crate::graph::Graph;
use crate::macro_cycle_stitcher::MacroCycleStitcher;
use crate::transitive_macro_splicer::TransitiveMacroSplicer;
use crate::multi_opt_sat_splicer::MultiOptSatSplicer;
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
            let giant_set: HashSet<i32> = current_giant.iter().copied().collect();
            remaining_cycles.sort_by_cached_key(|c| {
                let cross_count = c.iter().map(|&u| {
                    g.adjacency_list.get(&u).map_or(0, |nbrs| nbrs.iter().filter(|v| giant_set.contains(v)).count())
                }).sum::<usize>();
                std::cmp::Reverse(cross_count)
            });

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

    /// Simultaneously absorbs a cluster of candidate subcycles into the dominant giant cycle using a joint SAT formulation.
    /// Allows multi-swap simultaneous absorption (e.g. max_swaps = 16 or 32) without single-swap AMO restrictions on the giant cycle.
    pub fn absorb_cluster_into_giant(
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

        // 2. Cluster Candidate Selection:
        // Giant cycle is index 0 in the cluster.
        // Include candidate subcycles that have cross-edges connecting to the giant cycle.
        let giant_set: HashSet<i32> = cycles[giant_idx].iter().copied().collect();
        let mut candidate_subcycle_indices: Vec<usize> = Vec::new();

        for (i, c) in cycles.iter().enumerate() {
            if i == giant_idx {
                continue;
            }
            let cross_count = c.iter().map(|&u| {
                g.adjacency_list.get(&u).map_or(0, |nbrs| nbrs.iter().filter(|v| giant_set.contains(v)).count())
            }).sum::<usize>();
            if cross_count >= 1 {
                candidate_subcycle_indices.push(i);
            }
        }

        if candidate_subcycle_indices.is_empty() {
            return cycles.to_vec();
        }

        // Sort candidates by cross edge count descending (prefer highly connected subcycles)
        candidate_subcycle_indices.sort_by_cached_key(|&i| {
            let cross_count = cycles[i].iter().map(|&u| {
                g.adjacency_list.get(&u).map_or(0, |nbrs| nbrs.iter().filter(|v| giant_set.contains(v)).count())
            }).sum::<usize>();
            std::cmp::Reverse(cross_count)
        });

        // Limit cluster to top 32 candidate subcycles to keep SAT solver instance ultra-fast (< 5ms)
        let cluster_limit = 32;
        let selected_candidates: Vec<usize> = candidate_subcycle_indices.into_iter().take(cluster_limit).collect();

        // Build cluster cycles: index 0 is giant, indices 1.. are the candidates
        let mut cluster_cycles: Vec<&Vec<i32>> = Vec::with_capacity(1 + selected_candidates.len());
        cluster_cycles.push(&cycles[giant_idx]);
        let mut cluster_k_to_orig: HashMap<usize, usize> = HashMap::new();
        for (k, &orig_idx) in selected_candidates.iter().enumerate() {
            cluster_cycles.push(&cycles[orig_idx]);
            cluster_k_to_orig.insert(k + 1, orig_idx);
        }

        let total_v: usize = cluster_cycles.iter().map(|c| c.len()).sum();
        let mut vertex_to_cluster_cycle: HashMap<i32, usize> = HashMap::with_capacity(total_v);
        let mut f_neighbors: HashMap<i32, [i32; 2]> = HashMap::with_capacity(total_v);
        let mut f_edges: HashSet<(i32, i32)> = HashSet::with_capacity(total_v);

        for (cluster_k, &cycle) in cluster_cycles.iter().enumerate() {
            let n = cycle.len();
            for pos in 0..n {
                let u = cycle[pos];
                let prev = cycle[(pos + n - 1) % n];
                let next = cycle[(pos + 1) % n];
                vertex_to_cluster_cycle.insert(u, cluster_k);
                f_neighbors.insert(u, [prev, next]);
                f_edges.insert(min_max(u, prev));
                f_edges.insert(min_max(u, next));
            }
        }

        // Collect candidate cross edges between distinct cluster cycles
        let mut cross_edges: Vec<(i32, i32)> = Vec::new();
        let mut giant_cross_lits: Vec<Lit> = Vec::new();

        let mut removable_cycle_edges: Vec<(i32, i32)> = Vec::new();
        for &e in &f_edges {
            if !canonical_protected.contains(&e) {
                removable_cycle_edges.push(e);
            }
        }

        if removable_cycle_edges.is_empty() {
            return cycles.to_vec();
        }

        let mut y_map: HashMap<(i32, i32), Lit> = HashMap::new();
        let mut z_map: HashMap<(i32, i32), Lit> = HashMap::new();
        let mut cycle_removable_lits: HashMap<usize, Vec<Lit>> = HashMap::new();
        let mut var_idx: u32 = 0;

        for &e in &removable_cycle_edges {
            let lit = Lit::positive(var_idx);
            y_map.insert(e, lit);
            var_idx += 1;

            let ck0 = vertex_to_cluster_cycle[&e.0];
            let ck1 = vertex_to_cluster_cycle[&e.1];
            if ck0 == ck1 {
                cycle_removable_lits.entry(ck0).or_default().push(lit);
            }
        }

        for (&u, &ck_u) in &vertex_to_cluster_cycle {
            if let Some(nbrs) = g.adjacency_list.get(&u) {
                for &v in nbrs {
                    if u < v {
                        if let Some(&ck_v) = vertex_to_cluster_cycle.get(&v) {
                            if ck_u != ck_v {
                                let e = (u, v);
                                if !f_edges.contains(&e) {
                                    cross_edges.push(e);
                                    let lit = Lit::positive(var_idx);
                                    z_map.insert(e, lit);
                                    var_idx += 1;

                                    if ck_u == 0 || ck_v == 0 {
                                        giant_cross_lits.push(lit);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if giant_cross_lits.is_empty() {
            return cycles.to_vec();
        }

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

        // Construct SAT CNF
        let mut cnf = Cnf::new();

        // 1. Vertex parity: sum(z) == sum(y) for each vertex in cluster
        for (&u, c_edges) in &v_cycle_edges {
            let x_edges = v_cross_edges.get(&u).cloned().unwrap_or_default();
            if x_edges.is_empty() {
                for &(_, y_lit) in c_edges {
                    cnf.add_clause(Clause::from_iter([!y_lit]));
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
                // Cross implies cycle: !z \/ \/ y
                for &(_, z_lit) in &x_edges {
                    let mut cl = vec![!z_lit];
                    for &(_, y_lit) in c_edges {
                        cl.push(y_lit);
                    }
                    cnf.add_clause(Clause::from_iter(cl));
                }
                // Cycle implies cross: !y \/ \/ z
                for &(_, y_lit) in c_edges {
                    let mut cl = vec![!y_lit];
                    for &(_, z_lit) in &x_edges {
                        cl.push(z_lit);
                    }
                    cnf.add_clause(Clause::from_iter(cl));
                }
            }
        }

        for (&u, x_edges) in &v_cross_edges {
            if !v_cycle_edges.contains_key(&u) {
                for &(_, z_lit) in x_edges {
                    cnf.add_clause(Clause::from_iter([!z_lit]));
                }
            }
        }

        // 2. AMO on small cycles only (|C| < 50).
        // For giant cycle (|C| >= 50), do NOT add AMO, allowing multiple simultaneous swaps.
        for (cluster_k, cycle) in cluster_cycles.iter().enumerate() {
            if cycle.len() < 50 {
                if let Some(c_y_lits) = cycle_removable_lits.get(&cluster_k) {
                    for i in 0..c_y_lits.len() {
                        for j in (i + 1)..c_y_lits.len() {
                            cnf.add_clause(Clause::from_iter([!c_y_lits[i], !c_y_lits[j]]));
                        }
                    }
                }
            }
        }

        // 3. At least one cross-edge incident to giant cycle must be used
        cnf.add_clause(Clause::from_iter(giant_cross_lits));

        // Solve SAT subproblem with CaDiCaL
        let mut solver = CaDiCaL::default();
        if solver.add_cnf_ref(&cnf).is_err() {
            return cycles.to_vec();
        }

        let mut attempts = 0;
        while attempts < 30 {
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
                            if let Some((new_giant, absorbed_cluster_ks)) = Self::evaluate_cluster_absorption(
                                &cluster_cycles,
                                &f_neighbors,
                                &used_y,
                                &used_z,
                            ) {
                                if !absorbed_cluster_ks.is_empty() {
                                    let mut absorbed_orig_indices: HashSet<usize> = HashSet::new();
                                    for &k in &absorbed_cluster_ks {
                                        if let Some(&orig_idx) = cluster_k_to_orig.get(&k) {
                                            absorbed_orig_indices.insert(orig_idx);
                                        }
                                    }

                                    let mut result = Vec::new();
                                    result.push(new_giant);
                                    for (i, c) in cycles.iter().enumerate() {
                                        if i != giant_idx && !absorbed_orig_indices.contains(&i) {
                                            result.push(c.clone());
                                        }
                                    }
                                    return result;
                                }
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

        cycles.to_vec()
    }

    fn evaluate_cluster_absorption(
        cluster_cycles: &[&Vec<i32>],
        f_neighbors: &HashMap<i32, [i32; 2]>,
        used_y: &HashSet<(i32, i32)>,
        used_z: &HashSet<(i32, i32)>,
    ) -> Option<(Vec<i32>, HashSet<usize>)> {
        let total_v: usize = cluster_cycles.iter().map(|c| c.len()).sum();
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

        // Check 2-regularity for all vertices in cluster
        for (&u, nbrs) in &adj {
            if nbrs.len() != 2 {
                return None;
            }
            if nbrs[0] == nbrs[1] || nbrs[0] == u || nbrs[1] == u {
                return None;
            }
        }

        // Traverse giant cycle starting at cluster_cycles[0][0]
        let start_v = cluster_cycles[0][0];
        let mut visited: HashSet<i32> = HashSet::with_capacity(total_v);
        let mut new_giant = Vec::new();
        let mut curr = start_v;
        let mut prev: Option<i32> = None;

        loop {
            visited.insert(curr);
            new_giant.push(curr);

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

        if new_giant.len() <= cluster_cycles[0].len() {
            return None;
        }

        let new_giant_set: HashSet<i32> = new_giant.iter().copied().collect();
        let mut absorbed_ks = HashSet::new();

        for (k, &subcycle) in cluster_cycles.iter().enumerate().skip(1) {
            let in_count = subcycle.iter().filter(|v| new_giant_set.contains(v)).count();
            if in_count == subcycle.len() {
                absorbed_ks.insert(k);
            } else if in_count > 0 {
                // Partial absorption is invalid
                return None;
            }
        }

        if absorbed_ks.is_empty() {
            return None;
        }

        // Verify remaining vertices form valid cycles of length >= 3
        for &u in adj.keys() {
            if !visited.contains(&u) {
                let sub_start = u;
                let mut sub_curr = sub_start;
                let mut sub_prev: Option<i32> = None;
                let mut sub_len = 0;

                loop {
                    visited.insert(sub_curr);
                    sub_len += 1;

                    let nbrs = &adj[&sub_curr];
                    let next = if Some(nbrs[0]) == sub_prev {
                        nbrs[1]
                    } else {
                        nbrs[0]
                    };

                    if next == sub_start {
                        break;
                    }
                    if visited.contains(&next) {
                        return None;
                    }

                    sub_prev = Some(sub_curr);
                    sub_curr = next;
                }

                if sub_len < 3 {
                    return None;
                }
            }
        }

        if visited.len() != total_v {
            return None;
        }

        Some((new_giant, absorbed_ks))
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

            // 1. Multi-swap simultaneous cluster absorption into giant cycle (max_swaps = 16)
            let clustered_16 = Self::absorb_cluster_into_giant(&current_cycles, g, protected_edges, 16);
            if clustered_16.len() < current_cycles.len() {
                current_cycles = clustered_16;
                if current_cycles.len() <= 1 {
                    break;
                }
                continue;
            }

            // 2. Deeper multi-swap simultaneous cluster absorption (max_swaps = 32)
            let clustered_32 = Self::absorb_cluster_into_giant(&current_cycles, g, protected_edges, 32);
            if clustered_32.len() < current_cycles.len() {
                current_cycles = clustered_32;
                if current_cycles.len() <= 1 {
                    break;
                }
                continue;
            }

            // 3. Fallback sequential absorption with max_swaps = 4
            let absorbed_4 = Self::absorb_into_giant_cycle(&current_cycles, g, protected_edges, 4);
            if absorbed_4.len() < current_cycles.len() {
                current_cycles = absorbed_4;
                if current_cycles.len() <= 1 {
                    break;
                }
                continue;
            }

            // 4. Fallback sequential absorption with max_swaps = 6
            let absorbed_6 = Self::absorb_into_giant_cycle(&current_cycles, g, protected_edges, 6);
            if absorbed_6.len() < current_cycles.len() {
                current_cycles = absorbed_6;
                if current_cycles.len() <= 1 {
                    break;
                }
                continue;
            }

            // 5. Multi-Cycle Sweep: stitch remaining subcycles with MacroCycleStitcher
            let stitched = MacroCycleStitcher::stitch_until_fixed_point(&current_cycles, g, protected_edges);
            if stitched.len() < current_cycles.len() {
                current_cycles = stitched;
                if current_cycles.len() <= 1 {
                    break;
                }
                continue;
            }

            // 6. Transitive Macro-Graph Splicing: global multi-cycle spanning tree/forest splicing
            let spliced = TransitiveMacroSplicer::splice_transitive_macro_graph(&current_cycles, g, protected_edges);
            if spliced.len() < current_cycles.len() {
                current_cycles = spliced;
                if current_cycles.len() <= 1 {
                    break;
                }
                continue;
            }

            // 7. Multi-Opt SAT Splicing: exact 2-opt + 3-opt triangle spanning forest
            let multi_opt_spliced = MultiOptSatSplicer::splice_multi_opt_cycles(&current_cycles, g, protected_edges);
            if multi_opt_spliced.len() < current_cycles.len() {
                current_cycles = multi_opt_spliced;
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

