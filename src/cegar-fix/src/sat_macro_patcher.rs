use std::collections::{HashMap, HashSet, VecDeque};
use crate::graph::Graph;
use rustsat::clause;
use rustsat::instances::Cnf;
use rustsat::solvers::{Solve, SolverResult};
use rustsat::types::{Clause, Lit, TernaryVal, Var};
use rustsat_cadical::CaDiCaL;

#[inline]
fn min_max(u: i32, v: i32) -> (i32, i32) {
    if u < v {
        (u, v)
    } else {
        (v, u)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Bridge {
    pub c1: usize,
    pub c2: usize,
    pub rem1: (i32, i32),
    pub rem2: (i32, i32),
    pub add1: (i32, i32),
    pub add2: (i32, i32),
}

pub struct SatMacroPatcher;

impl SatMacroPatcher {
    /// Attempts to merge cycles in every connected component of the 2-opt bridge graph.
    /// Returns the consolidated list of cycles with reduced cycle count if any component was merged.
    pub fn try_patch_components(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
    ) -> Vec<Vec<i32>> {
        if cycles.is_empty() {
            return Vec::new();
        }

        if cycles.len() == 1 {
            return cycles.to_vec();
        }

        let k = cycles.len();
        let total_v: usize = cycles.iter().map(|c| c.len()).sum();

        let mut vertex_to_cycle: HashMap<i32, usize> = HashMap::with_capacity(total_v);
        let mut cycle_neighbors: HashMap<i32, [i32; 2]> = HashMap::with_capacity(total_v);

        for (c_idx, cycle) in cycles.iter().enumerate() {
            let n = cycle.len();
            for pos in 0..n {
                let u = cycle[pos];
                let prev = cycle[(pos + n - 1) % n];
                let next = cycle[(pos + 1) % n];
                vertex_to_cycle.insert(u, c_idx);
                cycle_neighbors.insert(u, [prev, next]);
            }
        }

        let canonical_protected: HashSet<(i32, i32)> = protected_edges
            .iter()
            .map(|&(u, v)| min_max(u, v))
            .collect();

        // 1. Build Macro-Adjacency Graph of 2-opt bridges
        let mut macro_adj: Vec<HashSet<usize>> = vec![HashSet::new(); k];

        for i in 0..k {
            let cycle = &cycles[i];
            let n = cycle.len();
            for pos in 0..n {
                let u1 = cycle[pos];
                let u2 = cycle[(pos + 1) % n];
                let e_i = min_max(u1, u2);
                if canonical_protected.contains(&e_i) {
                    continue;
                }

                if let Some(nbrs) = g.adjacency_list.get(&u1) {
                    for &v1 in nbrs {
                        let j = match vertex_to_cycle.get(&v1) {
                            Some(&idx) if idx > i => idx,
                            _ => continue,
                        };

                        let [v_prev, v_next] = cycle_neighbors[&v1];
                        for &v2 in &[v_prev, v_next] {
                            let e_j = min_max(v1, v2);
                            if canonical_protected.contains(&e_j) {
                                continue;
                            }

                            if let Some(u2_nbrs) = g.adjacency_list.get(&u2) {
                                if u2_nbrs.contains(&v2) {
                                    macro_adj[i].insert(j);
                                    macro_adj[j].insert(i);
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. Discover Connected Components in Macro-Graph
        let mut comp_visited = vec![false; k];
        let mut components: Vec<Vec<usize>> = Vec::new();

        for i in 0..k {
            if !comp_visited[i] {
                let mut comp = Vec::new();
                let mut queue = VecDeque::new();
                comp_visited[i] = true;
                queue.push_back(i);

                while let Some(curr) = queue.pop_front() {
                    comp.push(curr);
                    for &nbr in &macro_adj[curr] {
                        if !comp_visited[nbr] {
                            comp_visited[nbr] = true;
                            queue.push_back(nbr);
                        }
                    }
                }
                comp.sort_unstable();
                components.push(comp);
            }
        }

        // 3. Per-Component Exact SAT Spanning Tree Solving
        let mut result_cycles: Vec<Vec<i32>> = Vec::new();

        for comp in components {
            if comp.len() == 1 {
                result_cycles.push(cycles[comp[0]].clone());
            } else {
                let sub_cycles: Vec<Vec<i32>> = comp.iter().map(|&idx| cycles[idx].clone()).collect();
                if let Some(merged) = Self::try_patch_all_cycles(&sub_cycles, g, protected_edges) {
                    result_cycles.push(merged);
                } else {
                    for &idx in &comp {
                        result_cycles.push(cycles[idx].clone());
                    }
                }
            }
        }

        // 4. Deterministic sorting: descending by length, then tiebreak by min vertex
        result_cycles.sort_by_cached_key(|c| (std::cmp::Reverse(c.len()), c.iter().min().copied(), c.clone()));

        result_cycles
    }

    /// Solves an exact SAT spanning tree formulation over all candidate 2-opt bridges between the cycles.
    /// Returns Some(single_hamiltonian_cycle) if a valid simultaneous bridge set exists, or None.
    pub fn try_patch_all_cycles(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
    ) -> Option<Vec<i32>> {
        if cycles.is_empty() {
            return None;
        }

        if cycles.len() == 1 {
            if cycles[0].len() < 3 {
                return None;
            }
            return Some(cycles[0].clone());
        }

        if cycles.len() > 60 {
            return None;
        }

        for c in cycles {
            if c.len() < 3 {
                return None;
            }
        }

        let k = cycles.len();
        let total_v: usize = cycles.iter().map(|c| c.len()).sum();

        let mut vertex_to_cycle: HashMap<i32, usize> = HashMap::with_capacity(total_v);
        let mut cycle_neighbors: HashMap<i32, [i32; 2]> = HashMap::with_capacity(total_v);

        for (c_idx, cycle) in cycles.iter().enumerate() {
            let n = cycle.len();
            for pos in 0..n {
                let u = cycle[pos];
                let prev = cycle[(pos + n - 1) % n];
                let next = cycle[(pos + 1) % n];
                vertex_to_cycle.insert(u, c_idx);
                cycle_neighbors.insert(u, [prev, next]);
            }
        }

        let canonical_protected: HashSet<(i32, i32)> = protected_edges
            .iter()
            .map(|&(u, v)| min_max(u, v))
            .collect();

        // 1. Bridge Enumeration
        let mut seen_bridges: HashSet<Bridge> = HashSet::new();
        let mut candidate_bridges: Vec<Bridge> = Vec::new();

        for i in 0..k {
            let cycle = &cycles[i];
            let n = cycle.len();
            for pos in 0..n {
                let u1 = cycle[pos];
                let u2 = cycle[(pos + 1) % n];
                let e_i = min_max(u1, u2);
                if canonical_protected.contains(&e_i) {
                    continue;
                }

                if let Some(nbrs) = g.adjacency_list.get(&u1) {
                    for &v1 in nbrs {
                        let j = match vertex_to_cycle.get(&v1) {
                            Some(&idx) if idx > i => idx,
                            _ => continue,
                        };

                        let [v_prev, v_next] = cycle_neighbors[&v1];
                        for &v2 in &[v_prev, v_next] {
                            let e_j = min_max(v1, v2);
                            if canonical_protected.contains(&e_j) {
                                continue;
                            }

                            if let Some(u2_nbrs) = g.adjacency_list.get(&u2) {
                                if u2_nbrs.contains(&v2) {
                                    let x1 = min_max(u1, v1);
                                    let x2 = min_max(u2, v2);
                                    let (add1, add2) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
                                    let bridge = Bridge {
                                        c1: i,
                                        c2: j,
                                        rem1: e_i,
                                        rem2: e_j,
                                        add1,
                                        add2,
                                    };
                                    if seen_bridges.insert(bridge.clone()) {
                                        candidate_bridges.push(bridge);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if candidate_bridges.len() < k - 1 {
            return None;
        }

        // Fast macro graph connectivity check
        let mut adj_macro: Vec<Vec<usize>> = vec![Vec::new(); k];
        for b in &candidate_bridges {
            adj_macro[b.c1].push(b.c2);
            adj_macro[b.c2].push(b.c1);
        }

        let mut comp_visited = vec![false; k];
        let mut queue = VecDeque::new();
        comp_visited[0] = true;
        queue.push_back(0);
        let mut reached_count = 1;

        while let Some(curr) = queue.pop_front() {
            for &nxt in &adj_macro[curr] {
                if !comp_visited[nxt] {
                    comp_visited[nxt] = true;
                    reached_count += 1;
                    queue.push_back(nxt);
                }
            }
        }

        if reached_count < k {
            return None;
        }

        // 2. SAT Encoding
        let mut next_var_id: u32 = 0;

        let mut y_vars: Vec<Lit> = Vec::with_capacity(candidate_bridges.len());
        let mut d_vars: HashMap<usize, (Lit, Lit)> = HashMap::with_capacity(candidate_bridges.len());

        for b_idx in 0..candidate_bridges.len() {
            let y = Var::new(next_var_id).pos_lit();
            next_var_id += 1;
            y_vars.push(y);

            let d_12 = Var::new(next_var_id).pos_lit();
            next_var_id += 1;
            let d_21 = Var::new(next_var_id).pos_lit();
            next_var_id += 1;
            d_vars.insert(b_idx, (d_12, d_21));
        }

        // Unary ladder order variables u_{i, r} for non-root cycles i in 1..k (r in 0..k-2)
        let mut ladder: HashMap<usize, Vec<Lit>> = HashMap::new();
        for c_idx in 1..k {
            let mut row = Vec::with_capacity(k - 1);
            for _ in 0..(k - 1) {
                let lit = Var::new(next_var_id).pos_lit();
                next_var_id += 1;
                row.push(lit);
            }
            ladder.insert(c_idx, row);
        }

        let mut cnf = Cnf::new();

        // Direction & Selection constraints
        for (b_idx, _) in candidate_bridges.iter().enumerate() {
            let y = y_vars[b_idx];
            let (d_12, d_21) = d_vars[&b_idx];
            cnf.add_clause(clause![!d_12, y]);
            cnf.add_clause(clause![!d_21, y]);
            cnf.add_clause(clause![!y, d_12, d_21]);
            cnf.add_clause(clause![!d_12, !d_21]);
        }

        // Incoming edge constraints
        let mut in_lits: HashMap<usize, Vec<Lit>> = HashMap::with_capacity(k);
        for c_idx in 0..k {
            in_lits.insert(c_idx, Vec::new());
        }

        for (b_idx, bridge) in candidate_bridges.iter().enumerate() {
            let (d_12, d_21) = d_vars[&b_idx];
            in_lits.get_mut(&bridge.c2).unwrap().push(d_12);
            in_lits.get_mut(&bridge.c1).unwrap().push(d_21);
        }

        // Root cycle 0 has 0 incoming directed edges
        for &d_lit in &in_lits[&0] {
            cnf.add_clause(clause![!d_lit]);
        }

        // Non-root cycles 1..k have exactly 1 incoming directed edge
        for c_idx in 1..k {
            let incoming = &in_lits[&c_idx];
            if incoming.is_empty() {
                return None;
            }
            // At least 1
            cnf.add_clause(Clause::from_iter(incoming.clone()));
            // At most 1 (pairwise mutual exclusion)
            for p in 0..incoming.len() {
                for q in (p + 1)..incoming.len() {
                    cnf.add_clause(clause![!incoming[p], !incoming[q]]);
                }
            }
        }

        // Edge non-collision constraints: removed edges AMO and added edges AMO
        let mut removed_to_y: HashMap<(i32, i32), Vec<Lit>> = HashMap::new();
        let mut added_to_y: HashMap<(i32, i32), Vec<Lit>> = HashMap::new();

        for (b_idx, bridge) in candidate_bridges.iter().enumerate() {
            let y = y_vars[b_idx];
            removed_to_y.entry(bridge.rem1).or_default().push(y);
            removed_to_y.entry(bridge.rem2).or_default().push(y);
            added_to_y.entry(bridge.add1).or_default().push(y);
            added_to_y.entry(bridge.add2).or_default().push(y);
        }

        for (_, lits) in removed_to_y {
            for p in 0..lits.len() {
                for q in (p + 1)..lits.len() {
                    cnf.add_clause(clause![!lits[p], !lits[q]]);
                }
            }
        }

        for (_, lits) in added_to_y {
            for p in 0..lits.len() {
                for q in (p + 1)..lits.len() {
                    cnf.add_clause(clause![!lits[p], !lits[q]]);
                }
            }
        }

        // MTZ Ladder Order Constraints
        for c_idx in 1..k {
            let o_v = &ladder[&c_idx];
            for r in 0..(o_v.len().saturating_sub(1)) {
                cnf.add_clause(clause![!o_v[r + 1], o_v[r]]);
            }
        }

        let add_mtz_transition = |cnf: &mut Cnf, u: usize, v: usize, dir_lit: Lit, ladder: &HashMap<usize, Vec<Lit>>| {
            if v == 0 {
                return;
            }
            let o_v = &ladder[&v];
            if u == 0 {
                cnf.add_clause(clause![!dir_lit, o_v[0]]);
            } else {
                let o_u = &ladder[&u];
                cnf.add_clause(clause![!dir_lit, o_v[0]]);
                for r in 0..(k.saturating_sub(2)) {
                    cnf.add_clause(clause![!dir_lit, !o_u[r], o_v[r + 1]]);
                }
                if k >= 2 {
                    let max_idx = k - 2;
                    cnf.add_clause(clause![!dir_lit, !o_u[max_idx]]);
                }
            }
        };

        for (b_idx, bridge) in candidate_bridges.iter().enumerate() {
            let (d_12, d_21) = d_vars[&b_idx];
            add_mtz_transition(&mut cnf, bridge.c1, bridge.c2, d_12, &ladder);
            add_mtz_transition(&mut cnf, bridge.c2, bridge.c1, d_21, &ladder);
        }

        // Exact Spanning Tree Cardinality: sum(y_m) <= k - 1 encoded via Sinz sequential counter
        Self::add_at_most_c(&mut cnf, &mut next_var_id, &y_vars, k - 1);

        // 3. Solving & Tour Reconstruction
        let mut solver = CaDiCaL::default();
        if solver.add_cnf_ref(&cnf).is_err() {
            return None;
        }

        let res = solver.solve();
        if let Ok(SolverResult::Sat) = res {
            if let Ok(sol) = solver.full_solution() {
                let mut chosen_bridges: Vec<Bridge> = Vec::new();
                for (b_idx, bridge) in candidate_bridges.iter().enumerate() {
                    if sol.lit_value(y_vars[b_idx]) == TernaryVal::True {
                        chosen_bridges.push(bridge.clone());
                    }
                }

                if chosen_bridges.len() != k - 1 {
                    return None;
                }

                return Self::reconstruct_single_cycle(cycles, &chosen_bridges, g);
            }
        }

        None
    }

    /// Enforces sum(lits) <= c using Sinz sequential counter.
    fn add_at_most_c(cnf: &mut Cnf, next_var_id: &mut u32, lits: &[Lit], c: usize) {
        let n = lits.len();
        if c >= n {
            return;
        }
        if c == 0 {
            for &l in lits {
                cnf.add_clause(clause![!l]);
            }
            return;
        }

        let mut s: Vec<Vec<Lit>> = Vec::with_capacity(n - 1);
        for _ in 0..(n - 1) {
            let mut row = Vec::with_capacity(c);
            for _ in 0..c {
                let lit = Var::new(*next_var_id).pos_lit();
                *next_var_id += 1;
                row.push(lit);
            }
            s.push(row);
        }

        cnf.add_clause(clause![!lits[0], s[0][0]]);
        for j in 1..c {
            cnf.add_clause(clause![!s[0][j]]);
        }

        for i in 1..(n - 1) {
            for j in 0..c {
                cnf.add_clause(clause![!s[i - 1][j], s[i][j]]);
            }
            cnf.add_clause(clause![!lits[i], s[i][0]]);
            for j in 1..c {
                cnf.add_clause(clause![!lits[i], !s[i - 1][j - 1], s[i][j]]);
            }
            cnf.add_clause(clause![!lits[i], !s[i - 1][c - 1]]);
        }

        cnf.add_clause(clause![!lits[n - 1], !s[n - 2][c - 1]]);
    }

    /// Reconstructs the single Hamiltonian tour from chosen bridges and validates 2-regularity.
    fn reconstruct_single_cycle(
        cycles: &[Vec<i32>],
        chosen_bridges: &[Bridge],
        g: &Graph,
    ) -> Option<Vec<i32>> {
        let total_v: usize = cycles.iter().map(|c| c.len()).sum();
        let mut removed_edges: HashSet<(i32, i32)> = HashSet::with_capacity(chosen_bridges.len() * 2);
        let mut added_edges: Vec<(i32, i32)> = Vec::with_capacity(chosen_bridges.len() * 2);

        for bridge in chosen_bridges {
            removed_edges.insert(bridge.rem1);
            removed_edges.insert(bridge.rem2);
            added_edges.push(bridge.add1);
            added_edges.push(bridge.add2);
        }

        let mut adj: HashMap<i32, Vec<i32>> = HashMap::with_capacity(total_v);

        // Add remaining cycle edges
        for cycle in cycles {
            let n = cycle.len();
            for pos in 0..n {
                let u = cycle[pos];
                let v = cycle[(pos + 1) % n];
                let e = min_max(u, v);
                if !removed_edges.contains(&e) {
                    adj.entry(u).or_default().push(v);
                    adj.entry(v).or_default().push(u);
                }
            }
        }

        // Add cross edges
        for &(u, v) in &added_edges {
            adj.entry(u).or_default().push(v);
            adj.entry(v).or_default().push(u);
        }

        if adj.len() != total_v {
            return None;
        }

        // Validate 2-regularity and graph edge existence
        for (&u, nbrs) in &adj {
            if nbrs.len() != 2 {
                return None;
            }
            if nbrs[0] == nbrs[1] || nbrs[0] == u || nbrs[1] == u {
                return None;
            }
            if let Some(g_nbrs) = g.adjacency_list.get(&u) {
                if !g_nbrs.contains(&nbrs[0]) || !g_nbrs.contains(&nbrs[1]) {
                    return None;
                }
            } else {
                return None;
            }
        }

        // Extract single tour
        let start_v = cycles[0][0];
        let mut tour = Vec::with_capacity(total_v);
        let mut visited = HashSet::with_capacity(total_v);
        let mut curr = start_v;
        let mut prev: Option<i32> = None;

        loop {
            visited.insert(curr);
            tour.push(curr);

            let nbrs = match adj.get(&curr) {
                Some(n) => n,
                None => return None,
            };

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

        if tour.len() != total_v || visited.len() != total_v {
            return None;
        }

        Some(tour)
    }
}
