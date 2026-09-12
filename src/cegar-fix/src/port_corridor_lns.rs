use std::collections::{HashMap, HashSet};
use crate::graph::Graph;
use crate::alternating_port_engine::Port;

#[derive(Debug, Clone)]
pub struct PortSubpathCorridor {
    pub sat_blocks: HashSet<usize>,
    pub giant_subpath_blocks: Vec<usize>,
    pub entry_port: Port,
    pub exit_port: Port,
    pub unfrozen_blocks: HashSet<usize>,
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

        let span_len = std::cmp::min(aligned_len, max_span * 2);

        // Add buffer blocks with underflow protection
        let buf_ports = buffer * 2;
        let entry_idx = (aligned_start + n_giant_ports - (buf_ports % n_giant_ports)) % n_giant_ports;
        let total_subpath_ports = std::cmp::min(span_len + buf_ports * 2, n_giant_ports);
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
}
