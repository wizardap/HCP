use crate::contraction::Degree2Contractor;
use crate::graph::Graph;
use crate::hub_registry::HubRegistry;

pub struct HubPatcher;

impl HubPatcher {
    /// Splicing multiple satellite subcycles into a primary cycle visiting super-hubs.
    ///
    /// If all subcycles are successfully merged into one, returns `vec![main_cycle]`.
    /// Otherwise, returns `[main_cycle, remaining_cycles...]`.
    pub fn patch_cycles_via_hubs(
        cycles: &[Vec<i32>],
        g: &Graph,
        contractor: &Degree2Contractor,
        hub_registry: &HubRegistry,
    ) -> Vec<Vec<i32>> {
        if cycles.len() <= 1 || hub_registry.hub_vertices.is_empty() {
            return cycles.to_vec();
        }

        // Identify the longest cycle as main_cycle
        let (max_idx, _) = cycles
            .iter()
            .enumerate()
            .max_by_key(|(_, c)| c.len())
            .unwrap();

        let mut main_cycle = cycles[max_idx].clone();
        let mut remaining_cycles: Vec<Vec<i32>> = cycles
            .iter()
            .enumerate()
            .filter(|(idx, _)| *idx != max_idx)
            .map(|(_, c)| c.clone())
            .collect();

        // Greedily splice satellite cycles into main_cycle at visited hubs
        let mut progress = true;
        while progress && !remaining_cycles.is_empty() {
            progress = false;
            for &h in &hub_registry.hub_vertices {
                if !main_cycle.contains(&h) {
                    continue;
                }

                let mut i = 0;
                while i < remaining_cycles.len() {
                    if Self::try_splice_subcycle_at_hub(
                        &mut main_cycle,
                        &remaining_cycles[i],
                        h,
                        g,
                        contractor,
                    ) {
                        remaining_cycles.remove(i);
                        progress = true;
                    } else {
                        i += 1;
                    }
                }

                if remaining_cycles.is_empty() {
                    break;
                }
            }
        }

        if remaining_cycles.is_empty() {
            vec![main_cycle]
        } else {
            let mut result = Vec::with_capacity(1 + remaining_cycles.len());
            result.push(main_cycle);
            result.extend(remaining_cycles);
            result
        }
    }

    /// Attempts to splice a satellite subcycle into `main_cycle` at a specified `hub` vertex.
    ///
    /// Returns `true` if spliced successfully, modifying `main_cycle` in place.
    pub fn try_splice_subcycle_at_hub(
        main_cycle: &mut Vec<i32>,
        satellite_cycle: &[i32],
        hub: i32,
        g: &Graph,
        contractor: &Degree2Contractor,
    ) -> bool {
        let k = main_cycle.len();
        let r = satellite_cycle.len();
        if k < 3 || r < 2 {
            return false;
        }

        let p = match main_cycle.iter().position(|&v| v == hub) {
            Some(pos) => pos,
            None => return false,
        };

        let pred = main_cycle[(p + k - 1) % k];
        let succ = main_cycle[(p + 1) % k];

        for j in 0..r {
            let w_j = satellite_cycle[j];
            let w_j_plus_1 = satellite_cycle[(j + 1) % r];

            // Verify satellite edge (w_j, w_{j+1}) is safe to break
            if !is_safe_to_break(w_j, w_j_plus_1, contractor) {
                continue;
            }

            // Side 1: Between hub and succ
            if is_safe_to_break(hub, succ, contractor) {
                // Orientation A: (hub, w_j) in E(G) and (w_{j+1}, succ) in E(G)
                if has_edge(g, hub, w_j) && has_edge(g, w_j_plus_1, succ) {
                    let mut sat_seq = Vec::with_capacity(r);
                    for s in 0..r {
                        sat_seq.push(satellite_cycle[(j + r - s) % r]);
                    }
                    let candidate = build_spliced_cycle_side1(main_cycle, p, &sat_seq);
                    if is_valid_cycle(&candidate, g) {
                        *main_cycle = candidate;
                        return true;
                    }
                }

                // Orientation B: (hub, w_{j+1}) in E(G) and (w_j, succ) in E(G)
                if has_edge(g, hub, w_j_plus_1) && has_edge(g, w_j, succ) {
                    let mut sat_seq = Vec::with_capacity(r);
                    for s in 0..r {
                        sat_seq.push(satellite_cycle[(j + 1 + s) % r]);
                    }
                    let candidate = build_spliced_cycle_side1(main_cycle, p, &sat_seq);
                    if is_valid_cycle(&candidate, g) {
                        *main_cycle = candidate;
                        return true;
                    }
                }
            }

            // Side 2: Between pred and hub
            if is_safe_to_break(pred, hub, contractor) {
                // Orientation A: (pred, w_j) in E(G) and (w_{j+1}, hub) in E(G)
                if has_edge(g, pred, w_j) && has_edge(g, w_j_plus_1, hub) {
                    let mut sat_seq = Vec::with_capacity(r);
                    for s in 0..r {
                        sat_seq.push(satellite_cycle[(j + r - s) % r]);
                    }
                    let candidate = build_spliced_cycle_side2(main_cycle, p, &sat_seq);
                    if is_valid_cycle(&candidate, g) {
                        *main_cycle = candidate;
                        return true;
                    }
                }

                // Orientation B: (pred, w_{j+1}) in E(G) and (w_j, hub) in E(G)
                if has_edge(g, pred, w_j_plus_1) && has_edge(g, w_j, hub) {
                    let mut sat_seq = Vec::with_capacity(r);
                    for s in 0..r {
                        sat_seq.push(satellite_cycle[(j + 1 + s) % r]);
                    }
                    let candidate = build_spliced_cycle_side2(main_cycle, p, &sat_seq);
                    if is_valid_cycle(&candidate, g) {
                        *main_cycle = candidate;
                        return true;
                    }
                }
            }
        }

        false
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

fn build_spliced_cycle_side1(main_cycle: &[i32], p: usize, sat_seq: &[i32]) -> Vec<i32> {
    let mut new_cycle = Vec::with_capacity(main_cycle.len() + sat_seq.len());
    new_cycle.extend_from_slice(&main_cycle[0..=p]);
    new_cycle.extend_from_slice(sat_seq);
    if p + 1 < main_cycle.len() {
        new_cycle.extend_from_slice(&main_cycle[p + 1..]);
    }
    new_cycle
}

fn build_spliced_cycle_side2(main_cycle: &[i32], p: usize, sat_seq: &[i32]) -> Vec<i32> {
    let mut new_cycle = Vec::with_capacity(main_cycle.len() + sat_seq.len());
    if p > 0 {
        new_cycle.extend_from_slice(&main_cycle[0..p]);
        new_cycle.extend_from_slice(sat_seq);
        new_cycle.extend_from_slice(&main_cycle[p..]);
    } else {
        new_cycle.extend_from_slice(main_cycle);
        new_cycle.extend_from_slice(sat_seq);
    }
    new_cycle
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

    fn make_test_hub_registry(hubs: &[i32], g: &Graph) -> HubRegistry {
        let max_v = g.adjacency_list.keys().copied().max().unwrap_or(0) as usize;
        let mut is_hub = vec![false; max_v + 1];
        let mut hub_neighbors = HashMap::new();
        for &h in hubs {
            if (h as usize) < is_hub.len() {
                is_hub[h as usize] = true;
            }
            if let Some(adj) = g.adjacency_list.get(&h) {
                hub_neighbors.insert(h, adj.iter().cloned().collect());
            }
        }
        HubRegistry {
            is_hub,
            hub_vertices: hubs.to_vec(),
            hub_neighbors,
            min_hub_degree: 3,
        }
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
    fn test_hub_patcher_single_splice() {
        // Main cycle: 1 - 2 - 3 - 4 - 1 (hub is 1)
        // Satellite cycle: 5 - 6 - 7 - 5
        // Cross edges: (1, 5) and (6, 2)
        // Splice breaks (5, 6) and (1, 2), ordering satellite: 5 -> 7 -> 6
        // Result cycle: 1 -> 5 -> 7 -> 6 -> 2 -> 3 -> 4 -> 1
        let edges = vec![
            (1, 2), (2, 3), (3, 4), (4, 1),
            (5, 6), (6, 7), (7, 5),
            (1, 5), (6, 2),
        ];
        let g = build_test_graph(&edges);
        let contractor = empty_contractor();
        let hub_registry = make_test_hub_registry(&[1], &g);

        let cycles = vec![vec![1, 2, 3, 4], vec![5, 6, 7]];
        let patched = HubPatcher::patch_cycles_via_hubs(&cycles, &g, &contractor, &hub_registry);

        assert_eq!(patched.len(), 1, "Should merge into exactly 1 cycle");
        assert_eq!(patched[0].len(), 7, "Cycle should contain all 7 vertices");
        assert!(is_valid_cycle(&patched[0], &g), "Merged cycle must be valid in G");
    }

    #[test]
    fn test_hub_patcher_multi_satellite() {
        // Main cycle: 1 - 2 - 3 - 4 - 1 (hub is 1)
        // Satellite 1: 10 - 11 - 12 - 10, cross edges: (1, 10), (11, 2)
        // Satellite 2: 20 - 21 - 22 - 20, cross edges: (1, 20), (21, 10)
        // Satellite 3: 30 - 31 - 32 - 30, cross edges: (1, 30), (31, 20)
        let edges = vec![
            // Main cycle
            (1, 2), (2, 3), (3, 4), (4, 1),
            // Satellite 1
            (10, 11), (11, 12), (12, 10),
            (1, 10), (11, 2),
            // Satellite 2
            (20, 21), (21, 22), (22, 20),
            (1, 20), (21, 10),
            // Satellite 3
            (30, 31), (31, 32), (32, 30),
            (1, 30), (31, 20),
        ];
        let g = build_test_graph(&edges);
        let contractor = empty_contractor();
        let hub_registry = make_test_hub_registry(&[1], &g);

        let cycles = vec![
            vec![1, 2, 3, 4],
            vec![10, 11, 12],
            vec![20, 21, 22],
            vec![30, 31, 32],
        ];
        let patched = HubPatcher::patch_cycles_via_hubs(&cycles, &g, &contractor, &hub_registry);

        assert_eq!(patched.len(), 1, "Should merge all 4 cycles into 1");
        assert_eq!(patched[0].len(), 13, "Cycle should contain all 13 vertices");
        assert!(is_valid_cycle(&patched[0], &g), "Merged cycle must be valid in G");
    }

    #[test]
    fn test_hub_patcher_degree2_guard() {
        // Main cycle: 1 - 2 - 3 - 4 - 1 (hub is 1)
        // Satellite cycle: 5 - 6 - 7 - 5
        // Cross edges: (1, 5) and (6, 2)
        // Edge (5, 6) is in contractor.chain_map, so it cannot be severed!
        let edges = vec![
            (1, 2), (2, 3), (3, 4), (4, 1),
            (5, 6), (6, 7), (7, 5),
            (1, 5), (6, 2),
        ];
        let g = build_test_graph(&edges);
        let mut contractor = empty_contractor();
        contractor.chain_map.insert((5, 6), vec![5, 99, 6]);
        let hub_registry = make_test_hub_registry(&[1], &g);

        let cycles = vec![vec![1, 2, 3, 4], vec![5, 6, 7]];
        let patched = HubPatcher::patch_cycles_via_hubs(&cycles, &g, &contractor, &hub_registry);

        assert_eq!(patched.len(), 2, "Should NOT merge because contracted edge (5, 6) is guarded");
        assert_eq!(patched[0].len(), 4);
        assert_eq!(patched[1].len(), 3);
    }
}
