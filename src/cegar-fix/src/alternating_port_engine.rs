use std::collections::{HashMap, HashSet, VecDeque};
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

impl Port {
    #[inline]
    pub fn idx(&self) -> usize {
        self.block * 2 + (self.end as usize)
    }

    #[inline]
    pub fn from_idx(idx: usize) -> Self {
        Port {
            block: idx / 2,
            end: (idx % 2) as u8,
        }
    }
}

pub struct AlternatingPortEngine;

impl AlternatingPortEngine {
    pub fn get_cycles(
        edges: &HashSet<(i32, i32)>,
        node_to_port: &HashMap<i32, Port>,
        _port_to_node: &HashMap<Port, i32>,
        n_blocks: usize,
    ) -> (Vec<Vec<Port>>, HashMap<Port, Port>) {
        let mut pn_vec = vec![Port { block: usize::MAX, end: 0 }; n_blocks * 2];
        for &(u, v) in edges {
            if let (Some(&p1), Some(&p2)) = (node_to_port.get(&u), node_to_port.get(&v)) {
                pn_vec[p1.idx()] = p2;
                pn_vec[p2.idx()] = p1;
            }
        }

        let mut vis = vec![false; n_blocks * 2];
        let mut cycs = Vec::new();

        for b in 0..n_blocks {
            let p = Port { block: b, end: 0 };
            if !vis[p.idx()] && pn_vec[p.idx()].block != usize::MAX {
                let mut c = Vec::new();
                let mut curr = p;
                while !vis[curr.idx()] {
                    vis[curr.idx()] = true;
                    c.push(curr);
                    let nxt_p = pn_vec[curr.idx()];
                    if nxt_p.block == usize::MAX {
                        break;
                    }
                    vis[nxt_p.idx()] = true;
                    c.push(nxt_p);
                    curr = Port { block: nxt_p.block, end: 1 - nxt_p.end };
                }
                cycs.push(c);
            }
        }
        cycs.sort_by_key(|c| std::cmp::Reverse(c.len()));

        let mut pn = HashMap::with_capacity(n_blocks * 2);
        for idx in 0..(n_blocks * 2) {
            let nxt = pn_vec[idx];
            if nxt.block != usize::MAX {
                pn.insert(Port::from_idx(idx), nxt);
            }
        }

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

    pub fn extract_external_edges(
        cycles: &[Vec<i32>],
        node_to_port: &HashMap<i32, Port>,
    ) -> HashSet<(i32, i32)> {
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
        current_edges
    }

    pub fn reconstruct_cycles(
        current_edges: &HashSet<(i32, i32)>,
        node_to_port: &HashMap<i32, Port>,
        port_to_node: &HashMap<Port, i32>,
        n_blocks: usize,
        contractor: &Degree2Contractor,
        input_has_intermediate: bool,
    ) -> Vec<Vec<i32>> {
        let (final_port_cycs, _) = Self::get_cycles(current_edges, node_to_port, port_to_node, n_blocks);
        let mut final_cycles = Vec::new();

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
        final_cycles
    }

    pub fn repair_tier1_edges(
        current_edges: &mut HashSet<(i32, i32)>,
        g: &Graph,
        node_to_port: &HashMap<i32, Port>,
        port_to_node: &HashMap<Port, i32>,
        n_blocks: usize,
    ) {
        loop {
            let (cur_cycs, port_nbr) = Self::get_cycles(current_edges, node_to_port, port_to_node, n_blocks);
            if cur_cycs.len() <= 1 {
                break;
            }

            let mut port_to_cyc = HashMap::with_capacity(n_blocks * 2);
            for (i, c) in cur_cycs.iter().enumerate() {
                for &p in c {
                    port_to_cyc.insert(p, i);
                }
            }

            let mut inactive_partner: HashMap<Port, Port> = HashMap::with_capacity(n_blocks * 2);
            for b in 0..n_blocks {
                for end in 0..=1 {
                    let p = Port { block: b, end };
                    let raw_u = port_to_node[&p];
                    let act_nbr = port_nbr[&p];
                    if let Some(nbrs) = g.adjacency_list.get(&raw_u) {
                        for &raw_v in nbrs {
                            if let Some(&inact_p) = node_to_port.get(&raw_v) {
                                if inact_p != act_nbr {
                                    inactive_partner.insert(p, inact_p);
                                    break;
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
                    let p0 = Port { block: b, end };
                    if !vis_aux.contains(&p0) && port_nbr.contains_key(&p0) && inactive_partner.contains_key(&p0) {
                        let mut walk = Vec::new();
                        let mut walk_pos: HashMap<Port, usize> = HashMap::new();
                        let mut curr = p0;

                        while !vis_aux.contains(&curr) && port_nbr.contains_key(&curr) {
                            vis_aux.insert(curr);
                            walk_pos.insert(curr, walk.len());
                            walk.push(curr);

                            let nxt1 = port_nbr[&curr];
                            vis_aux.insert(nxt1);
                            walk_pos.insert(nxt1, walk.len());
                            walk.push(nxt1);

                            if let Some(&nxt2) = inactive_partner.get(&nxt1) {
                                if let Some(&start_idx) = walk_pos.get(&nxt2) {
                                    if start_idx % 2 == 0 {
                                        let cyc_slice = walk[start_idx..].to_vec();
                                        if cyc_slice.len() >= 4 && cyc_slice.len() % 2 == 0 {
                                            alt_cycles.push(cyc_slice);
                                        }
                                    }
                                    break;
                                } else {
                                    curr = nxt2;
                                }
                            } else {
                                break;
                            }
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

                    // Loop Closure & Graph Adjacency Guard
                    let valid_edges = add.iter().all(|&(u, v)| {
                        g.adjacency_list.get(&u).map_or(false, |nbrs| nbrs.contains(&v))
                    });
                    if !valid_edges {
                        continue;
                    }

                    let valid_rem = rem.iter().all(|e| current_edges.contains(e));
                    if !valid_rem {
                        continue;
                    }

                    let mut cand = current_edges.clone();
                    for e in &rem { cand.remove(e); }
                    for e in add { cand.insert(e); }

                    let (cand_cycs, _) = Self::get_cycles(&cand, node_to_port, port_to_node, n_blocks);
                    if cand_cycs.len() < best_count || (cand_cycs.len() == best_count && cand_cycs[0].len() > best_giant) {
                        best_count = cand_cycs.len();
                        best_giant = cand_cycs[0].len();
                        best_1step = Some(cand);
                    }
                }
            }

            if let Some(improved) = best_1step {
                if best_count < cur_cycs.len() || best_giant > cur_cycs[0].len() {
                    *current_edges = improved;
                    continue;
                }
            }

            break;
        }
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

        let mut current_edges = Self::extract_external_edges(cycles, &node_to_port);
        let (cycs, _) = Self::get_cycles(&current_edges, &node_to_port, &port_to_node, n_blocks);
        if cycs.is_empty() {
            return cycles.to_vec();
        }

        Self::repair_tier1_edges(&mut current_edges, g, &node_to_port, &port_to_node, n_blocks);

        let input_has_intermediate = cycles.iter().map(|c| c.len()).sum::<usize>() > n_blocks * 2;
        let final_cycles = Self::reconstruct_cycles(
            &current_edges,
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

    pub fn repair(
        cycles: &[Vec<i32>],
        g: &Graph,
        contractor: &Degree2Contractor,
        max_depth: usize,
        timeout_ms: u64,
    ) -> Vec<Vec<i32>> {
        let t_start = std::time::Instant::now();
        if cycles.len() <= 1 || contractor.chain_map.is_empty() {
            return cycles.to_vec();
        }

        let (n_blocks, node_to_port, port_to_node) = Self::setup_ports(contractor);
        if n_blocks == 0 {
            return cycles.to_vec();
        }

        let mut current_edges = Self::extract_external_edges(cycles, &node_to_port);
        let (cycs, _) = Self::get_cycles(&current_edges, &node_to_port, &port_to_node, n_blocks);
        if cycs.is_empty() {
            return cycles.to_vec();
        }

        // 1. Run Tier 1 greedy flips
        Self::repair_tier1_edges(&mut current_edges, g, &node_to_port, &port_to_node, n_blocks);

        // 2. Run Tier 2 bounded multi-hop BFS if multiple cycles remain
        loop {
            if t_start.elapsed().as_millis() as u64 >= timeout_ms {
                break;
            }

            let (cur_cycs, port_nbr) = Self::get_cycles(&current_edges, &node_to_port, &port_to_node, n_blocks);
            if cur_cycs.len() <= 1 {
                break;
            }

            let mut inactive_adj: HashMap<Port, Vec<Port>> = HashMap::new();
            for b in 0..n_blocks {
                for end in 0..=1 {
                    let p = Port { block: b, end };
                    if let (Some(&raw_u), Some(&act_nbr)) = (port_to_node.get(&p), port_nbr.get(&p)) {
                        if let Some(nbrs) = g.adjacency_list.get(&raw_u) {
                            for &raw_v in nbrs {
                                if let Some(&inact_p) = node_to_port.get(&raw_v) {
                                    if inact_p != act_nbr && inact_p.block != p.block {
                                        inactive_adj.entry(p).or_default().push(inact_p);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let mut found_improvement = false;

            // Search from smaller cycles (index 1..cur_cycs.len())
            for target_cycle_id in 1..cur_cycs.len() {
                if t_start.elapsed().as_millis() as u64 >= timeout_ms {
                    break;
                }
                let target_ports = &cur_cycs[target_cycle_id];
                for &p0 in target_ports {
                    if t_start.elapsed().as_millis() as u64 >= timeout_ms {
                        break;
                    }
                    let p_goal = match port_nbr.get(&p0) {
                        Some(&pg) => pg,
                        None => continue,
                    };

                    let mut queue = VecDeque::new();
                    let mut parent: HashMap<Port, (Port, Port)> = HashMap::new();
                    let mut visited = HashSet::new();

                    visited.insert(p0);
                    queue.push_back((p0, 0));

                    while let Some((curr, depth)) = queue.pop_front() {
                        if depth >= max_depth {
                            continue;
                        }
                        if let Some(inacts) = inactive_adj.get(&curr) {
                            for &inact in inacts {
                                let nxt2 = match port_nbr.get(&inact) {
                                    Some(&n) => n,
                                    None => continue,
                                };

                                if inact == p_goal && depth >= 1 {
                                    let mut path = Vec::new();
                                    let mut c_ptr = curr;
                                    let mut i_ptr = inact;
                                    path.push((c_ptr, i_ptr, p0));
                                    while c_ptr != p0 {
                                        if let Some(&(prev_curr, prev_inact)) = parent.get(&c_ptr) {
                                            path.push((prev_curr, prev_inact, c_ptr));
                                            c_ptr = prev_curr;
                                        } else {
                                            break;
                                        }
                                    }
                                    if c_ptr != p0 {
                                        continue;
                                    }
                                    path.reverse();

                                    // Verify cycle simplicity: exactly 2 distinct ports per hop
                                    let mut cycle_ports = HashSet::new();
                                    for &(c, i, _) in &path {
                                        cycle_ports.insert(c);
                                        cycle_ports.insert(i);
                                    }
                                    if cycle_ports.len() != path.len() * 2 {
                                        continue;
                                    }

                                    let mut rem = HashSet::new();
                                    let mut add = HashSet::new();
                                    for &(c, i, n2) in &path {
                                        add.insert(min_max(port_to_node[&c], port_to_node[&i]));
                                        rem.insert(min_max(port_to_node[&i], port_to_node[&n2]));
                                    }

                                    // Guard: all edges in add must exist in g
                                    let valid_edges = add.iter().all(|&(u, v)| {
                                        g.adjacency_list.get(&u).map_or(false, |nbrs| nbrs.contains(&v))
                                    });
                                    if !valid_edges {
                                        continue;
                                    }

                                    // Guard: all edges in rem must exist in current_edges
                                    let valid_rem = rem.iter().all(|e| current_edges.contains(e));
                                    if !valid_rem {
                                        continue;
                                    }

                                    let mut cand = current_edges.clone();
                                    for e in &rem { cand.remove(e); }
                                    for e in &add { cand.insert(*e); }

                                    let (cand_cycs, _) = Self::get_cycles(&cand, &node_to_port, &port_to_node, n_blocks);
                                    if cand_cycs.len() < cur_cycs.len() {
                                        current_edges = cand;
                                        found_improvement = true;
                                        break;
                                    }
                                }

                                if !visited.contains(&nxt2) {
                                    visited.insert(nxt2);
                                    parent.insert(nxt2, (curr, inact));
                                    queue.push_back((nxt2, depth + 1));
                                }
                            }
                        }
                        if found_improvement {
                            break;
                        }
                    }
                    if found_improvement {
                        break;
                    }
                }
                if found_improvement {
                    break;
                }
            }

            if found_improvement {
                Self::repair_tier1_edges(&mut current_edges, g, &node_to_port, &port_to_node, n_blocks);
            } else {
                break;
            }
        }

        let input_has_intermediate = cycles.iter().map(|c| c.len()).sum::<usize>() > n_blocks * 2;
        let final_cycles = Self::reconstruct_cycles(
            &current_edges,
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
