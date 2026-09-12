use std::collections::{HashMap, HashSet};
use crate::graph::Graph;
use crate::contraction::Degree2Contractor;
use crate::alternating_port_engine::{AlternatingPortEngine, min_max, Port};
use crate::encoder::Encoder;
use rustsat::instances::Cnf;
use rustsat::solvers::{Solve, SolveIncremental, SolverResult};
use rustsat::types::{Clause, Lit, TernaryVal};
use rustsat_cadical::CaDiCaL;

#[derive(Debug, Clone)]
pub struct PortSubpathCorridor {
    pub sat_blocks: HashSet<usize>,
    pub giant_subpath_blocks: Vec<usize>,
    pub entry_port: Port,
    pub exit_port: Port,
    pub unfrozen_blocks: HashSet<usize>,
}

#[derive(Debug, Clone)]
pub struct AnchorCandidate {
    pub start_pos: usize,
    pub end_pos: usize,
    pub span: usize,
    pub sat_dock1: Port,
    pub sat_dock2: Port,
}

pub struct PortCorridorLns;

impl PortCorridorLns {
    pub fn map_giant_coordinates(giant: &[Port], n_blocks: usize) -> Vec<usize> {
        let mut pos = vec![usize::MAX; n_blocks * 2];
        for (i, &p) in giant.iter().enumerate() {
            pos[p.idx()] = i;
        }
        pos
    }

    pub fn find_docking_ports(
        sat: &[Port],
        giant_blocks: &HashSet<usize>,
        g: &Graph,
        node_to_port: &HashMap<i32, Port>,
        port_to_node: &HashMap<Port, i32>,
    ) -> Vec<Port> {
        let mut docking = HashSet::new();
        for &p in sat {
            let u = port_to_node[&p];
            if let Some(nbrs) = g.adjacency_list.get(&u) {
                for &v in nbrs {
                    if let Some(&p_v) = node_to_port.get(&v) {
                        if giant_blocks.contains(&p_v.block) {
                            docking.insert(p_v);
                        }
                    }
                }
            }
        }
        docking.into_iter().collect()
    }

    pub fn build_subpath_corridor(
        sat: &[Port],
        giant: &[Port],
        giant_pos: &[usize],
        g: &Graph,
        node_to_port: &HashMap<i32, Port>,
        port_to_node: &HashMap<Port, i32>,
        max_span: usize,
        buffer: usize,
    ) -> Option<PortSubpathCorridor> {
        let giant_blocks: HashSet<usize> = giant.iter().map(|p| p.block).collect();
        let docking = Self::find_docking_ports(sat, &giant_blocks, g, node_to_port, port_to_node);
        if docking.is_empty() {
            return None;
        }

        let n_giant_ports = giant.len();
        let mut dock_positions: Vec<usize> = docking.iter().map(|p| giant_pos[p.idx()]).collect();
        dock_positions.sort();

        // Find the shortest circular interval covering all docking positions
        let mut best_start = dock_positions[0];
        let mut best_len = n_giant_ports;

        for i in 0..dock_positions.len() {
            let start = dock_positions[i];
            let end = dock_positions[(i + dock_positions.len() - 1) % dock_positions.len()];
            let len = if end >= start {
                end - start + 1
            } else {
                (n_giant_ports - start) + end + 1
            };
            if len < best_len {
                best_len = len;
                best_start = start;
            }
        }

        // Align boundaries to complete blocks to preserve bipartite parity:
        // In AlternatingPortEngine cycles, external edges connect index 2k to 2k+1,
        // and internal degree-2 chains connect 2k+1 to (2k+2) % 2L.
        // Therefore, entry_idx MUST be odd (2k+1) and exit_idx MUST be even (2m),
        // cutting strictly external edges and keeping all blocks complete.
        let aligned_start = if best_start % 2 == 0 {
            (best_start + n_giant_ports - 1) % n_giant_ports
        } else {
            best_start
        };

        let raw_end = (best_start + best_len - 1) % n_giant_ports;
        let aligned_end = if raw_end % 2 != 0 {
            (raw_end + 1) % n_giant_ports
        } else {
            raw_end
        };

        let mut aligned_len = if aligned_end >= aligned_start {
            aligned_end - aligned_start + 1
        } else {
            (n_giant_ports - aligned_start) + aligned_end + 1
        };
        if aligned_len > n_giant_ports {
            aligned_len = n_giant_ports;
        }

        // Ensure at least one block (2 ports) remains outside the corridor if giant is large enough
        let max_subpath_ports = if n_giant_ports > 2 {
            n_giant_ports - 2
        } else {
            n_giant_ports
        };

        let span_len = std::cmp::min(aligned_len, std::cmp::min(max_span * 2, max_subpath_ports));

        // Add buffer blocks with underflow protection and capacity constraint
        let max_buffer_ports = (max_subpath_ports.saturating_sub(span_len)) / 4 * 2;
        let buf_ports = std::cmp::min(buffer * 2, max_buffer_ports);

        let entry_idx = (aligned_start + n_giant_ports - (buf_ports % n_giant_ports)) % n_giant_ports;
        let total_subpath_ports = std::cmp::min(span_len + buf_ports * 2, max_subpath_ports);
        let exit_idx = (entry_idx + total_subpath_ports - 1) % n_giant_ports;

        let entry_port = giant[entry_idx];
        let exit_port = giant[exit_idx];

        let sat_blocks: HashSet<usize> = sat.iter().map(|p| p.block).collect();
        let mut giant_subpath_blocks = Vec::new();
        let mut unfrozen_blocks = sat_blocks.clone();

        for step in 0..total_subpath_ports {
            let idx = (entry_idx + step) % n_giant_ports;
            let b = giant[idx].block;
            if unfrozen_blocks.insert(b) {
                giant_subpath_blocks.push(b);
            }
        }

        Some(PortSubpathCorridor {
            sat_blocks,
            giant_subpath_blocks,
            entry_port,
            exit_port,
            unfrozen_blocks,
        })
    }

    pub fn find_anchor_candidates(
        sat: &[Port],
        giant: &[Port],
        giant_pos: &[usize],
        g: &Graph,
        node_to_port: &HashMap<i32, Port>,
        port_to_node: &HashMap<Port, i32>,
        max_span: usize,
    ) -> Vec<AnchorCandidate> {
        let giant_blocks: HashSet<usize> = giant.iter().map(|p| p.block).collect();
        let n_giant = giant.len();

        // Collect all (giant_port, sat_port) docking pairs
        let mut dock_pairs: Vec<(Port, Port)> = Vec::new();
        for &p_sat in sat {
            let u = port_to_node[&p_sat];
            if let Some(nbrs) = g.adjacency_list.get(&u) {
                for &v in nbrs {
                    if let Some(&p_giant) = node_to_port.get(&v) {
                        if giant_blocks.contains(&p_giant.block) {
                            dock_pairs.push((p_giant, p_sat));
                        }
                    }
                }
            }
        }

        if dock_pairs.len() < 2 {
            return Vec::new();
        }

        let mut candidates = Vec::new();
        for i in 0..dock_pairs.len() {
            let (dg1, ds1) = dock_pairs[i];
            let pos1 = giant_pos[dg1.idx()];
            for j in (i + 1)..dock_pairs.len() {
                let (dg2, ds2) = dock_pairs[j];
                let pos2 = giant_pos[dg2.idx()];
                if ds1.block == ds2.block && ds1.end == ds2.end {
                    continue; // Must connect distinct satellite ports or blocks
                }

                let d_fwd = (pos2 + n_giant - pos1) % n_giant;
                let d_bwd = (pos1 + n_giant - pos2) % n_giant;
                let (start_pos, end_pos, span, p1, p2) = if d_fwd <= d_bwd {
                    (pos1, pos2, d_fwd, ds1, ds2)
                } else {
                    (pos2, pos1, d_bwd, ds2, ds1)
                };

                if span <= max_span * 2 {
                    candidates.push(AnchorCandidate {
                        start_pos,
                        end_pos,
                        span,
                        sat_dock1: p1,
                        sat_dock2: p2,
                    });
                }
            }
        }

        candidates.sort_by_key(|c| c.span);
        // Deduplicate overlapping positions
        candidates.dedup_by(|a, b| a.start_pos == b.start_pos && a.end_pos == b.end_pos);
        candidates
    }

    pub fn build_corridor_from_anchor(
        anchor: &AnchorCandidate,
        sat: &[Port],
        giant: &[Port],
        buffer: usize,
    ) -> Option<PortSubpathCorridor> {
        let n_giant = giant.len();
        let max_subpath_ports = if n_giant > 2 { n_giant - 2 } else { n_giant };

        let buf_ports = buffer * 2;
        let raw_start = (anchor.start_pos + n_giant - (buf_ports % n_giant)) % n_giant;
        let entry_idx = if raw_start % 2 == 0 {
            (raw_start + n_giant - 1) % n_giant
        } else {
            raw_start
        };

        let raw_end = (anchor.end_pos + buf_ports) % n_giant;
        let exit_idx = if raw_end % 2 != 0 {
            (raw_end + 1) % n_giant
        } else {
            raw_end
        };

        let mut total_subpath_ports = if exit_idx >= entry_idx {
            exit_idx - entry_idx + 1
        } else {
            (n_giant - entry_idx) + exit_idx + 1
        };

        if total_subpath_ports > max_subpath_ports {
            return None;
        }

        // Parity invariant: total_subpath_ports must be even
        if total_subpath_ports % 2 != 0 {
            total_subpath_ports += 1;
        }

        let entry_port = giant[entry_idx];
        let exit_port = giant[(entry_idx + total_subpath_ports - 1) % n_giant];

        let sat_blocks: HashSet<usize> = sat.iter().map(|p| p.block).collect();
        let mut giant_subpath_blocks = Vec::new();
        let mut unfrozen_blocks = sat_blocks.clone();

        for step in 0..total_subpath_ports {
            let idx = (entry_idx + step) % n_giant;
            let b = giant[idx].block;
            if unfrozen_blocks.insert(b) {
                giant_subpath_blocks.push(b);
            }
        }

        Some(PortSubpathCorridor {
            sat_blocks,
            giant_subpath_blocks,
            entry_port,
            exit_port,
            unfrozen_blocks,
        })
    }

    pub fn try_absorb_single_cycle(
        sat: &[Port],
        giant: &[Port],
        giant_pos: &[usize],
        g: &Graph,
        contractor: &Degree2Contractor,
        node_to_port: &HashMap<i32, Port>,
        port_to_node: &HashMap<Port, i32>,
        encoder: &Encoder,
        base_cnf: &Cnf,
    ) -> Option<Vec<Port>> {
        Self::try_absorb_single_cycle_with_others(
            sat,
            giant,
            giant_pos,
            g,
            contractor,
            node_to_port,
            port_to_node,
            encoder,
            base_cnf,
            &[],
        )
    }

    pub fn try_absorb_single_cycle_with_others(
        sat: &[Port],
        giant: &[Port],
        giant_pos: &[usize],
        g: &Graph,
        contractor: &Degree2Contractor,
        node_to_port: &HashMap<i32, Port>,
        port_to_node: &HashMap<Port, i32>,
        encoder: &Encoder,
        base_cnf: &Cnf,
        other_cycles: &[Vec<Port>],
    ) -> Option<Vec<Port>> {
        let corridor = match Self::build_subpath_corridor(sat, giant, giant_pos, g, node_to_port, port_to_node, 20, 2) {
            Some(c) => c,
            None => return None,
        };

        let n_giant = giant.len();
        let entry_idx = giant_pos[corridor.entry_port.idx()];
        let exit_idx = giant_pos[corridor.exit_port.idx()];

        let entry_prev_idx = (entry_idx + n_giant - 1) % n_giant;
        let exit_next_idx = (exit_idx + 1) % n_giant;

        let p_entry_prev = giant[entry_prev_idx];
        let p_exit_next = giant[exit_next_idx];

        assert!(!corridor.unfrozen_blocks.contains(&p_entry_prev.block), "p_entry_prev must be outside Omega");
        assert!(!corridor.unfrozen_blocks.contains(&p_exit_next.block), "p_exit_next must be outside Omega");

        // Track active successor and predecessor for outside vertices
        let mut active_succ: HashMap<i32, i32> = HashMap::new();
        let mut active_pred: HashMap<i32, i32> = HashMap::new();

        // Giant cycle outside Omega and boundary cut edges
        for i in 0..n_giant {
            let p1 = giant[i];
            let p2 = giant[(i + 1) % n_giant];
            let u1 = port_to_node[&p1];
            let u2 = port_to_node[&p2];

            if i == entry_prev_idx {
                // Entry boundary edge: u_entry_prev -> u_entry
                active_succ.insert(u1, u2);
                active_pred.insert(u2, u1);
            } else if i == exit_idx {
                // Exit boundary edge: u_exit -> u_exit_next
                active_succ.insert(u1, u2);
                active_pred.insert(u2, u1);
            } else if !corridor.unfrozen_blocks.contains(&p1.block) && !corridor.unfrozen_blocks.contains(&p2.block) {
                // Giant external or internal edge outside Omega
                active_succ.insert(u1, u2);
                active_pred.insert(u2, u1);
            }
        }

        // Other satellite cycles outside Omega
        for o_cyc in other_cycles {
            let o_len = o_cyc.len();
            for i in 0..o_len {
                let p1 = o_cyc[i];
                let p2 = o_cyc[(i + 1) % o_len];
                if !corridor.unfrozen_blocks.contains(&p1.block) && !corridor.unfrozen_blocks.contains(&p2.block) {
                    let u1 = port_to_node[&p1];
                    let u2 = port_to_node[&p2];
                    active_succ.insert(u1, u2);
                    active_pred.insert(u2, u1);
                }
            }
        }

        // Build SAT assumptions
        let mut assumptions: Vec<Lit> = Vec::new();
        let mut assumed_lits: HashSet<Lit> = HashSet::new();

        let mut add_assumption = |lit: Lit| {
            if !assumed_lits.contains(&!lit) {
                if assumed_lits.insert(lit) {
                    assumptions.push(lit);
                }
            }
        };

        let n_blocks = node_to_port.len() / 2;

        // Freeze all outside vertices
        for b in 0..n_blocks {
            if !corridor.unfrozen_blocks.contains(&b) {
                for end in 0..2 {
                    let p = Port { block: b, end };
                    let u = port_to_node[&p];

                    // Outgoing edges from u
                    let succ_v = active_succ.get(&u).copied();
                    if let Some(nbrs) = g.adjacency_list.get(&u) {
                        for &w in nbrs {
                            if let Some(&lit) = encoder.graph_lit_map.get(&(u, w)) {
                                if Some(w) == succ_v {
                                    add_assumption(lit);
                                } else {
                                    add_assumption(!lit);
                                }
                            }
                        }
                    }

                    // Incoming edges to u
                    let pred_v = active_pred.get(&u).copied();
                    if let Some(nbrs) = g.adjacency_list.get(&u) {
                        for &z in nbrs {
                            if let Some(&lit) = encoder.graph_lit_map.get(&(z, u)) {
                                if Some(z) == pred_v {
                                    add_assumption(lit);
                                } else {
                                    add_assumption(!lit);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Initialize fresh CaDiCaL instance with base_cnf
        let mut local_solver = CaDiCaL::default();
        for cl in base_cnf.iter() {
            let _ = local_solver.add_clause(cl.clone());
        }

        // Inject mandatory degree-2 virtual edge constraints for all blocks
        for (&(u, w), _) in &contractor.chain_map {
            if u < w {
                if let (Some(&l_uw), Some(&l_wu)) = (
                    encoder.graph_lit_map.get(&(u, w)),
                    encoder.graph_lit_map.get(&(w, u)),
                ) {
                    let _ = local_solver.add_clause(Clause::from_iter(vec![l_uw, l_wu]));
                }
            }
        }

        // Local CEGAR loop (up to 8 iterations)
        let expected_len = giant.len() + sat.len();

        for _cegar_iter in 0..8 {
            match local_solver.solve_assumps(&assumptions) {
                Ok(SolverResult::Sat) => {
                    let sol = match local_solver.full_solution() {
                        Ok(s) => s,
                        Err(_) => break,
                    };

                    let mut active_external_edges = HashSet::new();
                    for (&(u, v), &lit) in &encoder.graph_lit_map {
                        if sol.lit_value(lit) == TernaryVal::True {
                            if let (Some(&p1), Some(&p2)) = (node_to_port.get(&u), node_to_port.get(&v)) {
                                if p1.block != p2.block {
                                    active_external_edges.insert(min_max(u, v));
                                }
                            }
                        }
                    }

                    let (cycs, _) = AlternatingPortEngine::get_cycles(
                        &active_external_edges,
                        node_to_port,
                        port_to_node,
                        n_blocks,
                    );

                    if let Some(merged) = cycs.iter().find(|c| c.len() == expected_len) {
                        return Some(merged.clone());
                    }

                    // Add subcycle cuts for any cycle strictly inside Omega
                    let mut cut_added = false;
                    for cyc in &cycs {
                        if cyc.len() < expected_len && cyc.iter().all(|p| corridor.unfrozen_blocks.contains(&p.block)) {
                            let cyc_nodes: HashSet<i32> = cyc.iter().map(|p| port_to_node[p]).collect();
                            let mut cut_lits = Vec::new();
                            for &u in &cyc_nodes {
                                if let Some(nbrs) = g.adjacency_list.get(&u) {
                                    for &v in nbrs {
                                        if !cyc_nodes.contains(&v) {
                                            if let Some(&lit) = encoder.graph_lit_map.get(&(u, v)) {
                                                cut_lits.push(lit);
                                            }
                                        }
                                    }
                                }
                            }
                            if !cut_lits.is_empty() {
                                let _ = local_solver.add_clause(Clause::from_iter(cut_lits));
                                cut_added = true;
                            }
                        }
                    }

                    if !cut_added {
                        break;
                    }
                }
                _ => break,
            }
        }

        None
    }

    pub fn repair(
        cycles: &[Vec<i32>],
        g: &Graph,
        contractor: &Degree2Contractor,
        encoder: &Encoder,
        base_cnf: &Cnf,
    ) -> Vec<Vec<i32>> {
        if cycles.len() <= 1 || contractor.chain_map.is_empty() {
            return cycles.to_vec();
        }

        let (n_blocks, node_to_port, port_to_node) = AlternatingPortEngine::setup_ports(contractor);
        if n_blocks == 0 {
            return cycles.to_vec();
        }

        let current_edges = AlternatingPortEngine::extract_external_edges(cycles, &node_to_port);
        let (port_cycles, _) = AlternatingPortEngine::get_cycles(&current_edges, &node_to_port, &port_to_node, n_blocks);
        if port_cycles.len() <= 1 {
            return cycles.to_vec();
        }

        let mut giant = port_cycles[0].clone();
        let mut sat_cycles = port_cycles[1..].to_vec();
        sat_cycles.sort_by_key(|c| c.len());

        let mut giant_pos = Self::map_giant_coordinates(&giant, n_blocks);

        let mut i = 0;
        while i < sat_cycles.len() {
            let sat = &sat_cycles[i];
            let mut other_sat = Vec::new();
            for (j, o) in sat_cycles.iter().enumerate() {
                if j != i {
                    other_sat.push(o.clone());
                }
            }

            if let Some(merged) = Self::try_absorb_single_cycle_with_others(
                sat,
                &giant,
                &giant_pos,
                g,
                contractor,
                &node_to_port,
                &port_to_node,
                encoder,
                base_cnf,
                &other_sat,
            ) {
                giant = merged;
                giant_pos = Self::map_giant_coordinates(&giant, n_blocks);
                sat_cycles.remove(i);
                if sat_cycles.is_empty() {
                    break;
                }
                i = 0;
            } else {
                i += 1;
            }
        }

        let mut final_edges = HashSet::new();
        for k in 0..(giant.len() / 2) {
            let u = port_to_node[&giant[2 * k]];
            let v = port_to_node[&giant[2 * k + 1]];
            final_edges.insert(min_max(u, v));
        }
        for sat in &sat_cycles {
            for k in 0..(sat.len() / 2) {
                let u = port_to_node[&sat[2 * k]];
                let v = port_to_node[&sat[2 * k + 1]];
                final_edges.insert(min_max(u, v));
            }
        }

        let input_has_intermediate = cycles.iter().map(|c| c.len()).sum::<usize>() > n_blocks * 2;
        let final_cycles = AlternatingPortEngine::reconstruct_cycles(
            &final_edges,
            &node_to_port,
            &port_to_node,
            n_blocks,
            contractor,
            input_has_intermediate,
        );

        if final_cycles.is_empty() {
            cycles.to_vec()
        } else {
            final_cycles
        }
    }
}
