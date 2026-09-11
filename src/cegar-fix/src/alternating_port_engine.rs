use std::collections::{HashMap, HashSet};
use crate::graph::Graph;
use crate::contraction::Degree2Contractor;

#[inline]
pub fn min_max(u: i32, v: i32) -> (i32, i32) {
    if u < v { (u, v) } else { (v, u) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Port {
    pub block: usize,
    pub end: u8, // 0 for u, 1 for w
}

pub struct AlternatingPortEngine;

impl AlternatingPortEngine {
    pub fn get_cycles(
        edges: &HashSet<(i32, i32)>,
        node_to_port: &HashMap<i32, Port>,
        _port_to_node: &HashMap<Port, i32>,
        n_blocks: usize,
    ) -> (Vec<Vec<Port>>, HashMap<Port, Port>) {
        let mut pn: HashMap<Port, Port> = HashMap::with_capacity(n_blocks * 2);
        for &(u, v) in edges {
            if let (Some(&p1), Some(&p2)) = (node_to_port.get(&u), node_to_port.get(&v)) {
                pn.insert(p1, p2);
                pn.insert(p2, p1);
            }
        }

        let mut vis = HashSet::with_capacity(n_blocks * 2);
        let mut cycs = Vec::new();

        for b in 0..n_blocks {
            let p = Port { block: b, end: 0 };
            if !vis.contains(&p) && pn.contains_key(&p) {
                let mut c = Vec::new();
                let mut curr = p;
                while !vis.contains(&curr) {
                    vis.insert(curr);
                    c.push(curr);
                    if let Some(&nxt_p) = pn.get(&curr) {
                        vis.insert(nxt_p);
                        c.push(nxt_p);
                        curr = Port { block: nxt_p.block, end: 1 - nxt_p.end };
                    } else {
                        break;
                    }
                }
                cycs.push(c);
            }
        }
        cycs.sort_by_key(|c| std::cmp::Reverse(c.len()));
        (cycs, pn)
    }

    pub fn setup_ports(
        contractor: &Degree2Contractor,
    ) -> (usize, HashMap<i32, Port>, HashMap<Port, i32>) {
        let mut sorted_chains: Vec<(i32, i32)> = contractor.chain_map.keys().copied().collect();
        sorted_chains.sort();

        let mut pairs: Vec<(i32, i32)> = Vec::new();
        for (u, w) in sorted_chains {
            if u < w {
                pairs.push((u, w));
            }
        }

        let n_blocks = pairs.len();
        let mut node_to_port: HashMap<i32, Port> = HashMap::new();
        let mut port_to_node: HashMap<Port, i32> = HashMap::new();

        for (b_id, &(u, w)) in pairs.iter().enumerate() {
            let p_u = Port { block: b_id, end: 0 };
            let p_w = Port { block: b_id, end: 1 };
            node_to_port.insert(u, p_u);
            node_to_port.insert(w, p_w);
            port_to_node.insert(p_u, u);
            port_to_node.insert(p_w, w);
        }

        (n_blocks, node_to_port, port_to_node)
    }

    pub fn repair_tier1(
        cycles: &[Vec<i32>],
        g: &Graph,
        contractor: &Degree2Contractor,
    ) -> Vec<Vec<i32>> {
        if cycles.len() <= 1 || contractor.chain_map.is_empty() {
            return cycles.to_vec();
        }

        let (n_blocks, node_to_port, port_to_node) = Self::setup_ports(contractor);
        if n_blocks == 0 {
            return cycles.to_vec();
        }

        // Collect current 2-factor external edges between ports
        let mut current_edges: HashSet<(i32, i32)> = HashSet::new();

        for c in cycles {
            let port_nodes: Vec<i32> = c.iter().copied().filter(|v| node_to_port.contains_key(v)).collect();
            if port_nodes.len() == 2 {
                let u = port_nodes[0];
                let v = port_nodes[1];
                current_edges.insert(min_max(u, v));
            } else if port_nodes.len() > 2 {
                let p_len = port_nodes.len();
                for i in 0..p_len {
                    let u = port_nodes[i];
                    let v = port_nodes[(i + 1) % p_len];
                    if node_to_port[&u].block != node_to_port[&v].block {
                        current_edges.insert(min_max(u, v));
                    }
                }
            }
        }

        let (cycs, _) = Self::get_cycles(&current_edges, &node_to_port, &port_to_node, n_blocks);
        if cycs.is_empty() {
            return cycles.to_vec();
        }

        loop {
            let (cur_cycs, port_nbr) = Self::get_cycles(&current_edges, &node_to_port, &port_to_node, n_blocks);
            if cur_cycs.len() <= 1 {
                break;
            }

            let mut port_to_cyc = HashMap::new();
            for (i, c) in cur_cycs.iter().enumerate() {
                for &p in c {
                    port_to_cyc.insert(p, i);
                }
            }

            let mut inactive_partner: HashMap<Port, Port> = HashMap::with_capacity(n_blocks * 2);
            for b in 0..n_blocks {
                for end in 0..=1 {
                    let p = Port { block: b, end };
                    if let (Some(&raw_u), Some(&act_nbr)) = (port_to_node.get(&p), port_nbr.get(&p)) {
                        if let Some(nbrs) = g.adjacency_list.get(&raw_u) {
                            for &raw_v in nbrs {
                                if let Some(&inact_p) = node_to_port.get(&raw_v) {
                                    if inact_p != act_nbr && inact_p.block != p.block {
                                        inactive_partner.insert(p, inact_p);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let mut vis_aux = HashSet::new();
            let mut alt_cycles = Vec::new();
            for b in 0..n_blocks {
                for end in 0..=1 {
                    let p = Port { block: b, end };
                    if !vis_aux.contains(&p) && port_nbr.contains_key(&p) && inactive_partner.contains_key(&p) {
                        let mut ac = Vec::new();
                        let mut curr = p;
                        while !vis_aux.contains(&curr) && port_nbr.contains_key(&curr) {
                            vis_aux.insert(curr);
                            ac.push(curr);
                            let nxt1 = port_nbr[&curr];
                            vis_aux.insert(nxt1);
                            ac.push(nxt1);
                            if let Some(&nxt2) = inactive_partner.get(&nxt1) {
                                curr = nxt2;
                            } else {
                                break;
                            }
                        }
                        if ac.len() >= 4 {
                            alt_cycles.push(ac);
                        }
                    }
                }
            }

            let mut best_1step = None;
            let mut best_count = cur_cycs.len();
            let mut best_giant = cur_cycs[0].len();

            for ac in &alt_cycles {
                let mut touched = HashSet::new();
                for p in ac {
                    if let Some(&cid) = port_to_cyc.get(p) {
                        touched.insert(cid);
                    }
                }
                if touched.len() > 1 {
                    let mut rem = HashSet::new();
                    let mut add = HashSet::new();
                    for idx in (0..ac.len()).step_by(2) {
                        let p1 = ac[idx];
                        let p2 = ac[idx + 1];
                        rem.insert(min_max(port_to_node[&p1], port_to_node[&p2]));

                        let p3 = ac[(idx + 1) % ac.len()];
                        let p4 = ac[(idx + 2) % ac.len()];
                        add.insert(min_max(port_to_node[&p3], port_to_node[&p4]));
                    }
                    let mut cand = current_edges.clone();
                    for e in &rem { cand.remove(e); }
                    for e in add { cand.insert(e); }

                    let (cand_cycs, _) = Self::get_cycles(&cand, &node_to_port, &port_to_node, n_blocks);
                    if cand_cycs.len() < best_count || (cand_cycs.len() == best_count && cand_cycs[0].len() > best_giant) {
                        best_count = cand_cycs.len();
                        best_giant = cand_cycs[0].len();
                        best_1step = Some(cand);
                    }
                }
            }

            if let Some(improved) = best_1step {
                if best_count < cur_cycs.len() {
                    current_edges = improved;
                    continue;
                }
            }

            break;
        }

        // Convert resulting cycles back to Vec<Vec<i32>>
        let (final_port_cycs, _) = Self::get_cycles(&current_edges, &node_to_port, &port_to_node, n_blocks);
        let mut final_cycles = Vec::new();
        let input_has_intermediate = cycles.iter().map(|c| c.len()).sum::<usize>() > n_blocks * 2;

        for pc in final_port_cycs {
            let mut c = Vec::new();
            for p in pc {
                c.push(port_to_node[&p]);
            }
            if input_has_intermediate {
                let uncontracted = contractor.uncontract_cycle(&c);
                final_cycles.push(if uncontracted.is_empty() { c } else { uncontracted });
            } else {
                final_cycles.push(c);
            }
        }
        if final_cycles.is_empty() {
            cycles.to_vec()
        } else {
            final_cycles
        }
    }
}
