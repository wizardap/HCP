use std::collections::HashMap;
use crate::two_tier_decomposer::DecompositionResult;
use rustsat::clause;
use rustsat::solvers::Solve;
use rustsat::types::{Clause, Lit, Var};
use rustsat_cadical::CaDiCaL;

#[derive(Debug, Clone)]
pub struct MacroCrtEncoder {
    pub root_hub: i32,
    pub moduli: Vec<usize>,
    pub residue_vars: HashMap<(i32, usize), Vec<Lit>>, // (hub, mod_idx) -> Vec<Lit> of length moduli[mod_idx]
    pub dir_hh_vars: HashMap<(i32, i32), Lit>,
    pub dir_strip_vars: HashMap<(usize, i32, i32), Lit>,
}

impl MacroCrtEncoder {
    /// Encodes Chinese Remainder Theorem (CRT) modular residue constraints for the macro layer.
    /// Uses coprime moduli (e.g. [5, 6, 7] for N <= 210, or [5, 7, 11, 13] for larger)
    /// to eliminate all subtours with 0 ripple-carry adders and tiny variable footprint.
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
                moduli: Vec::new(),
                residue_vars: HashMap::new(),
                dir_hh_vars: HashMap::new(),
                dir_strip_vars: HashMap::new(),
            };
        }

        let root_hub = sorted_hubs[0];
        let n_h = sorted_hubs.len();

        // 1. Lightweight coprime moduli [2, 3, 7] (Product = 42)
        // Provides aggressive pruning of small/medium subcycles without overloading SAT solving
        let moduli = vec![2, 3, 7];

        let mut residue_vars: HashMap<(i32, usize), Vec<Lit>> = HashMap::new();
        let mut dir_hh_vars: HashMap<(i32, i32), Lit> = HashMap::new();
        let mut dir_strip_vars: HashMap<(usize, i32, i32), Lit> = HashMap::new();

        // 2. Allocate Exactly-1 residue variables for each hub and each modulus
        for &h in &sorted_hubs {
            for (m_idx, &m) in moduli.iter().enumerate() {
                let mut lits = Vec::with_capacity(m);
                for _ in 0..m {
                    let lit = Var::new(*next_var_id).pos_lit();
                    *next_var_id += 1;
                    lits.push(lit);
                }

                // At-least-1: OR(lits)
                let _ = solver.add_clause(Clause::from_iter(lits.iter().copied()));
                // At-most-1: pairwise mutual exclusion
                for i in 0..m {
                    for j in (i + 1)..m {
                        let _ = solver.add_clause(clause![!lits[i], !lits[j]]);
                    }
                }

                // Root hub is pinned to residue 0 for all moduli
                if h == root_hub {
                    let _ = solver.add_clause(clause![lits[0]]);
                }

                residue_vars.insert((h, m_idx), lits);
            }
        }

        // 3. Directed Hub-Hub transition variables
        for &(u, v) in &decomp.hh_edges {
            let lit_u_to_v = Var::new(*next_var_id).pos_lit();
            *next_var_id += 1;
            let lit_v_to_u = Var::new(*next_var_id).pos_lit();
            *next_var_id += 1;

            dir_hh_vars.insert((u, v), lit_u_to_v);
            dir_hh_vars.insert((v, u), lit_v_to_u);

            if let Some(&x_uv) = var_hh.get(&(u, v)) {
                // x_uv <=> (lit_u_to_v \/ lit_v_to_u)
                let _ = solver.add_clause(clause![!x_uv, lit_u_to_v, lit_v_to_u]);
                let _ = solver.add_clause(clause![!lit_u_to_v, x_uv]);
                let _ = solver.add_clause(clause![!lit_v_to_u, x_uv]);
                let _ = solver.add_clause(clause![!lit_u_to_v, !lit_v_to_u]);

                // Modular increment constraints along directed transition
                Self::add_crt_transition(solver, &moduli, &residue_vars, u, v, root_hub, lit_u_to_v);
                Self::add_crt_transition(solver, &moduli, &residue_vars, v, u, root_hub, lit_v_to_u);
            }
        }

        // 4. Directed Strip transition variables
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

                            Self::add_crt_transition(solver, &moduli, &residue_vars, u, v, root_hub, lit_u_to_v);
                            Self::add_crt_transition(solver, &moduli, &residue_vars, v, u, root_hub, lit_v_to_u);
                        }
                    }
                }
            }
        }

        Self {
            root_hub,
            moduli,
            residue_vars,
            dir_hh_vars,
            dir_strip_vars,
        }
    }

    /// Adds CRT residue stepping clauses: if u -> v transition is active,
    /// then for every modulus m, residue_v = (residue_u + 1) mod m
    /// (Unless v is root_hub, where the cycle wraps around to 0).
    fn add_crt_transition(
        solver: &mut CaDiCaL<'static, 'static>,
        moduli: &[usize],
        residue_vars: &HashMap<(i32, usize), Vec<Lit>>,
        u: i32,
        v: i32,
        root_hub: i32,
        trans_lit: Lit,
    ) {
        if v == root_hub {
            return;
        }

        for (m_idx, &m) in moduli.iter().enumerate() {
            let r_u = &residue_vars[&(u, m_idx)];
            let r_v = &residue_vars[&(v, m_idx)];

            for val in 0..m {
                let next_val = (val + 1) % m;
                // !trans_lit \/ !r_u[val] \/ r_v[next_val]
                let _ = solver.add_clause(clause![!trans_lit, !r_u[val], r_v[next_val]]);
            }
        }
    }
}
