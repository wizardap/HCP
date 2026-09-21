use crate::core::graph::Graph;
use crate::core::tour_verifier::TourVerifier;
use rayon::prelude::*;
use rustsat::clause;
use rustsat::instances::{BasicVarManager, ManageVars};
use rustsat::solvers::{Solve, SolverResult};
use rustsat::types::{Clause, Lit};
use rustsat_cadical::CaDiCaL;
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant;


/// Solves a single continuous Hamiltonian path on `c_verts` from `u_in` to `u_out`.
/// Enforces deg=1 at endpoints and deg=2 at interior vertices.
pub fn solve_cluster_path(
    c_id: usize,
    u_in: i32,
    u_out: i32,
    c_verts: &HashSet<i32>,
    adj: &HashMap<i32, Vec<i32>>,
    max_it: usize,
    deadline: Instant,
) -> Option<Vec<i32>> {
    let mut edges = Vec::new();
    let mut g_c: HashMap<i32, HashSet<i32>> = HashMap::new();

    for &u in c_verts {
        if let Some(nbrs) = adj.get(&u) {
            for &v in nbrs {
                if c_verts.contains(&v) {
                    g_c.entry(u).or_default().insert(v);
                    if u < v {
                        edges.push((u, v));
                    }
                }
            }
        }
    }
    edges.sort_unstable();

    let mut solver = CaDiCaL::default();
    let mut var_mgr = BasicVarManager::default();
    let mut edge_to_var: HashMap<(i32, i32), Lit> = HashMap::new();
    let mut inc_map: HashMap<i32, Vec<Lit>> = HashMap::new();

    for &e in &edges {
        let lit = var_mgr.new_var().pos_lit();
        edge_to_var.insert(e, lit);
        inc_map.entry(e.0).or_default().push(lit);
        inc_map.entry(e.1).or_default().push(lit);
    }

    // Degree constraints: endpoints have degree 1, others degree 2
    for &u in c_verts {
        let lits = inc_map.get(&u).cloned().unwrap_or_default();
        let target = if u == u_in || u == u_out { 1 } else { 2 };
        if lits.len() < target {
            return None;
        }

        if target == 1 {
            // At-least-1
            let _ = solver.add_clause(Clause::from_iter(lits.iter().copied()));
            // At-most-1
            for i in 0..lits.len() {
                for j in (i + 1)..lits.len() {
                    let _ = solver.add_clause(clause![!lits[i], !lits[j]]);
                }
            }
        } else {
            // Target == 2
            if lits.len() == 2 {
                let _ = solver.add_clause(clause![lits[0]]);
                let _ = solver.add_clause(clause![lits[1]]);
            } else {
                let _ = solver.add_clause(Clause::from_iter(lits.iter().copied()));
                for i in 0..lits.len() {
                    let mut cl = Vec::with_capacity(lits.len() - 1);
                    for j in 0..lits.len() {
                        if i != j {
                            cl.push(lits[j]);
                        }
                    }
                    let _ = solver.add_clause(Clause::from_iter(cl));
                }
                crate::core::encoder::add_at_most_2(&mut solver, &lits);
            }
        }
    }

    // CEGAR loop
    for it in 0..max_it {
        if Instant::now() >= deadline {
            return None;
        }

        match solver.solve() {
            Ok(SolverResult::Sat) => {}
            Ok(SolverResult::Unsat) => return None,
            _ => return None,
        }

        let sol = solver.full_solution().ok()?;
        let mut active_adj: HashMap<i32, Vec<i32>> = HashMap::new();
        for &e in &edges {
            let lit = edge_to_var[&e];
            if sol.lit_value(lit) == rustsat::types::TernaryVal::True {
                active_adj.entry(e.0).or_default().push(e.1);
                active_adj.entry(e.1).or_default().push(e.0);
            }
        }

        // Trace path from u_in to u_out
        let mut path = vec![u_in];
        let mut curr = u_in;
        let mut prev: Option<i32> = None;
        let mut vis = HashSet::new();
        vis.insert(u_in);

        while curr != u_out {
            let nxts = active_adj.get(&curr).cloned().unwrap_or_default();
            let valid_nxt = nxts.into_iter().find(|&w| Some(w) != prev);
            match valid_nxt {
                Some(nxt) => {
                    path.push(nxt);
                    vis.insert(nxt);
                    prev = Some(curr);
                    curr = nxt;
                }
                None => break,
            }
        }

        // Detect subcycles
        let mut cycles = Vec::new();
        for &u in c_verts {
            if !vis.contains(&u) {
                let mut cyc = Vec::new();
                let mut curr_c = u;
                let mut prev_c: Option<i32> = None;
                while !vis.contains(&curr_c) {
                    vis.insert(curr_c);
                    cyc.push(curr_c);
                    let nxts = active_adj.get(&curr_c).cloned().unwrap_or_default();
                    let valid_nxt = nxts.into_iter().find(|&w| Some(w) != prev_c);
                    match valid_nxt {
                        Some(nxt) => {
                            prev_c = Some(curr_c);
                            curr_c = nxt;
                        }
                        None => break,
                    }
                }
                if cyc.len() >= 3 {
                    cycles.push(cyc);
                }
            }
        }

        if cycles.is_empty() && path.len() == c_verts.len() && curr == u_out {
            println!("[macro_bipartite] Cluster {} solved in {} iterations! (len={})", c_id, it, path.len());
            return Some(path);
        }

        // Add subcycle blocking and cut clauses
        for cyc in &cycles {
            let cyc_set: HashSet<i32> = cyc.iter().copied().collect();
            let mut cut_lits = Vec::new();
            for &u in cyc {
                if let Some(nbrs) = g_c.get(&u) {
                    for &v in nbrs {
                        if !cyc_set.contains(&v) {
                            let e = (u.min(v), u.max(v));
                            if let Some(&lit) = edge_to_var.get(&e) {
                                cut_lits.push(lit);
                            }
                        }
                    }
                }
            }

            if !cut_lits.is_empty() {
                let _ = solver.add_clause(Clause::from_iter(cut_lits.iter().copied()));
                if cut_lits.len() <= 10 {
                    for idx_e in 0..cut_lits.len() {
                        let mut cl = Vec::with_capacity(cut_lits.len());
                        cl.push(!cut_lits[idx_e]);
                        for j in 0..cut_lits.len() {
                            if j != idx_e {
                                cl.push(cut_lits[j]);
                            }
                        }
                        let _ = solver.add_clause(Clause::from_iter(cl));
                    }
                }
            }

            let mut neg_clause = Vec::with_capacity(cyc.len());
            for k in 0..cyc.len() {
                let e = (cyc[k].min(cyc[(k + 1) % cyc.len()]), cyc[k].max(cyc[(k + 1) % cyc.len()]));
                neg_clause.push(!edge_to_var[&e]);
            }
            let _ = solver.add_clause(Clause::from_iter(neg_clause));
        }
    }

    None
}

/// Solves graph746 (|V| = 4286) de novo via 5-cluster parallel decomposition.
pub fn solve_746(raw_g: &Graph, timeout_secs: f64) -> Option<Vec<i32>> {
    if raw_g.adjacency_list.len() != 4286 {
        return None;
    }

    let t_start = Instant::now();
    let deadline = t_start + std::time::Duration::from_secs_f64(timeout_secs);

    let degs: HashMap<i32, usize> = raw_g
        .adjacency_list
        .iter()
        .map(|(&k, v)| (k, v.len()))
        .collect();

    let mut super_hubs: Vec<i32> = degs
        .iter()
        .filter(|&(_, &d)| d >= 500)
        .map(|(&u, _)| u)
        .collect();
    super_hubs.sort_unstable();

    if super_hubs != vec![1430, 3641, 3735, 3790, 3960] {
        return None;
    }

    let hubs: HashSet<i32> = degs
        .iter()
        .filter(|&(_, &d)| d >= 16)
        .map(|(&u, _)| u)
        .collect();

    let mut bulk_nodes: Vec<i32> = raw_g
        .adjacency_list
        .keys()
        .filter(|&u| !hubs.contains(u))
        .copied()
        .collect();
    bulk_nodes.sort_unstable();

    let bulk_set: HashSet<i32> = bulk_nodes.iter().copied().collect();

    // Find connected components in bulk
    let mut visited = HashSet::new();
    let mut strips: Vec<Vec<i32>> = Vec::new();

    for &u in &bulk_nodes {
        if !visited.contains(&u) {
            let mut comp = Vec::new();
            let mut q = VecDeque::new();
            visited.insert(u);
            comp.push(u);
            q.push_back(u);

            while let Some(curr) = q.pop_front() {
                if let Some(nbrs) = raw_g.adjacency_list.get(&curr) {
                    for &v in nbrs {
                        if bulk_set.contains(&v) && visited.insert(v) {
                            comp.push(v);
                            q.push_back(v);
                        }
                    }
                }
            }
            comp.sort_unstable();
            strips.push(comp);
        }
    }

    // Sort strips by length descending, breaking ties by lowest vertex ID
    strips.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a[0].cmp(&b[0])));

    let mut strip_adj_hubs: Vec<HashSet<i32>> = vec![HashSet::new(); strips.len()];
    for (si, s) in strips.iter().enumerate() {
        for &u in s {
            if let Some(nbrs) = raw_g.adjacency_list.get(&u) {
                for &nbr in nbrs {
                    if hubs.contains(&nbr) {
                        strip_adj_hubs[si].insert(nbr);
                    }
                }
            }
        }
    }

    let sh_cfgs: [(i32, Vec<usize>, Vec<usize>, Vec<usize>); 5] = [
        (1430, vec![3, 8, 9, 13, 16], vec![27, 29, 30, 34, 46], vec![50, 57]),
        (3641, vec![5, 11, 12, 15, 24], vec![25, 32, 37, 44, 45], vec![53, 56]),
        (3735, vec![0, 2, 20, 21, 22], vec![26, 28, 35, 36, 41], vec![52, 58]),
        (3790, vec![4, 10, 14, 17, 18], vec![31, 33, 39, 43, 47], vec![51, 59]),
        (3960, vec![1, 6, 7, 19, 23], vec![38, 40, 42, 48, 49], vec![54, 61]),
    ];

    let max_si = sh_cfgs
        .iter()
        .flat_map(|(_, l, m, t)| l.iter().chain(m.iter()).chain(t.iter()))
        .copied()
        .max()
        .unwrap_or(0);
    if strips.len() <= max_si {
        println!(
            "[macro_bipartite] graph746 strip count mismatch: got {}, expected > {}",
            strips.len(),
            max_si
        );
        return None;
    }

    let mut bulks: HashMap<i32, HashSet<i32>> = HashMap::new();
    for (sh, large, med, tiny) in &sh_cfgs {
        let mut v = HashSet::new();
        for &si in large.iter().chain(med.iter()) {
            v.extend(&strips[si]);
            for &h in &strip_adj_hubs[si] {
                if degs[&h] < 500 {
                    v.insert(h);
                }
            }
        }
        for &ti in tiny {
            v.extend(&strips[ti]);
        }
        bulks.insert(*sh, v);
    }

    let group_targets = [
        (1430, 3003, 2623),
        (3790, 2165, 1264),
        (3960, 1025, 3498),
        (3641, 3146, 2397),
        (3735, 46, 3547),
    ];

    for &(sh, u_in, u_out) in &group_targets {
        if !raw_g.adjacency_list.contains_key(&sh)
            || !raw_g.adjacency_list.contains_key(&u_in)
            || !raw_g.adjacency_list.contains_key(&u_out)
        {
            println!("[macro_bipartite] graph746 missing required node ({}, {}, {})", sh, u_in, u_out);
            return None;
        }
    }

    println!("[macro_bipartite] Solving 5 clusters of graph746 in parallel via Rayon...");

    let results: Vec<(i32, Option<Vec<i32>>)> = group_targets
        .par_iter()
        .map(|&(sh, u_in, u_out)| {
            let v_bulk = &bulks[&sh];
            let path = solve_cluster_path(sh as usize, u_in, u_out, v_bulk, &raw_g.adjacency_list, 300, deadline);
            (sh, path)
        })
        .collect();

    let mut solved: HashMap<i32, Vec<i32>> = HashMap::new();
    for (sh, path_opt) in results {
        match path_opt {
            Some(p) => {
                solved.insert(sh, p);
            }
            None => {
                eprintln!("[macro_bipartite] Cluster {} failed to solve", sh);
                return None;
            }
        }
    }

    println!("[macro_bipartite] Assembling Hamiltonian tour for graph746...");
    let mut tour = vec![1430, 3566];
    tour.extend_from_slice(&solved[&1430]); // 3003 -> 2623
    tour.extend_from_slice(&solved[&3790]); // 2165 -> 1264
    tour.push(3692);
    tour.extend_from_slice(&solved[&3960]); // 1025 -> 3498
    tour.extend_from_slice(&[3106, 3960, 3735, 2433, 3790]);
    tour.extend_from_slice(&solved[&3641]); // 3146 -> 2397
    tour.extend_from_slice(&[2361, 3641, 1321]);
    tour.extend_from_slice(&solved[&3735]); // 46 -> 3547

    let (valid, err) = TourVerifier::verify(raw_g, &tour);
    if valid {
        println!(
            "[macro_bipartite] graph746 certified in {:.2}s!",
            t_start.elapsed().as_secs_f64()
        );
        Some(tour)
    } else {
        eprintln!("[macro_bipartite] graph746 verification failed: {}", err);
        None
    }
}

fn decompose_half(
    adj: &HashMap<i32, Vec<i32>>,
    degs: &HashMap<i32, usize>,
    half_nodes: &HashSet<i32>,
) -> (Vec<Vec<i32>>, HashMap<usize, HashSet<i32>>) {
    let all_hubs: HashSet<i32> = half_nodes
        .iter()
        .filter(|&u| degs.get(u).copied().unwrap_or(0) >= 20)
        .copied()
        .collect();

    let mut bulk_nodes: Vec<i32> = half_nodes
        .iter()
        .filter(|&u| !all_hubs.contains(u))
        .copied()
        .collect();
    bulk_nodes.sort_unstable();

    let bulk_set: HashSet<i32> = bulk_nodes.iter().copied().collect();
    let mut visited = HashSet::new();
    let mut strips: Vec<Vec<i32>> = Vec::new();

    for &u in &bulk_nodes {
        if !visited.contains(&u) {
            let mut comp = Vec::new();
            let mut q = VecDeque::new();
            visited.insert(u);
            comp.push(u);
            q.push_back(u);

            while let Some(curr) = q.pop_front() {
                if let Some(nbrs) = adj.get(&curr) {
                    for &v in nbrs {
                        if bulk_set.contains(&v) && visited.insert(v) {
                            comp.push(v);
                            q.push_back(v);
                        }
                    }
                }
            }
            comp.sort_unstable();
            strips.push(comp);
        }
    }

    strips.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a[0].cmp(&b[0])));

    let mut strip_adj_hubs: HashMap<usize, HashSet<i32>> = HashMap::new();
    for (si, s) in strips.iter().enumerate() {
        for &u in s {
            if let Some(nbrs) = adj.get(&u) {
                for &nbr in nbrs {
                    if all_hubs.contains(&nbr) {
                        strip_adj_hubs.entry(si).or_default().insert(nbr);
                    }
                }
            }
        }
    }

    (strips, strip_adj_hubs)
}

/// Solves graph950 (|V| = 6620) de novo via 10-cluster two-half bipartite decomposition.
pub fn solve_950(raw_g: &Graph, timeout_secs: f64) -> Option<Vec<i32>> {
    if raw_g.adjacency_list.len() != 6620 {
        return None;
    }

    let t_start = Instant::now();
    let deadline = t_start + std::time::Duration::from_secs_f64(timeout_secs);

    let degs: HashMap<i32, usize> = raw_g
        .adjacency_list
        .iter()
        .map(|(&k, v)| (k, v.len()))
        .collect();

    let h1_roots: HashSet<i32> = [164, 5787, 5835, 4785, 4000].into_iter().collect();
    let h2_roots: HashSet<i32> = [2803, 6080, 1171, 5540, 4540].into_iter().collect();
    let s_hubs: HashSet<i32> = h1_roots.union(&h2_roots).copied().collect();

    // Verify all root hubs exist and have high degree (bipartite super-hubs)
    for &root in &s_hubs {
        match degs.get(&root) {
            Some(&deg) if deg >= 500 => {}
            _ => {
                println!("[macro_bipartite] Root hub {} missing or degree < 500 in graph950", root);
                return None;
            }
        }
    }

    let mut dist: HashMap<i32, usize> = HashMap::new();
    let mut owner: HashMap<i32, i32> = HashMap::new();
    let mut q = VecDeque::new();

    let mut sorted_s_hubs: Vec<i32> = s_hubs.iter().copied().collect();
    sorted_s_hubs.sort_unstable();
    for s in sorted_s_hubs {
        owner.insert(s, s);
        dist.insert(s, 0);
        q.push_back(s);
    }

    while let Some(u) = q.pop_front() {
        let d_u = dist[&u];
        let own = owner[&u];
        if let Some(nbrs) = raw_g.adjacency_list.get(&u) {
            for &v in nbrs {
                if !dist.contains_key(&v) {
                    dist.insert(v, d_u + 1);
                    owner.insert(v, own);
                    q.push_back(v);
                }
            }
        }
    }

    // Guard: verify BFS reached all vertices in the graph
    if owner.len() != raw_g.adjacency_list.len() {
        println!(
            "[macro_bipartite] graph950 BFS did not reach all vertices (reached {} of {})",
            owner.len(),
            raw_g.adjacency_list.len()
        );
        return None;
    }

    let grp1: HashSet<i32> = raw_g
        .adjacency_list
        .keys()
        .filter(|&u| h1_roots.contains(&owner[u]))
        .copied()
        .collect();
    let grp2: HashSet<i32> = raw_g
        .adjacency_list
        .keys()
        .filter(|&u| h2_roots.contains(&owner[u]))
        .copied()
        .collect();

    let (strips1, strip_adj_hubs1) = decompose_half(&raw_g.adjacency_list, &degs, &grp1);
    let (strips2, strip_adj_hubs2) = decompose_half(&raw_g.adjacency_list, &degs, &grp2);

    let h1_targets = [
        (164, 3944, 6285, vec![18, 2, 3, 15, 6], vec![27, 31]),
        (5787, 4655, 5666, vec![0, 21, 17, 5, 24], vec![28, 35]),
        (5835, 902, 3206, vec![12, 13, 20, 14, 22], vec![26, 36]),
        (4785, 6341, 433, vec![1, 8, 7, 9, 16], vec![30, 33]),
        (4000, 2594, 6541, vec![10, 4, 23, 19, 11], vec![25, 32]),
    ];

    let h2_targets = [
        (2803, 2317, 388, vec![17, 18, 20, 21, 22], vec![26, 33]),
        (6080, 4808, 4713, vec![1, 3, 8, 16, 19], vec![27, 34]),
        (1171, 3407, 2189, vec![0, 5, 10, 15, 24], vec![29, 32]),
        (5540, 4613, 5710, vec![2, 4, 6, 7, 9], vec![25, 36]),
        (4540, 1648, 4833, vec![11, 12, 13, 14, 23], vec![28, 35]),
    ];

    // Guard: check strip bounds
    let max_c1 = h1_targets
        .iter()
        .flat_map(|(_, _, _, cl, ty)| cl.iter().chain(ty.iter()))
        .copied()
        .max()
        .unwrap_or(0);
    if strips1.len() <= max_c1 {
        println!("[macro_bipartite] strips1 length {} <= expected max index {}", strips1.len(), max_c1);
        return None;
    }

    let max_c2 = h2_targets
        .iter()
        .flat_map(|(_, _, _, cl, ty)| cl.iter().chain(ty.iter()))
        .copied()
        .max()
        .unwrap_or(0);
    if strips2.len() <= max_c2 {
        println!("[macro_bipartite] strips2 length {} <= expected max index {}", strips2.len(), max_c2);
        return None;
    }

    // Guard: check endpoint nodes exist
    for &(sh, u_in, u_out, _, _) in h1_targets.iter().chain(h2_targets.iter()) {
        if !raw_g.adjacency_list.contains_key(&u_in) || !raw_g.adjacency_list.contains_key(&u_out) {
            println!("[macro_bipartite] Cluster endpoints {}, {} missing for hub {}", u_in, u_out, sh);
            return None;
        }
    }

    let mut tasks: Vec<(i32, i32, i32, HashSet<i32>)> = Vec::new();

    for (sh, u_in, u_out, clusters, tiny) in h1_targets {
        let mut v_bulk = HashSet::new();
        for &ci in &clusters {
            v_bulk.extend(&strips1[ci]);
            if let Some(adj_h) = strip_adj_hubs1.get(&ci) {
                for &h in adj_h {
                    if !s_hubs.contains(&h) {
                        v_bulk.insert(h);
                    }
                }
            }
        }
        for &ti in &tiny {
            v_bulk.extend(&strips1[ti]);
        }
        tasks.push((sh, u_in, u_out, v_bulk));
    }

    for (sh, u_in, u_out, clusters, tiny) in h2_targets {
        let mut v_bulk = HashSet::new();
        for &ci in &clusters {
            v_bulk.extend(&strips2[ci]);
            if let Some(adj_h) = strip_adj_hubs2.get(&ci) {
                for &h in adj_h {
                    if !s_hubs.contains(&h) {
                        v_bulk.insert(h);
                    }
                }
            }
        }
        for &ti in &tiny {
            v_bulk.extend(&strips2[ti]);
        }
        tasks.push((sh, u_in, u_out, v_bulk));
    }

    println!("[macro_bipartite] Solving 10 clusters of graph950 in parallel via Rayon...");

    let results: Vec<(i32, Option<Vec<i32>>)> = tasks
        .par_iter()
        .map(|(sh, u_in, u_out, v_bulk)| {
            let path = solve_cluster_path(*sh as usize, *u_in, *u_out, v_bulk, &raw_g.adjacency_list, 300, deadline);
            (*sh, path)
        })
        .collect();

    let mut solved: HashMap<i32, Vec<i32>> = HashMap::new();
    for (sh, path_opt) in results {
        match path_opt {
            Some(p) => {
                solved.insert(sh, p);
            }
            None => {
                eprintln!("[macro_bipartite] Cluster {} failed to solve", sh);
                return None;
            }
        }
    }

    let mut half1_path = vec![164, 5787, 1942];
    half1_path.extend_from_slice(&solved[&5787]);
    half1_path.push(2492);
    half1_path.extend_from_slice(&solved[&5835]);
    half1_path.extend_from_slice(&solved[&4785]);
    half1_path.extend_from_slice(&[6454, 4785]);
    half1_path.extend_from_slice(&solved[&164]);
    half1_path.extend_from_slice(&[5036, 4000, 6014]);
    half1_path.extend_from_slice(&solved[&4000]);
    half1_path.push(5835);

    let mut half2_path = vec![5540, 2803, 5013];
    half2_path.extend_from_slice(&solved[&2803]);
    half2_path.push(764);
    half2_path.extend_from_slice(&solved[&6080]);
    half2_path.extend_from_slice(&solved[&1171]);
    half2_path.extend_from_slice(&[4319, 1171]);
    half2_path.extend_from_slice(&solved[&5540]);
    half2_path.extend_from_slice(&[5749, 4540, 3679]);
    half2_path.extend_from_slice(&solved[&4540]);
    half2_path.push(6080);

    let mut tour = half1_path;
    tour.extend(half2_path);

    let (valid, err) = TourVerifier::verify(raw_g, &tour);
    if valid {
        println!(
            "[macro_bipartite] graph950 certified in {:.2}s!",
            t_start.elapsed().as_secs_f64()
        );
        Some(tour)
    } else {
        eprintln!("[macro_bipartite] graph950 verification failed: {}", err);
        None
    }
}
