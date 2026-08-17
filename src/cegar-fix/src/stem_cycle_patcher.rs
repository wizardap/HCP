use std::collections::HashSet;
use std::time::Instant;
use crate::contraction::Degree2Contractor;
use crate::graph::Graph;
use crate::hub_registry::HubRegistry;

/// Fast pseudo-random number generator for deterministic Stem-and-Cycle rotations.
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0x853c49e6748fea9b } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    fn gen_range(&mut self, low: usize, high: usize) -> usize {
        if low >= high {
            return low;
        }
        let range = (high - low) as u64;
        low + (self.next_u64() % range) as usize
    }
}

pub struct StemCyclePatcher;

impl StemCyclePatcher {
    /// Solves Hamiltonian cycle using fast multi-start Stem-and-Cycle Extension-Rotation.
    /// Strictly protects contracted degree-2 chains from contractor.chain_map.
    pub fn solve_via_stem_and_cycle(
        g: &Graph,
        contractor: &Degree2Contractor,
        _hub_registry: &HubRegistry,
        time_limit_secs: f64,
    ) -> Option<Vec<i32>> {
        let total_v = g.adjacency_list.len();
        if total_v < 3 {
            return None;
        }

        let max_v = g.adjacency_list.keys().copied().max().unwrap_or(0).max(total_v as i32) as usize;
        let mut vertex_list: Vec<i32> = g.adjacency_list.keys().copied().collect();
        vertex_list.sort_unstable();

        // Build flat adjacency arrays and lookup table for O(1) checks
        let mut adj_flat: Vec<Vec<i32>> = vec![Vec::new(); max_v + 1];
        let mut adj_set: Vec<HashSet<i32>> = vec![HashSet::new(); max_v + 1];
        let mut deg: Vec<usize> = vec![0; max_v + 1];

        for (&u, neighbors) in &g.adjacency_list {
            let u_idx = u as usize;
            if u_idx <= max_v {
                adj_flat[u_idx] = neighbors.clone();
                adj_set[u_idx] = neighbors.iter().copied().collect();
                deg[u_idx] = neighbors.len();
            }
        }

        // Build protected edge lookup for degree-2 contraction safety
        // An undirected edge (u, v) is protected if (u, v) or (v, u) is in chain_map
        let mut is_protected_edge: HashSet<(i32, i32)> = HashSet::new();
        for (&(u, v), _) in &contractor.chain_map {
            is_protected_edge.insert((u, v));
            is_protected_edge.insert((v, u));
        }

        let start_time = Instant::now();
        let mut rng = SimpleRng::new(42);
        let mut best_length = 0;

        // Preallocate data structures
        let mut stem: Vec<i32> = Vec::with_capacity(total_v + 1);
        let mut in_stem: Vec<bool> = vec![false; max_v + 1];
        let mut stem_pos: Vec<usize> = vec![0; max_v + 1];

        let mut restart = 0;
        while start_time.elapsed().as_secs_f64() < time_limit_secs {
            restart += 1;
            stem.clear();
            in_stem.fill(false);

            // Pick a random starting vertex
            let start_idx = rng.gen_range(0, vertex_list.len());
            let start_v = vertex_list[start_idx];
            stem.push(start_v);
            in_stem[start_v as usize] = true;
            stem_pos[start_v as usize] = 0;

            let max_steps = total_v * 100;
            let mut steps_since_extension = 0;

            for _step in 0..max_steps {
                if start_time.elapsed().as_secs_f64() >= time_limit_secs {
                    break;
                }

                // If path reaches full vertex count, check for closing edge
                if stem.len() == total_v {
                    let head = stem[0];
                    let tail = stem[total_v - 1];
                    if adj_set[head as usize].contains(&tail) {
                        if is_valid_cycle(&stem, g) {
                            return Some(stem);
                        }
                    }

                    // Posa rotation on full path to find closing edge
                    let tail_nbrs = &adj_flat[tail as usize];
                    if !tail_nbrs.is_empty() {
                        let target = tail_nbrs[rng.gen_range(0, tail_nbrs.len())];
                        let t_idx = stem_pos[target as usize];
                        if t_idx + 1 < total_v {
                            let u = stem[t_idx];
                            let v = stem[t_idx + 1];
                            if !is_protected_edge.contains(&(u, v)) {
                                stem[t_idx + 1..].reverse();
                                for k in (t_idx + 1)..total_v {
                                    stem_pos[stem[k] as usize] = k;
                                }
                            }
                        }
                    }
                    continue;
                }

                // Extension from tail
                let tail = stem[stem.len() - 1];
                let tail_nbrs = &adj_flat[tail as usize];
                let mut best_unvis: Option<i32> = None;
                let mut min_d = usize::MAX;

                for &nbr in tail_nbrs {
                    if !in_stem[nbr as usize] {
                        let d = deg[nbr as usize];
                        if d < min_d {
                            min_d = d;
                            best_unvis = Some(nbr);
                        }
                    }
                }

                if let Some(nxt) = best_unvis {
                    let pos = stem.len();
                    stem.push(nxt);
                    in_stem[nxt as usize] = true;
                    stem_pos[nxt as usize] = pos;
                    steps_since_extension = 0;
                    continue;
                }

                // Extension from head
                let head = stem[0];
                let head_nbrs = &adj_flat[head as usize];
                let mut best_head_unvis: Option<i32> = None;
                let mut min_hd = usize::MAX;

                for &nbr in head_nbrs {
                    if !in_stem[nbr as usize] {
                        let d = deg[nbr as usize];
                        if d < min_hd {
                            min_hd = d;
                            best_head_unvis = Some(nbr);
                        }
                    }
                }

                if let Some(nxt) = best_head_unvis {
                    stem.insert(0, nxt);
                    in_stem[nxt as usize] = true;
                    for (k, &v) in stem.iter().enumerate() {
                        stem_pos[v as usize] = k;
                    }
                    steps_since_extension = 0;
                    continue;
                }

                // Guided Posa rotation from tail
                let mut rotated = false;
                if !tail_nbrs.is_empty() {
                    let mut best_target: Option<i32> = None;
                    let mut max_new_unvis = 0;
                    let mut valid_targets: Vec<i32> = Vec::with_capacity(tail_nbrs.len());

                    for &target in tail_nbrs {
                        if in_stem[target as usize] {
                            let t_idx = stem_pos[target as usize];
                            if t_idx + 1 < stem.len() {
                                let u = stem[t_idx];
                                let v = stem[t_idx + 1];
                                if !is_protected_edge.contains(&(u, v)) {
                                    valid_targets.push(target);
                                    // Check how many unvisited neighbors the new tail (v) has
                                    let v_unvis = adj_flat[v as usize].iter().filter(|&&w| !in_stem[w as usize]).count();
                                    if v_unvis > max_new_unvis {
                                        max_new_unvis = v_unvis;
                                        best_target = Some(target);
                                    }
                                }
                            }
                        }
                    }

                    let chosen_target = if max_new_unvis > 0 {
                        best_target
                    } else if !valid_targets.is_empty() {
                        Some(valid_targets[rng.gen_range(0, valid_targets.len())])
                    } else {
                        None
                    };

                    if let Some(target) = chosen_target {
                        let t_idx = stem_pos[target as usize];
                        stem[t_idx + 1..].reverse();
                        for k in (t_idx + 1)..stem.len() {
                            stem_pos[stem[k] as usize] = k;
                        }
                        rotated = true;
                    }
                }

                // Guided Posa rotation from head
                if !rotated && !head_nbrs.is_empty() {
                    let mut best_target: Option<i32> = None;
                    let mut max_new_unvis = 0;
                    let mut valid_targets: Vec<i32> = Vec::with_capacity(head_nbrs.len());

                    for &target in head_nbrs {
                        if in_stem[target as usize] {
                            let t_idx = stem_pos[target as usize];
                            if t_idx > 0 {
                                let u = stem[t_idx - 1];
                                let v = stem[t_idx];
                                if !is_protected_edge.contains(&(u, v)) {
                                    valid_targets.push(target);
                                    let u_unvis = adj_flat[u as usize].iter().filter(|&&w| !in_stem[w as usize]).count();
                                    if u_unvis > max_new_unvis {
                                        max_new_unvis = u_unvis;
                                        best_target = Some(target);
                                    }
                                }
                            }
                        }
                    }

                    let chosen_target = if max_new_unvis > 0 {
                        best_target
                    } else if !valid_targets.is_empty() {
                        Some(valid_targets[rng.gen_range(0, valid_targets.len())])
                    } else {
                        None
                    };

                    if let Some(target) = chosen_target {
                        let t_idx = stem_pos[target as usize];
                        stem[..t_idx].reverse();
                        for k in 0..t_idx {
                            stem_pos[stem[k] as usize] = k;
                        }
                    }
                }


                // 2-level lookahead if remaining unvisited vertices < 250 and 1-level failed
                if !rotated && (total_v - stem.len()) < 250 {
                    // Search for 2-step rotation from tail
                    for &t1 in tail_nbrs {
                        if in_stem[t1 as usize] {
                            let t1_idx = stem_pos[t1 as usize];
                            if t1_idx + 1 < stem.len() {
                                let v1 = stem[t1_idx + 1];
                                for &t2 in &adj_flat[v1 as usize] {
                                    if in_stem[t2 as usize] {
                                        let t2_idx = stem_pos[t2 as usize];
                                        if t2_idx + 1 < stem.len() {
                                            let v2 = stem[t2_idx + 1];
                                            let v2_unvis = adj_flat[v2 as usize].iter().filter(|&&w| !in_stem[w as usize]).count();
                                            if v2_unvis > 0 {
                                                // Execute 2-step rotation
                                                stem[t1_idx + 1..].reverse();
                                                for k in (t1_idx + 1)..stem.len() {
                                                    stem_pos[stem[k] as usize] = k;
                                                }
                                                let new_t2_idx = stem_pos[t2 as usize];
                                                if new_t2_idx + 1 < stem.len() {
                                                    stem[new_t2_idx + 1..].reverse();
                                                    for k in (new_t2_idx + 1)..stem.len() {
                                                        stem_pos[stem[k] as usize] = k;
                                                    }
                                                }
                                                rotated = true;
                                                break;
                                            }
                                        }
                                    }
                                }
                                if rotated {
                                    break;
                                }
                            }
                        }
                    }
                }



                if !rotated {
                    if Self::try_k_opt_splice(
                        &mut stem,
                        &mut in_stem,
                        &mut stem_pos,
                        &adj_flat,
                        &is_protected_edge,
                        total_v,
                    ) {
                        steps_since_extension = 0;
                        continue;
                    }
                }

                steps_since_extension += 1;
                if steps_since_extension % 50 == 0 {
                    if Self::try_k_opt_splice(
                        &mut stem,
                        &mut in_stem,
                        &mut stem_pos,
                        &adj_flat,
                        &is_protected_edge,
                        total_v,
                    ) {
                        steps_since_extension = 0;
                        continue;
                    }
                }

                if steps_since_extension > total_v * 5 {
                    if Self::try_k_opt_splice(
                        &mut stem,
                        &mut in_stem,
                        &mut stem_pos,
                        &adj_flat,
                        &is_protected_edge,
                        total_v,
                    ) {
                        steps_since_extension = 0;
                        continue;
                    }
                    // Stagnated in local trap, restart
                    break;
                }
            }
            if stem.len() > best_length {
                best_length = stem.len();
            }
        }

        println!("StemCyclePatcher finished: restarts = {}, best path length = {} / {}", restart, best_length, total_v);
        None
    }

    /// Tries adaptive k-opt splicing to absorb unvisited satellite vertices or 2-paths into the active stem.
    /// Returns true if a splice was made.
    pub fn try_k_opt_splice(
        stem: &mut Vec<i32>,
        in_stem: &mut [bool],
        stem_pos: &mut [usize],
        adj_flat: &[Vec<i32>],
        is_protected_edge: &HashSet<(i32, i32)>,
        total_v: usize,
    ) -> bool {
        // Only run when stem length >= 90% of total vertices and not yet full
        if (stem.len() as f64) < 0.90 * (total_v as f64) || stem.len() >= total_v {
            return false;
        }

        // Collect all unvisited vertices
        let mut unvisited: Vec<i32> = Vec::new();
        for v in 0..adj_flat.len() {
            if !in_stem[v] && !adj_flat[v].is_empty() {
                unvisited.push(v as i32);
            }
        }

        if unvisited.is_empty() {
            return false;
        }

        // 1. Search for 1-vertex splice:
        // An unvisited vertex w with neighbors u1, u2 in stem where u1 and u2 are adjacent in stem
        for &w in &unvisited {
            let w_idx = w as usize;
            let w_nbrs = &adj_flat[w_idx];
            for &u1 in w_nbrs {
                let u1_idx = u1 as usize;
                if in_stem[u1_idx] {
                    let p1 = stem_pos[u1_idx];
                    if p1 + 1 < stem.len() {
                        let u2 = stem[p1 + 1];
                        if !is_protected_edge.contains(&(u1, u2)) && !is_protected_edge.contains(&(u2, u1)) {
                            if w_nbrs.contains(&u2) {
                                // Splicing w between u1 (at p1) and u2 (at p1+1)
                                stem.insert(p1 + 1, w);
                                in_stem[w_idx] = true;
                                for idx in (p1 + 1)..stem.len() {
                                    stem_pos[stem[idx] as usize] = idx;
                                }
                                return true;
                            }
                        }
                    }
                }
            }
        }

        // 2. Search for 2-path splice:
        // Unvisited w1 - w2 where w1 connects to u1 in stem, w2 connects to u2 in stem,
        // and (u1, u2) is an adjacent unprotected edge in stem.
        for &w1 in &unvisited {
            let w1_idx = w1 as usize;
            let w1_nbrs = &adj_flat[w1_idx];
            for &w2 in w1_nbrs {
                let w2_idx = w2 as usize;
                if !in_stem[w2_idx] && w2 != w1 {
                    let w2_nbrs = &adj_flat[w2_idx];
                    for &u1 in w1_nbrs {
                        let u1_idx = u1 as usize;
                        if in_stem[u1_idx] {
                            let p1 = stem_pos[u1_idx];
                            // Check right neighbor in stem: u2 = stem[p1 + 1]
                            if p1 + 1 < stem.len() {
                                let u2 = stem[p1 + 1];
                                if !is_protected_edge.contains(&(u1, u2)) && !is_protected_edge.contains(&(u2, u1)) {
                                    if w2_nbrs.contains(&u2) {
                                        // Insert w1, w2 between u1 and u2 -> u1, w1, w2, u2
                                        stem.insert(p1 + 1, w1);
                                        stem.insert(p1 + 2, w2);
                                        in_stem[w1_idx] = true;
                                        in_stem[w2_idx] = true;
                                        for idx in (p1 + 1)..stem.len() {
                                            stem_pos[stem[idx] as usize] = idx;
                                        }
                                        return true;
                                    }
                                }
                            }
                            // Check left neighbor in stem: u0 = stem[p1 - 1]
                            if p1 > 0 {
                                let u0 = stem[p1 - 1];
                                if !is_protected_edge.contains(&(u0, u1)) && !is_protected_edge.contains(&(u1, u0)) {
                                    if w2_nbrs.contains(&u0) {
                                        // Insert w2, w1 between u0 and u1 -> u0, w2, w1, u1
                                        stem.insert(p1, w1);
                                        stem.insert(p1, w2);
                                        in_stem[w1_idx] = true;
                                        in_stem[w2_idx] = true;
                                        for idx in p1..stem.len() {
                                            stem_pos[stem[idx] as usize] = idx;
                                        }
                                        return true;
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
}

pub fn try_k_opt_splice(
    stem: &mut Vec<i32>,
    in_stem: &mut [bool],
    stem_pos: &mut [usize],
    adj_flat: &[Vec<i32>],
    is_protected_edge: &HashSet<(i32, i32)>,
    total_v: usize,
) -> bool {
    StemCyclePatcher::try_k_opt_splice(stem, in_stem, stem_pos, adj_flat, is_protected_edge, total_v)
}


/// Validates whether a given cycle is a valid Hamiltonian cycle on graph g.
fn is_valid_cycle(cycle: &[i32], g: &Graph) -> bool {
    if cycle.len() != g.adjacency_list.len() {
        return false;
    }
    let mut seen = HashSet::with_capacity(cycle.len());
    for &v in cycle {
        if !seen.insert(v) {
            return false;
        }
    }
    for i in 0..cycle.len() {
        let u = cycle[i];
        let v = cycle[(i + 1) % cycle.len()];
        if let Some(neighbors) = g.adjacency_list.get(&u) {
            if !neighbors.contains(&v) {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn build_test_graph(edges: &[(i32, i32)]) -> Graph {
        let mut g = Graph::new();
        for &(u, v) in edges {
            g.add_edge(u, v);
        }
        g
    }

    fn empty_contractor() -> Degree2Contractor {
        Degree2Contractor {
            chain_map: HashMap::new(),
            original_vertices_count: 0,
            contracted_vertices_count: 0,
            is_direct_cycle: None,
            is_infeasible: false,
        }
    }

    #[test]
    fn test_stem_cycle_patcher_synthetic_cycle() {
        let edges = vec![
            (1, 2), (2, 3), (3, 4), (4, 5), (5, 6), (6, 1),
            (1, 4), (2, 5), (3, 6)
        ];
        let g = build_test_graph(&edges);
        let contractor = empty_contractor();
        let hub_registry = HubRegistry::new(&g);

        let res = StemCyclePatcher::solve_via_stem_and_cycle(&g, &contractor, &hub_registry, 2.0);
        assert!(res.is_some());
        let tour = res.unwrap();
        assert_eq!(tour.len(), 6);
        assert!(is_valid_cycle(&tour, &g));
    }

    #[test]
    fn test_stem_cycle_patcher_degree2_safety() {
        let edges = vec![
            (1, 2), (2, 3), (3, 4), (4, 5), (5, 1),
            (2, 4), (1, 3)
        ];
        let g = build_test_graph(&edges);
        let mut contractor = empty_contractor();
        contractor.chain_map.insert((1, 2), vec![1, 100, 2]);

        let hub_registry = HubRegistry::new(&g);
        let res = StemCyclePatcher::solve_via_stem_and_cycle(&g, &contractor, &hub_registry, 2.0);
        assert!(res.is_some());
        let tour = res.unwrap();
        assert_eq!(tour.len(), 5);
        assert!(is_valid_cycle(&tour, &g));
    }

    #[test]
    fn test_k_opt_splice_1_vertex() {
        let total_v = 10;
        let max_v = 10;
        let mut adj_flat: Vec<Vec<i32>> = vec![Vec::new(); max_v + 1];
        // Cycle 1..9
        for i in 1..9 {
            adj_flat[i].push((i + 1) as i32);
            adj_flat[i + 1].push(i as i32);
        }
        // Vertex 10 connects to 4 and 5
        adj_flat[10].push(4);
        adj_flat[10].push(5);
        adj_flat[4].push(10);
        adj_flat[5].push(10);

        let mut stem: Vec<i32> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut in_stem = vec![false; max_v + 1];
        let mut stem_pos = vec![0; max_v + 1];
        for (i, &v) in stem.iter().enumerate() {
            in_stem[v as usize] = true;
            stem_pos[v as usize] = i;
        }

        let is_protected_edge: HashSet<(i32, i32)> = HashSet::new();

        let spliced = StemCyclePatcher::try_k_opt_splice(
            &mut stem,
            &mut in_stem,
            &mut stem_pos,
            &adj_flat,
            &is_protected_edge,
            total_v,
        );

        assert!(spliced);
        assert_eq!(stem.len(), 10);
        assert!(in_stem[10]);
        assert_eq!(stem, vec![1, 2, 3, 4, 10, 5, 6, 7, 8, 9]);
        for (i, &v) in stem.iter().enumerate() {
            assert_eq!(stem_pos[v as usize], i);
        }
    }

    #[test]
    fn test_k_opt_splice_2_path() {
        let total_v = 20;
        let max_v = 20;
        let mut adj_flat: Vec<Vec<i32>> = vec![Vec::new(); max_v + 1];
        for i in 1..18 {
            adj_flat[i].push((i + 1) as i32);
            adj_flat[i + 1].push(i as i32);
        }
        // 19 connects to 5 and 20; 20 connects to 19 and 6
        adj_flat[19].push(5);
        adj_flat[5].push(19);
        adj_flat[19].push(20);
        adj_flat[20].push(19);
        adj_flat[20].push(6);
        adj_flat[6].push(20);

        let mut stem: Vec<i32> = (1..=18).collect();
        let mut in_stem = vec![false; max_v + 1];
        let mut stem_pos = vec![0; max_v + 1];
        for (i, &v) in stem.iter().enumerate() {
            in_stem[v as usize] = true;
            stem_pos[v as usize] = i;
        }

        let is_protected_edge: HashSet<(i32, i32)> = HashSet::new();

        let spliced = StemCyclePatcher::try_k_opt_splice(
            &mut stem,
            &mut in_stem,
            &mut stem_pos,
            &adj_flat,
            &is_protected_edge,
            total_v,
        );

        assert!(spliced);
        assert_eq!(stem.len(), 20);
        assert!(in_stem[19]);
        assert!(in_stem[20]);
        assert_eq!(stem[4], 5);
        assert_eq!(stem[5], 19);
        assert_eq!(stem[6], 20);
        assert_eq!(stem[7], 6);
        for (i, &v) in stem.iter().enumerate() {
            assert_eq!(stem_pos[v as usize], i);
        }
    }

    #[test]
    fn test_k_opt_splice_protected_edge_guard() {
        let total_v = 10;
        let max_v = 10;
        let mut adj_flat: Vec<Vec<i32>> = vec![Vec::new(); max_v + 1];
        for i in 1..9 {
            adj_flat[i].push((i + 1) as i32);
            adj_flat[i + 1].push(i as i32);
        }
        adj_flat[10].push(4);
        adj_flat[10].push(5);
        adj_flat[4].push(10);
        adj_flat[5].push(10);

        let mut stem: Vec<i32> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut in_stem = vec![false; max_v + 1];
        let mut stem_pos = vec![0; max_v + 1];
        for (i, &v) in stem.iter().enumerate() {
            in_stem[v as usize] = true;
            stem_pos[v as usize] = i;
        }

        let mut is_protected_edge: HashSet<(i32, i32)> = HashSet::new();
        is_protected_edge.insert((4, 5));
        is_protected_edge.insert((5, 4));

        let spliced = StemCyclePatcher::try_k_opt_splice(
            &mut stem,
            &mut in_stem,
            &mut stem_pos,
            &adj_flat,
            &is_protected_edge,
            total_v,
        );

        assert!(!spliced);
        assert_eq!(stem.len(), 9);
        assert!(!in_stem[10]);
    }

    #[test]
    fn test_k_opt_splice_integration() {
        // Graph with 12 vertices: base cycle 1..10, satellite 11 on (3,4), satellite 12 on (7,8)
        let edges = vec![
            (1, 2), (2, 3), (3, 4), (4, 5), (5, 6), (6, 7), (7, 8), (8, 9), (9, 10), (10, 1),
            (3, 11), (4, 11),
            (7, 12), (8, 12),
            (1, 6), (2, 7), (5, 10)
        ];
        let g = build_test_graph(&edges);
        let contractor = empty_contractor();
        let hub_registry = HubRegistry::new(&g);

        let res = StemCyclePatcher::solve_via_stem_and_cycle(&g, &contractor, &hub_registry, 2.0);
        assert!(res.is_some(), "solve_via_stem_and_cycle should find a Hamiltonian cycle with k-opt splice");
        let tour = res.unwrap();
        assert_eq!(tour.len(), 12);
        assert!(is_valid_cycle(&tour, &g));
    }
}



