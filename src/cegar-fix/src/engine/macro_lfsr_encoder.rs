use std::collections::HashMap;
use crate::two_tier_decomposer::DecompositionResult;
use rustsat::clause;
use rustsat::solvers::Solve;
use rustsat::types::{Clause, Lit, Var};
use rustsat_cadical::CaDiCaL;

#[derive(Debug, Clone)]
pub struct LfsrSpec {
    pub num_bits: usize,
    pub tap: usize, // XOR tap position with (num_bits - 1)
    pub period: usize,
}

#[derive(Debug, Clone)]
pub struct MacroLfsrEncoder {
    pub root_hub: i32,
    pub specs: Vec<LfsrSpec>,
    pub bit_vars: HashMap<(i32, usize), Vec<Lit>>, // (hub, spec_idx) -> Vec<Lit> of length num_bits
    pub dir_hh_vars: HashMap<(i32, i32), Lit>,
    pub dir_strip_vars: HashMap<(usize, i32, i32), Lit>,
}

impl MacroLfsrEncoder {
    /// Encodes Linear-Feedback Shift Register (LFSR) modular position registers inspired by
    /// Marijn Heule's encoding in SAT_HCP.
    /// Provides maximal-period modular state tracking with minimal boolean variables and XOR gates.
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
                specs: Vec::new(),
                bit_vars: HashMap::new(),
                dir_hh_vars: HashMap::new(),
                dir_strip_vars: HashMap::new(),
            };
        }

        let root_hub = sorted_hubs[0];
        let n_h = sorted_hubs.len();

        // Choose coprime LFSR specifications whose period product > N_H
        // 1. 3-bit LFSR: period 7 (1 + x^2 + x^3, tap=1)
        // 2. 4-bit LFSR: period 15 (1 + x^3 + x^4, tap=2)
        // 3. 5-bit LFSR: period 31 (1 + x^3 + x^5, tap=2) -> Product 7 * 15 * 31 = 3,255 >> N_H
        let specs = if n_h <= 100 {
            vec![
                LfsrSpec { num_bits: 3, tap: 1, period: 7 },
                LfsrSpec { num_bits: 4, tap: 2, period: 15 },
            ] // Product = 105
        } else if n_h <= 500 {
            vec![
                LfsrSpec { num_bits: 3, tap: 1, period: 7 },
                LfsrSpec { num_bits: 4, tap: 2, period: 15 },
                LfsrSpec { num_bits: 5, tap: 2, period: 31 },
            ] // Product = 3,255
        } else {
            vec![
                LfsrSpec { num_bits: 4, tap: 2, period: 15 },
                LfsrSpec { num_bits: 5, tap: 2, period: 31 },
                LfsrSpec { num_bits: 6, tap: 4, period: 63 },
            ] // Product = 29,295
        };

        let mut bit_vars: HashMap<(i32, usize), Vec<Lit>> = HashMap::new();
        let mut dir_hh_vars: HashMap<(i32, i32), Lit> = HashMap::new();
        let mut dir_strip_vars: HashMap<(usize, i32, i32), Lit> = HashMap::new();

        // 1. Allocate LFSR state bits for each hub
        for &h in &sorted_hubs {
            for (s_idx, spec) in specs.iter().enumerate() {
                let mut bits = Vec::with_capacity(spec.num_bits);
                for _ in 0..spec.num_bits {
                    let lit = Var::new(*next_var_id).pos_lit();
                    *next_var_id += 1;
                    bits.push(lit);
                }

                // Non-zero constraint: At least one bit must be true
                let _ = solver.add_clause(Clause::from_iter(bits.iter().copied()));

                // Root hub is pinned to state (00...01) = bit[0] true, other bits false
                if h == root_hub {
                    let _ = solver.add_clause(clause![bits[0]]);
                    for b in 1..spec.num_bits {
                        let _ = solver.add_clause(clause![!bits[b]]);
                    }
                }

                bit_vars.insert((h, s_idx), bits);
            }
        }

        // 2. Directed Hub-Hub transition variables and LFSR stepping
        for &(u, v) in &decomp.hh_edges {
            let lit_u_to_v = Var::new(*next_var_id).pos_lit();
            *next_var_id += 1;
            let lit_v_to_u = Var::new(*next_var_id).pos_lit();
            *next_var_id += 1;

            dir_hh_vars.insert((u, v), lit_u_to_v);
            dir_hh_vars.insert((v, u), lit_v_to_u);

            if let Some(&x_uv) = var_hh.get(&(u, v)) {
                let _ = solver.add_clause(clause![!x_uv, lit_u_to_v, lit_v_to_u]);
                let _ = solver.add_clause(clause![!lit_u_to_v, x_uv]);
                let _ = solver.add_clause(clause![!lit_v_to_u, x_uv]);
                let _ = solver.add_clause(clause![!lit_u_to_v, !lit_v_to_u]);

                Self::add_lfsr_transitions(solver, &specs, &bit_vars, u, v, root_hub, lit_u_to_v);
                Self::add_lfsr_transitions(solver, &specs, &bit_vars, v, u, root_hub, lit_v_to_u);
            }
        }

        // 3. Directed Strip transition variables and LFSR stepping
        for (si, _strip) in decomp.strips.iter().enumerate() {
            if let Some(adj) = decomp.strip_adj_hubs.get(&si) {
                let mut sorted_adj: Vec<i32> = adj.iter().copied().collect();
                sorted_adj.sort_unstable();

                for i in 0..sorted_adj.len() {
                    for j in (i + 1)..sorted_adj.len() {
                        let u = sorted_adj[i];
                        let v = sorted_adj[j];

                        let lit_u_to_v = Var::new(*next_var_id).pos_lit();
                        *next_var_id += 1;
                        let lit_v_to_u = Var::new(*next_var_id).pos_lit();
                        *next_var_id += 1;

                        dir_strip_vars.insert((si, u, v), lit_u_to_v);
                        dir_strip_vars.insert((si, v, u), lit_v_to_u);

                        let d1_u = var_d1.get(&(si, u)).copied();
                        let d1_v = var_d1.get(&(si, v)).copied();

                        if let (Some(du), Some(dv)) = (d1_u, d1_v) {
                            let _ = solver.add_clause(clause![!lit_u_to_v, du]);
                            let _ = solver.add_clause(clause![!lit_u_to_v, dv]);
                            let _ = solver.add_clause(clause![!lit_v_to_u, du]);
                            let _ = solver.add_clause(clause![!lit_v_to_u, dv]);
                            let _ = solver.add_clause(clause![!lit_u_to_v, !lit_v_to_u]);

                            Self::add_lfsr_transitions(solver, &specs, &bit_vars, u, v, root_hub, lit_u_to_v);
                            Self::add_lfsr_transitions(solver, &specs, &bit_vars, v, u, root_hub, lit_v_to_u);
                        }
                    }
                }
            }
        }

        // 4. Exact-1 In / Out transitions for every hub
        for &u in &sorted_hubs {
            let mut out_lits = Vec::new();
            let mut in_lits = Vec::new();

            for &(h1, h2) in &decomp.hh_edges {
                if h1 == u {
                    if let Some(&lit) = dir_hh_vars.get(&(h1, h2)) { out_lits.push(lit); }
                    if let Some(&lit) = dir_hh_vars.get(&(h2, h1)) { in_lits.push(lit); }
                } else if h2 == u {
                    if let Some(&lit) = dir_hh_vars.get(&(h2, h1)) { out_lits.push(lit); }
                    if let Some(&lit) = dir_hh_vars.get(&(h1, h2)) { in_lits.push(lit); }
                }
            }

            if let Some(strip_indices) = decomp.hub_adj_strips.get(&u) {
                for &si in strip_indices {
                    if let Some(adj) = decomp.strip_adj_hubs.get(&si) {
                        for &v in adj {
                            if u != v {
                                if let Some(&lit) = dir_strip_vars.get(&(si, u, v)) {
                                    out_lits.push(lit);
                                }
                                if let Some(&lit) = dir_strip_vars.get(&(si, v, u)) {
                                    in_lits.push(lit);
                                }
                            }
                        }
                    }
                }
            }

            out_lits.sort();
            out_lits.dedup();
            in_lits.sort();
            in_lits.dedup();

            if !out_lits.is_empty() {
                let _ = solver.add_clause(Clause::from_iter(out_lits.iter().copied()));
                for i in 0..out_lits.len() {
                    for j in (i + 1)..out_lits.len() {
                        let _ = solver.add_clause(clause![!out_lits[i], !out_lits[j]]);
                    }
                }
            }

            if !in_lits.is_empty() {
                let _ = solver.add_clause(Clause::from_iter(in_lits.iter().copied()));
                for i in 0..in_lits.len() {
                    for j in (i + 1)..in_lits.len() {
                        let _ = solver.add_clause(clause![!in_lits[i], !in_lits[j]]);
                    }
                }
            }
        }

        Self {
            root_hub,
            specs,
            bit_vars,
            dir_hh_vars,
            dir_strip_vars,
        }
    }

    /// Adds LFSR transition clauses under condition `trans_lit`:
    /// Y_{b+1} = X_b for b in 0..k-2
    /// Y_0 = X_{k-1} ^ X_{tap}
    fn add_lfsr_transitions(
        solver: &mut CaDiCaL<'static, 'static>,
        specs: &[LfsrSpec],
        bit_vars: &HashMap<(i32, usize), Vec<Lit>>,
        u: i32,
        v: i32,
        root_hub: i32,
        trans_lit: Lit,
    ) {
        if v == root_hub {
            return;
        }

        for (s_idx, spec) in specs.iter().enumerate() {
            let k = spec.num_bits;
            let tap = spec.tap;
            let p_u = &bit_vars[&(u, s_idx)];
            let p_v = &bit_vars[&(v, s_idx)];

            // 1. Shift register connections: P_v[b+1] = P_u[b] for b = 0..k-2
            for b in 0..(k - 1) {
                // !trans_lit \/ !P_u[b] \/ P_v[b+1]
                let _ = solver.add_clause(clause![!trans_lit, !p_u[b], p_v[b + 1]]);
                // !trans_lit \/ P_u[b] \/ !P_v[b+1]
                let _ = solver.add_clause(clause![!trans_lit, p_u[b], !p_v[b + 1]]);
            }

            // 2. Feedback XOR: P_v[0] = P_u[k-1] ^ P_u[tap]
            let xm1 = p_u[k - 1];
            let xtap = p_u[tap];
            let y0 = p_v[0];

            // !trans_lit \/ !y0 \/ xm1 \/ xtap
            let _ = solver.add_clause(clause![!trans_lit, !y0, xm1, xtap]);
            // !trans_lit \/ !y0 \/ !xm1 \/ !xtap
            let _ = solver.add_clause(clause![!trans_lit, !y0, !xm1, !xtap]);
            // !trans_lit \/ y0 \/ !xm1 \/ xtap
            let _ = solver.add_clause(clause![!trans_lit, y0, !xm1, xtap]);
            // !trans_lit \/ y0 \/ xm1 \/ !xtap
            let _ = solver.add_clause(clause![!trans_lit, y0, xm1, !xtap]);
        }
    }
}
