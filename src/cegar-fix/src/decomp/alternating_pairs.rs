use crate::core::graph::Graph;
use crate::core::tour_verifier::TourVerifier;
use crate::decomp::spqr_series::{contract_series_chains, expand_series_tour};
use rustsat::clause;
use rustsat::instances::{BasicVarManager, Cnf, ManageVars};
use rustsat::solvers::{ControlSignal, PhaseLit, Solve, SolverResult, Terminate};
use rustsat::types::{Clause, Lit};
use rustsat_cadical::CaDiCaL;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

enum WorkerCmd {
    Solve(Vec<Lit>),
    AddCnf(Cnf),
    Reseed(Cnf, Vec<Lit>),
    Stop,
}

enum WorkerMsg {
    Solution(usize, Vec<Lit>),
    #[allow(dead_code)]
    Unsat,
    Cancelled,
}

fn absorb_2opt(
    giant: &mut Vec<usize>,
    small_cycles: &[Vec<usize>],
    dir_adj: &[Vec<usize>],
) -> (usize, Vec<Vec<usize>>) {
    let mut current_giant = giant.clone();
    let mut remaining = Vec::new();
    let mut total_absorbed = 0;

    for _pass in 0..10 {
        let mut pass_absorbed = 0;
        let candidates = if remaining.is_empty() && total_absorbed == 0 {
            small_cycles.to_vec()
        } else {
            remaining.clone()
        };
        remaining.clear();

        for small in candidates {
            let n_g = current_giant.len();
            let n_s = small.len();
            let small_set: HashSet<usize> = small.iter().copied().collect();
            let giant_pos: HashMap<usize, usize> = current_giant.iter().enumerate().map(|(i, &v)| (v, i)).collect();
            let small_pos: HashMap<usize, usize> = small.iter().enumerate().map(|(i, &v)| (v, i)).collect();

            let mut found_swap = None;

            for &u1 in &current_giant {
                for &v2 in &dir_adj[u1] {
                    if small_set.contains(&v2) {
                        let i1 = giant_pos[&u1];
                        let v1 = current_giant[(i1 + 1) % n_g];

                        let j2 = small_pos[&v2];
                        let u2 = small[(j2 + n_s - 1) % n_s];

                        if dir_adj[u2].contains(&v1) {
                            found_swap = Some((i1, j2));
                            break;
                        }
                    }
                }
                if found_swap.is_some() {
                    break;
                }
            }

            if let Some((i1, j2)) = found_swap {
                let mut new_giant = Vec::with_capacity(n_g + n_s);
                for k in 0..=i1 {
                    new_giant.push(current_giant[k]);
                }
                for k in 0..n_s {
                    new_giant.push(small[(j2 + k) % n_s]);
                }
                for k in (i1 + 1)..n_g {
                    new_giant.push(current_giant[k]);
                }
                current_giant = new_giant;
                pass_absorbed += 1;
                total_absorbed += 1;
            } else {
                remaining.push(small);
            }
        }

        if pass_absorbed == 0 {
            break;
        }
    }

    *giant = current_giant;
    (total_absorbed, remaining)
}

fn sat_absorb_small_cycle(
    giant: &mut Vec<usize>,
    small: &[usize],
    dir_adj: &[Vec<usize>],
) -> bool {
    let n_g = giant.len();
    let n_s = small.len();
    if n_g < 3 || n_s < 3 {
        return false;
    }

    let giant_pos: HashMap<usize, usize> = giant.iter().enumerate().map(|(i, &v)| (v, i)).collect();
    let small_pos: HashMap<usize, usize> = small.iter().enumerate().map(|(i, &v)| (v, i)).collect();
    let small_set: HashSet<usize> = small.iter().copied().collect();

    for &u1 in giant.iter() {
        for &v2 in &dir_adj[u1] {
            if small_set.contains(&v2) {
                let i1 = giant_pos[&u1];
                let j2 = small_pos[&v2];

                for step in 1..n_s {
                    let u2 = small[(j2 + step) % n_s];
                    let w2 = small[(j2 + step + 1) % n_s];

                    for &w1 in &dir_adj[u2] {
                        if giant_pos.contains_key(&w1) {
                            let k1 = giant_pos[&w1];
                            let prev_k1 = giant[(k1 + n_g - 1) % n_g];

                            if dir_adj[prev_k1].contains(&w2) {
                                if (i1 < k1 && k1 <= n_g) || (i1 > k1) {
                                    let mut new_giant = Vec::with_capacity(n_g + n_s);
                                    let mut curr = 0;
                                    while curr <= i1 {
                                        new_giant.push(giant[curr]);
                                        curr += 1;
                                    }
                                    let mut s_idx = j2;
                                    loop {
                                        new_giant.push(small[s_idx]);
                                        if s_idx == (j2 + step) % n_s {
                                            break;
                                        }
                                        s_idx = (s_idx + 1) % n_s;
                                    }
                                    curr = k1;
                                    let end_curr = if i1 < k1 { n_g } else { i1 };
                                    while curr < end_curr {
                                        new_giant.push(giant[curr]);
                                        curr += 1;
                                    }
                                    if i1 > k1 {
                                        let mut s_rem = w2;
                                        while s_rem != v2 {
                                            new_giant.push(s_rem);
                                            let s_pos = small_pos[&s_rem];
                                            s_rem = small[(s_pos + 1) % n_s];
                                        }
                                    }
                                    if new_giant.len() == n_g + n_s {
                                        *giant = new_giant;
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

struct AlternatingPairGraph {
    contracted_g: Graph,
    chain_map: HashMap<(i32, i32), Vec<i32>>,
    pairs: Vec<(i32, i32)>,
    v_partner: HashMap<i32, i32>,
    color: HashMap<i32, u8>,
}

const MIN_DEG2_FRACTION: f64 = 0.15;

fn extract_alternating_pairs(raw_g: &Graph) -> Option<AlternatingPairGraph> {
    let deg2_count = raw_g.adjacency_list.values().filter(|nbrs| nbrs.len() == 2).count();
    let n = raw_g.adjacency_list.len();
    let deg2_fraction = deg2_count as f64 / n as f64;
    if deg2_fraction < MIN_DEG2_FRACTION {
        return None;
    }

    let series_decomp = contract_series_chains(raw_g);
    let g = series_decomp.contracted_g;
    let chain_map = series_decomp.chain_map;

    let mut v_partner: HashMap<i32, i32> = HashMap::new();
    let mut pairs: Vec<(i32, i32)> = Vec::new();
    let mut sorted_chains: Vec<(i32, i32)> = chain_map.keys().copied().collect();
    sorted_chains.sort_unstable();

    for (u, w) in sorted_chains {
        if u < w {
            pairs.push((u, w));
            v_partner.insert(u, w);
            v_partner.insert(w, u);
        }
    }

    if pairs.is_empty() {
        return None;
    }

    let mut color: HashMap<i32, u8> = HashMap::new();
    let (u0, w0) = pairs[0];
    color.insert(u0, 0);
    color.insert(w0, 1);
    let mut q = vec![u0, w0];

    while let Some(curr) = q.pop() {
        let curr_c = color[&curr];

        if let Some(&vp) = v_partner.get(&curr) {
            if let Some(&existing_color) = color.get(&vp) {
                if existing_color == curr_c {
                    return None;
                }
            } else {
                color.insert(vp, 1 - curr_c);
                q.push(vp);
            }
        }

        if let Some(nbrs) = g.adjacency_list.get(&curr) {
            let vp = v_partner.get(&curr).copied().unwrap_or(-1);
            for &nxt in nbrs {
                if nxt != vp {
                    if let Some(&existing_color) = color.get(&nxt) {
                        if existing_color == curr_c {
                            return None;
                        }
                    } else {
                        color.insert(nxt, 1 - curr_c);
                        q.push(nxt);
                    }
                }
            }
        }
    }

    if g.adjacency_list.len() != pairs.len() * 2 || color.len() != g.adjacency_list.len() {
        return None;
    }

    Some(AlternatingPairGraph {
        contracted_g: g,
        chain_map,
        pairs,
        v_partner,
        color,
    })
}

pub fn can_solve_alternating_pairs(raw_g: &Graph) -> bool {
    extract_alternating_pairs(raw_g).is_some()
}

pub fn solve_alternating_pairs(raw_g: &Graph, timeout_secs: f64) -> Option<Vec<i32>> {
    let t_start = Instant::now();
    let deadline = t_start + Duration::from_secs_f64(timeout_secs);

    let apg = extract_alternating_pairs(raw_g)?;

    let AlternatingPairGraph {
        contracted_g: g,
        chain_map,
        pairs,
        v_partner,
        color,
    } = apg;

    let n_dir = pairs.len();
    let mut node_to_id: HashMap<i32, usize> = HashMap::new();
    let mut id_to_pair: HashMap<usize, (i32, i32)> = HashMap::new();

    for (idx, &(u, w)) in pairs.iter().enumerate() {
        node_to_id.insert(u, idx);
        node_to_id.insert(w, idx);
        let u_c = color.get(&u).copied().unwrap_or(0);
        let in_v = if u_c == 0 { u } else { w };
        let out_v = if u_c == 1 { u } else { w };
        id_to_pair.insert(idx, (in_v, out_v));
    }

    let mut dir_adj: Vec<Vec<usize>> = vec![Vec::new(); n_dir];
    let mut arc_set: HashSet<(usize, usize)> = HashSet::new();

    for (&u, &c) in &color {
        if c == 1 {
            let u_id = node_to_id[&u];
            if let Some(nbrs) = g.adjacency_list.get(&u) {
                let mut sorted_nbrs = nbrs.clone();
                sorted_nbrs.sort_unstable();
                let vp = v_partner.get(&u).copied().unwrap_or(0);
                for v in sorted_nbrs {
                    if v != vp {
                        if let Some(&v_id) = node_to_id.get(&v) {
                            if arc_set.insert((u_id, v_id)) {
                                dir_adj[u_id].push(v_id);
                            }
                        }
                    }
                }
            }
        }
    }

    let mut var_mgr = BasicVarManager::default();
    let mut arc_lit_map: HashMap<(usize, usize), Lit> = HashMap::new();
    for &arc in &arc_set {
        arc_lit_map.insert(arc, var_mgr.new_var().pos_lit());
    }

    let mut in_arcs: Vec<Vec<usize>> = vec![Vec::new(); n_dir];
    for &(u, v) in &arc_set {
        in_arcs[v].push(u);
    }

    let mut cnf = Cnf::new();
    for u in 0..n_dir {
        let out_lits: Vec<Lit> = dir_adj[u].iter().map(|&v| arc_lit_map[&(u, v)]).collect();
        cnf.add_clause(Clause::from_iter(out_lits.iter().copied()));
        for i in 0..out_lits.len() {
            for j in (i + 1)..out_lits.len() {
                cnf.add_clause(clause!(!out_lits[i], !out_lits[j]));
            }
        }
    }
    for v in 0..n_dir {
        let in_lits: Vec<Lit> = in_arcs[v].iter().map(|&u| arc_lit_map[&(u, v)]).collect();
        cnf.add_clause(Clause::from_iter(in_lits.iter().copied()));
        for i in 0..in_lits.len() {
            for j in (i + 1)..in_lits.len() {
                cnf.add_clause(clause!(!in_lits[i], !in_lits[j]));
            }
        }
    }
    for &(u, v) in &arc_set {
        if u < v && arc_set.contains(&(v, u)) {
            cnf.add_clause(clause!(!arc_lit_map[&(u, v)], !arc_lit_map[&(v, u)]));
        }
    }

    // Static short cycle cuts
    let mut static_3c = Vec::new();
    for u in 0..n_dir {
        for &v in &dir_adj[u] {
            for &w in &dir_adj[v] {
                if w != u && dir_adj[w].contains(&u) {
                    if u < v && u < w {
                        static_3c.push(vec![u, v, w]);
                    }
                }
            }
        }
    }
    for c in &static_3c {
        let lits = vec![
            !arc_lit_map[&(c[0], c[1])],
            !arc_lit_map[&(c[1], c[2])],
            !arc_lit_map[&(c[2], c[0])],
        ];
        cnf.add_clause(Clause::from_iter(lits));
    }

    let backbone_hints: Vec<Lit> = Vec::new();
    let num_workers = 3;
    let (tx_res, rx_res) = mpsc::channel::<WorkerMsg>();
    let mut worker_senders: Vec<mpsc::Sender<WorkerCmd>> = Vec::new();
    let mut cancel_flags: Vec<Arc<AtomicBool>> = Vec::new();
    let mut worker_handles = Vec::new();

    for worker_id in 0..num_workers {
        let (tx_cmd, rx_cmd) = mpsc::channel::<WorkerCmd>();
        worker_senders.push(tx_cmd);
        let cancel_flag = Arc::new(AtomicBool::new(false));
        cancel_flags.push(cancel_flag.clone());

        let tx_res_clone = tx_res.clone();
        let cnf_clone = cnf.clone();
        let backbone_hints_clone = backbone_hints.clone();

        let handle = thread::spawn(move || {
            let mut solver = CaDiCaL::default();
            let _ = solver.set_option("chrono", 1);
            match worker_id {
                0 => {
                    let _ = solver.set_option("seed", 777);
                    let _ = solver.set_option("restartint", 50);
                }
                1 => {
                    let _ = solver.set_option("seed", 42);
                    let _ = solver.set_option("restartint", 100);
                }
                2 => {
                    let _ = solver.set_option("seed", 1337);
                    let _ = solver.set_option("restartint", 200);
                    let _ = solver.set_option("walk", 1);
                }
                _ => {}
            }

            for &lit in &backbone_hints_clone {
                let _ = solver.phase_lit(lit);
            }
            let _ = solver.add_cnf(cnf_clone.clone());

            let cancel_ref = cancel_flag.clone();
            solver.attach_terminator(move || {
                if cancel_ref.load(Ordering::Relaxed) || Instant::now() >= deadline {
                    ControlSignal::Terminate
                } else {
                    ControlSignal::Continue
                }
            });

            while let Ok(cmd) = rx_cmd.recv() {
                match cmd {
                    WorkerCmd::Solve(phase_hints) => {
                        cancel_flag.store(false, Ordering::SeqCst);
                        for &lit in &phase_hints {
                            let _ = solver.phase_lit(lit);
                        }
                        let res = solver.solve();
                        if cancel_flag.load(Ordering::Relaxed) {
                            let _ = tx_res_clone.send(WorkerMsg::Cancelled);
                        } else if matches!(res, Ok(SolverResult::Sat)) {
                            let sol = solver.full_solution().unwrap();
                            let _ = tx_res_clone.send(WorkerMsg::Solution(worker_id, sol.into_iter().collect()));
                        } else if matches!(res, Ok(SolverResult::Unsat)) {
                            let _ = tx_res_clone.send(WorkerMsg::Unsat);
                        } else {
                            let _ = tx_res_clone.send(WorkerMsg::Cancelled);
                        }
                    }
                    WorkerCmd::AddCnf(cuts) => {
                        let _ = solver.add_cnf(cuts);
                    }
                    WorkerCmd::Reseed(accumulated_cuts, hints) => {
                        solver = CaDiCaL::default();
                        let _ = solver.set_option("chrono", 1);
                        match worker_id {
                            0 => {
                                let _ = solver.set_option("seed", 777);
                                let _ = solver.set_option("restartint", 50);
                            }
                            1 => {
                                let _ = solver.set_option("seed", 42);
                                let _ = solver.set_option("restartint", 100);
                            }
                            2 => {
                                let _ = solver.set_option("seed", 1337);
                                let _ = solver.set_option("restartint", 200);
                                let _ = solver.set_option("walk", 1);
                            }
                            _ => {}
                        }
                        for &lit in &hints {
                            let _ = solver.phase_lit(lit);
                        }
                        let _ = solver.add_cnf(cnf_clone.clone());
                        let _ = solver.add_cnf(accumulated_cuts);
                        let c_ref = cancel_flag.clone();
                        solver.attach_terminator(move || {
                            if c_ref.load(Ordering::Relaxed) || Instant::now() >= deadline {
                                ControlSignal::Terminate
                            } else {
                                ControlSignal::Continue
                            }
                        });
                    }
                    WorkerCmd::Stop => break,
                }
            }
        });
        worker_handles.push(handle);
    }

    let mut current_hints = backbone_hints;
    let mut accumulated_cuts = Cnf::new();
    let mut result_tour = None;

    for round in 1..=500 {
        let t_round = Instant::now();

        if t_start.elapsed().as_secs_f64() > timeout_secs {
            break;
        }

        for flag in &cancel_flags {
            flag.store(false, Ordering::SeqCst);
        }
        for tx in &worker_senders {
            let _ = tx.send(WorkerCmd::Solve(current_hints.clone()));
        }

        let mut winning_sol = None;
        let mut finished_count = 0;

        while finished_count < num_workers {
            match rx_res.recv() {
                Ok(WorkerMsg::Solution(w_id, sol)) => {
                    if winning_sol.is_none() {
                        winning_sol = Some(sol);
                        for (idx, flag) in cancel_flags.iter().enumerate() {
                            if idx != w_id {
                                flag.store(true, Ordering::SeqCst);
                            }
                        }
                    }
                    finished_count += 1;
                }
                Ok(WorkerMsg::Unsat) | Ok(WorkerMsg::Cancelled) => {
                    finished_count += 1;
                }
                Err(_) => break,
            }
        }

        if winning_sol.is_none() {
            break;
        }

        let sol = winning_sol.unwrap();
        let sol_set: HashSet<Lit> = sol.into_iter().collect();

        let mut successor: HashMap<usize, usize> = HashMap::new();
        for (&(u, v), &lit) in &arc_lit_map {
            if sol_set.contains(&lit) {
                successor.insert(u, v);
            }
        }

        let mut visited = HashSet::new();
        let mut cycles = Vec::new();
        for start in 0..n_dir {
            if !visited.contains(&start) {
                let mut c = Vec::new();
                let mut curr = start;
                while !visited.contains(&curr) {
                    visited.insert(curr);
                    c.push(curr);
                    if let Some(&nxt) = successor.get(&curr) {
                        curr = nxt;
                    } else {
                        break;
                    }
                }
                if !c.is_empty() {
                    cycles.push(c);
                }
            }
        }

        cycles.sort_by_key(|c| std::cmp::Reverse(c.len()));

        let mut giant = cycles[0].clone();
        let (absorbed, remaining) = absorb_2opt(&mut giant, &cycles[1..], &dir_adj);
        let mut absorbed_3opt = 0;
        let mut final_remaining = Vec::new();
        for small in remaining {
            if sat_absorb_small_cycle(&mut giant, &small, &dir_adj) {
                absorbed_3opt += 1;
            } else {
                final_remaining.push(small);
            }
        }
        if absorbed > 0 || absorbed_3opt > 0 {
            cycles.clear();
            cycles.push(giant);
            cycles.extend(final_remaining);
            cycles.sort_by_key(|c| std::cmp::Reverse(c.len()));
        }

        let lens: Vec<usize> = cycles.iter().map(|c| c.len()).collect();
        let top5: Vec<usize> = lens.iter().take(5).copied().collect();
        println!(
            "[alternating_pairs] Round {:2} ({:?}): {} cycles (largest={}, top5={:?})",
            round,
            t_round.elapsed(),
            cycles.len(),
            cycles[0].len(),
            top5
        );

        if cycles.len() == 1 || cycles[0].len() == n_dir {
            let final_dir_tour = &cycles[0];
            let mut contracted_tour = Vec::with_capacity(n_dir * 2);
            for &dir_v in final_dir_tour {
                let (in_v, out_v) = id_to_pair[&dir_v];
                contracted_tour.push(in_v);
                contracted_tour.push(out_v);
            }
            let full_tour = expand_series_tour(&contracted_tour, &chain_map);
            let (valid, _err) = TourVerifier::verify(raw_g, &full_tour);
            if valid {
                result_tour = Some(full_tour);
                break;
            }
            break;
        }

        current_hints.clear();
        for c in &cycles {
            for i in 0..c.len() {
                let u = c[i];
                let v = c[(i + 1) % c.len()];
                if let Some(&lit) = arc_lit_map.get(&(u, v)) {
                    current_hints.push(lit);
                }
            }
        }

        let mut cuts = Cnf::new();
        for c in &cycles {
            if c.len() < n_dir {
                let mut lits = Vec::new();
                for i in 0..c.len() {
                    let u = c[i];
                    let v = c[(i + 1) % c.len()];
                    lits.push(!arc_lit_map[&(u, v)]);
                }
                cuts.add_clause(Clause::from_iter(lits));

                if c.len() <= 16 {
                    let c_set: HashSet<usize> = c.iter().copied().collect();
                    let mut out_lits = Vec::new();
                    for &u in c {
                        for &v in &dir_adj[u] {
                            if !c_set.contains(&v) { out_lits.push(arc_lit_map[&(u, v)]); }
                        }
                    }
                    if !out_lits.is_empty() { cuts.add_clause(Clause::from_iter(out_lits)); }

                    let mut in_lits = Vec::new();
                    for &v in c {
                        for &u in &in_arcs[v] {
                            if !c_set.contains(&u) { in_lits.push(arc_lit_map[&(u, v)]); }
                        }
                    }
                    if !in_lits.is_empty() { cuts.add_clause(Clause::from_iter(in_lits)); }
                }
            }
        }

        for cl in cuts.iter() {
            accumulated_cuts.add_clause(cl.clone());
        }

        let round_dt = t_round.elapsed();
        if round_dt > Duration::from_secs(15) || (round % 10 == 0 && round > 0) {
            println!(
                "[alternating_pairs] Round took {:?}. Reseeding workers with {} accumulated cuts...",
                round_dt,
                accumulated_cuts.len()
            );
            for tx in &worker_senders {
                let _ = tx.send(WorkerCmd::Reseed(accumulated_cuts.clone(), current_hints.clone()));
            }
        } else {
            for tx in &worker_senders {
                let _ = tx.send(WorkerCmd::AddCnf(cuts.clone()));
            }
        }
    }

    for tx in &worker_senders {
        let _ = tx.send(WorkerCmd::Stop);
    }
    for handle in worker_handles {
        let _ = handle.join();
    }

    result_tour
}
