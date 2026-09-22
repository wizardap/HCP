use std::collections::{HashMap, HashSet, VecDeque};
use crate::graph::Graph;
use rustsat::clause;
use rustsat::instances::Cnf;
use rustsat::solvers::{LimitConflicts, Solve, SolverResult};
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

#[derive(Debug, Clone)]
struct Bridge {
    id: usize,
    c1: usize,
    c2: usize,
    e1: (i32, i32),
    e2: (i32, i32),
    x1: (i32, i32),
    x2: (i32, i32),
}

pub struct TransitiveMacroSplicer;

impl TransitiveMacroSplicer {
    /// Attempts transitive global macro-graph splicing across all m cycles.
    /// Returns the reduced cycle list (or a single Hamiltonian tour if all cycles are spliced).
    pub fn splice_transitive_macro_graph(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
    ) -> Vec<Vec<i32>> {
        if cycles.len() <= 1 {
            return cycles.to_vec();
        }

        // Validate cycle lengths
        for c in cycles {
            if c.len() < 3 {
                return cycles.to_vec();
            }
        }

        let mut current_cycles = cycles.to_vec();
        let max_passes = 6;

        for _ in 0..max_passes {
            if current_cycles.len() <= 1 {
                break;
            }
            let next_cycles = Self::splice_one_pass(&current_cycles, g, protected_edges);
            if next_cycles.len() < current_cycles.len() {
                current_cycles = next_cycles;
            } else {
                break;
            }
        }

        current_cycles
    }

    /// Single pass: constructs macro-graph, finds connected components, solves exact SAT spanning forest,
    /// and reconstructs spliced cycles.
    fn splice_one_pass(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
    ) -> Vec<Vec<i32>> {
        let m = cycles.len();
        if m <= 1 {
            return cycles.to_vec();
        }

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

        // 1. Candidate 2-opt Bridge Discovery
        let mut bridges: Vec<Bridge> = Vec::new();
        let mut seen_bridges: HashSet<(usize, usize, (i32, i32), (i32, i32), (i32, i32), (i32, i32))> =
            HashSet::new();

        for i in 0..m {
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
                        if let Some(&j) = vertex_to_cycle.get(&v1) {
                            if j == i {
                                continue;
                            }

                            let [v_prev, v_next] = cycle_neighbors[&v1];

                            // Candidate 1: v2 = v_next
                            let v2_next = v_next;
                            let e_j_next = min_max(v1, v2_next);
                            if !canonical_protected.contains(&e_j_next) {
                                if let Some(u2_nbrs) = g.adjacency_list.get(&u2) {
                                    if u2_nbrs.contains(&v2_next) {
                                        let x1 = min_max(u1, v1);
                                        let x2 = min_max(u2, v2_next);
                                        let (c1, c2, e1, e2) =
                                            if i < j { (i, j, e_i, e_j_next) } else { (j, i, e_j_next, e_i) };
                                        let (xa, xb) = if x1 < x2 { (x1, x2) } else { (x2, x1) };
                                        let key = (c1, c2, e1, e2, xa, xb);
                                        if seen_bridges.insert(key) {
                                            bridges.push(Bridge {
                                                id: bridges.len(),
                                                c1,
                                                c2,
                                                e1,
                                                e2,
                                                x1: xa,
                                                x2: xb,
                                            });
                                        }
                                    }
                                }
                            }

                            // Candidate 2: v2 = v_prev
                            let v2_prev = v_prev;
                            let e_j_prev = min_max(v1, v2_prev);
                            if !canonical_protected.contains(&e_j_prev) {
                                if let Some(u2_nbrs) = g.adjacency_list.get(&u2) {
                                    if u2_nbrs.contains(&v2_prev) {
                                        let x1 = min_max(u1, v1);
                                        let x2 = min_max(u2, v2_prev);
                                        let (c1, c2, e1, e2) =
                                            if i < j { (i, j, e_i, e_j_prev) } else { (j, i, e_j_prev, e_i) };
                                        let (xa, xb) = if x1 < x2 { (x1, x2) } else { (x2, x1) };
                                        let key = (c1, c2, e1, e2, xa, xb);
                                        if seen_bridges.insert(key) {
                                            bridges.push(Bridge {
                                                id: bridges.len(),
                                                c1,
                                                c2,
                                                e1,
                                                e2,
                                                x1: xa,
                                                x2: xb,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if bridges.is_empty() {
            return cycles.to_vec();
        }

        // 2. Macro-Graph Connected Components
        let mut macro_adj: Vec<Vec<usize>> = vec![Vec::new(); m];
        for b in &bridges {
            macro_adj[b.c1].push(b.c2);
            macro_adj[b.c2].push(b.c1);
        }

        let mut comp_visited = vec![false; m];
        let mut components: Vec<Vec<usize>> = Vec::new();

        for i in 0..m {
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

        // 3. Solve exact SAT spanning tree/forest per component
        let mut all_selected_bridges: Vec<Bridge> = Vec::new();

        for comp in &components {
            if comp.len() < 2 {
                continue;
            }
            if let Some(selected_for_comp) = Self::solve_component_spanning_forest(comp, &bridges) {
                all_selected_bridges.extend(selected_for_comp);
            }
        }

        if all_selected_bridges.is_empty() {
            return cycles.to_vec();
        }

        // 4. Apply 2-opt swaps and reconstruct cycles
        if let Some(new_cycles) = Self::reconstruct_spliced_cycles(cycles, &all_selected_bridges, g) {
            if new_cycles.len() < cycles.len() {
                return new_cycles;
            }
        }

        cycles.to_vec()
    }

    /// Exact SAT formulation for finding the maximum-size spanning forest on a macro-graph component.
    fn solve_component_spanning_forest(
        comp: &[usize],
        bridges: &[Bridge],
    ) -> Option<Vec<Bridge>> {
        let n = comp.len();
        if n < 2 {
            return None;
        }

        let mut node_to_loc: HashMap<usize, usize> = HashMap::with_capacity(n);
        for (loc, &c_idx) in comp.iter().enumerate() {
            node_to_loc.insert(c_idx, loc);
        }

        let mut comp_bridges: Vec<Bridge> = Vec::new();
        for b in bridges {
            if node_to_loc.contains_key(&b.c1) && node_to_loc.contains_key(&b.c2) {
                comp_bridges.push(b.clone());
            }
        }

        if comp_bridges.is_empty() {
            return None;
        }

        let mut next_var_id: u32 = 0;

        // Variables:
        // For each bridge b: B_b, D_{b, c1 -> c2}, D_{b, c2 -> c1}
        let mut b_var: HashMap<usize, Lit> = HashMap::new();
        let mut d_c1_to_c2: HashMap<usize, Lit> = HashMap::new();
        let mut d_c2_to_c1: HashMap<usize, Lit> = HashMap::new();

        for b in &comp_bridges {
            b_var.insert(b.id, Var::new(next_var_id).pos_lit());
            next_var_id += 1;
            d_c1_to_c2.insert(b.id, Var::new(next_var_id).pos_lit());
            next_var_id += 1;
            d_c2_to_c1.insert(b.id, Var::new(next_var_id).pos_lit());
            next_var_id += 1;
        }

        // For each non-root node (loc in 1..n):
        // Ladder order variables O_{loc, 1 .. n-1}
        let mut ladder: HashMap<usize, Vec<Lit>> = HashMap::new();
        let mut att_var: HashMap<usize, Lit> = HashMap::new();

        for loc in 1..n {
            let mut row = Vec::with_capacity(n - 1);
            for _ in 0..(n - 1) {
                let lit = Var::new(next_var_id).pos_lit();
                next_var_id += 1;
                row.push(lit);
            }
            ladder.insert(loc, row);

            let a_lit = Var::new(next_var_id).pos_lit();
            next_var_id += 1;
            att_var.insert(loc, a_lit);
        }

        let mut base_cnf = Cnf::new();

        // 1. Bridge direction constraints
        for b in &comp_bridges {
            let lit_b = b_var[&b.id];
            let lit_12 = d_c1_to_c2[&b.id];
            let lit_21 = d_c2_to_c1[&b.id];

            base_cnf.add_clause(clause![!lit_12, lit_b]);
            base_cnf.add_clause(clause![!lit_21, lit_b]);
            base_cnf.add_clause(clause![!lit_b, lit_12, lit_21]);
            base_cnf.add_clause(clause![!lit_12, !lit_21]);
        }

        // 2. Cycle edge AMO
        let mut edge_to_b_lits: HashMap<(i32, i32), Vec<Lit>> = HashMap::new();
        for b in &comp_bridges {
            edge_to_b_lits.entry(b.e1).or_default().push(b_var[&b.id]);
            edge_to_b_lits.entry(b.e2).or_default().push(b_var[&b.id]);
        }

        for (_, lits) in edge_to_b_lits {
            for p in 0..lits.len() {
                for q in (p + 1)..lits.len() {
                    base_cnf.add_clause(clause![!lits[p], !lits[q]]);
                }
            }
        }

        // 3. Root node incoming constraint: Root loc = 0 has 0 incoming directed edges
        for b in &comp_bridges {
            let u_loc = node_to_loc[&b.c1];
            let v_loc = node_to_loc[&b.c2];
            if u_loc == 0 {
                base_cnf.add_clause(clause![!d_c2_to_c1[&b.id]]);
            }
            if v_loc == 0 {
                base_cnf.add_clause(clause![!d_c1_to_c2[&b.id]]);
            }
        }

        // 4. Non-root nodes incoming edges and attached variable
        let mut in_lits: HashMap<usize, Vec<Lit>> = HashMap::new();
        for loc in 1..n {
            in_lits.insert(loc, Vec::new());
        }

        for b in &comp_bridges {
            let u_loc = node_to_loc[&b.c1];
            let v_loc = node_to_loc[&b.c2];
            if u_loc != 0 {
                in_lits.get_mut(&u_loc).unwrap().push(d_c2_to_c1[&b.id]);
            }
            if v_loc != 0 {
                in_lits.get_mut(&v_loc).unwrap().push(d_c1_to_c2[&b.id]);
            }
        }

        for loc in 1..n {
            let incoming = &in_lits[&loc];
            let a_lit = att_var[&loc];

            // AMO incoming
            for p in 0..incoming.len() {
                for q in (p + 1)..incoming.len() {
                    base_cnf.add_clause(clause![!incoming[p], !incoming[q]]);
                }
            }

            // Attached variable linking: a_lit <=> \/ incoming
            if incoming.is_empty() {
                base_cnf.add_clause(clause![!a_lit]);
            } else {
                for &d_lit in incoming {
                    base_cnf.add_clause(clause![!d_lit, a_lit]);
                }
                let mut cl = vec![!a_lit];
                cl.extend_from_slice(incoming);
                base_cnf.add_clause(Clause::from_iter(cl));
            }
        }

        // 5. MTZ ladder acyclicity constraints
        for loc in 1..n {
            let o_v = &ladder[&loc];
            for k in 0..(o_v.len().saturating_sub(1)) {
                base_cnf.add_clause(clause![!o_v[k + 1], o_v[k]]);
            }
        }

        for b in &comp_bridges {
            let u_loc = node_to_loc[&b.c1];
            let v_loc = node_to_loc[&b.c2];

            // Direction u_loc -> v_loc
            if v_loc != 0 {
                let lit_uv = d_c1_to_c2[&b.id];
                let o_v = &ladder[&v_loc];
                if u_loc == 0 {
                    base_cnf.add_clause(clause![!lit_uv, o_v[0]]);
                } else {
                    let o_u = &ladder[&u_loc];
                    base_cnf.add_clause(clause![!lit_uv, o_v[0]]);
                    for k in 0..(n.saturating_sub(2)) {
                        base_cnf.add_clause(clause![!lit_uv, !o_u[k], o_v[k + 1]]);
                    }
                    if n >= 2 {
                        let max_idx = n - 2;
                        base_cnf.add_clause(clause![!lit_uv, !o_u[max_idx]]);
                    }
                }
            }

            // Direction v_loc -> u_loc
            if u_loc != 0 {
                let lit_vu = d_c2_to_c1[&b.id];
                let o_u = &ladder[&u_loc];
                if v_loc == 0 {
                    base_cnf.add_clause(clause![!lit_vu, o_u[0]]);
                } else {
                    let o_v = &ladder[&v_loc];
                    base_cnf.add_clause(clause![!lit_vu, o_u[0]]);
                    for k in 0..(n.saturating_sub(2)) {
                        base_cnf.add_clause(clause![!lit_vu, !o_v[k], o_u[k + 1]]);
                    }
                    if n >= 2 {
                        let max_idx = n - 2;
                        base_cnf.add_clause(clause![!lit_vu, !o_v[max_idx]]);
                    }
                }
            }
        }

        // 6. Solve with target attached nodes k from num_att down to 1
        let att_lits: Vec<Lit> = (1..n).map(|loc| att_var[&loc]).collect();
        let num_att = att_lits.len();
        let target_ks: Vec<usize> = if num_att <= 4 {
            (1..=num_att).rev().collect()
        } else {
            let mut targets = vec![num_att];
            let t34 = (num_att * 3) / 4;
            if t34 > 1 && t34 < num_att { targets.push(t34); }
            let t12 = num_att / 2;
            if t12 > 1 && t12 < t34 { targets.push(t12); }
            if !targets.contains(&1) { targets.push(1); }
            targets
        };

        for k in target_ks {
            let mut solver = CaDiCaL::default();
            let _ = solver.limit_conflicts(Some(2000));
            let mut cnf = base_cnf.clone();
            let c = num_att - k; // At most c unattached nodes allowed

            if c == 0 {
                // All non-root nodes must be attached
                for &a_lit in &att_lits {
                    cnf.add_clause(clause![a_lit]);
                }
            } else {
                // Sinz Sequential Counter on neg_att_lits: sum(!A_v) <= c
                let neg_lits: Vec<Lit> = att_lits.iter().map(|&l| !l).collect();
                Self::add_at_most_c(&mut cnf, &mut next_var_id, &neg_lits, c);
            }

            if solver.add_cnf_ref(&cnf).is_ok() {
                let res = solver.solve();
                if let Ok(SolverResult::Sat) = res {
                    if let Ok(sol) = solver.full_solution() {
                        let mut selected: Vec<Bridge> = Vec::new();
                        for b in &comp_bridges {
                            if sol.lit_value(b_var[&b.id]) == TernaryVal::True {
                                selected.push(b.clone());
                            }
                        }
                        if !selected.is_empty() {
                            return Some(selected);
                        }
                    }
                }
            }
        }

        None
    }

    /// Adds cardinality constraints to enforce sum(lits) <= c using Sinz sequential counter.
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

    /// Reconstructs new 2-factor cycles from current cycles and selected bridges.
    /// Validates 2-regularity, vertex set preservation, and edge validity.
    fn reconstruct_spliced_cycles(
        cycles: &[Vec<i32>],
        selected_bridges: &[Bridge],
        g: &Graph,
    ) -> Option<Vec<Vec<i32>>> {
        let total_v: usize = cycles.iter().map(|c| c.len()).sum();
        let mut removed_edges: HashSet<(i32, i32)> = HashSet::new();
        let mut added_edges: Vec<(i32, i32)> = Vec::new();

        for b in selected_bridges {
            removed_edges.insert(b.e1);
            removed_edges.insert(b.e2);
            added_edges.push(b.x1);
            added_edges.push(b.x2);
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

        // Check 2-regularity and graph edge validity
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

        // Extract cycles
        let mut visited: HashSet<i32> = HashSet::with_capacity(total_v);
        let mut new_cycles: Vec<Vec<i32>> = Vec::new();

        let mut all_verts: Vec<i32> = adj.keys().copied().collect();
        all_verts.sort_unstable();

        for &start_v in &all_verts {
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

        new_cycles.sort_by(|a, b| {
            b.len()
                .cmp(&a.len())
                .then_with(|| a.iter().min().cmp(&b.iter().min()))
        });

        Some(new_cycles)
    }
}
