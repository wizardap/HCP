use crate::component_meta_graph::ComponentMetaGraph;
use crate::graph::Graph;
use crate::two_tier_decomposer::DecompositionResult;
use std::collections::{HashMap, HashSet};

/// Independent Raw Graph Tour Verifier.
/// Verifies:
/// 1. tour.len() == g.adjacency_list.len() (matches total vertices in g)
/// 2. all vertices are distinct
/// 3. every consecutive pair (tour[i], tour[(i+1)%n]) is an edge in g
pub fn verify_tour_on_raw_graph(tour: &[i32], g: &Graph) -> bool {
    let n = g.adjacency_list.len();
    if n == 0 || tour.len() != n {
        return false;
    }

    let mut seen = HashSet::with_capacity(n);
    for &v in tour {
        if !g.adjacency_list.contains_key(&v) {
            return false;
        }
        if !seen.insert(v) {
            return false; // Duplicate vertex
        }
    }

    for i in 0..n {
        let u = tour[i];
        let v = tour[(i + 1) % n];
        if !is_edge_in_graph(g, u, v) {
            return false;
        }
    }

    true
}

#[inline]
pub fn is_edge_in_graph(g: &Graph, u: i32, v: i32) -> bool {
    if let Some(nbrs) = g.adjacency_list.get(&u) {
        nbrs.contains(&v)
    } else {
        false
    }
}

/// Helper struct for matching endpoint slots to hub demand slots for strip `si`.
fn find_boundary_matching_local(
    g: &Graph,
    decomp: &DecompositionResult,
    _si: usize,
    paths: &[Vec<i32>],
    dem: &HashMap<i32, usize>,
) -> Option<Vec<(i32, i32)>> {
    // 1. Collect endpoint slots: (vertex_id, slot_id_on_vertex)
    // For path of length > 1: (path[0], 0) and (path[last], 0)
    // For path of length == 1: (path[0], 0) and (path[0], 1)
    let mut endpt_slots: Vec<(i32, usize)> = Vec::new();
    for p in paths {
        if p.is_empty() {
            continue;
        }
        if p.len() == 1 {
            endpt_slots.push((p[0], 0));
            endpt_slots.push((p[0], 1));
        } else {
            endpt_slots.push((p[0], 0));
            endpt_slots.push((p[p.len() - 1], 0));
        }
    }

    // 2. Collect hub demand slots: (hub_id, slot_id_on_hub)
    // Sort hubs: M-hubs first, then B-hubs, then S-hubs (or by degree ascending)
    let mut relevant_hubs: Vec<i32> = dem
        .iter()
        .filter(|&(_, &count)| count > 0)
        .map(|(&h, _)| h)
        .collect();

    // Sort relevant hubs: M-hubs first, then B-hubs, then S-hubs
    relevant_hubs.sort_by_key(|&h| {
        if decomp.m_hubs.contains(&h) {
            0
        } else if decomp.b_hubs.contains(&h) {
            1
        } else if decomp.s_hubs.contains(&h) {
            2
        } else {
            3
        }
    });

    let mut hub_slots: Vec<(i32, usize)> = Vec::new();
    for &h in &relevant_hubs {
        let count = dem.get(&h).copied().unwrap_or(0);
        for slot_idx in 0..count {
            hub_slots.push((h, slot_idx));
        }
    }

    if hub_slots.len() != endpt_slots.len() {
        return None;
    }

    let n_h = hub_slots.len();
    let n_e = endpt_slots.len();
    if n_h == 0 {
        return Some(Vec::new());
    }

    // Build compatibility list: hub_slots[i] can connect to endpt_slots[j] iff (hub_id, vert_id) is an edge in G
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n_h];
    for i in 0..n_h {
        let h = hub_slots[i].0;
        for j in 0..n_e {
            let v = endpt_slots[j].0;
            if is_edge_in_graph(g, h, v) {
                adj[i].push(j);
            }
        }
    }

    // Kuhn's algorithm / augmenting path DFS with single-edge constraint per (hub, vertex)
    let mut match_r: Vec<Option<usize>> = vec![None; n_e];
    let mut match_l: Vec<Option<usize>> = vec![None; n_h];

    fn dfs_match(
        u: usize,
        visited: &mut [bool],
        match_r: &mut [Option<usize>],
        match_l: &mut [Option<usize>],
        adj: &[Vec<usize>],
        hub_slots: &[(i32, usize)],
        endpt_slots: &[(i32, usize)],
    ) -> bool {
        for &v in &adj[u] {
            if visited[v] {
                continue;
            }
            let h = hub_slots[u].0;
            let vert = endpt_slots[v].0;

            // Ensure no two slots of the same hub are matched to the same vertex
            let mut duplicate = false;
            for (other_u, &matched_v_opt) in match_l.iter().enumerate() {
                if other_u != u && hub_slots[other_u].0 == h {
                    if let Some(matched_v) = matched_v_opt {
                        if endpt_slots[matched_v].0 == vert {
                            duplicate = true;
                            break;
                        }
                    }
                }
            }
            if duplicate {
                continue;
            }

            visited[v] = true;
            if match_r[v].is_none() {
                match_r[v] = Some(u);
                match_l[u] = Some(v);
                return true;
            } else {
                let prev_u = match_r[v].unwrap();
                match_l[prev_u] = None;
                if dfs_match(
                    prev_u,
                    visited,
                    match_r,
                    match_l,
                    adj,
                    hub_slots,
                    endpt_slots,
                ) {
                    match_r[v] = Some(u);
                    match_l[u] = Some(v);
                    return true;
                } else {
                    match_l[prev_u] = Some(v);
                }
            }
        }
        false
    }

    for u in 0..n_h {
        let mut visited = vec![false; n_e];
        dfs_match(
            u,
            &mut visited,
            &mut match_r,
            &mut match_l,
            &adj,
            &hub_slots,
            &endpt_slots,
        );
    }

    if match_l.iter().any(|m| m.is_none()) {
        return None; // Could not match 100% of endpoints
    }

    let mut result_edges = Vec::with_capacity(n_h);
    for (u, opt_v) in match_l.iter().enumerate() {
        let v = opt_v.unwrap();
        let h = hub_slots[u].0;
        let endpoint_v = endpt_slots[v].0;
        result_edges.push((h, endpoint_v));
    }

    Some(result_edges)
}

/// Fast local 2-opt cycle patching to merge adjacent subcycles in 2-factor into fewer, larger cycles.
pub fn patch_cycles_2opt(mut cycles: Vec<Vec<i32>>, g: &Graph) -> Vec<Vec<i32>> {
    if cycles.len() <= 1 {
        return cycles;
    }

    // Repeatedly find a 2-opt merge between pairs of cycles until no more merges can be made.
    loop {
        if cycles.len() <= 1 {
            break;
        }

        let meta_graph = ComponentMetaGraph::build(&cycles, g);
        if meta_graph.cross_edges.is_empty() {
            return cycles;
        }

        let mut merged = false;
        let mut vert_map: HashMap<i32, (usize, usize)> = HashMap::new();
        for (c_idx, c) in cycles.iter().enumerate() {
            for (pos, &v) in c.iter().enumerate() {
                vert_map.insert(v, (c_idx, pos));
            }
        }

        'outer: for c1_idx in 0..cycles.len() {
            let p = cycles[c1_idx].len();
            for i in 0..p {
                let u1 = cycles[c1_idx][i];
                let v1 = cycles[c1_idx][(i + 1) % p];

                if let Some(neighbors) = g.adjacency_list.get(&u1) {
                    for &u2 in neighbors {
                        if let Some(&(c2_idx, j)) = vert_map.get(&u2) {
                            if c2_idx == c1_idx {
                                continue;
                            }
                            if !meta_graph.has_merge_potential(c1_idx, c2_idx) {
                                continue;
                            }

                            let q = cycles[c2_idx].len();
                            let v2 = cycles[c2_idx][(j + 1) % q];
                            let w2 = cycles[c2_idx][(j + q - 1) % q];

                            // Case 1: Cross edges (u1, u2) and (v1, v2) exist in G
                            // Replaces (u1, v1) in C1 and (u2, v2) in C2
                            if is_edge_in_graph(g, v1, v2) {
                                let mut new_cycle = Vec::with_capacity(p + q);
                                // C1 forward from v1 to u1:
                                for k in 1..=p {
                                    new_cycle.push(cycles[c1_idx][(i + k) % p]);
                                }
                                // C2 reverse from u2 to v2:
                                for k in 0..q {
                                    new_cycle.push(cycles[c2_idx][(j + q - (k % q)) % q]);
                                }

                                let mut next_cycles = Vec::with_capacity(cycles.len() - 1);
                                for (idx, c) in cycles.into_iter().enumerate() {
                                    if idx != c1_idx && idx != c2_idx {
                                        next_cycles.push(c);
                                    }
                                }
                                next_cycles.push(new_cycle);
                                cycles = next_cycles;
                                merged = true;
                                break 'outer;
                            }

                            // Case 2: Cross edges (u1, u2) and (v1, w2) exist in G
                            // Replaces (u1, v1) in C1 and (w2, u2) in C2
                            if is_edge_in_graph(g, v1, w2) {
                                let mut new_cycle = Vec::with_capacity(p + q);
                                // C1 forward from v1 to u1:
                                for k in 1..=p {
                                    new_cycle.push(cycles[c1_idx][(i + k) % p]);
                                }
                                // C2 forward from u2 to w2:
                                for k in 0..q {
                                    new_cycle.push(cycles[c2_idx][(j + k) % q]);
                                }

                                let mut next_cycles = Vec::with_capacity(cycles.len() - 1);
                                for (idx, c) in cycles.into_iter().enumerate() {
                                    if idx != c1_idx && idx != c2_idx {
                                        next_cycles.push(c);
                                    }
                                }
                                next_cycles.push(new_cycle);
                                cycles = next_cycles;
                                merged = true;
                                break 'outer;
                            }
                        }
                    }
                }
            }
        }

        if !merged {
            break;
        }
    }

    cycles
}

/// Splicer connecting internal paths, Hub-Hub edges, and boundary connections into a 2-factor (or single tour).
/// If a single valid tour is formed, returns (true, vec![tour]).
/// If multiple disjoint cycles remain, returns (false, cycles).
pub fn splice_macro_tour(
    g: &Graph,
    decomp: &DecompositionResult,
    hh_edges: &[(i32, i32)],
    strip_paths: &HashMap<usize, Vec<Vec<i32>>>,
    strip_demands: &HashMap<usize, HashMap<i32, usize>>,
    enable_patching: bool,
) -> (bool, Vec<Vec<i32>>) {
    let mut adj: HashMap<i32, Vec<i32>> = HashMap::new();
    for &v in g.adjacency_list.keys() {
        adj.insert(v, Vec::new());
    }

    // 1. Add internal edges from strip_paths
    for paths in strip_paths.values() {
        for p in paths {
            for i in 0..(p.len().saturating_sub(1)) {
                let u = p[i];
                let v = p[i + 1];
                adj.entry(u).or_default().push(v);
                adj.entry(v).or_default().push(u);
            }
        }
    }

    // 2. Add active HH edges
    for &(u, v) in hh_edges {
        adj.entry(u).or_default().push(v);
        adj.entry(v).or_default().push(u);
    }

    // 3. Boundary matching for all strips
    for (si, _) in decomp.strips.iter().enumerate() {
        if let Some(paths) = strip_paths.get(&si) {
            let dem = strip_demands.get(&si).cloned().unwrap_or_default();
            if let Some(matched_edges) = find_boundary_matching_local(g, decomp, si, paths, &dem) {
                for (h, v) in matched_edges {
                    adj.entry(h).or_default().push(v);
                    adj.entry(v).or_default().push(h);
                }
            } else {
                // Boundary matching failed for strip si
                return (false, Vec::new());
            }
        }
    }

    // 4. Validate exact degree == 2 on all vertices in G
    for (&_v, nbrs) in &adj {
        if nbrs.len() != 2 {
            return (false, Vec::new());
        }
    }

    // 5. Extract disjoint cycles
    let mut visited: HashSet<i32> = HashSet::new();
    let mut cycles: Vec<Vec<i32>> = Vec::new();

    let mut all_verts: Vec<i32> = adj.keys().copied().collect();
    all_verts.sort_unstable();

    for &start_v in &all_verts {
        if visited.contains(&start_v) {
            continue;
        }

        let mut cyc = Vec::new();
        let mut curr = start_v;
        let mut prev = -1;

        loop {
            visited.insert(curr);
            cyc.push(curr);

            let nbrs = &adj[&curr];
            let next_v = if nbrs[0] != prev { nbrs[0] } else { nbrs[1] };
            if next_v == start_v {
                break;
            }
            prev = curr;
            curr = next_v;
        }

        cycles.push(cyc);
    }

    // Sort cycles deterministically by descending length, then by min vertex
    cycles.sort_by(|a, b| {
        b.len()
            .cmp(&a.len())
            .then_with(|| a.iter().min().cmp(&b.iter().min()))
    });

    // 6. Optional 2-opt cycle patching
    if enable_patching && cycles.len() > 1 {
        cycles = patch_cycles_2opt(cycles, g);
        cycles.sort_by(|a, b| {
            b.len()
                .cmp(&a.len())
                .then_with(|| a.iter().min().cmp(&b.iter().min()))
        });
    }

    // 7. Verify single tour
    if cycles.len() == 1 && cycles[0].len() == g.adjacency_list.len() {
        if verify_tour_on_raw_graph(&cycles[0], g) {
            return (true, cycles);
        }
    }

    (false, cycles)
}
