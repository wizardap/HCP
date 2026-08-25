use std::collections::{HashSet, HashMap};
use crate::graph::Graph;
use crate::encoder::Encoder;
use rustsat::types::Clause;

#[derive(Debug, Clone)]
pub struct GadgetResult {
    pub direct_spliced_tour: Option<Vec<i32>>,
    pub pruning_clauses: Vec<Clause>,
    pub cut_parity_clauses: Vec<Clause>,
}

pub struct GadgetInterfaceParityEngine;

impl GadgetInterfaceParityEngine {
    /// Analyzes an isolated subcycle gadget (<= 32 vertices).
    /// 1. Determines feasible internal Hamiltonian paths between interface ports.
    /// 2. Attempts direct 0ms RAM splice if entry/exit touchpoints on C_giant are adjacent.
    /// 3. Generates port-infeasibility exclusion clauses and boundary cut parity clauses.
    pub fn analyze_subcycle_gadget(
        gadget: &[i32],
        g: &Graph,
        giant_cycle: Option<&[i32]>,
        encoder: &Encoder,
    ) -> GadgetResult {
        let mut result = GadgetResult {
            direct_spliced_tour: None,
            pruning_clauses: Vec::new(),
            cut_parity_clauses: Vec::new(),
        };

        let k = gadget.len();
        if k < 3 || k > 32 {
            return result;
        }

        let gadget_set: HashSet<i32> = gadget.iter().copied().collect();

        // 1. Identify interface port vertices (vertices in gadget with neighbors outside gadget)
        let mut ports = Vec::new();
        let mut port_to_external_neighbors: HashMap<i32, Vec<i32>> = HashMap::new();

        for &u in gadget {
            if let Some(neighbors) = g.adjacency_list.get(&u) {
                let ext: Vec<i32> = neighbors.iter().copied().filter(|v| !gadget_set.contains(v)).collect();
                if !ext.is_empty() {
                    ports.push(u);
                    port_to_external_neighbors.insert(u, ext);
                }
            }
        }

        if ports.len() < 2 {
            return result;
        }

        // 2. Find all feasible internal Hamiltonian paths in G[gadget] from u_in to u_out
        let mut feasible_paths: Vec<(i32, i32, Vec<i32>)> = Vec::new();
        let mut feasible_port_pairs: HashSet<(i32, i32)> = HashSet::new();

        for i in 0..ports.len() {
            for j in (i + 1)..ports.len() {
                let u_in = ports[i];
                let u_out = ports[j];

                if let Some(path) = Self::find_internal_hamiltonian_path(u_in, u_out, gadget, g, &gadget_set) {
                    feasible_paths.push((u_in, u_out, path.clone()));
                    feasible_port_pairs.insert((u_in, u_out));
                    feasible_port_pairs.insert((u_out, u_in));
                }
            }
        }

        // 3. Attempt direct 0ms RAM splice with C_giant
        if let Some(giant) = giant_cycle {
            let n_giant = giant.len();
            if n_giant >= 3 {
                let mut giant_pos = HashMap::new();
                for (idx, &v) in giant.iter().enumerate() {
                    giant_pos.insert(v, idx);
                }

                for (u_in, u_out, path) in &feasible_paths {
                    if let (Some(ext_in), Some(ext_out)) = (port_to_external_neighbors.get(u_in), port_to_external_neighbors.get(u_out)) {
                        for &v_in in ext_in {
                            for &v_out in ext_out {
                                if let (Some(&pos_in), Some(&pos_out)) = (giant_pos.get(&v_in), giant_pos.get(&v_out)) {
                                    // Case A: v_in and v_out are immediately adjacent on C_giant (v_in -> v_out)
                                    if (pos_in + 1) % n_giant == pos_out {
                                        let mut tour = Vec::with_capacity(n_giant + k);
                                        for i in 0..n_giant {
                                            tour.push(giant[(pos_out + i) % n_giant]);
                                        }
                                        for &v in path.iter() {
                                            tour.push(v);
                                        }
                                        result.direct_spliced_tour = Some(tour);
                                        return result;
                                    } else if (pos_out + 1) % n_giant == pos_in {
                                        // Case B: v_out and v_in are immediately adjacent on C_giant (v_out -> v_in)
                                        let mut tour = Vec::with_capacity(n_giant + k);
                                        for i in 0..n_giant {
                                            tour.push(giant[(pos_in + i) % n_giant]);
                                        }
                                        for &v in path.iter().rev() {
                                            tour.push(v);
                                        }
                                        result.direct_spliced_tour = Some(tour);
                                        return result;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 4. Generate Infeasible Port Pruning Clauses:
        // For any port pair (p1, p2) with no feasible internal Hamiltonian path, forbid entering at p1 and exiting at p2, and vice-versa
        for i in 0..ports.len() {
            for j in (i + 1)..ports.len() {
                let p1 = ports[i];
                let p2 = ports[j];
                if !feasible_port_pairs.contains(&(p1, p2)) {
                    if let (Some(ext1), Some(ext2)) = (port_to_external_neighbors.get(&p1), port_to_external_neighbors.get(&p2)) {
                        for &v1 in ext1 {
                            for &v2 in ext2 {
                                if let (Some(&lit1), Some(&lit2)) = (encoder.graph_lit_map.get(&(v1, p1)), encoder.graph_lit_map.get(&(p2, v2))) {
                                    result.pruning_clauses.push(Clause::from_iter([!lit1, !lit2]));
                                }
                                if let (Some(&lit2_in), Some(&lit1_out)) = (encoder.graph_lit_map.get(&(v2, p2)), encoder.graph_lit_map.get(&(p1, v1))) {
                                    result.pruning_clauses.push(Clause::from_iter([!lit2_in, !lit1_out]));
                                }
                            }
                        }
                    }
                }
            }
        }

        // 5. Boundary Cut Parity: At least 2 edges crossing delta(gadget)
        let mut boundary_lits = Vec::new();
        for &u in &ports {
            if let Some(neighbors) = port_to_external_neighbors.get(&u) {
                for &v in neighbors {
                    if let Some(&lit) = encoder.graph_lit_map.get(&(u, v)) {
                        boundary_lits.push(lit);
                    }
                    if let Some(&lit) = encoder.graph_lit_map.get(&(v, u)) {
                        boundary_lits.push(lit);
                    }
                }
            }
        }

        if !boundary_lits.is_empty() {
            boundary_lits.sort_unstable();
            boundary_lits.dedup();
            result.cut_parity_clauses.push(Clause::from_iter(boundary_lits));
        }

        result
    }

    /// Exact Hamiltonian path search in G[gadget] between start and end.
    fn find_internal_hamiltonian_path(
        start: i32,
        end: i32,
        gadget: &[i32],
        g: &Graph,
        gadget_set: &HashSet<i32>,
    ) -> Option<Vec<i32>> {
        let k = gadget.len();
        let mut visited = HashSet::with_capacity(k);
        visited.insert(start);
        let mut path = Vec::with_capacity(k);
        path.push(start);

        if Self::dfs_hamiltonian_path(start, end, k, &mut visited, &mut path, g, gadget_set) {
            Some(path)
        } else {
            None
        }
    }

    fn dfs_hamiltonian_path(
        curr: i32,
        target: i32,
        total_k: usize,
        visited: &mut HashSet<i32>,
        path: &mut Vec<i32>,
        g: &Graph,
        gadget_set: &HashSet<i32>,
    ) -> bool {
        if path.len() == total_k {
            return curr == target;
        }

        if let Some(neighbors) = g.adjacency_list.get(&curr) {
            for &next in neighbors {
                if gadget_set.contains(&next) && !visited.contains(&next) {
                    // Pruning: do not visit target early before visiting all other gadget vertices
                    if next == target && path.len() + 1 < total_k {
                        continue;
                    }

                    visited.insert(next);
                    path.push(next);

                    if Self::dfs_hamiltonian_path(next, target, total_k, visited, path, g, gadget_set) {
                        return true;
                    }

                    path.pop();
                    visited.remove(&next);
                }
            }
        }

        false
    }
}
