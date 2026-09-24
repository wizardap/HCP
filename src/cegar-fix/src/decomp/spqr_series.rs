use crate::core::graph::Graph;
use std::collections::{HashMap, HashSet, VecDeque};

pub struct SeriesContraction {
    pub contracted_g: Graph,
    pub chain_map: HashMap<(i32, i32), Vec<i32>>,
    pub contracted_count: usize,
}

fn get_chain_between(
    chain_map: &HashMap<(i32, i32), Vec<i32>>,
    from: i32,
    to: i32,
) -> Vec<i32> {
    let key = (from.min(to), from.max(to));
    if let Some(nodes) = chain_map.get(&key) {
        if from < to {
            nodes.clone()
        } else {
            nodes.iter().rev().copied().collect()
        }
    } else {
        Vec::new()
    }
}

fn store_chain(
    chain_map: &mut HashMap<(i32, i32), Vec<i32>>,
    from: i32,
    to: i32,
    mut intermediates: Vec<i32>,
) {
    let key = (from.min(to), from.max(to));
    if from > to {
        intermediates.reverse();
    }
    chain_map.insert(key, intermediates);
}

fn path_intermediates(
    path: &[i32],
    chain_map: &HashMap<(i32, i32), Vec<i32>>,
) -> Vec<i32> {
    let mut res = Vec::new();
    for i in 0..path.len() - 1 {
        if i > 0 {
            res.push(path[i]);
        }
        res.extend(get_chain_between(chain_map, path[i], path[i + 1]));
    }
    res
}

pub fn contract_series_chains(g: &Graph) -> SeriesContraction {
    let mut adj: HashMap<i32, HashSet<i32>> = HashMap::new();
    for (&u, nbrs) in &g.adjacency_list {
        adj.insert(u, nbrs.iter().copied().collect());
    }

    let mut chain_map: HashMap<(i32, i32), Vec<i32>> = HashMap::new();
    let mut removed = HashSet::new();

    loop {
        let deg2_set: HashSet<i32> = adj
            .iter()
            .filter(|(&v, nbrs)| !removed.contains(&v) && nbrs.len() == 2)
            .map(|(&v, _)| v)
            .collect();

        if deg2_set.is_empty() {
            break;
        }

        // Decompose deg2_set into connected components in the induced subgraph G[deg2_set]
        let mut visited_comp = HashSet::new();
        let mut components = Vec::new();
        for &v in &deg2_set {
            if visited_comp.contains(&v) {
                continue;
            }
            let mut comp = Vec::new();
            let mut q = VecDeque::new();
            q.push_back(v);
            visited_comp.insert(v);
            while let Some(node) = q.pop_front() {
                comp.push(node);
                if let Some(nbrs) = adj.get(&node) {
                    for &nbr in nbrs {
                        if deg2_set.contains(&nbr) && visited_comp.insert(nbr) {
                            q.push_back(nbr);
                        }
                    }
                }
            }
            components.push(comp);
        }

        let mut contracted_any = false;

        for comp in components {
            // Check if this component has already been touched in this round
            if comp.iter().any(|v| removed.contains(v)) {
                continue;
            }

            // Check if component is an isolated cycle
            let is_cycle = comp.iter().all(|&x| {
                adj.get(&x)
                    .map(|nbrs| nbrs.iter().filter(|n| deg2_set.contains(n)).count() == 2)
                    .unwrap_or(false)
            });

            if is_cycle {
                let n = comp.len();
                if n <= 3 {
                    // Cannot contract cycle below 3 vertices
                    continue;
                }

                // Traverse cycle in order
                let start = comp[0];
                let mut cycle_nodes = Vec::with_capacity(n);
                let mut prev = -1;
                let mut curr = start;
                while cycle_nodes.len() < n {
                    cycle_nodes.push(curr);
                    let nbrs: Vec<i32> = adj[&curr]
                        .iter()
                        .filter(|n| deg2_set.contains(n))
                        .copied()
                        .collect();
                    let next = if nbrs[0] == prev { nbrs[1] } else { nbrs[0] };
                    prev = curr;
                    curr = next;
                }

                let a = n / 3;
                let b = (2 * n) / 3;
                let p0 = cycle_nodes[0];
                let p1 = cycle_nodes[a];
                let p2 = cycle_nodes[b];

                let inter1 = path_intermediates(&cycle_nodes[0..=a], &chain_map);
                let inter2 = path_intermediates(&cycle_nodes[a..=b], &chain_map);
                let mut path3 = cycle_nodes[b..n].to_vec();
                path3.push(p0);
                let inter3 = path_intermediates(&path3, &chain_map);

                // Clean up any old subchains between cycle nodes
                for i in 0..n {
                    let u = cycle_nodes[i];
                    let v = cycle_nodes[(i + 1) % n];
                    chain_map.remove(&(u.min(v), u.max(v)));
                }

                store_chain(&mut chain_map, p0, p1, inter1);
                store_chain(&mut chain_map, p1, p2, inter2);
                store_chain(&mut chain_map, p2, p0, inter3);

                // Mark intermediate nodes as removed
                for &v in &cycle_nodes {
                    if v != p0 && v != p1 && v != p2 {
                        removed.insert(v);
                        adj.remove(&v);
                    }
                }

                // Update adjacency for preserved vertices p0, p1, p2
                adj.get_mut(&p0).unwrap().retain(|n| !removed.contains(n));
                adj.get_mut(&p1).unwrap().retain(|n| !removed.contains(n));
                adj.get_mut(&p2).unwrap().retain(|n| !removed.contains(n));

                adj.get_mut(&p0).unwrap().insert(p1);
                adj.get_mut(&p1).unwrap().insert(p0);
                adj.get_mut(&p1).unwrap().insert(p2);
                adj.get_mut(&p2).unwrap().insert(p1);
                adj.get_mut(&p2).unwrap().insert(p0);
                adj.get_mut(&p0).unwrap().insert(p2);

                contracted_any = true;
            } else {
                // Component is a path of degree-2 vertices
                if comp.len() == 1 {
                    let c = comp[0];
                    let nbrs: Vec<i32> = adj[&c].iter().copied().collect();
                    if nbrs.len() != 2 {
                        continue;
                    }
                    let (u, w) = (nbrs[0], nbrs[1]);
                    if u == w || u == c || w == c {
                        // Guard against self-loops
                        continue;
                    }

                    let path = [u, c, w];
                    let inter = path_intermediates(&path, &chain_map);

                    chain_map.remove(&(u.min(c), u.max(c)));
                    chain_map.remove(&(c.min(w), c.max(w)));
                    store_chain(&mut chain_map, u, w, inter);

                    removed.insert(c);
                    adj.get_mut(&u).unwrap().remove(&c);
                    adj.get_mut(&w).unwrap().remove(&c);
                    adj.get_mut(&u).unwrap().insert(w);
                    adj.get_mut(&w).unwrap().insert(u);
                    adj.remove(&c);

                    contracted_any = true;
                } else {
                    // Path of length >= 2
                    let endpoints: Vec<i32> = comp
                        .iter()
                        .filter(|&&x| {
                            adj[&x]
                                .iter()
                                .filter(|nbr| deg2_set.contains(nbr))
                                .count()
                                == 1
                        })
                        .copied()
                        .collect();

                    if endpoints.len() != 2 {
                        continue;
                    }

                    let c_first = endpoints[0];
                    let c_last = endpoints[1];

                    let mut path_nodes = Vec::with_capacity(comp.len());
                    let mut prev = -1;
                    let mut curr = c_first;
                    while path_nodes.len() < comp.len() {
                        path_nodes.push(curr);
                        if curr == c_last {
                            break;
                        }
                        let nbrs: Vec<i32> = adj[&curr]
                            .iter()
                            .filter(|nbr| deg2_set.contains(nbr))
                            .copied()
                            .collect();
                        let next = if nbrs[0] == prev { nbrs[1] } else { nbrs[0] };
                        prev = curr;
                        curr = next;
                    }

                    let u_opt = adj[&c_first]
                        .iter()
                        .find(|nbr| !deg2_set.contains(nbr))
                        .copied();
                    let w_opt = adj[&c_last]
                        .iter()
                        .find(|nbr| !deg2_set.contains(nbr))
                        .copied();

                    let (u, w) = match (u_opt, w_opt) {
                        (Some(u), Some(w)) => (u, w),
                        _ => continue,
                    };

                    if u == w {
                        // Guard against self-loops
                        continue;
                    }

                    let mut full_path = Vec::with_capacity(path_nodes.len() + 2);
                    full_path.push(u);
                    full_path.extend(&path_nodes);
                    full_path.push(w);

                    let inter = path_intermediates(&full_path, &chain_map);

                    for i in 0..full_path.len() - 1 {
                        let a = full_path[i];
                        let b = full_path[i + 1];
                        chain_map.remove(&(a.min(b), a.max(b)));
                    }
                    store_chain(&mut chain_map, u, w, inter);

                    for &v in &path_nodes {
                        removed.insert(v);
                        adj.remove(&v);
                    }

                    adj.get_mut(&u).unwrap().remove(&c_first);
                    adj.get_mut(&w).unwrap().remove(&c_last);
                    adj.get_mut(&u).unwrap().insert(w);
                    adj.get_mut(&w).unwrap().insert(u);

                    contracted_any = true;
                }
            }
        }

        if !contracted_any {
            break;
        }
    }

    let mut contracted_g = Graph::new();
    for &u in adj.keys() {
        if !removed.contains(&u) {
            contracted_g
                .adjacency_list
                .entry(u)
                .or_default();
            contracted_g
                .adjacency_list_btree
                .entry(u)
                .or_default();
        }
    }
    for (&u, nbrs) in &adj {
        if !removed.contains(&u) {
            for &v in nbrs {
                if !removed.contains(&v) && u < v {
                    contracted_g.add_edge(u, v);
                }
            }
        }
    }

    SeriesContraction {
        contracted_g,
        chain_map,
        contracted_count: removed.len(),
    }
}

pub fn expand_series_tour(tour: &[i32], chain_map: &HashMap<(i32, i32), Vec<i32>>) -> Vec<i32> {
    if tour.is_empty() {
        return Vec::new();
    }
    let mut expanded = Vec::with_capacity(tour.len() * 2);
    let n = tour.len();
    for i in 0..n {
        let u = tour[i];
        let v = tour[(i + 1) % n];
        expanded.push(u);

        let key = (u.min(v), u.max(v));
        if let Some(intermediates) = chain_map.get(&key) {
            if u < v {
                expanded.extend(intermediates.iter().copied());
            } else {
                expanded.extend(intermediates.iter().rev().copied());
            }
        }
    }
    expanded
}
