use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::file_operations;
use cegar_fix::tour_verifier::TourVerifier;
use rustsat::clause;
use rustsat::instances::{BasicVarManager, Cnf, ManageVars};
use rustsat::solvers::{ControlSignal, PhaseLit, Solve, SolverResult, Terminate};
use rustsat::types::{Clause, Lit};
use rustsat_cadical::CaDiCaL;

enum WorkerCmd {
    Solve(Vec<Lit>),
    AddCnf(Cnf),
    Reseed(Cnf, Vec<Lit>),
    Stop,
}

enum WorkerMsg {
    Solution(usize, Vec<Lit>),
    Unsat(usize),
    #[allow(dead_code)]
    Cancelled(usize),
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

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let graph_path = if args.len() > 1 { &args[1] } else { "../../FHCPCS-col/graph788.col" };
    let timeout_secs: f64 = if args.len() > 2 { args[2].parse().unwrap_or(1200.0) } else { 1200.0 };

    println!("===============================================================");
    println!("   OPTIMIZED RESEEDING PORTFOLIO CEGAR: GRAPH788.COL           ");
    println!("===============================================================");

    let t_start = Instant::now();
    let raw_g = file_operations::input_to_graph(graph_path);
    let (g, contractor) = Degree2Contractor::contract(&raw_g);

    let mut v_partner: HashMap<i32, i32> = HashMap::new();
    let mut pairs: Vec<(i32, i32)> = Vec::new();
    let mut sorted_chains: Vec<(i32, i32)> = contractor.chain_map.keys().copied().collect();
    sorted_chains.sort();

    for (u, w) in sorted_chains {
        if u < w {
            pairs.push((u, w));
            v_partner.insert(u, w);
            v_partner.insert(w, u);
        }
    }

    let mut color: HashMap<i32, u8> = HashMap::new();
    let (u0, w0) = pairs[0];
    color.insert(u0, 0);
    color.insert(w0, 1);
    let mut q = vec![u0, w0];

    while let Some(curr) = q.pop() {
        let curr_c = color[&curr];
        let vp = v_partner[&curr];
        if !color.contains_key(&vp) {
            color.insert(vp, 1 - curr_c);
            q.push(vp);
        }
        if let Some(nbrs) = g.adjacency_list.get(&curr) {
            for &nxt in nbrs {
                if nxt != vp && !color.contains_key(&nxt) {
                    color.insert(nxt, 1 - curr_c);
                    q.push(nxt);
                }
            }
        }
    }

    let n_dir = pairs.len();
    let mut node_to_id: HashMap<i32, usize> = HashMap::new();
    let mut id_to_pair: HashMap<usize, (i32, i32)> = HashMap::new();

    for (idx, &(u, w)) in pairs.iter().enumerate() {
        node_to_id.insert(u, idx);
        node_to_id.insert(w, idx);
        let in_v = if color[&u] == 0 { u } else { w };
        let out_v = if color[&u] == 1 { u } else { w };
        id_to_pair.insert(idx, (in_v, out_v));
    }

    let mut dir_adj: Vec<Vec<usize>> = vec![Vec::new(); n_dir];
    let mut arc_set: HashSet<(usize, usize)> = HashSet::new();

    for (&u, &c) in &color {
        if c == 1 {
            let u_id = node_to_id[&u];
            if let Some(nbrs) = g.adjacency_list.get(&u) {
                let mut sorted_nbrs = nbrs.clone();
                sorted_nbrs.sort();
                for v in sorted_nbrs {
                    if v != v_partner[&u] {
                        let v_id = node_to_id[&v];
                        if arc_set.insert((u_id, v_id)) {
                            dir_adj[u_id].push(v_id);
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
    println!("Pre-injecting {} static 3-cycles...", static_3c.len());
    for c in &static_3c {
        let lits = vec![
            !arc_lit_map[&(c[0], c[1])],
            !arc_lit_map[&(c[1], c[2])],
            !arc_lit_map[&(c[2], c[0])],
        ];
        cnf.add_clause(Clause::from_iter(lits));
    }

    let mut static_4c = Vec::new();
    for u in 0..n_dir {
        for &v in &dir_adj[u] {
            for &w in &dir_adj[v] {
                if w != u {
                    for &x in &dir_adj[w] {
                        if x != u && x != v && dir_adj[x].contains(&u) {
                            if u < v && u < w && u < x {
                                static_4c.push(vec![u, v, w, x]);
                            }
                        }
                    }
                }
            }
        }
    }
    println!("Pre-injecting {} static 4-cycles...", static_4c.len());
    for c in &static_4c {
        let lits = vec![
            !arc_lit_map[&(c[0], c[1])],
            !arc_lit_map[&(c[1], c[2])],
            !arc_lit_map[&(c[2], c[3])],
            !arc_lit_map[&(c[3], c[0])],
        ];
        cnf.add_clause(Clause::from_iter(lits));
    }

    // Load clean backbone hints (giant only!)
    let possible_paths = [
        "../../scratch/graph788_best_cegar_edges.txt",
        "scratch/graph788_best_cegar_edges.txt",
        "../../scratch/graph788_giant_only_edges.txt",
        "scratch/graph788_giant_only_edges.txt",
        "/home/ubuntu/HCP/scratch/graph788_giant_only_edges.txt",
        "../../scratch/graph788_giant_raw_edges.txt",
    ];
    let mut backbone_hints: Vec<Lit> = Vec::new();
    for p in &possible_paths {
        if let Ok(bb_content) = std::fs::read_to_string(p) {
            for line in bb_content.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() == 2 {
                    if let (Ok(u), Ok(v)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>()) {
                        if let (Some(&b1), Some(&b2)) = (node_to_id.get(&u), node_to_id.get(&v)) {
                            let arc = if color[&u] == 1 && color[&v] == 0 {
                                (b1, b2)
                            } else {
                                (b2, b1)
                            };
                            if let Some(&lit) = arc_lit_map.get(&arc) {
                                backbone_hints.push(lit);
                            }
                        }
                    }
                }
            }
            println!("Loaded {} clean backbone phase hints from {}.", backbone_hints.len(), p);
            break;
        }
    }

    // Spawn 3 persistent parallel workers with chrono=1 and dynamic reseeding
    let num_workers = 3;
    let (tx_res, rx_res) = mpsc::channel::<WorkerMsg>();
    let mut worker_senders: Vec<mpsc::Sender<WorkerCmd>> = Vec::new();
    let mut cancel_flags: Vec<Arc<AtomicBool>> = Vec::new();

    for worker_id in 0..num_workers {
        let (tx_cmd, rx_cmd) = mpsc::channel::<WorkerCmd>();
        worker_senders.push(tx_cmd);
        let cancel_flag = Arc::new(AtomicBool::new(false));
        cancel_flags.push(cancel_flag.clone());

        let tx_res_clone = tx_res.clone();
        let cnf_clone = cnf.clone();
        let backbone_hints_clone = backbone_hints.clone();

        thread::spawn(move || {
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
                if cancel_ref.load(Ordering::Relaxed) {
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
                            let _ = tx_res_clone.send(WorkerMsg::Cancelled(worker_id));
                        } else if matches!(res, Ok(SolverResult::Sat)) {
                            let sol = solver.full_solution().unwrap();
                            let _ = tx_res_clone.send(WorkerMsg::Solution(worker_id, sol.into_iter().collect()));
                        } else if matches!(res, Ok(SolverResult::Unsat)) {
                            let _ = tx_res_clone.send(WorkerMsg::Unsat(worker_id));
                        } else {
                            let _ = tx_res_clone.send(WorkerMsg::Cancelled(worker_id));
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
                            if c_ref.load(Ordering::Relaxed) {
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
    }

    println!("Starting 3-Worker Reseeding Portfolio CEGAR Solving on {}...", graph_path);

    let mut current_hints = backbone_hints;
    let mut best_giant_len = 0;
    let mut best_cycle_count = usize::MAX;
    let mut accumulated_cuts = Cnf::new();

    for round in 1..=500 {
        let t_round = Instant::now();

        // Reset cancellation flags and broadcast Solve
        for flag in &cancel_flags {
            flag.store(false, Ordering::SeqCst);
        }
        for tx in &worker_senders {
            let _ = tx.send(WorkerCmd::Solve(current_hints.clone()));
        }

        // Wait for first winning solution
        let mut winning_sol = None;
        let mut winning_worker = 0;
        let mut finished_count = 0;

        while finished_count < num_workers {
            match rx_res.recv() {
                Ok(WorkerMsg::Solution(w_id, sol)) => {
                    if winning_sol.is_none() {
                        winning_sol = Some(sol);
                        winning_worker = w_id;
                        for (idx, flag) in cancel_flags.iter().enumerate() {
                            if idx != w_id {
                                flag.store(true, Ordering::SeqCst);
                            }
                        }
                    }
                    finished_count += 1;
                }
                Ok(WorkerMsg::Unsat(w_id)) => {
                    println!("Worker {} returned UNSAT", w_id);
                    finished_count += 1;
                }
                Ok(WorkerMsg::Cancelled(_)) => {
                    finished_count += 1;
                }
                Err(_) => break,
            }
        }

        if winning_sol.is_none() {
            println!("All workers failed or returned UNSAT.");
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

        // Fast 2-opt and 3-opt absorber
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
            println!("  >>> Splicer: 2-opt={}, 3-opt={} | Giant len: {} -> {}, remaining: {}",
                absorbed, absorbed_3opt, cycles[0].len(), giant.len(), final_remaining.len());
            cycles.clear();
            cycles.push(giant);
            cycles.extend(final_remaining);
            cycles.sort_by_key(|c| std::cmp::Reverse(c.len()));
        }

        let lens: Vec<usize> = cycles.iter().map(|c| c.len()).collect();
        let top5: Vec<usize> = lens.iter().take(5).copied().collect();
        println!("Round {:2} ({:?} | Worker {} won): {} cycles (largest={}, top5={:?})",
            round, t_round.elapsed(), winning_worker, cycles.len(), cycles[0].len(), top5);

        if cycles[0].len() > best_giant_len {
            best_giant_len = cycles[0].len();
            println!("  >>> New Best Giant: {}/{} blocks ({:.1}%)",
                best_giant_len, n_dir, 100.0 * (best_giant_len as f64) / (n_dir as f64));
        }

        if cycles.len() < best_cycle_count {
            best_cycle_count = cycles.len();
            println!("  >>> [Checkpoint] Best cycle count: {} cycles! Saving active edges...", best_cycle_count);
            let mut edge_out = String::new();
            for (&(u, v), &lit) in &arc_lit_map {
                if sol_set.contains(&lit) {
                    let u_raw = id_to_pair[&u].1;
                    let v_raw = id_to_pair[&v].0;
                    edge_out.push_str(&format!("{} {}\n", u_raw, v_raw));
                }
            }
            let _ = std::fs::write("../../scratch/graph788_best_cegar_edges.txt", edge_out);
        }

        if cycles.len() == 1 || cycles[0].len() == n_dir {
            println!("
=======================================================");
            println!("*** 100% HAMILTONIAN TOUR FOUND ON {} in {:?}! ***", graph_path, t_start.elapsed());
            println!("=======================================================");
            let final_dir_tour = &cycles[0];
            let mut contracted_tour = Vec::with_capacity(n_dir * 2);
            for &dir_v in final_dir_tour {
                let (in_v, out_v) = id_to_pair[&dir_v];
                contracted_tour.push(in_v);
                contracted_tour.push(out_v);
            }
            let full_tour = contractor.uncontract_cycle(&contracted_tour);
            if TourVerifier::verify_raw_tour(&full_tour, &raw_g).is_ok() {
                println!("s SATISFIABLE");
                println!("SUCCESSFULLY VERIFIED FULL HAMILTONIAN TOUR OF {} VERTICES!", full_tour.len());
                let output_hcp = "../../scratch/graph788/found_tour_graph788.hcp";
                let _ = std::fs::create_dir_all("../../scratch/graph788");
                let _ = TourVerifier::write_tsplib_hcp(&full_tour, graph_path, output_hcp);
            } else {
                println!("Tour verification failed on uncontracted graph!");
            }
            break;
        }

        if t_start.elapsed().as_secs_f64() > timeout_secs {
            println!("Timeout {:.1}s reached", timeout_secs);
            break;
        }

        // Dynamic phase hints: update to all cycles of current round
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

        // SMART BALANCED CUT SELECTION:
        let mut cuts = Cnf::new();
        for c in &cycles {
            if c.len() < n_dir {
                // 1. Negative clause for EVERY cycle < n_dir (including giant)
                let mut lits = Vec::new();
                for i in 0..c.len() {
                    let u = c[i];
                    let v = c[(i + 1) % c.len()];
                    lits.push(!arc_lit_map[&(u, v)]);
                }
                cuts.add_clause(Clause::from_iter(lits));

                // 2. Dual boundary cuts ONLY for small cycles (c.len() <= 16)
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

        // Accumulate cuts
        for cl in cuts.iter() {
            accumulated_cuts.add_clause(cl.clone());
        }

        // DYNAMIC RESEEDING:
        // If last round took > 15s or every 10 rounds: flush bloated learned clauses!
        let round_dt = t_round.elapsed();
        if round_dt > Duration::from_secs(15) || (round % 10 == 0 && round > 0) {
            println!("  >>> [Reseeder] Round took {:?}. Reseeding workers with {} accumulated clean cuts...",
                round_dt, accumulated_cuts.len());
            for tx in &worker_senders {
                let _ = tx.send(WorkerCmd::Reseed(accumulated_cuts.clone(), current_hints.clone()));
            }
        } else {
            // Broadcast incremental cuts
            for tx in &worker_senders {
                let _ = tx.send(WorkerCmd::AddCnf(cuts.clone()));
            }
        }
    }

    // Stop all workers
    for tx in &worker_senders {
        let _ = tx.send(WorkerCmd::Stop);
    }
}
