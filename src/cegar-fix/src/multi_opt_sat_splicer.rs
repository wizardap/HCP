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
pub enum CycleSwap {
    TwoOpt {
        c1: usize,
        c2: usize,
        rem1: (i32, i32),
        rem2: (i32, i32),
        add1: (i32, i32),
        add2: (i32, i32),
    },
    ThreeOptTriangle {
        c1: usize,
        c2: usize,
        c3: usize,
        rem1: (i32, i32),
        rem2: (i32, i32),
        rem3: (i32, i32),
        add1: (i32, i32),
        add2: (i32, i32),
        add3: (i32, i32),
    },
}

impl CycleSwap {
    pub fn removed_edges(&self) -> Vec<(i32, i32)> {
        match self {
            CycleSwap::TwoOpt { rem1, rem2, .. } => vec![*rem1, *rem2],
            CycleSwap::ThreeOptTriangle { rem1, rem2, rem3, .. } => vec![*rem1, *rem2, *rem3],
        }
    }

    pub fn added_edges(&self) -> Vec<(i32, i32)> {
        match self {
            CycleSwap::TwoOpt { add1, add2, .. } => vec![*add1, *add2],
            CycleSwap::ThreeOptTriangle { add1, add2, add3, .. } => vec![*add1, *add2, *add3],
        }
    }

    pub fn cycles(&self) -> Vec<usize> {
        match self {
            CycleSwap::TwoOpt { c1, c2, .. } => vec![*c1, *c2],
            CycleSwap::ThreeOptTriangle { c1, c2, c3, .. } => vec![*c1, *c2, *c3],
        }
    }
}

pub struct MultiOptSatSplicer;

impl MultiOptSatSplicer {
    /// Attempts multi-opt (2-opt bridges and 3-opt triangle swaps) macro-graph splicing across all disjoint 2-factor cycles.
    /// Solves an exact SAT spanning forest formulation with MTZ ordering to merge multi-cycle clusters simultaneously.
    pub fn splice_multi_opt_cycles(
        cycles: &[Vec<i32>],
        g: &Graph,
        protected_edges: &HashSet<(i32, i32)>,
    ) -> Vec<Vec<i32>> {
        if cycles.len() <= 1 {
            return cycles.to_vec();
        }

        for c in cycles {
            if c.len() < 3 {
                return cycles.to_vec();
            }
        }

        let mut current_cycles = cycles.to_vec();
        let max_passes = 10;

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

        // 1. Discover Candidate Swaps (2-opt and 3-opt triangles)
        let candidate_swaps = Self::discover_swaps(
            cycles,
            g,
            &vertex_to_cycle,
            &cycle_neighbors,
            &canonical_protected,
        );

        if candidate_swaps.is_empty() {
            return cycles.to_vec();
        }

        // 2. Find Connected Components on Macro-Graph
        let mut macro_adj: Vec<Vec<usize>> = vec![Vec::new(); m];
        for swap in &candidate_swaps {
            let cycs = swap.cycles();
            if cycs.len() == 2 {
                macro_adj[cycs[0]].push(cycs[1]);
                macro_adj[cycs[1]].push(cycs[0]);
            } else if cycs.len() == 3 {
                macro_adj[cycs[0]].push(cycs[1]);
                macro_adj[cycs[1]].push(cycs[0]);
                macro_adj[cycs[1]].push(cycs[2]);
                macro_adj[cycs[2]].push(cycs[1]);
                macro_adj[cycs[2]].push(cycs[0]);
                macro_adj[cycs[0]].push(cycs[2]);
            }
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

        // 3. Solve exact SAT spanning forest per component
        let mut all_selected_swaps: Vec<CycleSwap> = Vec::new();

        for comp in &components {
            if comp.len() < 2 {
                continue;
            }
            if let Some(selected_for_comp) = Self::solve_component_spanning_forest(comp, &candidate_swaps) {
                all_selected_swaps.extend(selected_for_comp);
            }
        }

        if all_selected_swaps.is_empty() {
            return cycles.to_vec();
        }

        // 4. Apply selected swaps and reconstruct cycles
        if let Some(new_cycles) = Self::reconstruct_spliced_cycles(cycles, &all_selected_swaps, g) {
            if new_cycles.len() < cycles.len() {
                return new_cycles;
            }
        }

        cycles.to_vec()
    }

    /// Discovers candidate 2-opt bridges and 3-opt triangle swaps.
    fn discover_swaps(
        cycles: &[Vec<i32>],
        g: &Graph,
        vertex_to_cycle: &HashMap<i32, usize>,
        cycle_neighbors: &HashMap<i32, [i32; 2]>,
        canonical_protected: &HashSet<(i32, i32)>,
    ) -> Vec<CycleSwap> {
        let m = cycles.len();
        let mut seen_swaps: HashSet<CycleSwap> = HashSet::new();
        let mut candidate_swaps: Vec<CycleSwap> = Vec::new();

        // 1. Candidate 2-opt bridges
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
                            if j <= i {
                                continue;
                            }

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
                                        let swap = CycleSwap::TwoOpt {
                                            c1: i,
                                            c2: j,
                                            rem1: e_i,
                                            rem2: e_j,
                                            add1,
                                            add2,
                                        };
                                        if seen_swaps.insert(swap.clone()) {
                                            candidate_swaps.push(swap);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. Candidate 3-opt triangle swaps
        for i in 0..m {
            let cycle = &cycles[i];
            let n = cycle.len();
            for pos in 0..n {
                let u1 = cycle[pos];
                let [u_prev, u_next] = cycle_neighbors[&u1];

                for &u2 in &[u_prev, u_next] {
                    let e1 = min_max(u1, u2);
                    if canonical_protected.contains(&e1) {
                        continue;
                    }

                    if let Some(nbrs_u1) = g.adjacency_list.get(&u1) {
                        for &v2 in nbrs_u1 {
                            let j = match vertex_to_cycle.get(&v2) {
                                Some(&idx) if idx != i => idx,
                                _ => continue,
                            };

                            let [v_prev, v_next] = cycle_neighbors[&v2];
                            for &v1 in &[v_prev, v_next] {
                                let e2 = min_max(v1, v2);
                                if canonical_protected.contains(&e2) {
                                    continue;
                                }

                                if let Some(nbrs_v1) = g.adjacency_list.get(&v1) {
                                    for &w2 in nbrs_v1 {
                                        let k = match vertex_to_cycle.get(&w2) {
                                            Some(&idx) if idx != i && idx != j => idx,
                                            _ => continue,
                                        };

                                        let [w_prev, w_next] = cycle_neighbors[&w2];
                                        for &w1 in &[w_prev, w_next] {
                                            let e3 = min_max(w1, w2);
                                            if canonical_protected.contains(&e3) {
                                                continue;
                                            }

                                            if let Some(nbrs_w1) = g.adjacency_list.get(&w1) {
                                                if nbrs_w1.contains(&u2) {
                                                    let a1 = min_max(u1, v2);
                                                    let a2 = min_max(v1, w2);
                                                    let a3 = min_max(w1, u2);

                                                    // Canonicalize by rotating so minimum cycle index is first
                                                    let swap = if i < j && i < k {
                                                        CycleSwap::ThreeOptTriangle {
                                                            c1: i,
                                                            c2: j,
                                                            c3: k,
                                                            rem1: e1,
                                                            rem2: e2,
                                                            rem3: e3,
                                                            add1: a1,
                                                            add2: a2,
                                                            add3: a3,
                                                        }
                                                    } else if j < i && j < k {
                                                        CycleSwap::ThreeOptTriangle {
                                                            c1: j,
                                                            c2: k,
                                                            c3: i,
                                                            rem1: e2,
                                                            rem2: e3,
                                                            rem3: e1,
                                                            add1: a2,
                                                            add2: a3,
                                                            add3: a1,
                                                        }
                                                    } else {
                                                        CycleSwap::ThreeOptTriangle {
                                                            c1: k,
                                                            c2: i,
                                                            c3: j,
                                                            rem1: e3,
                                                            rem2: e1,
                                                            rem3: e2,
                                                            add1: a3,
                                                            add2: a1,
                                                            add3: a2,
                                                        }
                                                    };

                                                    if seen_swaps.insert(swap.clone()) {
                                                        candidate_swaps.push(swap);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        candidate_swaps
    }

    /// Formulates and solves the exact SAT Spanning Forest problem on a macro-graph component.
    fn solve_component_spanning_forest(
        comp: &[usize],
        candidate_swaps: &[CycleSwap],
    ) -> Option<Vec<CycleSwap>> {
        let n = comp.len();
        if n < 2 {
            return None;
        }

        let mut node_to_loc: HashMap<usize, usize> = HashMap::with_capacity(n);
        for (loc, &c_idx) in comp.iter().enumerate() {
            node_to_loc.insert(c_idx, loc);
        }

        let mut comp_swaps: Vec<CycleSwap> = Vec::new();
        for swap in candidate_swaps {
            if swap.cycles().iter().all(|c| node_to_loc.contains_key(c)) {
                comp_swaps.push(swap.clone());
            }
        }

        if comp_swaps.is_empty() {
            return None;
        }

        let mut next_var_id: u32 = 0;

        // Variables:
        // Choice variable b_var for each swap
        let mut b_var: Vec<Lit> = Vec::with_capacity(comp_swaps.len());
        // For 2-opt: d_dir1 (c1 -> c2), d_dir2 (c2 -> c1)
        // For 3-opt: d_p1 (parent c1), d_p2 (parent c2), d_p3 (parent c3)
        let mut d_vars_2opt: HashMap<usize, (Lit, Lit)> = HashMap::new();
        let mut d_vars_3opt: HashMap<usize, (Lit, Lit, Lit)> = HashMap::new();

        for (s_idx, swap) in comp_swaps.iter().enumerate() {
            let b_lit = Var::new(next_var_id).pos_lit();
            next_var_id += 1;
            b_var.push(b_lit);

            match swap {
                CycleSwap::TwoOpt { .. } => {
                    let d1 = Var::new(next_var_id).pos_lit();
                    next_var_id += 1;
                    let d2 = Var::new(next_var_id).pos_lit();
                    next_var_id += 1;
                    d_vars_2opt.insert(s_idx, (d1, d2));
                }
                CycleSwap::ThreeOptTriangle { .. } => {
                    let dp1 = Var::new(next_var_id).pos_lit();
                    next_var_id += 1;
                    let dp2 = Var::new(next_var_id).pos_lit();
                    next_var_id += 1;
                    let dp3 = Var::new(next_var_id).pos_lit();
                    next_var_id += 1;
                    d_vars_3opt.insert(s_idx, (dp1, dp2, dp3));
                }
            }
        }

        // Ladder order variables O_{loc, 0 .. n-2} and attached variables A_{loc} for loc in 1..n
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

        // 1. Swap direction constraints
        for (s_idx, swap) in comp_swaps.iter().enumerate() {
            let b_lit = b_var[s_idx];
            match swap {
                CycleSwap::TwoOpt { .. } => {
                    let (d1, d2) = d_vars_2opt[&s_idx];
                    base_cnf.add_clause(clause![!d1, b_lit]);
                    base_cnf.add_clause(clause![!d2, b_lit]);
                    base_cnf.add_clause(clause![!b_lit, d1, d2]);
                    base_cnf.add_clause(clause![!d1, !d2]);
                }
                CycleSwap::ThreeOptTriangle { .. } => {
                    let (dp1, dp2, dp3) = d_vars_3opt[&s_idx];
                    base_cnf.add_clause(clause![!dp1, b_lit]);
                    base_cnf.add_clause(clause![!dp2, b_lit]);
                    base_cnf.add_clause(clause![!dp3, b_lit]);
                    base_cnf.add_clause(clause![!b_lit, dp1, dp2, dp3]);
                    base_cnf.add_clause(clause![!dp1, !dp2]);
                    base_cnf.add_clause(clause![!dp1, !dp3]);
                    base_cnf.add_clause(clause![!dp2, !dp3]);
                }
            }
        }

        // 2. Removed Edge AMO and Added Edge AMO
        let mut removed_to_b: HashMap<(i32, i32), Vec<Lit>> = HashMap::new();
        let mut added_to_b: HashMap<(i32, i32), Vec<Lit>> = HashMap::new();

        for (s_idx, swap) in comp_swaps.iter().enumerate() {
            let b_lit = b_var[s_idx];
            for rem in swap.removed_edges() {
                removed_to_b.entry(rem).or_default().push(b_lit);
            }
            for add in swap.added_edges() {
                added_to_b.entry(add).or_default().push(b_lit);
            }
        }

        for (_, lits) in removed_to_b {
            for p in 0..lits.len() {
                for q in (p + 1)..lits.len() {
                    base_cnf.add_clause(clause![!lits[p], !lits[q]]);
                }
            }
        }

        for (_, lits) in added_to_b {
            for p in 0..lits.len() {
                for q in (p + 1)..lits.len() {
                    base_cnf.add_clause(clause![!lits[p], !lits[q]]);
                }
            }
        }

        // 3. Collect Incoming Directed Literals per Node
        let mut in_lits: HashMap<usize, Vec<Lit>> = HashMap::new();
        for loc in 0..n {
            in_lits.insert(loc, Vec::new());
        }

        for (s_idx, swap) in comp_swaps.iter().enumerate() {
            match swap {
                &CycleSwap::TwoOpt { c1, c2, .. } => {
                    let u_loc = node_to_loc[&c1];
                    let v_loc = node_to_loc[&c2];
                    let (d_uv, d_vu) = d_vars_2opt[&s_idx];
                    in_lits.get_mut(&v_loc).unwrap().push(d_uv);
                    in_lits.get_mut(&u_loc).unwrap().push(d_vu);
                }
                &CycleSwap::ThreeOptTriangle { c1, c2, c3, .. } => {
                    let u_loc = node_to_loc[&c1];
                    let v_loc = node_to_loc[&c2];
                    let w_loc = node_to_loc[&c3];
                    let (dp_u, dp_v, dp_w) = d_vars_3opt[&s_idx];

                    // dp_u directs u -> v and u -> w
                    in_lits.get_mut(&v_loc).unwrap().push(dp_u);
                    in_lits.get_mut(&w_loc).unwrap().push(dp_u);

                    // dp_v directs v -> u and v -> w
                    in_lits.get_mut(&u_loc).unwrap().push(dp_v);
                    in_lits.get_mut(&w_loc).unwrap().push(dp_v);

                    // dp_w directs w -> u and w -> v
                    in_lits.get_mut(&u_loc).unwrap().push(dp_w);
                    in_lits.get_mut(&v_loc).unwrap().push(dp_w);
                }
            }
        }

        // Root (loc = 0) has 0 incoming directed edges
        for &d_lit in &in_lits[&0] {
            base_cnf.add_clause(clause![!d_lit]);
        }

        // Non-root incoming AMO and Attached linking
        for loc in 1..n {
            let incoming = &in_lits[&loc];
            let a_lit = att_var[&loc];

            for p in 0..incoming.len() {
                for q in (p + 1)..incoming.len() {
                    base_cnf.add_clause(clause![!incoming[p], !incoming[q]]);
                }
            }

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

        // 4. Parent Attachment Constraints: non-root source node must be attached
        for (s_idx, swap) in comp_swaps.iter().enumerate() {
            match swap {
                &CycleSwap::TwoOpt { c1, c2, .. } => {
                    let u_loc = node_to_loc[&c1];
                    let v_loc = node_to_loc[&c2];
                    let (d_uv, d_vu) = d_vars_2opt[&s_idx];

                    if u_loc != 0 {
                        let a_u = att_var[&u_loc];
                        base_cnf.add_clause(clause![!d_uv, a_u]);
                    }
                    if v_loc != 0 {
                        let a_v = att_var[&v_loc];
                        base_cnf.add_clause(clause![!d_vu, a_v]);
                    }
                }
                &CycleSwap::ThreeOptTriangle { c1, c2, c3, .. } => {
                    let u_loc = node_to_loc[&c1];
                    let v_loc = node_to_loc[&c2];
                    let w_loc = node_to_loc[&c3];
                    let (dp_u, dp_v, dp_w) = d_vars_3opt[&s_idx];

                    if u_loc != 0 {
                        let a_u = att_var[&u_loc];
                        base_cnf.add_clause(clause![!dp_u, a_u]);
                    }
                    if v_loc != 0 {
                        let a_v = att_var[&v_loc];
                        base_cnf.add_clause(clause![!dp_v, a_v]);
                    }
                    if w_loc != 0 {
                        let a_w = att_var[&w_loc];
                        base_cnf.add_clause(clause![!dp_w, a_w]);
                    }
                }
            }
        }

        // 5. MTZ Ladder Order Constraints
        for loc in 1..n {
            let o_v = &ladder[&loc];
            for k in 0..(o_v.len().saturating_sub(1)) {
                base_cnf.add_clause(clause![!o_v[k + 1], o_v[k]]);
            }
        }

        let add_mtz_transition = |cnf: &mut Cnf, u_loc: usize, v_loc: usize, dir_lit: Lit, ladder: &HashMap<usize, Vec<Lit>>| {
            if v_loc == 0 {
                return;
            }
            let o_v = &ladder[&v_loc];
            if u_loc == 0 {
                cnf.add_clause(clause![!dir_lit, o_v[0]]);
            } else {
                let o_u = &ladder[&u_loc];
                cnf.add_clause(clause![!dir_lit, o_v[0]]);
                for k in 0..(n.saturating_sub(2)) {
                    cnf.add_clause(clause![!dir_lit, !o_u[k], o_v[k + 1]]);
                }
                if n >= 2 {
                    let max_idx = n - 2;
                    cnf.add_clause(clause![!dir_lit, !o_u[max_idx]]);
                }
            }
        };

        for (s_idx, swap) in comp_swaps.iter().enumerate() {
            match swap {
                &CycleSwap::TwoOpt { c1, c2, .. } => {
                    let u_loc = node_to_loc[&c1];
                    let v_loc = node_to_loc[&c2];
                    let (d_uv, d_vu) = d_vars_2opt[&s_idx];
                    add_mtz_transition(&mut base_cnf, u_loc, v_loc, d_uv, &ladder);
                    add_mtz_transition(&mut base_cnf, v_loc, u_loc, d_vu, &ladder);
                }
                &CycleSwap::ThreeOptTriangle { c1, c2, c3, .. } => {
                    let u_loc = node_to_loc[&c1];
                    let v_loc = node_to_loc[&c2];
                    let w_loc = node_to_loc[&c3];
                    let (dp_u, dp_v, dp_w) = d_vars_3opt[&s_idx];

                    // dp_u directs u -> v and u -> w
                    add_mtz_transition(&mut base_cnf, u_loc, v_loc, dp_u, &ladder);
                    add_mtz_transition(&mut base_cnf, u_loc, w_loc, dp_u, &ladder);

                    // dp_v directs v -> u and v -> w
                    add_mtz_transition(&mut base_cnf, v_loc, u_loc, dp_v, &ladder);
                    add_mtz_transition(&mut base_cnf, v_loc, w_loc, dp_v, &ladder);

                    // dp_w directs w -> u and w -> v
                    add_mtz_transition(&mut base_cnf, w_loc, u_loc, dp_w, &ladder);
                    add_mtz_transition(&mut base_cnf, w_loc, v_loc, dp_w, &ladder);
                }
            }
        }

        // 6. Maximize attached nodes k from num_att down to 1
        let att_lits: Vec<Lit> = (1..n).map(|loc| att_var[&loc]).collect();
        let num_att = att_lits.len();

        for k in (1..=num_att).rev() {
            let mut solver = CaDiCaL::default();
            let mut cnf = base_cnf.clone();
            let c = num_att - k;

            if c == 0 {
                for &a_lit in &att_lits {
                    cnf.add_clause(clause![a_lit]);
                }
            } else {
                let neg_lits: Vec<Lit> = att_lits.iter().map(|&l| !l).collect();
                Self::add_at_most_c(&mut cnf, &mut next_var_id, &neg_lits, c);
            }

            if solver.add_cnf_ref(&cnf).is_ok() {
                let res = solver.solve();
                if let Ok(SolverResult::Sat) = res {
                    if let Ok(sol) = solver.full_solution() {
                        let mut selected: Vec<CycleSwap> = Vec::new();
                        for (s_idx, swap) in comp_swaps.iter().enumerate() {
                            if sol.lit_value(b_var[s_idx]) == TernaryVal::True {
                                selected.push(swap.clone());
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

    /// Reconstructs new 2-factor cycles from current cycles and selected swaps.
    fn reconstruct_spliced_cycles(
        cycles: &[Vec<i32>],
        selected_swaps: &[CycleSwap],
        g: &Graph,
    ) -> Option<Vec<Vec<i32>>> {
        let total_v: usize = cycles.iter().map(|c| c.len()).sum();
        let mut removed_edges: HashSet<(i32, i32)> = HashSet::new();
        let mut added_edges: Vec<(i32, i32)> = Vec::new();

        for swap in selected_swaps {
            for rem in swap.removed_edges() {
                removed_edges.insert(rem);
            }
            for add in swap.added_edges() {
                added_edges.push(add);
            }
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

        let new_total_v: usize = new_cycles.iter().map(|c| c.len()).sum();
        if new_total_v != total_v {
            return None;
        }

        Some(new_cycles)
    }
}
