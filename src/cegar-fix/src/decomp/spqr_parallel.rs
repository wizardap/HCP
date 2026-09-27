use crate::core::graph::Graph;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeparationPair {
    pub u: i32,
    pub v: i32,
    pub components: Vec<Vec<i32>>,
}

/// Finds 2-cut separation pairs {u, v} that partition G into >= 2 components
/// where each component connects to both u and v.
///
/// Uses Hopcroft-Tarjan articulation point DFS on G \ {u} in O(V + E) time per candidate.
pub fn find_separation_pairs(g: &Graph) -> Vec<SeparationPair> {
    let mut nodes: Vec<i32> = g.adjacency_list.keys().copied().collect();
    nodes.sort_unstable();
    let n = nodes.len();
    if n < 4 {
        return Vec::new();
    }

    let node_to_idx: HashMap<i32, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, &node)| (node, i))
        .collect();

    let adj: Vec<Vec<usize>> = nodes
        .iter()
        .map(|&u| {
            g.adjacency_list
                .get(&u)
                .map(|nbrs| {
                    nbrs.iter()
                        .filter_map(|nbr| node_to_idx.get(nbr).copied())
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect();

    let mut results = Vec::new();
    let min_comp_size = 2;

    for u in 0..n {
        if results.len() >= 20 {
            break;
        }
        // Run Tarjan articulation point DFS on G \ {u}
        let start = if u == 0 { 1 } else { 0 };
        let mut tin = vec![-1i32; n];
        let mut low = vec![-1i32; n];
        let mut timer = 0;
        let mut art_pts = HashSet::new();

        tin[u] = i32::MAX;
        tin[start] = 0;
        low[start] = 0;

        let mut stack = Vec::with_capacity(n);
        stack.push((start, None::<usize>, 0usize));
        let mut root_children = 0;

        while let Some((curr, parent, edge_idx)) = stack.last_mut() {
            let c = *curr;
            let p = *parent;
            if *edge_idx < adj[c].len() {
                let to = adj[c][*edge_idx];
                *edge_idx += 1;
                if to == u {
                    continue;
                }
                if Some(to) == p {
                    continue;
                }
                if tin[to] != -1 {
                    low[c] = low[c].min(tin[to]);
                } else {
                    timer += 1;
                    tin[to] = timer;
                    low[to] = timer;
                    if p.is_none() {
                        root_children += 1;
                    }
                    stack.push((to, Some(c), 0usize));
                }
            } else {
                stack.pop();
                if let Some((prev, _, _)) = stack.last() {
                    let prev_node = *prev;
                    low[prev_node] = low[prev_node].min(low[c]);
                    if p.is_some() && low[c] >= tin[prev_node] {
                        art_pts.insert(prev_node);
                    }
                }
            }
        }
        if root_children > 1 {
            art_pts.insert(start);
        }
        if timer < n as i32 - 2 {
            continue;
        }

        // For each articulation point v, test if {u, v} is a valid 2-cut
        for &v in &art_pts {
            if u >= v {
                continue;
            }
            let (u_min, v_min) = (u, v);
            // BFS to find components in G \ {u, v}
            let mut comp_tag = vec![false; n];
            comp_tag[u_min] = true;
            comp_tag[v_min] = true;
            let mut components = Vec::new();

            for start_node in 0..n {
                if comp_tag[start_node] {
                    continue;
                }
                let mut comp = Vec::new();
                let mut q = VecDeque::new();
                comp_tag[start_node] = true;
                q.push_back(start_node);

                while let Some(cur) = q.pop_front() {
                    comp.push(nodes[cur]);
                    for &nxt in &adj[cur] {
                        if !comp_tag[nxt] {
                            comp_tag[nxt] = true;
                            q.push_back(nxt);
                        }
                    }
                }
                comp.sort_unstable();
                components.push(comp);
            }

            if components.len() >= 2 && components.iter().all(|c| c.len() >= min_comp_size) {
                // Verify each component connects to both u and v
                let valid = components.iter().all(|c_nodes| {
                    let has_u = c_nodes.iter().any(|&node| {
                        let c_idx = node_to_idx[&node];
                        adj[c_idx].contains(&u_min)
                    });
                    let has_v = c_nodes.iter().any(|&node| {
                        let c_idx = node_to_idx[&node];
                        adj[c_idx].contains(&v_min)
                    });
                    has_u && has_v
                });

                if valid {
                    components.sort();
                    let pair = SeparationPair {
                        u: nodes[u_min],
                        v: nodes[v_min],
                        components,
                    };
                    if !results.contains(&pair) {
                        results.push(pair);
                    }
                    if results.len() >= 20 {
                        return results;
                    }
                }
            }
        }
    }

    results
}

/// Extracts the subgraph induced by `comp ∪ {port_u, port_v}`.
pub fn extract_subcomponent_graph(g: &Graph, comp: &[i32], port_u: i32, port_v: i32) -> Graph {
    let mut node_set: HashSet<i32> = comp.iter().copied().collect();
    node_set.insert(port_u);
    node_set.insert(port_v);

    let mut sub_g = Graph::new();
    for &u in &node_set {
        sub_g.adjacency_list.entry(u).or_default();
        sub_g.adjacency_list_btree.entry(u).or_default();
    }

    for &u in &node_set {
        if let Some(nbrs) = g.adjacency_list.get(&u) {
            for &v in nbrs {
                if u < v && node_set.contains(&v) {
                    sub_g.add_edge(u, v);
                }
            }
        }
    }

    sub_g
}
