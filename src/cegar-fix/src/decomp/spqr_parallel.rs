use crate::core::graph::Graph;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeparationPair {
    pub u: i32,
    pub v: i32,
    pub components: Vec<Vec<i32>>,
}

/// Finds all 2-cut separation pairs {u, v} that partition G into >= 2 components
/// where each component has at least one edge connecting to u and at least one edge connecting to v.
pub fn find_separation_pairs(g: &Graph) -> Vec<SeparationPair> {
    let mut nodes: Vec<i32> = g.adjacency_list.keys().copied().collect();
    nodes.sort();
    let n = nodes.len();
    if n < 4 {
        return Vec::new();
    }

    let node_to_idx: HashMap<i32, usize> = nodes.iter().enumerate().map(|(i, &v)| (v, i)).collect();
    let adj_idx: Vec<Vec<usize>> = nodes
        .iter()
        .map(|&u| {
            g.adjacency_list
                .get(&u)
                .map(|nbrs| {
                    nbrs.iter()
                        .filter_map(|&v| node_to_idx.get(&v).copied())
                        .collect()
                })
                .unwrap_or_default()
        })
        .collect();

    let mut tag = vec![0u32; n];
    let mut cur_tag = 0u32;
    let mut results = Vec::new();

    for i in 0..n {
        for j in (i + 1)..n {
            cur_tag += 1;
            if cur_tag == u32::MAX {
                tag.fill(0);
                cur_tag = 1;
            }

            tag[i] = cur_tag;
            tag[j] = cur_tag;

            let mut components = Vec::new();
            for start_idx in 0..n {
                if tag[start_idx] == cur_tag {
                    continue;
                }
                let mut comp = Vec::new();
                let mut q = VecDeque::new();
                tag[start_idx] = cur_tag;
                q.push_back(start_idx);

                while let Some(curr) = q.pop_front() {
                    comp.push(nodes[curr]);
                    for &nxt in &adj_idx[curr] {
                        if tag[nxt] != cur_tag {
                            tag[nxt] = cur_tag;
                            q.push_back(nxt);
                        }
                    }
                }
                comp.sort();
                components.push(comp);
            }

            if components.len() >= 2 {
                // Verify each component has edges to both u and v
                let valid = components.iter().all(|comp| {
                    let connects_u = comp.iter().any(|&c| {
                        let c_idx = node_to_idx[&c];
                        adj_idx[c_idx].contains(&i)
                    });
                    let connects_v = comp.iter().any(|&c| {
                        let c_idx = node_to_idx[&c];
                        adj_idx[c_idx].contains(&j)
                    });
                    connects_u && connects_v
                });

                if valid {
                    components.sort();
                    results.push(SeparationPair {
                        u: nodes[i],
                        v: nodes[j],
                        components,
                    });
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
