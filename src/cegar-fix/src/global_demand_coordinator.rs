use std::collections::{HashMap, HashSet};
use crate::graph::Graph;
use crate::two_tier_decomposer::DecompositionResult;
use rustsat::clause;
use rustsat::solvers::{Solve, SolverResult};
use rustsat::types::{Clause, Lit, TernaryVal, Var};
use rustsat_cadical::CaDiCaL;

pub struct GlobalDemandCoordinator<'a> {
    pub g: &'a Graph,
    pub decomp: &'a DecompositionResult,
    pub solver: CaDiCaL<'static, 'static>,
    pub var_hh: HashMap<(i32, i32), Lit>,
    pub var_d1: HashMap<(usize, i32), Lit>,
    pub var_d2: HashMap<(usize, i32), Lit>,
    pub next_var_id: u32,
}

impl<'a> GlobalDemandCoordinator<'a> {
    pub fn new(g: &'a Graph, decomp: &'a DecompositionResult) -> Self {
        let mut solver = CaDiCaL::default();
        let mut var_hh = HashMap::new();
        let mut var_d1 = HashMap::new();
        let mut var_d2 = HashMap::new();
        let mut next_var_id: u32 = 0;

        // 1. Variables for Hub-Hub direct edges
        for &(u, v) in &decomp.hh_edges {
            let lit = Var::new(next_var_id).pos_lit();
            next_var_id += 1;
            var_hh.insert((u, v), lit);
            var_hh.insert((v, u), lit);
        }

        // 2. Variables for Strip-Hub port allocations for all adjacent hubs
        for (si, _s) in decomp.strips.iter().enumerate() {
            if let Some(adj) = decomp.strip_adj_hubs.get(&si) {
                let mut sorted_adj: Vec<i32> = adj.iter().copied().collect();
                sorted_adj.sort_unstable();
                for &h in &sorted_adj {
                    let lit1 = Var::new(next_var_id).pos_lit();
                    next_var_id += 1;
                    let lit2 = Var::new(next_var_id).pos_lit();
                    next_var_id += 1;

                    var_d1.insert((si, h), lit1);
                    var_d2.insert((si, h), lit2);

                    // d2 => d1  (!d2 \/ d1)
                    let _ = solver.add_clause(clause![!lit2, lit1]);
                }
            }
        }

        // 3. Exact-2 degree constraint on ALL Hubs
        let mut sorted_hubs: Vec<i32> = decomp.all_hubs.iter().copied().collect();
        sorted_hubs.sort_unstable();
        for &h in &sorted_hubs {
            let mut inc_lits = Vec::new();
            if let Some(nbrs) = g.adjacency_list.get(&h) {
                for &nbr in nbrs {
                    if decomp.all_hubs.contains(&nbr) {
                        if let Some(&lit) = var_hh.get(&(h, nbr)) {
                            inc_lits.push(lit);
                        }
                    }
                }
            }

            if let Some(adj_strips) = decomp.hub_adj_strips.get(&h) {
                let mut sorted_strips: Vec<usize> = adj_strips.iter().copied().collect();
                sorted_strips.sort_unstable();
                for &si in &sorted_strips {
                    if let Some(&lit1) = var_d1.get(&(si, h)) {
                        inc_lits.push(lit1);
                    }
                    if let Some(&lit2) = var_d2.get(&(si, h)) {
                        inc_lits.push(lit2);
                    }
                }
            }

            add_exact_2(&mut solver, &mut next_var_id, &inc_lits);
        }

        // 4. Parity & Endpoint Bounds per strip
        for (si, s) in decomp.strips.iter().enumerate() {
            let mut adj_hubs: Vec<i32> = decomp
                .strip_adj_hubs
                .get(&si)
                .map_or_else(Vec::new, |set| set.iter().copied().collect());
            adj_hubs.sort_unstable();

            let mut strip_lits = Vec::new();
            for &h in &adj_hubs {
                if let Some(&lit1) = var_d1.get(&(si, h)) {
                    strip_lits.push(lit1);
                }
                if let Some(&lit2) = var_d2.get(&(si, h)) {
                    strip_lits.push(lit2);
                }
            }

            if s.len() < 10 {
                // Small strip (size 2-3): exactly 2 endpoints (K = 1)
                add_exact_2(&mut solver, &mut next_var_id, &strip_lits);

                // Per-vertex endpoint capacity in small strip: at most 1 hub per bulk vertex
                for &u in s {
                    if let Some(nbrs) = g.adjacency_list.get(&u) {
                        let mut u_lits = Vec::new();
                        for &h in nbrs {
                            if decomp.all_hubs.contains(&h) {
                                if let Some(&lit1) = var_d1.get(&(si, h)) {
                                    u_lits.push(lit1);
                                }
                            }
                        }
                        for i in 0..u_lits.len() {
                            for j in (i + 1)..u_lits.len() {
                                let _ = solver.add_clause(clause![!u_lits[i], !u_lits[j]]);
                            }
                        }
                    }
                }
            } else {
                // Large strip (size 125): exactly one K in {2, 3, 4, 5}
                let mut k_vars = Vec::new();
                for _ in 2..=5 {
                    let lit = Var::new(next_var_id).pos_lit();
                    next_var_id += 1;
                    k_vars.push(lit);
                }
                add_exact_1(&mut solver, &mut next_var_id, &k_vars);

                for (idx, &k) in [2, 3, 4, 5].iter().enumerate() {
                    let target = 2 * k;
                    add_cond_at_least_k(&mut solver, &mut next_var_id, k_vars[idx], &strip_lits, target);
                    add_cond_at_most_k(&mut solver, &mut next_var_id, k_vars[idx], &strip_lits, target);
                }
            }
        }

        Self {
            g,
            decomp,
            solver,
            var_hh,
            var_d1,
            var_d2,
            next_var_id,
        }
    }

    /// Solves the global assignment and returns (active_hh_edges, strip_demands) if SAT.
    /// strip_demands: strip_index -> (hub_id -> demand_count: 0, 1, or 2).
    pub fn solve_assignment(
        &mut self,
    ) -> Option<(Vec<(i32, i32)>, HashMap<usize, HashMap<i32, usize>>)> {
        match self.solver.solve() {
            Ok(SolverResult::Sat) => {
                let sol = self.solver.full_solution().expect("SAT solution must exist");
                let mut active_hh = Vec::new();
                for (&(u, v), &lit) in &self.var_hh {
                    if u < v && sol.lit_value(lit) == TernaryVal::True {
                        active_hh.push((u, v));
                    }
                }
                active_hh.sort_unstable();

                let mut strip_demands = HashMap::new();
                for si in 0..self.decomp.strips.len() {
                    let mut dem = HashMap::new();
                    if let Some(adj) = self.decomp.strip_adj_hubs.get(&si) {
                        for &h in adj {
                            let v1_val = self
                                .var_d1
                                .get(&(si, h))
                                .map_or(TernaryVal::False, |&l| sol.lit_value(l));
                            let v2_val = self
                                .var_d2
                                .get(&(si, h))
                                .map_or(TernaryVal::False, |&l| sol.lit_value(l));
                            let d = if v1_val == TernaryVal::True {
                                if v2_val == TernaryVal::True {
                                    2
                                } else {
                                    1
                                }
                            } else {
                                0
                            };
                            dem.insert(h, d);
                        }
                    }
                    strip_demands.insert(si, dem);
                }

                Some((active_hh, strip_demands))
            }
            _ => None,
        }
    }

    /// Adds a mathematically exact flipped-literal conflict clause for an infeasible strip demand.
    /// `failed_hubs`: minimal UNSAT core from Task 2.
    pub fn add_conflict_clause(
        &mut self,
        si: usize,
        dem: &HashMap<i32, usize>,
        failed_hubs: &[i32],
    ) {
        let mut clause = Vec::new();
        let target_hubs: Vec<i32> = if !failed_hubs.is_empty() {
            failed_hubs.to_vec()
        } else {
            dem.keys().copied().collect()
        };

        for h in target_hubs {
            let req = dem.get(&h).copied().unwrap_or(0);
            let v1 = self.var_d1.get(&(si, h)).copied();
            let v2 = self.var_d2.get(&(si, h)).copied();

            if req == 0 {
                if let Some(l1) = v1 {
                    clause.push(l1);
                }
            } else if req == 1 {
                if let Some(l1) = v1 {
                    clause.push(!l1);
                }
                if let Some(l2) = v2 {
                    clause.push(l2);
                }
            } else if req == 2 {
                if let Some(l2) = v2 {
                    clause.push(!l2);
                }
            }
        }

        clause.sort();
        clause.dedup();
        if !clause.is_empty() {
            let _ = self.solver.add_clause(Clause::from_iter(clause));
        }
    }

    /// Adds a True Indicator Cut-Crossing Subtour Elimination Clause for a subtour/cycle on the 310 Hub partition.
    /// Forces at least one crossing HH edge or at least one strip bridging H_inside and H_outside.
    pub fn add_macro_cut(&mut self, cyc_verts: &HashSet<i32>) {
        let h_inside: HashSet<i32> = cyc_verts.intersection(&self.decomp.all_hubs).copied().collect();
        let h_outside: HashSet<i32> = self.decomp.all_hubs.difference(&h_inside).copied().collect();

        if h_inside.is_empty() || h_outside.is_empty() {
            return;
        }

        let mut cut_clause: Vec<Lit> = Vec::new();

        // 1. Crossing HH edges
        for &(u, v) in &self.decomp.hh_edges {
            let u_in = h_inside.contains(&u);
            let v_in = h_inside.contains(&v);
            if u_in != v_in {
                if let Some(&lit) = self.var_hh.get(&(u, v)) {
                    cut_clause.push(lit);
                }
            }
        }

        // 2. Bridging strips
        for (si, _strip) in self.decomp.strips.iter().enumerate() {
            if let Some(adj) = self.decomp.strip_adj_hubs.get(&si) {
                let in_hubs: Vec<i32> = adj.iter().copied().filter(|h| h_inside.contains(h)).collect();
                let out_hubs: Vec<i32> = adj.iter().copied().filter(|h| h_outside.contains(h)).collect();

                if !in_hubs.is_empty() && !out_hubs.is_empty() {
                    let y_var = Var::new(self.next_var_id);
                    self.next_var_id += 1;
                    let y_si = y_var.pos_lit();
                    cut_clause.push(y_si);

                    // y_si => OR(var_d1[(si, u)] for u in in_hubs)
                    // clause: !y_si \/ (lits in in_hubs)
                    let mut cl_in = vec![!y_si];
                    for u in in_hubs {
                        if let Some(&lit) = self.var_d1.get(&(si, u)) {
                            cl_in.push(lit);
                        }
                    }
                    let _ = self.solver.add_clause(Clause::from_iter(cl_in));

                    // y_si => OR(var_d1[(si, v)] for v in out_hubs)
                    // clause: !y_si \/ (lits in out_hubs)
                    let mut cl_out = vec![!y_si];
                    for v in out_hubs {
                        if let Some(&lit) = self.var_d1.get(&(si, v)) {
                            cl_out.push(lit);
                        }
                    }
                    let _ = self.solver.add_clause(Clause::from_iter(cl_out));
                }
            }
        }

        cut_clause.sort();
        cut_clause.dedup();
        if !cut_clause.is_empty() {
            let _ = self.solver.add_clause(Clause::from_iter(cut_clause));
        }
    }
}

/// Adds cardinality constraints to enforce sum(lits) == 1
fn add_exact_1<S: Solve>(
    solver: &mut S,
    _next_var_id: &mut u32,
    lits: &[Lit],
) {
    let n = lits.len();
    if n == 0 {
        let _ = solver.add_clause(Clause::new());
        return;
    }
    if n == 1 {
        let _ = solver.add_clause(clause![lits[0]]);
        return;
    }

    // At least 1
    let _ = solver.add_clause(Clause::from_iter(lits.iter().copied()));

    // At most 1
    for i in 0..n {
        for j in (i + 1)..n {
            let _ = solver.add_clause(clause![!lits[i], !lits[j]]);
        }
    }
}

/// Adds cardinality constraints to enforce sum(lits) <= k using Sinz sequential counter.
fn add_at_most_k<S: Solve>(
    solver: &mut S,
    next_var_id: &mut u32,
    lits: &[Lit],
    k: usize,
) {
    let n = lits.len();
    if k >= n {
        return;
    }
    if k == 0 {
        for &l in lits {
            let _ = solver.add_clause(clause![!l]);
        }
        return;
    }
    if k == 1 && n <= 10 {
        for i in 0..n {
            for j in (i + 1)..n {
                let _ = solver.add_clause(clause![!lits[i], !lits[j]]);
            }
        }
        return;
    }
    if k == 2 && n <= 8 {
        for i in 0..n {
            for j in (i + 1)..n {
                for m in (j + 1)..n {
                    let _ = solver.add_clause(clause![!lits[i], !lits[j], !lits[m]]);
                }
            }
        }
        return;
    }

    let mut s: Vec<Vec<Lit>> = Vec::with_capacity(n - 1);
    for _ in 0..(n - 1) {
        let mut row = Vec::with_capacity(k);
        for _ in 0..k {
            let lit = Var::new(*next_var_id).pos_lit();
            *next_var_id += 1;
            row.push(lit);
        }
        s.push(row);
    }

    let _ = solver.add_clause(clause![!lits[0], s[0][0]]);
    for j in 1..k {
        let _ = solver.add_clause(clause![!s[0][j]]);
    }

    for i in 1..(n - 1) {
        for j in 0..k {
            let _ = solver.add_clause(clause![!s[i - 1][j], s[i][j]]);
        }
        let _ = solver.add_clause(clause![!lits[i], s[i][0]]);
        for j in 1..k {
            let _ = solver.add_clause(clause![!lits[i], !s[i - 1][j - 1], s[i][j]]);
        }
        let _ = solver.add_clause(clause![!lits[i], !s[i - 1][k - 1]]);
    }

    let _ = solver.add_clause(clause![!lits[n - 1], !s[n - 2][k - 1]]);
}

/// Adds cardinality constraints to enforce sum(lits) >= k
fn add_at_least_k<S: Solve>(
    solver: &mut S,
    next_var_id: &mut u32,
    lits: &[Lit],
    k: usize,
) {
    let n = lits.len();
    if k == 0 {
        return;
    }
    if k > n {
        let _ = solver.add_clause(Clause::new());
        return;
    }
    if k == n {
        for &l in lits {
            let _ = solver.add_clause(clause![l]);
        }
        return;
    }
    if k == 1 {
        let _ = solver.add_clause(Clause::from_iter(lits.iter().copied()));
        return;
    }
    if k == 2 && n <= 8 {
        let _ = solver.add_clause(Clause::from_iter(lits.iter().copied()));
        for i in 0..n {
            let mut cl = vec![!lits[i]];
            for j in 0..n {
                if i != j {
                    cl.push(lits[j]);
                }
            }
            let _ = solver.add_clause(Clause::from_iter(cl));
        }
        return;
    }

    let neg_lits: Vec<Lit> = lits.iter().map(|&l| !l).collect();
    add_at_most_k(solver, next_var_id, &neg_lits, n - k);
}

/// Adds cardinality constraints to enforce sum(lits) == 2
fn add_exact_2<S: Solve>(
    solver: &mut S,
    next_var_id: &mut u32,
    lits: &[Lit],
) {
    add_at_most_k(solver, next_var_id, lits, 2);
    add_at_least_k(solver, next_var_id, lits, 2);
}

/// Adds conditional cardinality constraints: cond => sum(lits) <= k using Sinz sequential counter.
fn add_cond_at_most_k<S: Solve>(
    solver: &mut S,
    next_var_id: &mut u32,
    cond: Lit,
    lits: &[Lit],
    k: usize,
) {
    let n = lits.len();
    if k >= n {
        return;
    }
    if k == 0 {
        for &l in lits {
            let _ = solver.add_clause(clause![!cond, !l]);
        }
        return;
    }

    let mut s: Vec<Vec<Lit>> = Vec::with_capacity(n - 1);
    for _ in 0..(n - 1) {
        let mut row = Vec::with_capacity(k);
        for _ in 0..k {
            let lit = Var::new(*next_var_id).pos_lit();
            *next_var_id += 1;
            row.push(lit);
        }
        s.push(row);
    }

    let _ = solver.add_clause(clause![!cond, !lits[0], s[0][0]]);
    for j in 1..k {
        let _ = solver.add_clause(clause![!cond, !s[0][j]]);
    }

    for i in 1..(n - 1) {
        for j in 0..k {
            let _ = solver.add_clause(clause![!cond, !s[i - 1][j], s[i][j]]);
        }
        let _ = solver.add_clause(clause![!cond, !lits[i], s[i][0]]);
        for j in 1..k {
            let _ = solver.add_clause(clause![!cond, !lits[i], !s[i - 1][j - 1], s[i][j]]);
        }
        let _ = solver.add_clause(clause![!cond, !lits[i], !s[i - 1][k - 1]]);
    }

    let _ = solver.add_clause(clause![!cond, !lits[n - 1], !s[n - 2][k - 1]]);
}

/// Adds conditional cardinality constraints: cond => sum(lits) >= k
fn add_cond_at_least_k<S: Solve>(
    solver: &mut S,
    next_var_id: &mut u32,
    cond: Lit,
    lits: &[Lit],
    k: usize,
) {
    let n = lits.len();
    if k == 0 {
        return;
    }
    if k > n {
        let _ = solver.add_clause(clause![!cond]);
        return;
    }
    let neg_lits: Vec<Lit> = lits.iter().map(|&l| !l).collect();
    add_cond_at_most_k(solver, next_var_id, cond, &neg_lits, n - k);
}
