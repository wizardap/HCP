use crate::core::graph::Graph;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct BipartitePartition {
    pub super_hubs: Vec<i32>,
    pub clusters: HashMap<i32, HashSet<i32>>,
    pub owner: HashMap<i32, i32>,
    pub boundary_ports: HashMap<i32, Vec<i32>>,
    pub connectors: Vec<i32>,
    pub macro_edges: Vec<(i32, i32)>,
}

pub fn can_solve_bipartite(raw_g: &Graph) -> bool {
    let super_hubs_count = raw_g
        .adjacency_list
        .iter()
        .filter(|&(_, nbrs)| nbrs.len() >= 400)
        .count();
    super_hubs_count == 5 || super_hubs_count == 10
}

pub fn detect_and_partition(raw_g: &Graph) -> Option<BipartitePartition> {
    let mut super_hubs: Vec<i32> = raw_g
        .adjacency_list
        .iter()
        .filter(|&(_, nbrs)| nbrs.len() >= 400)
        .map(|(&u, _)| u)
        .collect();
    super_hubs.sort_unstable();

    let k = super_hubs.len();
    if k != 5 && k != 10 {
        return None;
    }

    let sh_set: HashSet<i32> = super_hubs.iter().copied().collect();

    // 1-hop direct hub signatures
    let mut direct_hubs: HashMap<i32, HashSet<i32>> = HashMap::new();
    for (&u, nbrs) in &raw_g.adjacency_list {
        let dh: HashSet<i32> = nbrs.iter().filter(|v| sh_set.contains(v)).copied().collect();
        direct_hubs.insert(u, dh);
    }

    // 2-hop hub signatures for vertices with 0 direct hubs
    let mut two_hop_hubs: HashMap<i32, HashSet<i32>> = HashMap::new();
    for (&u, nbrs) in &raw_g.adjacency_list {
        if !sh_set.contains(&u) && direct_hubs[&u].is_empty() {
            let mut th = HashSet::new();
            for &w in nbrs {
                if let Some(dh) = direct_hubs.get(&w) {
                    th.extend(dh);
                }
            }
            two_hop_hubs.insert(u, th);
        }
    }

    let mut clusters: HashMap<i32, HashSet<i32>> = HashMap::new();
    let mut owner: HashMap<i32, i32> = HashMap::new();
    let mut unassigned: HashSet<i32> = HashSet::new();

    for &h in &super_hubs {
        clusters.entry(h).or_default().insert(h);
        owner.insert(h, h);
    }

    for (&u, _) in &raw_g.adjacency_list {
        if sh_set.contains(&u) {
            continue;
        }
        let dh = &direct_hubs[&u];
        if dh.len() == 1 {
            let h = *dh.iter().next().unwrap();
            clusters.entry(h).or_default().insert(u);
            owner.insert(u, h);
        } else if dh.is_empty() {
            if let Some(th) = two_hop_hubs.get(&u) {
                if th.len() == 1 {
                    let h = *th.iter().next().unwrap();
                    clusters.entry(h).or_default().insert(u);
                    owner.insert(u, h);
                } else {
                    unassigned.insert(u);
                }
            } else {
                unassigned.insert(u);
            }
        } else {
            unassigned.insert(u);
        }
    }

    // Dominant hub absorption: if >= 90% of a vertex's neighbors belong to one cluster, absorb it
    let mut unassigned_vec: Vec<i32> = unassigned.iter().copied().collect();
    unassigned_vec.sort_unstable();
    for u in unassigned_vec {
        let nbrs = match raw_g.adjacency_list.get(&u) {
            Some(n) => n,
            None => continue,
        };
        let mut cluster_counts: HashMap<i32, usize> = HashMap::new();
        for &w in nbrs {
            if let Some(&h) = owner.get(&w) {
                *cluster_counts.entry(h).or_default() += 1;
            }
        }
        if let Some((&dominant_hub, &count)) = cluster_counts.iter().max_by_key(|&(_, c)| *c) {
            if count as f64 / nbrs.len() as f64 >= 0.90 {
                clusters.entry(dominant_hub).or_default().insert(u);
                owner.insert(u, dominant_hub);
                unassigned.remove(&u);
            }
        }
    }

    let mut connectors: Vec<i32> = unassigned.into_iter().collect();
    connectors.sort_unstable();

    // Extract boundary ports per cluster
    let mut boundary_ports: HashMap<i32, Vec<i32>> = HashMap::new();
    let mut all_ports: HashSet<i32> = HashSet::new();

    for (&h, c_nodes) in &clusters {
        let mut ports = Vec::new();
        for &u in c_nodes {
            if let Some(nbrs) = raw_g.adjacency_list.get(&u) {
                let has_ext = nbrs.iter().any(|v| !c_nodes.contains(v));
                if has_ext {
                    ports.push(u);
                    all_ports.insert(u);
                }
            }
        }
        ports.sort_unstable();
        boundary_ports.insert(h, ports);
    }

    let mut all_macro_nodes: HashSet<i32> = all_ports;
    all_macro_nodes.extend(&connectors);

    let mut macro_edges = Vec::new();
    for &u in &all_macro_nodes {
        if let Some(nbrs) = raw_g.adjacency_list.get(&u) {
            for &v in nbrs {
                if u < v && all_macro_nodes.contains(&v) {
                    let ou = owner.get(&u);
                    let ov = owner.get(&v);
                    if ou != ov || ou.is_none() || ov.is_none() {
                        macro_edges.push((u, v));
                    }
                }
            }
        }
    }
    macro_edges.sort_unstable();

    Some(BipartitePartition {
        super_hubs,
        clusters,
        owner,
        boundary_ports,
        connectors,
        macro_edges,
    })
}
