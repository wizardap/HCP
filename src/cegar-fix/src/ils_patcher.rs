use crate::chained_lk::ChainedLKSolver;
use crate::contraction::Degree2Contractor;
use crate::graph::Graph;
use crate::hub_registry::HubRegistry;
use crate::matching_patcher::MatchingPatcher;
use crate::patching::HubPatcher;

/// Fast pseudo-random number generator for deterministic ILS perturbations.
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

/// Iterated Local Search (ILS) Patcher using Double-Bridge 4-opt and non-improving 2-opt kicks.
pub struct IteratedLocalSearchPatcher;

impl IteratedLocalSearchPatcher {
    /// Attempts to escape local minima and merge subcycles into a single Hamiltonian cycle
    /// using Iterated Local Search (ILS) with randomized double-bridge perturbation kicks.
    pub fn solve_via_ils(
        cycles: &[Vec<i32>],
        g: &Graph,
        contractor: &Degree2Contractor,
        hub_registry: &HubRegistry,
        max_kicks: usize,
    ) -> Vec<Vec<i32>> {
        if cycles.len() <= 1 {
            return cycles.to_vec();
        }

        let mut best_cycles = cycles.to_vec();
        let total_nodes = g.adjacency_list.len();

        if best_cycles.len() == 1 && best_cycles[0].len() == total_nodes {
            return best_cycles;
        }

        // Initial local patch cascade
        let mut init_candidate = best_cycles.clone();
        init_candidate = HubPatcher::patch_cycles_via_hubs(&init_candidate, g, contractor, hub_registry);
        if init_candidate.len() > 1 {
            init_candidate = MatchingPatcher::patch_cycles_via_matching(&init_candidate, g, contractor, hub_registry);
        }
        if init_candidate.len() > 1 {
            init_candidate = ChainedLKSolver::patch_cycles_via_chained_lk(&init_candidate, g, contractor, hub_registry, 6);
        }

        if init_candidate.len() == 1 && init_candidate[0].len() == total_nodes {
            return init_candidate;
        }
        if init_candidate.len() < best_cycles.len() {
            best_cycles = init_candidate;
        }

        for kick in 0..max_kicks {
            if best_cycles.len() <= 1 {
                break;
            }

            // Find eligible cycles (length >= 4) for perturbation
            let mut eligible_indices: Vec<usize> = (0..best_cycles.len())
                .filter(|&idx| best_cycles[idx].len() >= 4)
                .collect();

            if eligible_indices.is_empty() {
                break;
            }

            // Sort eligible cycles by length descending
            eligible_indices.sort_by_key(|&idx| std::cmp::Reverse(best_cycles[idx].len()));

            let target_idx = eligible_indices[kick % eligible_indices.len()];
            let target_cycle = &best_cycles[target_idx];

            let seed = ((kick as u64) + 1).wrapping_mul(0x9e3779b97f4a7c15);
            if let Some(perturbed) = Self::perturb_cycle(target_cycle, g, contractor, seed) {
                let mut candidate_cycles = best_cycles.clone();
                candidate_cycles[target_idx] = perturbed;

                // Run local patch cascade on candidate_cycles
                candidate_cycles = HubPatcher::patch_cycles_via_hubs(&candidate_cycles, g, contractor, hub_registry);
                if candidate_cycles.len() > 1 {
                    candidate_cycles = MatchingPatcher::patch_cycles_via_matching(&candidate_cycles, g, contractor, hub_registry);
                }
                if candidate_cycles.len() > 1 {
                    candidate_cycles = ChainedLKSolver::patch_cycles_via_chained_lk(&candidate_cycles, g, contractor, hub_registry, 6);
                }

                if candidate_cycles.len() == 1 && candidate_cycles[0].len() == total_nodes {
                    return candidate_cycles;
                }

                if candidate_cycles.len() < best_cycles.len() {
                    best_cycles = candidate_cycles;
                }
            }
        }

        best_cycles
    }

    /// Perturbs a single cycle using either a randomized Double-Bridge 4-opt swap
    /// or a randomized non-improving 2-opt chord reconnection.
    ///
    /// Preserves degree-2 contracted edges in `contractor.chain_map` and validates
    /// that all new connecting edges exist in `g`.
    pub fn perturb_cycle(
        cycle: &[i32],
        g: &Graph,
        contractor: &Degree2Contractor,
        seed: u64,
    ) -> Option<Vec<i32>> {
        let n = cycle.len();
        if n < 4 {
            return None;
        }

        let mut rng = SimpleRng::new(seed);

        // 1. Randomized Double-Bridge 4-opt kicks
        let max_4opt_attempts = (n * 10).clamp(100, 1000);
        for _ in 0..max_4opt_attempts {
            let mut idxs = [
                rng.gen_range(0, n),
                rng.gen_range(0, n),
                rng.gen_range(0, n),
                rng.gen_range(0, n),
            ];
            idxs.sort_unstable();
            if idxs[0] == idxs[1] || idxs[1] == idxs[2] || idxs[2] == idxs[3] {
                continue;
            }

            let (i1, i2, i3, i4) = (idxs[0], idxs[1], idxs[2], idxs[3]);

            let u1 = cycle[i1];
            let v1 = cycle[i1 + 1];
            let u2 = cycle[i2];
            let v2 = cycle[i2 + 1];
            let u3 = cycle[i3];
            let v3 = cycle[i3 + 1];
            let u4 = cycle[i4];
            let v4 = cycle[(i4 + 1) % n];

            if !is_safe_to_break(u1, v1, contractor)
                || !is_safe_to_break(u2, v2, contractor)
                || !is_safe_to_break(u3, v3, contractor)
                || !is_safe_to_break(u4, v4, contractor)
            {
                continue;
            }

            if !has_edge(g, u1, v3)
                || !has_edge(g, u4, v2)
                || !has_edge(g, u3, v1)
                || !has_edge(g, u2, v4)
            {
                continue;
            }

            let mut candidate = Vec::with_capacity(n);
            candidate.extend_from_slice(&cycle[0..=i1]);
            candidate.extend_from_slice(&cycle[i3 + 1..=i4]);
            candidate.extend_from_slice(&cycle[i2 + 1..=i3]);
            candidate.extend_from_slice(&cycle[i1 + 1..=i2]);
            if i4 + 1 < n {
                candidate.extend_from_slice(&cycle[i4 + 1..]);
            }

            if is_valid_cycle(&candidate, g) {
                return Some(candidate);
            }
        }

        // Exhaustive scan for small cycles if randomized sampling missed
        if n <= 30 {
            for i1 in 0..n {
                for i2 in (i1 + 1)..n {
                    for i3 in (i2 + 1)..n {
                        for i4 in (i3 + 1)..n {
                            let u1 = cycle[i1];
                            let v1 = cycle[i1 + 1];
                            let u2 = cycle[i2];
                            let v2 = cycle[i2 + 1];
                            let u3 = cycle[i3];
                            let v3 = cycle[i3 + 1];
                            let u4 = cycle[i4];
                            let v4 = cycle[(i4 + 1) % n];

                            if !is_safe_to_break(u1, v1, contractor)
                                || !is_safe_to_break(u2, v2, contractor)
                                || !is_safe_to_break(u3, v3, contractor)
                                || !is_safe_to_break(u4, v4, contractor)
                            {
                                continue;
                            }

                            if !has_edge(g, u1, v3)
                                || !has_edge(g, u4, v2)
                                || !has_edge(g, u3, v1)
                                || !has_edge(g, u2, v4)
                            {
                                continue;
                            }

                            let mut candidate = Vec::with_capacity(n);
                            candidate.extend_from_slice(&cycle[0..=i1]);
                            candidate.extend_from_slice(&cycle[i3 + 1..=i4]);
                            candidate.extend_from_slice(&cycle[i2 + 1..=i3]);
                            candidate.extend_from_slice(&cycle[i1 + 1..=i2]);
                            if i4 + 1 < n {
                                candidate.extend_from_slice(&cycle[i4 + 1..]);
                            }

                            if is_valid_cycle(&candidate, g) {
                                return Some(candidate);
                            }
                        }
                    }
                }
            }
        }

        // 2. Randomized valid non-improving 2-opt chord swap inside cycle
        let max_2opt_attempts = (n * 10).clamp(50, 500);
        for _ in 0..max_2opt_attempts {
            let i = rng.gen_range(0, n);
            let j = rng.gen_range(0, n);
            let (i, j) = if i < j { (i, j) } else { (j, i) };
            if j <= i + 1 || (i == 0 && j == n - 1) {
                continue;
            }

            let u_i = cycle[i];
            let u_ip1 = cycle[i + 1];
            let u_j = cycle[j];
            let u_jp1 = cycle[(j + 1) % n];

            if !is_safe_to_break(u_i, u_ip1, contractor) || !is_safe_to_break(u_j, u_jp1, contractor) {
                continue;
            }

            if !has_edge(g, u_i, u_j) || !has_edge(g, u_ip1, u_jp1) {
                continue;
            }

            let mut candidate = Vec::with_capacity(n);
            candidate.extend_from_slice(&cycle[0..=i]);
            for k in (i + 1..=j).rev() {
                candidate.push(cycle[k]);
            }
            if j + 1 < n {
                candidate.extend_from_slice(&cycle[j + 1..]);
            }

            if is_valid_cycle(&candidate, g) {
                return Some(candidate);
            }
        }

        // Deterministic 2-opt chord swap scan for small/medium cycles
        if n <= 50 {
            for i in 0..n {
                for j in (i + 2)..n {
                    if i == 0 && j == n - 1 {
                        continue;
                    }
                    let u_i = cycle[i];
                    let u_ip1 = cycle[i + 1];
                    let u_j = cycle[j];
                    let u_jp1 = cycle[(j + 1) % n];

                    if !is_safe_to_break(u_i, u_ip1, contractor) || !is_safe_to_break(u_j, u_jp1, contractor) {
                        continue;
                    }

                    if !has_edge(g, u_i, u_j) || !has_edge(g, u_ip1, u_jp1) {
                        continue;
                    }

                    let mut candidate = Vec::with_capacity(n);
                    candidate.extend_from_slice(&cycle[0..=i]);
                    for k in (i + 1..=j).rev() {
                        candidate.push(cycle[k]);
                    }
                    if j + 1 < n {
                        candidate.extend_from_slice(&cycle[j + 1..]);
                    }

                    if is_valid_cycle(&candidate, g) {
                        return Some(candidate);
                    }
                }
            }
        }

        None
    }
}

#[inline]
fn is_safe_to_break(u: i32, v: i32, contractor: &Degree2Contractor) -> bool {
    !contractor.chain_map.contains_key(&(u, v)) && !contractor.chain_map.contains_key(&(v, u))
}

#[inline]
fn has_edge(g: &Graph, u: i32, v: i32) -> bool {
    g.adjacency_list.get(&u).map_or(false, |adj| adj.contains(&v))
}

fn is_valid_cycle(cycle: &[i32], g: &Graph) -> bool {
    let len = cycle.len();
    if len < 3 {
        return false;
    }
    for i in 0..len {
        let u = cycle[i];
        let v = cycle[(i + 1) % len];
        if !has_edge(g, u, v) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contraction::Degree2Contractor;
    use crate::graph::Graph;
    use crate::hub_registry::HubRegistry;
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

    fn empty_hub_registry() -> HubRegistry {
        HubRegistry {
            is_hub: Vec::new(),
            hub_vertices: Vec::new(),
            hub_neighbors: HashMap::new(),
            min_hub_degree: 3,
        }
    }

    #[test]
    fn test_ils_double_bridge_validity() {
        // Cycle: 1 - 2 - 3 - 4 - 5 - 6 - 7 - 8 - 1
        // Break points: i1=1 (edge 2-3), i2=3 (edge 4-5), i3=5 (edge 6-7), i4=7 (edge 8-1)
        // Segments: A=[1, 2], B=[3, 4], C=[5, 6], D=[7, 8]
        // Double-bridge order: A -> D -> C -> B = [1, 2, 7, 8, 5, 6, 3, 4]
        // Connecting edges needed: (2, 7), (8, 5), (6, 3), (4, 1)
        let edges = vec![
            (1, 2), (2, 3), (3, 4), (4, 5), (5, 6), (6, 7), (7, 8), (8, 1),
            (2, 7), (8, 5), (6, 3), (4, 1),
        ];
        let g = build_test_graph(&edges);
        let contractor = empty_contractor();

        let cycle = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let perturbed_opt = IteratedLocalSearchPatcher::perturb_cycle(&cycle, &g, &contractor, 42);

        assert!(perturbed_opt.is_some(), "Double-bridge perturbation must succeed");
        let perturbed = perturbed_opt.unwrap();
        assert_eq!(perturbed.len(), 8, "Perturbed cycle must have identical length");

        let mut sorted = perturbed.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, vec![1, 2, 3, 4, 5, 6, 7, 8], "Must have identical vertex multiset");

        assert!(is_valid_cycle(&perturbed, &g), "Perturbed cycle must be valid in G");
        assert_ne!(perturbed, cycle, "Perturbed cycle must differ from original cycle");
    }

    #[test]
    fn test_ils_escapes_local_minimum() {
        // Synthetic 4-subcycle graph where standard 2-opt/matching/chained-lk is stuck,
        // but an ILS double-bridge kick on Cycle 1 exposes the edges needed to merge all 4 cycles into 1.
        // Cycle 1: 1..8
        // Cycle 2: 9..12
        // Cycle 3: 13..16
        // Cycle 4: 17..20
        let edges = vec![
            // Cycle 1 edges
            (1, 2), (2, 3), (3, 4), (4, 5), (5, 6), (6, 7), (7, 8), (8, 1),
            // Cycle 2 edges
            (9, 10), (10, 11), (11, 12), (12, 9),
            // Cycle 3 edges
            (13, 14), (14, 15), (15, 16), (16, 13),
            // Cycle 4 edges
            (17, 18), (18, 19), (19, 20), (20, 17),
            // Double-bridge chords for Cycle 1: creates perturbed [1, 2, 7, 8, 5, 6, 3, 4]
            (2, 7), (8, 5), (6, 3), (4, 1),
            // Cross edges from perturbed Cycle 1 edges to satellite cycles:
            // Edge (8, 5) in perturbed C1 connects to (9, 10) in C2
            (8, 9), (5, 10),
            // Edge (6, 3) in perturbed C1 connects to (13, 14) in C3
            (6, 13), (3, 14),
            // Edge (4, 1) in perturbed C1 connects to (17, 18) in C4
            (4, 17), (1, 18),
        ];

        let g = build_test_graph(&edges);
        let contractor = empty_contractor();
        let hub_registry = empty_hub_registry();

        let initial_cycles = vec![
            vec![1, 2, 3, 4, 5, 6, 7, 8],
            vec![9, 10, 11, 12],
            vec![13, 14, 15, 16],
            vec![17, 18, 19, 20],
        ];

        // Verify initial configuration cannot merge via matching/hub/chained LK directly
        let matching_only = MatchingPatcher::patch_cycles_via_matching(&initial_cycles, &g, &contractor, &hub_registry);
        assert_eq!(matching_only.len(), 4, "Initial cycles cannot be merged by matching without perturbation");

        // ILS kicks Cycle 1, triggering the full merge cascade down to 1 cycle
        let result = IteratedLocalSearchPatcher::solve_via_ils(&initial_cycles, &g, &contractor, &hub_registry, 100);

        assert_eq!(result.len(), 1, "ILS must merge all 4 cycles into 1 Hamiltonian cycle");
        assert_eq!(result[0].len(), 20, "Final cycle must visit all 20 vertices");
        assert!(is_valid_cycle(&result[0], &g), "Final cycle must be a valid simple tour in G");
    }

    #[test]
    fn test_ils_degree2_safety() {
        let edges = vec![
            (1, 2), (2, 3), (3, 4), (4, 5), (5, 6), (6, 7), (7, 8), (8, 1),
            (2, 7), (8, 5), (6, 3), (4, 1),
        ];
        let g = build_test_graph(&edges);
        let cycle = vec![1, 2, 3, 4, 5, 6, 7, 8];

        // 1. Guard edge (2, 3): any valid perturbation must strictly preserve edge (2, 3)
        let mut contractor = empty_contractor();
        contractor.chain_map.insert((2, 3), vec![2, 99, 3]);
        contractor.chain_map.insert((3, 2), vec![2, 99, 3]);

        let perturbed_opt = IteratedLocalSearchPatcher::perturb_cycle(&cycle, &g, &contractor, 42);
        if let Some(perturbed) = perturbed_opt {
            let n = perturbed.len();
            let mut preserves_2_3 = false;
            for i in 0..n {
                let u = perturbed[i];
                let v = perturbed[(i + 1) % n];
                if (u == 2 && v == 3) || (u == 3 && v == 2) {
                    preserves_2_3 = true;
                    break;
                }
            }
            assert!(preserves_2_3, "Perturbation must preserve guarded degree-2 edge (2, 3)");
        }

        // 2. Guard all breakable edges of the cycle: perturbation must return None
        for i in 0..cycle.len() {
            let u = cycle[i];
            let v = cycle[(i + 1) % cycle.len()];
            contractor.chain_map.insert((u, v), vec![u, 100 + i as i32, v]);
            contractor.chain_map.insert((v, u), vec![u, 100 + i as i32, v]);
        }

        let blocked_opt = IteratedLocalSearchPatcher::perturb_cycle(&cycle, &g, &contractor, 42);
        assert!(blocked_opt.is_none(), "Must not break any guarded degree-2 edges when all are locked");

        // 3. Lifting all guards allows perturbation to succeed freely
        let clean_contractor = empty_contractor();
        let perturbed_clean = IteratedLocalSearchPatcher::perturb_cycle(&cycle, &g, &clean_contractor, 42);
        assert!(perturbed_clean.is_some(), "Perturbation succeeds when degree-2 edges are safe");
    }
}
