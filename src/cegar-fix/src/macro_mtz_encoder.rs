use std::collections::{HashMap, HashSet};
use crate::two_tier_decomposer::DecompositionResult;
use rustsat::clause;
use rustsat::solvers::Solve;
use rustsat::types::{Lit, Var};
use rustsat_cadical::CaDiCaL;

#[derive(Debug, Clone)]
pub struct MacroMtzEncoder {
    pub root_hub: i32,
    pub hub_order_vars: HashMap<i32, Vec<Lit>>,
    pub dir_hh_vars: HashMap<(i32, i32), Lit>,
    pub dir_strip_vars: HashMap<(usize, i32, i32), Lit>,
}

impl MacroMtzEncoder {
    /// Encodes Miller-Tucker-Zemlin (MTZ) unary ladder order constraints and directed macro
    /// transitions into CaDiCaL to guarantee that the hub-level macro solution forms a single connected cycle.
    pub fn encode(
        solver: &mut CaDiCaL<'static, 'static>,
        next_var_id: &mut u32,
        decomp: &DecompositionResult,
        var_hh: &HashMap<(i32, i32), Lit>,
        var_d1: &HashMap<(usize, i32), Lit>,
    ) -> Self {
        let mut sorted_hubs: Vec<i32> = decomp.all_hubs.iter().copied().collect();
        sorted_hubs.sort_unstable();

        if sorted_hubs.is_empty() {
            return Self {
                root_hub: -1,
                hub_order_vars: HashMap::new(),
                dir_hh_vars: HashMap::new(),
                dir_strip_vars: HashMap::new(),
            };
        }

        let root_hub = sorted_hubs[0];
        let n_h = sorted_hubs.len();

        let mut hub_order_vars: HashMap<i32, Vec<Lit>> = HashMap::new();
        let mut dir_hh_vars: HashMap<(i32, i32), Lit> = HashMap::new();
        let mut dir_strip_vars: HashMap<(usize, i32, i32), Lit> = HashMap::new();

        // 1. Unary Order Variables for non-root hubs (k = 1 .. N_H - 1)
        if n_h >= 2 {
            for &h in &sorted_hubs[1..] {
                let mut o_h = Vec::with_capacity(n_h - 1);
                for _ in 0..(n_h - 1) {
                    let lit = Var::new(*next_var_id).pos_lit();
                    *next_var_id += 1;
                    o_h.push(lit);
                }

                // Monotonicity clauses: !O_{h, k+1} \/ O_{h, k} for k = 1 .. N_H - 2
                for idx in 0..(o_h.len().saturating_sub(1)) {
                    let _ = solver.add_clause(clause![!o_h[idx + 1], o_h[idx]]);
                }

                hub_order_vars.insert(h, o_h);
            }
        }

        // Helper to add MTZ implication clauses for a directed transition u -> v
        let add_mtz_implication = |solver: &mut CaDiCaL<'static, 'static>,
                                   u: i32,
                                   v: i32,
                                   e_uv: Lit,
                                   order_vars: &HashMap<i32, Vec<Lit>>| {
            if u == root_hub && v != root_hub {
                if let Some(o_v) = order_vars.get(&v) {
                    let _ = solver.add_clause(clause![!e_uv, o_v[0]]);
                }
            } else if u != root_hub && v != root_hub {
                if let (Some(o_u), Some(o_v)) = (order_vars.get(&u), order_vars.get(&v)) {
                    // !e_{u->v} \/ O_{v, 1}
                    let _ = solver.add_clause(clause![!e_uv, o_v[0]]);
                    // For k = 1 .. N_H - 2: !e_{u->v} \/ !O_{u, k} \/ O_{v, k+1}
                    for idx in 0..(n_h.saturating_sub(2)) {
                        let _ = solver.add_clause(clause![!e_uv, !o_u[idx], o_v[idx + 1]]);
                    }
                    // If u transitions to v (both non-root), u cannot be at the maximum position N_H - 1:
                    // !e_{u->v} \/ !O_{u, N_H - 1}
                    if n_h >= 2 {
                        let max_idx = n_h - 2;
                        let _ = solver.add_clause(clause![!e_uv, !o_u[max_idx]]);
                    }
                }
            }
        };

        // 2. Directed Hub-Hub transitions: for each undirected (u, v) in decomp.hh_edges with u < v
        let mut hh_pairs = HashSet::new();
        for &(a, b) in &decomp.hh_edges {
            if a != b {
                let u = a.min(b);
                let v = a.max(b);
                hh_pairs.insert((u, v));
            }
        }
        let mut sorted_hh_pairs: Vec<(i32, i32)> = hh_pairs.into_iter().collect();
        sorted_hh_pairs.sort_unstable();

        for &(u, v) in &sorted_hh_pairs {
            let x_u_to_v = Var::new(*next_var_id).pos_lit();
            *next_var_id += 1;
            let x_v_to_u = Var::new(*next_var_id).pos_lit();
            *next_var_id += 1;

            dir_hh_vars.insert((u, v), x_u_to_v);
            dir_hh_vars.insert((v, u), x_v_to_u);

            // Link directed variables with undirected x_uv:
            // !x_{u->v} \/ x_{uv}
            // !x_{v->u} \/ x_{uv}
            // !x_{u->v} \/ !x_{v->u}
            // !x_{uv} \/ x_{u->v} \/ x_{v->u}
            if let Some(&x_uv) = var_hh.get(&(u, v)).or_else(|| var_hh.get(&(v, u))) {
                let _ = solver.add_clause(clause![!x_u_to_v, x_uv]);
                let _ = solver.add_clause(clause![!x_v_to_u, x_uv]);
                let _ = solver.add_clause(clause![!x_u_to_v, !x_v_to_u]);
                let _ = solver.add_clause(clause![!x_uv, x_u_to_v, x_v_to_u]);
            }

            // MTZ implications
            add_mtz_implication(solver, u, v, x_u_to_v, &hub_order_vars);
            add_mtz_implication(solver, v, u, x_v_to_u, &hub_order_vars);
        }

        // 3. Directed Strip transitions: for each strip si and adjacent hubs u, v (u != v)
        let mut strip_keys: Vec<usize> = decomp.strip_adj_hubs.keys().copied().collect();
        strip_keys.sort_unstable();

        for si in strip_keys {
            if let Some(adj) = decomp.strip_adj_hubs.get(&si) {
                let mut sorted_adj: Vec<i32> = adj.iter().copied().collect();
                sorted_adj.sort_unstable();

                for &u in &sorted_adj {
                    for &v in &sorted_adj {
                        if u == v {
                            continue;
                        }
                        let s_uv = Var::new(*next_var_id).pos_lit();
                        *next_var_id += 1;

                        dir_strip_vars.insert((si, u, v), s_uv);

                        // Link: !s_{si, u->v} \/ d_1(si, u), !s_{si, u->v} \/ d_1(si, v)
                        if let Some(&d1_u) = var_d1.get(&(si, u)) {
                            let _ = solver.add_clause(clause![!s_uv, d1_u]);
                        }
                        if let Some(&d1_v) = var_d1.get(&(si, v)) {
                            let _ = solver.add_clause(clause![!s_uv, d1_v]);
                        }

                        // MTZ implications
                        add_mtz_implication(solver, u, v, s_uv, &hub_order_vars);
                    }
                }

                // If strip endpoints u and v are both active, exactly one traversal direction must be chosen
                for i in 0..sorted_adj.len() {
                    for j in (i + 1)..sorted_adj.len() {
                        let u = sorted_adj[i];
                        let v = sorted_adj[j];
                        if let (Some(&d1_u), Some(&d1_v)) = (var_d1.get(&(si, u)), var_d1.get(&(si, v))) {
                            if let (Some(&s_uv), Some(&s_vu)) = (dir_strip_vars.get(&(si, u, v)), dir_strip_vars.get(&(si, v, u))) {
                                let _ = solver.add_clause(clause![!d1_u, !d1_v, s_uv, s_vu]);
                                let _ = solver.add_clause(clause![!s_uv, !s_vu]);
                            }
                        }
                    }
                }
            }
        }

        // 4. In/Out degree at-most-one constraints on directed macro transitions at each hub
        for &h in &sorted_hubs {
            let mut out_lits = Vec::new();
            let mut in_lits = Vec::new();

            // HH transitions
            for &other in &sorted_hubs {
                if other != h {
                    if let Some(&x_out) = dir_hh_vars.get(&(h, other)) {
                        out_lits.push(x_out);
                    }
                    if let Some(&x_in) = dir_hh_vars.get(&(other, h)) {
                        in_lits.push(x_in);
                    }
                }
            }

            // Strip transitions: check all strips
            let mut strip_keys: Vec<usize> = decomp.strip_adj_hubs.keys().copied().collect();
            strip_keys.sort_unstable();
            for si in strip_keys {
                if let Some(adj_hubs) = decomp.strip_adj_hubs.get(&si) {
                    if adj_hubs.contains(&h) {
                        let mut sorted_other: Vec<i32> = adj_hubs.iter().copied().filter(|&x| x != h).collect();
                        sorted_other.sort_unstable();
                        for other in sorted_other {
                            if let Some(&s_out) = dir_strip_vars.get(&(si, h, other)) {
                                out_lits.push(s_out);
                            }
                            if let Some(&s_in) = dir_strip_vars.get(&(si, other, h)) {
                                in_lits.push(s_in);
                            }
                        }
                    }
                }
            }

            // At-most-one outgoing: for all i < j, !out_lits[i] \/ !out_lits[j]
            for i in 0..out_lits.len() {
                for j in (i + 1)..out_lits.len() {
                    let _ = solver.add_clause(clause![!out_lits[i], !out_lits[j]]);
                }
            }

            // At-most-one incoming: for all i < j, !in_lits[i] \/ !in_lits[j]
            for i in 0..in_lits.len() {
                for j in (i + 1)..in_lits.len() {
                    let _ = solver.add_clause(clause![!in_lits[i], !in_lits[j]]);
                }
            }
        }

        Self {
            root_hub,
            hub_order_vars,
            dir_hh_vars,
            dir_strip_vars,
        }
    }
}
