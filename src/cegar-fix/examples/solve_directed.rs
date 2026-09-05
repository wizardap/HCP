use std::collections::{HashMap, HashSet};
use std::time::Instant;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::file_operations;
use cegar_fix::tour_verifier::TourVerifier;
use rustsat::clause;
use rustsat::instances::{BasicVarManager, Cnf, ManageVars};
use rustsat::solvers::{Solve, SolverResult};
use rustsat::types::{Clause, Lit};
use rustsat_cadical::CaDiCaL;

fn parallel_absorb_into_giant(
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
                let v1 = giant[(i1 + 1) % n_g];
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
    let graph_path = if args.len() > 1 { &args[1] } else { "FHCPCS-col/graph788.col" };
    let timeout_secs = if args.len() > 2 { args[2].parse::<f64>().unwrap_or(1800.0) } else { 1800.0 };

    let t_start = Instant::now();
    let raw_g = file_operations::input_to_graph(graph_path);
    let (g, contractor) = Degree2Contractor::contract(&raw_g);

    let mut v_partner: HashMap<i32, i32> = HashMap::new();
    let mut pairs: Vec<(i32, i32)> = Vec::new();
    for (&(u, w), _) in &contractor.chain_map {
        if u < w {
            pairs.push((u, w));
            v_partner.insert(u, w);
            v_partner.insert(w, u);
        }
    }

    if pairs.is_empty() {
        println!("Error: No degree-2 virtual pairs detected.");
        return;
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
                for &v in nbrs {
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
    let mut in_arcs: Vec<Vec<usize>> = vec![Vec::new(); n_dir];
    for &(u, v) in &arc_set {
        in_arcs[v].push(u);
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

    println!("Starting Directed Solving on {} (N_dir={}, Arcs={})...", graph_path, n_dir, arc_set.len());

    let mut solver = CaDiCaL::default();
    let _ = solver.add_cnf(cnf);

    for round in 1..=1000 {
        let t_round = Instant::now();
        let res = solver.solve();
        if !matches!(res, Ok(SolverResult::Sat)) {
            println!("Solver returned {:?}", res);
            break;
        }

        let sol = solver.full_solution().unwrap();
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
        println!("Round {:2} ({:?}): {} cycles (largest={})",
            round, t_round.elapsed(), cycles.len(), cycles[0].len());

        let mut giant = cycles[0].clone();
        let (absorbed_2opt, remaining) = parallel_absorb_into_giant(&mut giant, &cycles[1..], &dir_adj);

        let mut absorbed_3opt = 0;
        let mut final_remaining = Vec::new();
        for small in remaining {
            if sat_absorb_small_cycle(&mut giant, &small, &dir_adj) {
                absorbed_3opt += 1;
            } else {
                final_remaining.push(small);
            }
        }

        if absorbed_2opt + absorbed_3opt > 0 {
            println!("  >>> Splicer: 2-opt={}, 3-opt={} | Giant len: {} -> {}, remaining cycles: {}",
                absorbed_2opt, absorbed_3opt, cycles[0].len(), giant.len(), final_remaining.len());
        }

        if giant.len() == n_dir || cycles.len() == 1 {
            println!("\n=======================================================");
            println!("*** 100% HAMILTONIAN TOUR FOUND ON {} in {:?}! ***", graph_path, t_start.elapsed());
            println!("=======================================================");
            let final_dir_tour = if giant.len() == n_dir { giant } else { cycles[0].clone() };
            let mut contracted_tour = Vec::with_capacity(n_dir * 2);
            for &dir_v in &final_dir_tour {
                let (in_v, out_v) = id_to_pair[&dir_v];
                contracted_tour.push(in_v);
                contracted_tour.push(out_v);
            }
            let full_tour = contractor.uncontract_cycle(&contracted_tour);
            if TourVerifier::verify_raw_tour(&full_tour, &raw_g).is_ok() {
                println!("s SATISFIABLE");
                println!("SUCCESSFULLY VERIFIED FULL HAMILTONIAN TOUR OF {} VERTICES!", full_tour.len());
                let output_hcp = format!("scratch/found_tour_{}_certified.hcp", std::path::Path::new(graph_path).file_stem().unwrap().to_string_lossy());
                let _ = TourVerifier::write_tsplib_hcp(&full_tour, graph_path, &output_hcp);
            } else {
                println!("Tour verification failed on uncontracted graph!");
            }
            break;
        }

        if t_start.elapsed().as_secs_f64() > timeout_secs {
            println!("Timeout {:.1}s reached", timeout_secs);
            break;
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
                if c.len() <= 32 {
                    let c_set: HashSet<usize> = c.iter().copied().collect();
                    let mut out_lits = Vec::new();
                    for &u in c {
                        for &v in &dir_adj[u] {
                            if !c_set.contains(&v) {
                                out_lits.push(arc_lit_map[&(u, v)]);
                            }
                        }
                    }
                    if !out_lits.is_empty() {
                        cuts.add_clause(Clause::from_iter(out_lits));
                    }
                }
            }
        }
        let _ = solver.add_cnf(cuts);
    }
}
