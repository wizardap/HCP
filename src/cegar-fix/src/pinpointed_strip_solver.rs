use std::collections::{HashMap, HashSet, VecDeque};
use crate::graph::Graph;
use crate::two_tier_decomposer::DecompositionResult;
use rustsat::clause;
use rustsat::instances::{BasicVarManager, ManageVars};
use rustsat::solvers::{Solve, SolveIncremental, SolverResult};
use rustsat::types::{Clause, Lit, TernaryVal};
use rustsat_cadical::CaDiCaL;

pub struct PinpointedStripSolver<'a> {
    pub g: &'a Graph,
    pub decomp: &'a DecompositionResult,
}

impl<'a> PinpointedStripSolver<'a> {
    pub fn new(g: &'a Graph, decomp: &'a DecompositionResult) -> Self {
        Self { g, decomp }
    }

    /// Solves the strip path-cover problem of size K = tot_demand / 2 for strip `si`.
    /// `dem` specifies the port demands on adjacent hubs: hub -> count (0, 1, or 2).
    /// If SAT, returns Ok(paths) where `paths` is a Vec<Vec<i32>> of vertex paths covering all vertices in strip `si`.
    /// If UNSAT, returns Err(unsat_core_hubs) where `unsat_core_hubs` is a minimal subset of Hubs whose assumption caused UNSAT.
    pub fn solve_strip(
        &mut self,
        si: usize,
        dem: &HashMap<i32, usize>,
        _s_hub: Option<i32>,
        _b_hub: Option<i32>,
        k: usize,
    ) -> Result<Vec<Vec<i32>>, Vec<i32>> {
        if si >= self.decomp.strips.len() {
            let mut failed_hubs: Vec<i32> = dem.keys().copied().filter(|&h| dem.get(&h).copied().unwrap_or(0) > 0).collect();
            failed_hubs.sort_unstable();
            return Err(failed_hubs);
        }

        let strip_verts = &self.decomp.strips[si];
        if strip_verts.is_empty() {
            if k == 0 {
                return Ok(vec![]);
            } else {
                let mut failed_hubs: Vec<i32> = dem.keys().copied().filter(|&h| dem.get(&h).copied().unwrap_or(0) > 0).collect();
                failed_hubs.sort_unstable();
                return Err(failed_hubs);
            }
        }

        let strip_set: HashSet<i32> = strip_verts.iter().copied().collect();
        let mut solver = CaDiCaL::default();
        let mut var_mgr = BasicVarManager::default();

        // 1. Internal edges
        let mut edge_vars: HashMap<(i32, i32), Lit> = HashMap::new();
        let mut vert_internal_lits: HashMap<i32, Vec<Lit>> = HashMap::new();
        for &v in strip_verts {
            vert_internal_lits.insert(v, Vec::new());
        }

        for &u in strip_verts {
            if let Some(neighbors) = self.g.adjacency_list.get(&u) {
                for &v in neighbors {
                    if u < v && strip_set.contains(&v) {
                        let lit = var_mgr.new_var().pos_lit();
                        edge_vars.insert((u, v), lit);
                        vert_internal_lits.get_mut(&u).unwrap().push(lit);
                        vert_internal_lits.get_mut(&v).unwrap().push(lit);
                    }
                }
            }
        }

        // 2. External edges to hubs
        let mut ext_edge_vars: HashMap<(i32, i32), Lit> = HashMap::new();
        let mut hub_to_ext_lits: HashMap<i32, Vec<Lit>> = HashMap::new();
        let mut vert_ext_lits: HashMap<i32, Vec<Lit>> = HashMap::new();
        for &v in strip_verts {
            vert_ext_lits.insert(v, Vec::new());
        }

        for &u in strip_verts {
            if let Some(neighbors) = self.g.adjacency_list.get(&u) {
                for &h in neighbors {
                    if self.decomp.all_hubs.contains(&h) {
                        let lit = var_mgr.new_var().pos_lit();
                        ext_edge_vars.insert((u, h), lit);
                        hub_to_ext_lits.entry(h).or_default().push(lit);
                        vert_ext_lits.get_mut(&u).unwrap().push(lit);
                    }
                }
            }
        }

        // 3. Degree == 2 constraint for each vertex in strip
        for &u in strip_verts {
            let mut inc_lits = Vec::new();
            if let Some(int_lits) = vert_internal_lits.get(&u) {
                inc_lits.extend_from_slice(int_lits);
            }
            if let Some(ext_lits) = vert_ext_lits.get(&u) {
                inc_lits.extend_from_slice(ext_lits);
            }
            add_exact_k(&mut solver, &mut var_mgr, &inc_lits, 2);
        }

        // 4. Total external edges must equal 2 * K
        let mut all_ext_lits = Vec::new();
        for &lit in ext_edge_vars.values() {
            all_ext_lits.push(lit);
        }
        add_exact_k(&mut solver, &mut var_mgr, &all_ext_lits, 2 * k);

        // 5. Hub demand assumptions
        let mut assump_lits = Vec::new();
        let mut assump_to_hub: HashMap<Lit, i32> = HashMap::new();

        let mut all_relevant_hubs: HashSet<i32> = HashSet::new();
        for &h in hub_to_ext_lits.keys() {
            all_relevant_hubs.insert(h);
        }
        for &h in dem.keys() {
            all_relevant_hubs.insert(h);
        }

        for &h in &all_relevant_hubs {
            let demand = dem.get(&h).copied().unwrap_or(0);
            let ext_lits = hub_to_ext_lits.get(&h).cloned().unwrap_or_default();

            if demand == 0 {
                // No external edges to this hub if demand is 0
                for &lit in &ext_lits {
                    let _ = solver.add_clause(clause![!lit]);
                }
            } else {
                let a_lit = var_mgr.new_var().pos_lit();
                assump_lits.push(a_lit);
                assump_to_hub.insert(a_lit, h);

                if ext_lits.len() < demand {
                    let _ = solver.add_clause(clause![!a_lit]);
                } else if demand == 1 {
                    // At least 1
                    let mut cl = vec![!a_lit];
                    cl.extend_from_slice(&ext_lits);
                    let _ = solver.add_clause(Clause::from_iter(cl));
                    // At most 1
                    for i in 0..ext_lits.len() {
                        for j in (i + 1)..ext_lits.len() {
                            let _ = solver.add_clause(clause![!a_lit, !ext_lits[i], !ext_lits[j]]);
                        }
                    }
                } else if demand == 2 {
                    // At least 2
                    let mut cl = vec![!a_lit];
                    cl.extend_from_slice(&ext_lits);
                    let _ = solver.add_clause(Clause::from_iter(cl));
                    for i in 0..ext_lits.len() {
                        let mut cl_i = vec![!a_lit, !ext_lits[i]];
                        for j in 0..ext_lits.len() {
                            if i != j {
                                cl_i.push(ext_lits[j]);
                            }
                        }
                        let _ = solver.add_clause(Clause::from_iter(cl_i));
                    }
                    // At most 2
                    for i in 0..ext_lits.len() {
                        for j in (i + 1)..ext_lits.len() {
                            for m in (j + 1)..ext_lits.len() {
                                let _ = solver.add_clause(clause![!a_lit, !ext_lits[i], !ext_lits[j], !ext_lits[m]]);
                            }
                        }
                    }
                }
            }
        }

        loop {
            match solver.solve_assumps(&assump_lits) {
                Ok(SolverResult::Sat) => {
                    let sol = solver.full_solution().expect("SAT solution must exist");
                    let mut active_edges: Vec<(i32, i32)> = Vec::new();
                    for (&(u, v), &lit) in &edge_vars {
                        if sol.lit_value(lit) == TernaryVal::True {
                            active_edges.push((u, v));
                        }
                    }

                    // Build adjacency graph for internal paths
                    let mut adj: HashMap<i32, Vec<i32>> = HashMap::new();
                    for &v in strip_verts {
                        adj.insert(v, Vec::new());
                    }
                    for &(u, v) in &active_edges {
                        adj.get_mut(&u).unwrap().push(v);
                        adj.get_mut(&v).unwrap().push(u);
                    }

                    let mut visited = HashSet::new();
                    let mut paths = Vec::new();
                    let mut cycles = Vec::new();

                    for &start_v in strip_verts {
                        if visited.contains(&start_v) {
                            continue;
                        }

                        let mut comp_verts = Vec::new();
                        let mut queue = VecDeque::new();
                        visited.insert(start_v);
                        queue.push_back(start_v);

                        while let Some(curr) = queue.pop_front() {
                            comp_verts.push(curr);
                            for &nxt in &adj[&curr] {
                                if !visited.contains(&nxt) {
                                    visited.insert(nxt);
                                    queue.push_back(nxt);
                                }
                            }
                        }

                        let is_cycle = comp_verts.len() >= 3 && comp_verts.iter().all(|&v| adj[&v].len() == 2);

                        if is_cycle {
                            let mut cyc = Vec::new();
                            let mut c_curr = comp_verts[0];
                            let mut c_prev = -1;
                            for _ in 0..comp_verts.len() {
                                cyc.push(c_curr);
                                let nbrs = &adj[&c_curr];
                                let nxt = if nbrs[0] != c_prev { nbrs[0] } else { nbrs[1] };
                                c_prev = c_curr;
                                c_curr = nxt;
                            }
                            cycles.push(cyc);
                        } else {
                            if comp_verts.len() == 1 {
                                paths.push(comp_verts);
                            } else {
                                let start_ep = comp_verts.iter().find(|&&v| adj[&v].len() == 1).copied().unwrap_or(comp_verts[0]);
                                let mut path = Vec::new();
                                let mut p_curr = start_ep;
                                let mut p_prev = -1;
                                loop {
                                    path.push(p_curr);
                                    let nbrs = &adj[&p_curr];
                                    let next_opts: Vec<i32> = nbrs.iter().copied().filter(|&x| x != p_prev).collect();
                                    if next_opts.is_empty() {
                                        break;
                                    }
                                    p_prev = p_curr;
                                    p_curr = next_opts[0];
                                }
                                paths.push(path);
                            }
                        }
                    }

                    if !cycles.is_empty() {
                        for cyc in cycles {
                            let mut block_clause = Vec::new();
                            for i in 0..cyc.len() {
                                let u = cyc[i];
                                let v = cyc[(i + 1) % cyc.len()];
                                let edge_key = if u < v { (u, v) } else { (v, u) };
                                if let Some(&lit) = edge_vars.get(&edge_key) {
                                    block_clause.push(!lit);
                                }
                            }
                            if !block_clause.is_empty() {
                                let _ = solver.add_clause(Clause::from_iter(block_clause));
                            }
                        }
                        continue;
                    }

                    paths.sort_by_key(|p| p[0]);
                    return Ok(paths);
                }
                Ok(SolverResult::Unsat) => {
                    let core_lits = solver.core().unwrap_or_default();
                    let mut failed_hubs = Vec::new();
                    for lit in core_lits {
                        if let Some(&h) = assump_to_hub.get(&lit) {
                            failed_hubs.push(h);
                        } else if let Some(&h) = assump_to_hub.get(&(!lit)) {
                            failed_hubs.push(h);
                        }
                    }
                    if failed_hubs.is_empty() {
                        for &h in dem.keys() {
                            if dem[&h] > 0 {
                                failed_hubs.push(h);
                            }
                        }
                    }
                    failed_hubs.sort_unstable();
                    failed_hubs.dedup();
                    return Err(failed_hubs);
                }
                _ => {
                    let mut failed_hubs: Vec<i32> = dem.keys().copied().filter(|&h| dem.get(&h).copied().unwrap_or(0) > 0).collect();
                    failed_hubs.sort_unstable();
                    return Err(failed_hubs);
                }
            }
        }
    }
}

/// Adds cardinality constraints to enforce sum(lits) <= k using Sinz sequential counter.
fn add_at_most_k<S: Solve>(
    solver: &mut S,
    var_mgr: &mut BasicVarManager,
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
            row.push(var_mgr.new_var().pos_lit());
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
    var_mgr: &mut BasicVarManager,
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
    if k == 2 {
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
    add_at_most_k(solver, var_mgr, &neg_lits, n - k);
}

/// Adds cardinality constraints to enforce sum(lits) == k
fn add_exact_k<S: Solve>(
    solver: &mut S,
    var_mgr: &mut BasicVarManager,
    lits: &[Lit],
    k: usize,
) {
    add_at_most_k(solver, var_mgr, lits, k);
    add_at_least_k(solver, var_mgr, lits, k);
}
