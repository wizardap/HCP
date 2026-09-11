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
                                if inact_p != act_nbr && inact_p.block != p.block {
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


    pub fn repair_tier2_bidirectional_bfs(
        current_edges: &mut HashSet<(i32, i32)>,
        g: &Graph,
        node_to_port: &HashMap<i32, Port>,
        port_to_node: &HashMap<Port, i32>,
        n_blocks: usize,
        max_depth_per_dir: usize,
        timeout_ms: u64,
        t_start: std::time::Instant,
    ) -> bool {
        let deadline_ms = timeout_ms.saturating_sub(50);
        if t_start.elapsed().as_millis() as u64 >= deadline_ms {
            return false;
        }

        let (mut cur_cycs, port_nbr) = Self::get_cycles(current_edges, node_to_port, port_to_node, n_blocks);
        if cur_cycs.len() <= 1 {
            return false;
        }

        // Smallest-Cycle-First: sort cycles by length ascending
        cur_cycs.sort_by_key(|c| c.len());

        let mut port_to_cyc = vec![usize::MAX; n_blocks * 2];
        for (cid, c) in cur_cycs.iter().enumerate() {
            for &p in c {
                port_to_cyc[p.idx()] = cid;
            }
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

        let search_cycles_count = if cur_cycs.len() > 1 { cur_cycs.len() - 1 } else { 1 };

        for target_cycle_id in 0..search_cycles_count {
            if t_start.elapsed().as_millis() as u64 >= deadline_ms {
                return false;
            }
            let target_ports = &cur_cycs[target_cycle_id];
            for &p0 in target_ports {
                if t_start.elapsed().as_millis() as u64 >= deadline_ms {
                    return false;
                }
                let p_goal = match port_nbr.get(&p0) {
                    Some(&pg) => pg,
                    None => continue,
                };

                // Forward BFS from p0
                // Queue: (curr_port, depth)
                // parent_fwd: nxt -> (prev_curr, inact)
                let mut queue_fwd = VecDeque::new();
                let mut parent_fwd: HashMap<Port, (Port, Port)> = HashMap::new();
                let mut visited_fwd = HashSet::new();

                visited_fwd.insert(p0);
                queue_fwd.push_back((p0, 0));

                let mut forward_direct_sol = None;

                while let Some((curr, depth)) = queue_fwd.pop_front() {
                    if depth >= max_depth_per_dir {
                        continue;
                    }
                    if let Some(inacts) = inactive_adj.get(&curr) {
                        for &inact in inacts {
                            if inact == p_goal && depth >= 1 {
                                forward_direct_sol = Some((curr, inact));
                                break;
                            }
                            let nxt = match port_nbr.get(&inact) {
                                Some(&n) => n,
                                None => continue,
                            };
                            if !visited_fwd.contains(&nxt) {
                                visited_fwd.insert(nxt);
                                parent_fwd.insert(nxt, (curr, inact));
                                queue_fwd.push_back((nxt, depth + 1));
                            }
                        }
                    }
                    if forward_direct_sol.is_some() {
                        break;
                    }
                }

                // Check forward direct closure at p_goal
                if let Some((last_curr, last_inact)) = forward_direct_sol {
                    let mut path = Vec::new();
                    let mut c_ptr = last_curr;
                    path.push((c_ptr, last_inact, p0));
                    while c_ptr != p0 {
                        if let Some(&(prev_curr, prev_inact)) = parent_fwd.get(&c_ptr) {
                            path.push((prev_curr, prev_inact, c_ptr));
                            c_ptr = prev_curr;
                        } else {
                            break;
                        }
                    }
                    if c_ptr == p0 {
                        let mut cycle_ports = HashSet::new();
                        for &(c, i, _) in &path {
                            cycle_ports.insert(c);
                            cycle_ports.insert(i);
                        }
                        if cycle_ports.len() == path.len() * 2 {
                            let start_cid = port_to_cyc[p0.idx()];
                            if !cycle_ports.iter().any(|p| port_to_cyc[p.idx()] != start_cid) {
                                continue;
                            }
                            let mut rem = HashSet::new();
                            let mut add = HashSet::new();
                            for &(c, i, n2) in &path {
                                add.insert(min_max(port_to_node[&c], port_to_node[&i]));
                                rem.insert(min_max(port_to_node[&i], port_to_node[&n2]));
                            }
                            let valid_edges = add.iter().all(|&(u, v)| {
                                g.adjacency_list.get(&u).map_or(false, |nbrs| nbrs.contains(&v))
                            });
                            let valid_rem = rem.iter().all(|e| current_edges.contains(e));
                            if valid_edges && valid_rem {
                                let mut cand = current_edges.clone();
                                for e in &rem { cand.remove(e); }
                                for e in &add { cand.insert(*e); }
                                let (cand_cycs, _) = Self::get_cycles(&cand, node_to_port, port_to_node, n_blocks);
                                if cand_cycs.len() < cur_cycs.len() || (cand_cycs.len() == cur_cycs.len() && cand_cycs[0].len() > cur_cycs[0].len()) {
                                    *current_edges = cand;
                                    return true;
                                }
                            }
                        }
                    }
                }

                // Backward BFS from p_goal
                // Queue: (curr_port, depth)
                // parent_bwd: prev_port -> (next_port, act_hop_port)
                let mut queue_bwd = VecDeque::new();
                let mut parent_bwd: HashMap<Port, (Port, Port)> = HashMap::new();
                let mut visited_bwd = HashSet::new();

                if let Some(inacts) = inactive_adj.get(&p_goal) {
                    for &w in inacts {
                        if !visited_bwd.contains(&w) {
                            visited_bwd.insert(w);
                            parent_bwd.insert(w, (p_goal, p_goal));
                            queue_bwd.push_back((w, 1));
                        }
                    }
                }

                // Expand backward frontier and check collisions with forward frontier
                while let Some((curr, depth)) = queue_bwd.pop_front() {
                    if visited_fwd.contains(&curr) {
                        let p_mid = curr;
                        let mut fwd_hops = Vec::new();
                        let mut c_ptr = p_mid;
                        while c_ptr != p0 {
                            if let Some(&(prev_curr, inact)) = parent_fwd.get(&c_ptr) {
                                fwd_hops.push((prev_curr, inact, c_ptr));
                                c_ptr = prev_curr;
                            } else {
                                break;
                            }
                        }
                        if c_ptr == p0 {
                            fwd_hops.reverse();

                            let mut add = HashSet::new();
                            let mut rem = HashSet::new();
                            let mut cycle_ports = HashSet::new();

                            cycle_ports.insert(p0);
                            cycle_ports.insert(p_goal);
                            rem.insert(min_max(port_to_node[&p_goal], port_to_node[&p0]));

                            for &(u, i, v) in &fwd_hops {
                                cycle_ports.insert(u);
                                cycle_ports.insert(i);
                                cycle_ports.insert(v);
                                add.insert(min_max(port_to_node[&u], port_to_node[&i]));
                                rem.insert(min_max(port_to_node[&i], port_to_node[&v]));
                            }

                            let mut b_curr = p_mid;
                            let mut b_valid = true;
                            while b_curr != p_goal {
                                if let Some(&(next_curr, act)) = parent_bwd.get(&b_curr) {
                                    if next_curr == p_goal {
                                        cycle_ports.insert(b_curr);
                                        add.insert(min_max(port_to_node[&b_curr], port_to_node[&p_goal]));
                                        break;
                                    } else {
                                        cycle_ports.insert(b_curr);
                                        cycle_ports.insert(act);
                                        cycle_ports.insert(next_curr);
                                        add.insert(min_max(port_to_node[&b_curr], port_to_node[&act]));
                                        rem.insert(min_max(port_to_node[&act], port_to_node[&next_curr]));
                                        b_curr = next_curr;
                                    }
                                } else {
                                    b_valid = false;
                                    break;
                                }
                            }

                            if b_valid && add.len() == rem.len() && cycle_ports.len() == add.len() * 2 {
                                let start_cid = port_to_cyc[p0.idx()];
                                if !cycle_ports.iter().any(|p| port_to_cyc[p.idx()] != start_cid) {
                                    continue;
                                }
                                let valid_edges = add.iter().all(|&(u, v)| {
                                    g.adjacency_list.get(&u).map_or(false, |nbrs| nbrs.contains(&v))
                                });
                                let valid_rem = rem.iter().all(|e| current_edges.contains(e));
                                if valid_edges && valid_rem {
                                    let mut cand = current_edges.clone();
                                    for e in &rem { cand.remove(e); }
                                    for e in &add { cand.insert(*e); }
                                    let (cand_cycs, _) = Self::get_cycles(&cand, node_to_port, port_to_node, n_blocks);
                                    if cand_cycs.len() < cur_cycs.len() || (cand_cycs.len() == cur_cycs.len() && cand_cycs[0].len() > cur_cycs[0].len()) {
                                        *current_edges = cand;
                                        return true;
                                    }
                                }
                            }
                        }
                    }

                    if depth >= max_depth_per_dir {
                        continue;
                    }

                    let act = match port_nbr.get(&curr) {
                        Some(&a) => a,
                        None => continue,
                    };
                    if let Some(inacts) = inactive_adj.get(&act) {
                        for &prev in inacts {
                            if !visited_bwd.contains(&prev) {
                                visited_bwd.insert(prev);
                                parent_bwd.insert(prev, (curr, act));
                                queue_bwd.push_back((prev, depth + 1));
                            }
                        }
                    }
                }
            }
        }

        false
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

        // 2. Run Tier 2 bounded multi-hop bidirectional BFS if multiple cycles remain
        loop {
            if t_start.elapsed().as_millis() as u64 >= timeout_ms.saturating_sub(50) {
                break;
            }

            let (cur_cycs, _) = Self::get_cycles(&current_edges, &node_to_port, &port_to_node, n_blocks);
            if cur_cycs.len() <= 1 {
                break;
            }

            let improved = Self::repair_tier2_bidirectional_bfs(
                &mut current_edges,
                g,
                &node_to_port,
                &port_to_node,
                n_blocks,
                max_depth,
                timeout_ms,
                t_start,
            );

            if improved {
                // Call repair_tier1_edges immediately whenever an improvement is found
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
