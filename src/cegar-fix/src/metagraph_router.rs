use std::collections::{HashMap, HashSet, VecDeque};
use crate::graph::Graph;
use crate::encoder::Encoder;
use rustsat::clause;
use rustsat::instances::Cnf;
use rustsat::types::{Clause, Lit};

#[derive(Debug, Clone)]
pub struct GadgetModule {
    pub id: usize,
    pub vertices: Vec<i32>,
    pub boundary_edges: Vec<(i32, i32)>, // directed edges (u, v) with u in this module, v in another module
}

#[derive(Debug, Clone)]
pub struct ChannelModule {
    pub id: usize,
    pub parent_gadget_id: usize,
    pub channel_idx: usize,
    pub vertices: Vec<i32>,
    pub boundary_edges: Vec<(i32, i32)>,
}

pub struct MetagraphRouter;

impl MetagraphRouter {
    /// Partitions the graph into gadget modules (connected clusters of vertices).
    pub fn detect_gadget_modules(g: &Graph) -> Vec<GadgetModule> {
        let n = g.adjacency_list.len();
        let target_size = if n > 100 {
            (n / 40).clamp(25, 80)
        } else {
            25
        };
        Self::detect_gadget_modules_with_size(g, target_size)
    }

    /// Partitions the graph into gadget modules with a specified maximum module size.
    pub fn detect_gadget_modules_with_size(g: &Graph, max_module_size: usize) -> Vec<GadgetModule> {
        let mut vertices: Vec<i32> = g.adjacency_list.keys().copied().collect();
        if vertices.is_empty() {
            return Vec::new();
        }
        vertices.sort_unstable();

        let max_size = max_module_size.max(1);

        // Precompute neighbor sets for fast intersection
        let mut neighbor_sets: HashMap<i32, HashSet<i32>> = HashMap::new();
        for &u in &vertices {
            if let Some(neighbors) = g.adjacency_list.get(&u) {
                neighbor_sets.insert(u, neighbors.iter().copied().collect());
            }
        }

        // Count shared neighbors on edges
        let mut strong_adj: HashMap<i32, Vec<i32>> = HashMap::new();
        let mut strong_edge_count = 0;

        for &u in &vertices {
            if let Some(u_set) = neighbor_sets.get(&u) {
                if let Some(neighbors) = g.adjacency_list.get(&u) {
                    for &v in neighbors {
                        if u < v {
                            if let Some(v_set) = neighbor_sets.get(&v) {
                                let common_count = u_set.intersection(v_set).count();
                                if common_count > 0 {
                                    strong_edge_count += 1;
                                    strong_adj.entry(u).or_default().push(v);
                                    strong_adj.entry(v).or_default().push(u);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Determine if strong edges cover most vertices
        let strong_covered_vertices = strong_adj.len();
        let use_strong = strong_edge_count > 0 && strong_covered_vertices >= (vertices.len() * 3) / 4;

        let mut initial_components: Vec<Vec<i32>> = Vec::new();

        if use_strong {
            let mut visited = HashSet::new();
            for &v in &vertices {
                if visited.contains(&v) {
                    continue;
                }

                let mut comp = Vec::new();
                let mut queue = VecDeque::new();
                visited.insert(v);
                queue.push_back(v);

                while let Some(curr) = queue.pop_front() {
                    comp.push(curr);
                    if let Some(neighbors) = strong_adj.get(&curr) {
                        for &next in neighbors {
                            if !visited.contains(&next) {
                                visited.insert(next);
                                queue.push_back(next);
                            }
                        }
                    }
                }
                comp.sort_unstable();
                initial_components.push(comp);
            }
        } else {
            // BFS partitioning directly on G into chunks of size max_size
            let mut unassigned: HashSet<i32> = vertices.iter().copied().collect();
            for &start_v in &vertices {
                if !unassigned.contains(&start_v) {
                    continue;
                }

                let mut comp = Vec::new();
                let mut queue = VecDeque::new();
                let mut in_queue = HashSet::new();

                queue.push_back(start_v);
                in_queue.insert(start_v);

                while let Some(curr) = queue.pop_front() {
                    if !unassigned.contains(&curr) {
                        continue;
                    }
                    unassigned.remove(&curr);
                    comp.push(curr);

                    if comp.len() >= max_size {
                        break;
                    }

                    if let Some(neighbors) = g.adjacency_list.get(&curr) {
                        let mut sorted_neighbors = neighbors.clone();
                        sorted_neighbors.sort_unstable();
                        for &next in &sorted_neighbors {
                            if unassigned.contains(&next) && !in_queue.contains(&next) {
                                in_queue.insert(next);
                                queue.push_back(next);
                            }
                        }
                    }
                }
                comp.sort_unstable();
                if !comp.is_empty() {
                    initial_components.push(comp);
                }
            }
        }

        // Subdivide components larger than max_size using BFS
        let mut raw_clusters: Vec<Vec<i32>> = Vec::new();

        for comp in initial_components {
            if comp.len() <= max_size {
                raw_clusters.push(comp);
            } else {
                let mut remaining: HashSet<i32> = comp.iter().copied().collect();
                let sorted_comp = comp;

                for &start_v in &sorted_comp {
                    if !remaining.contains(&start_v) {
                        continue;
                    }

                    let mut cluster = Vec::new();
                    let mut queue = VecDeque::new();
                    let mut in_queue = HashSet::new();

                    queue.push_back(start_v);
                    in_queue.insert(start_v);

                    while let Some(curr) = queue.pop_front() {
                        if !remaining.contains(&curr) {
                            continue;
                        }
                        remaining.remove(&curr);
                        cluster.push(curr);

                        if cluster.len() >= max_size {
                            break;
                        }

                        if let Some(neighbors) = g.adjacency_list.get(&curr) {
                            let mut sorted_neighbors = neighbors.clone();
                            sorted_neighbors.sort_unstable();
                            for &next_v in &sorted_neighbors {
                                if remaining.contains(&next_v) && !in_queue.contains(&next_v) {
                                    in_queue.insert(next_v);
                                    queue.push_back(next_v);
                                }
                            }
                        }
                    }

                    cluster.sort_unstable();
                    if !cluster.is_empty() {
                        raw_clusters.push(cluster);
                    }
                }
            }
        }

        // Iteratively merge degree <= 1 leaf clusters in metagraph to guarantee 2-connectivity/soundness
        let mut degree_merge_loop = 0;
        while raw_clusters.len() > 2 && degree_merge_loop < 10000 {
            degree_merge_loop += 1;
            let mut v_to_c: HashMap<i32, usize> = HashMap::new();
            for (c_idx, cl) in raw_clusters.iter().enumerate() {
                for &v in cl {
                    v_to_c.insert(v, c_idx);
                }
            }

            let mut leaf_to_merge = None;
            for (src_idx, cl) in raw_clusters.iter().enumerate() {
                let mut neighbor_clusters: HashSet<usize> = HashSet::new();
                for &u in cl {
                    if let Some(neighbors) = g.adjacency_list.get(&u) {
                        for &v in neighbors {
                            if let Some(&other_c) = v_to_c.get(&v) {
                                if other_c != src_idx {
                                    neighbor_clusters.insert(other_c);
                                }
                            }
                        }
                    }
                }

                if neighbor_clusters.len() == 1 {
                    let tgt_idx = *neighbor_clusters.iter().next().unwrap();
                    leaf_to_merge = Some((src_idx, tgt_idx));
                    break;
                }
            }

            if let Some((src_idx, tgt_idx)) = leaf_to_merge {
                let src_nodes = raw_clusters.remove(src_idx);
                let actual_tgt = if tgt_idx > src_idx { tgt_idx - 1 } else { tgt_idx };
                raw_clusters[actual_tgt].extend(src_nodes);
                raw_clusters[actual_tgt].sort_unstable();
            } else {
                break;
            }
        }

        // Merge clusters until total cluster count <= target_max_clusters (default 120)
        let target_max_clusters = 120;
        let mut loop_count = 0;
        while raw_clusters.len() > target_max_clusters && loop_count < 10000 {
            loop_count += 1;
            let mut merged_in_pass = false;

            // Rebuild vertex to cluster map
            let mut v_to_c: HashMap<i32, usize> = HashMap::new();
            for (c_idx, cl) in raw_clusters.iter().enumerate() {
                for &v in cl {
                    v_to_c.insert(v, c_idx);
                }
            }

            // Find the smallest cluster that has an adjacent neighbor cluster
            let mut candidate_order: Vec<usize> = (0..raw_clusters.len()).collect();
            candidate_order.sort_by_key(|&idx| raw_clusters[idx].len());

            for src_idx in candidate_order {
                let mut neighbor_cluster_edges: HashMap<usize, usize> = HashMap::new();
                for &u in &raw_clusters[src_idx] {
                    if let Some(neighbors) = g.adjacency_list.get(&u) {
                        for &v in neighbors {
                            if let Some(&other_c) = v_to_c.get(&v) {
                                if other_c != src_idx {
                                    *neighbor_cluster_edges.entry(other_c).or_insert(0) += 1;
                                }
                            }
                        }
                    }
                }

                let mut best_target = None;
                let mut best_score = 0;
                let mut best_target_size = usize::MAX;

                for (&other_c, &edge_count) in &neighbor_cluster_edges {
                    let target_size = raw_clusters[other_c].len();
                    if edge_count > best_score || (edge_count == best_score && target_size < best_target_size) {
                        best_score = edge_count;
                        best_target_size = target_size;
                        best_target = Some(other_c);
                    }
                }

                if let Some(tgt_idx) = best_target {
                    let src_nodes = raw_clusters.remove(src_idx);
                    let actual_tgt = if tgt_idx > src_idx { tgt_idx - 1 } else { tgt_idx };
                    raw_clusters[actual_tgt].extend(src_nodes);
                    raw_clusters[actual_tgt].sort_unstable();
                    merged_in_pass = true;
                    break;
                }
            }

            if !merged_in_pass {
                break;
            }
        }

        // Second pass: clean up any degree <= 1 leaf clusters created after size merging
        let mut degree_merge_loop2 = 0;
        while raw_clusters.len() > 2 && degree_merge_loop2 < 10000 {
            degree_merge_loop2 += 1;
            let mut v_to_c: HashMap<i32, usize> = HashMap::new();
            for (c_idx, cl) in raw_clusters.iter().enumerate() {
                for &v in cl {
                    v_to_c.insert(v, c_idx);
                }
            }

            let mut leaf_to_merge = None;
            for (src_idx, cl) in raw_clusters.iter().enumerate() {
                let mut neighbor_clusters: HashSet<usize> = HashSet::new();
                for &u in cl {
                    if let Some(neighbors) = g.adjacency_list.get(&u) {
                        for &v in neighbors {
                            if let Some(&other_c) = v_to_c.get(&v) {
                                if other_c != src_idx {
                                    neighbor_clusters.insert(other_c);
                                }
                            }
                        }
                    }
                }

                if neighbor_clusters.len() == 1 {
                    let tgt_idx = *neighbor_clusters.iter().next().unwrap();
                    leaf_to_merge = Some((src_idx, tgt_idx));
                    break;
                }
            }

            if let Some((src_idx, tgt_idx)) = leaf_to_merge {
                let src_nodes = raw_clusters.remove(src_idx);
                let actual_tgt = if tgt_idx > src_idx { tgt_idx - 1 } else { tgt_idx };
                raw_clusters[actual_tgt].extend(src_nodes);
                raw_clusters[actual_tgt].sort_unstable();
            } else {
                break;
            }
        }

        // Sort clusters deterministically by smallest vertex ID
        raw_clusters.sort_by(|a, b| {
            let min_a = a.first().copied().unwrap_or(i32::MAX);
            let min_b = b.first().copied().unwrap_or(i32::MAX);
            min_a.cmp(&min_b)
        });

        // Build vertex to module ID map
        let mut vertex_to_module: HashMap<i32, usize> = HashMap::new();
        for (mod_id, cluster) in raw_clusters.iter().enumerate() {
            for &v in cluster {
                vertex_to_module.insert(v, mod_id);
            }
        }

        // Compute boundary edges for each module
        let mut modules: Vec<GadgetModule> = Vec::with_capacity(raw_clusters.len());
        for (mod_id, cluster) in raw_clusters.into_iter().enumerate() {
            let mut boundary_edges = Vec::new();
            for &u in &cluster {
                if let Some(neighbors) = g.adjacency_list.get(&u) {
                    for &v in neighbors {
                        if let Some(&other_mod) = vertex_to_module.get(&v) {
                            if other_mod != mod_id {
                                boundary_edges.push((u, v));
                            }
                        }
                    }
                }
            }
            boundary_edges.sort_unstable();
            boundary_edges.dedup();

            modules.push(GadgetModule {
                id: mod_id,
                vertices: cluster,
                boundary_edges,
            });
        }

        modules
    }

    /// Encodes MTZ unary order constraints across all supernodes into cnf using meta-edge indicator variables X_ij.
    pub fn encode_supernode_mtz(
        modules: &[GadgetModule],
        _g: &Graph,
        encoder: &mut Encoder,
        cnf: &mut Cnf,
    ) {
        let k = modules.len();
        if k <= 2 {
            return;
        }

        // 1. Unary Order Variables for each module i in 0..k and step t in 1..k (k - 1 variables per module)
        let mut order_vars: Vec<Vec<Lit>> = Vec::with_capacity(k);
        for _ in 0..k {
            let mut o_i = Vec::with_capacity(k - 1);
            for _ in 1..k {
                let lit = encoder.instance.new_lit();
                o_i.push(lit);
            }
            order_vars.push(o_i);
        }

        // 2. Order monotonicity: !O_{i, t} \/ O_{i, t-1} for all 2 <= t < k
        // In 0-based indexing: for t in 1..(k - 1): !order_vars[i][t] \/ order_vars[i][t - 1]
        for i in 0..k {
            for t in 1..(k - 1) {
                cnf.add_clause(clause![!order_vars[i][t], order_vars[i][t - 1]]);
            }
        }

        // 3. Root fixing for module 0: u_0 = 0 => !O_{0, 1}
        cnf.add_clause(clause![!order_vars[0][0]]);

        // Build vertex to module index mapping
        let mut vertex_to_module: HashMap<i32, usize> = HashMap::new();
        for (idx, module) in modules.iter().enumerate() {
            for &v in &module.vertices {
                vertex_to_module.insert(v, idx);
            }
        }

        // 4. Group boundary edges by directed module pair (i, j)
        let mut module_pair_edges: HashMap<(usize, usize), Vec<Lit>> = HashMap::new();
        let mut module_out_lits: HashMap<usize, Vec<Lit>> = HashMap::new();
        let mut module_in_lits: HashMap<usize, Vec<Lit>> = HashMap::new();

        for (i, module) in modules.iter().enumerate() {
            for &(u, v) in &module.boundary_edges {
                if let Some(&j) = vertex_to_module.get(&v) {
                    if i != j {
                        if let Some(&l_uv) = encoder.graph_lit_map.get(&(u, v)) {
                            module_pair_edges.entry((i, j)).or_default().push(l_uv);
                            module_out_lits.entry(i).or_default().push(l_uv);
                            module_in_lits.entry(j).or_default().push(l_uv);
                        }
                    }
                }
            }
        }

        // 5. Allocate X_ij meta-edge indicator variables and apply MTZ implications
        for (&(i, j), lits) in &module_pair_edges {
            if j == 0 {
                continue;
            }

            let x_ij = encoder.instance.new_lit();
            // For each underlying boundary edge: !l_uv \/ X_ij
            for &l_uv in lits {
                cnf.add_clause(clause![!l_uv, x_ij]);
            }

            // !X_ij \/ O_{j, 1}
            cnf.add_clause(clause![!x_ij, order_vars[j][0]]);

            // For 1 <= t < K - 1: !X_ij \/ !O_{i, t} \/ O_{j, t+1}
            for t_idx in 0..(k - 2) {
                cnf.add_clause(clause![
                    !x_ij,
                    !order_vars[i][t_idx],
                    order_vars[j][t_idx + 1]
                ]);
            }

            // !X_ij \/ !O_{i, K-1}
            cnf.add_clause(clause![!x_ij, !order_vars[i][k - 2]]);
        }

        // 6. Boundary cut constraints: at least 1 outgoing boundary edge and at least 1 incoming boundary edge
        for i in 0..k {
            if let Some(mut out_lits) = module_out_lits.remove(&i) {
                out_lits.sort_unstable();
                out_lits.dedup();
                cnf.add_clause(Clause::from_iter(out_lits));
            }
            if let Some(mut in_lits) = module_in_lits.remove(&i) {
                in_lits.sort_unstable();
                in_lits.dedup();
                cnf.add_clause(Clause::from_iter(in_lits));
            }
        }
    }

    /// Detects modules and splits each module into dual sub-channels.
    pub fn detect_dual_channels(g: &Graph) -> Vec<ChannelModule> {
        let modules = Self::detect_gadget_modules(g);
        if modules.is_empty() {
            return Vec::new();
        }

        let mut raw_channels: Vec<ChannelModule> = Vec::new();
        let mut next_id = 0;

        for module in modules {
            if module.vertices.len() <= 12 {
                raw_channels.push(ChannelModule {
                    id: next_id,
                    parent_gadget_id: module.id,
                    channel_idx: 0,
                    vertices: module.vertices,
                    boundary_edges: Vec::new(),
                });
                next_id += 1;
            } else {
                let target_size = (module.vertices.len() + 1) / 2;
                let mut remaining: HashSet<i32> = module.vertices.iter().copied().collect();

                let start_v = module
                    .boundary_edges
                    .first()
                    .map(|&(u, _)| u)
                    .filter(|u| remaining.contains(u))
                    .unwrap_or(module.vertices[0]);

                let mut chan0 = Vec::new();
                let mut queue = VecDeque::new();
                let mut in_queue = HashSet::new();

                queue.push_back(start_v);
                in_queue.insert(start_v);

                while let Some(curr) = queue.pop_front() {
                    if !remaining.contains(&curr) {
                        continue;
                    }
                    remaining.remove(&curr);
                    chan0.push(curr);

                    if chan0.len() >= target_size {
                        break;
                    }

                    if let Some(neighbors) = g.adjacency_list.get(&curr) {
                        let mut sorted_neighbors = neighbors.clone();
                        sorted_neighbors.sort_unstable();
                        for &next_v in &sorted_neighbors {
                            if remaining.contains(&next_v) && !in_queue.contains(&next_v) {
                                in_queue.insert(next_v);
                                queue.push_back(next_v);
                            }
                        }
                    }
                }

                while chan0.len() < target_size && !remaining.is_empty() {
                    let mut sorted_remaining: Vec<i32> = remaining.iter().copied().collect();
                    sorted_remaining.sort_unstable();
                    let next_start = sorted_remaining[0];

                    queue.push_back(next_start);
                    in_queue.insert(next_start);

                    while let Some(curr) = queue.pop_front() {
                        if !remaining.contains(&curr) {
                            continue;
                        }
                        remaining.remove(&curr);
                        chan0.push(curr);

                        if chan0.len() >= target_size {
                            break;
                        }

                        if let Some(neighbors) = g.adjacency_list.get(&curr) {
                            let mut sorted_neighbors = neighbors.clone();
                            sorted_neighbors.sort_unstable();
                            for &next_v in &sorted_neighbors {
                                if remaining.contains(&next_v) && !in_queue.contains(&next_v) {
                                    in_queue.insert(next_v);
                                    queue.push_back(next_v);
                                }
                            }
                        }
                    }
                }

                let mut chan1: Vec<i32> = remaining.into_iter().collect();
                chan0.sort_unstable();
                chan1.sort_unstable();

                raw_channels.push(ChannelModule {
                    id: next_id,
                    parent_gadget_id: module.id,
                    channel_idx: 0,
                    vertices: chan0,
                    boundary_edges: Vec::new(),
                });
                raw_channels.push(ChannelModule {
                    id: next_id + 1,
                    parent_gadget_id: module.id,
                    channel_idx: 1,
                    vertices: chan1,
                    boundary_edges: Vec::new(),
                });
                next_id += 2;
            }
        }

        // Build vertex to channel mapping
        let mut vertex_to_channel: HashMap<i32, usize> = HashMap::new();
        for ch in &raw_channels {
            for &v in &ch.vertices {
                vertex_to_channel.insert(v, ch.id);
            }
        }

        // Compute boundary edges for each channel
        for ch in &mut raw_channels {
            let mut boundary_edges = Vec::new();
            for &u in &ch.vertices {
                if let Some(neighbors) = g.adjacency_list.get(&u) {
                    for &v in neighbors {
                        if let Some(&other_ch_id) = vertex_to_channel.get(&v) {
                            if other_ch_id != ch.id {
                                boundary_edges.push((u, v));
                            }
                        }
                    }
                }
            }
            boundary_edges.sort_unstable();
            boundary_edges.dedup();
            ch.boundary_edges = boundary_edges;
        }

        raw_channels
    }

    /// Encodes MTZ unary order constraints across all channel supernodes into cnf using meta-edge indicator variables X_ij.
    pub fn encode_dual_channel_mtz(
        channels: &[ChannelModule],
        encoder: &mut Encoder,
        cnf: &mut Cnf,
    ) {
        let k = channels.len();
        if k <= 2 {
            return;
        }

        // 1. Unary Order Variables for each channel i in 0..k and step t in 1..k (k - 1 variables per channel)
        let mut order_vars: Vec<Vec<Lit>> = Vec::with_capacity(k);
        for _ in 0..k {
            let mut o_i = Vec::with_capacity(k - 1);
            for _ in 1..k {
                let lit = encoder.instance.new_lit();
                o_i.push(lit);
            }
            order_vars.push(o_i);
        }

        // 2. Order monotonicity: !O_{i, t} \/ O_{i, t-1} for all 2 <= t < k
        // In 0-based indexing: for t in 1..(k - 1): !order_vars[i][t] \/ order_vars[i][t - 1]
        for i in 0..k {
            for t in 1..(k - 1) {
                cnf.add_clause(clause![!order_vars[i][t], order_vars[i][t - 1]]);
            }
        }

        // 3. Root fixing for channel 0: u_0 = 0 => !O_{0, 1}
        cnf.add_clause(clause![!order_vars[0][0]]);

        // Build vertex to channel index mapping
        let mut vertex_to_channel_idx: HashMap<i32, usize> = HashMap::new();
        for (idx, channel) in channels.iter().enumerate() {
            for &v in &channel.vertices {
                vertex_to_channel_idx.insert(v, idx);
            }
        }

        // 4. Group boundary edges by directed channel pair (i, j)
        let mut channel_pair_edges: HashMap<(usize, usize), Vec<Lit>> = HashMap::new();
        let mut channel_out_lits: HashMap<usize, Vec<Lit>> = HashMap::new();
        let mut channel_in_lits: HashMap<usize, Vec<Lit>> = HashMap::new();

        for (i, channel) in channels.iter().enumerate() {
            for &(u, v) in &channel.boundary_edges {
                if let Some(&j) = vertex_to_channel_idx.get(&v) {
                        if let Some(&l_uv) = encoder.graph_lit_map.get(&(u, v)) {
                            channel_pair_edges.entry((i, j)).or_default().push(l_uv);
                            channel_out_lits.entry(i).or_default().push(l_uv);
                            channel_in_lits.entry(j).or_default().push(l_uv);
                        }
                }
            }
        }

        // 5. Allocate X_ij meta-edge indicator variables and apply MTZ implications
        for (&(i, j), lits) in &channel_pair_edges {
            if j == 0 {
                continue;
            }

            let x_ij = encoder.instance.new_lit();
            // For each underlying boundary edge: !l_uv \/ X_ij
            for &l_uv in lits {
                cnf.add_clause(clause![!l_uv, x_ij]);
            }

            // !X_ij \/ O_{j, 1}
            cnf.add_clause(clause![!x_ij, order_vars[j][0]]);

            // For 1 <= t < K - 1: !X_ij \/ !O_{i, t} \/ O_{j, t+1}
            for t_idx in 0..(k - 2) {
                cnf.add_clause(clause![
                    !x_ij,
                    !order_vars[i][t_idx],
                    order_vars[j][t_idx + 1]
                ]);
            }

            // !X_ij \/ !O_{i, K-1}
            cnf.add_clause(clause![!x_ij, !order_vars[i][k - 2]]);
        }

        // 6. Channel boundary cut constraints: at least 1 outgoing boundary edge and at least 1 incoming boundary edge
        for i in 0..k {
            if let Some(mut out_lits) = channel_out_lits.remove(&i) {
                out_lits.sort_unstable();
                out_lits.dedup();
                cnf.add_clause(Clause::from_iter(out_lits));
            }
            if let Some(mut in_lits) = channel_in_lits.remove(&i) {
                in_lits.sort_unstable();
                in_lits.dedup();
                cnf.add_clause(Clause::from_iter(in_lits));
            }
        }
    }
}
