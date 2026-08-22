use crate::file_operations::input_to_graph;
use crate::graph::Graph;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone)]
pub struct DecompositionResult {
    pub s_hubs: Vec<i32>,
    pub b_hubs: Vec<i32>,
    pub m_hubs: Vec<i32>,
    pub all_hubs: HashSet<i32>,
    pub hh_edges: Vec<(i32, i32)>,
    pub strips: Vec<Vec<i32>>,
    pub strip_adj_hubs: HashMap<usize, HashSet<i32>>,
    pub hub_adj_strips: HashMap<i32, HashSet<usize>>,
}

/// Decomposes the given graph G into a two-tier hierarchy of Hubs (S, B, M) and Non-Hub Strips.
pub fn decompose_graph(g: &Graph) -> DecompositionResult {
    let mut s_hubs = Vec::new();
    let mut b_hubs = Vec::new();
    let mut m_hubs = Vec::new();
    let mut all_hubs = HashSet::new();

    // 1. Classify hubs based on degree in G
    for (&v, neighbors) in &g.adjacency_list {
        let degree = neighbors.len();
        if degree >= 500 {
            s_hubs.push(v);
            all_hubs.insert(v);
        } else if degree >= 100 {
            b_hubs.push(v);
            all_hubs.insert(v);
        } else if degree >= 20 {
            m_hubs.push(v);
            all_hubs.insert(v);
        }
    }

    s_hubs.sort_unstable();
    b_hubs.sort_unstable();
    m_hubs.sort_unstable();

    // 2. Extract Hub-Hub (HH) edges (u < v)
    let mut hh_set = HashSet::new();
    for &u in &all_hubs {
        if let Some(neighbors) = g.adjacency_list.get(&u) {
            for &v in neighbors {
                if all_hubs.contains(&v) {
                    let edge = if u < v { (u, v) } else { (v, u) };
                    hh_set.insert(edge);
                }
            }
        }
    }
    let mut hh_edges: Vec<(i32, i32)> = hh_set.into_iter().collect();
    hh_edges.sort_unstable();

    // 3. Extract Strips (connected components in G \ all_hubs)
    let mut non_hub_vertices: Vec<i32> = g
        .adjacency_list
        .keys()
        .copied()
        .filter(|v| !all_hubs.contains(v))
        .collect();
    non_hub_vertices.sort_unstable();

    let mut visited: HashSet<i32> = HashSet::new();
    let mut strips: Vec<Vec<i32>> = Vec::new();

    for &start_v in &non_hub_vertices {
        if visited.contains(&start_v) {
            continue;
        }

        let mut component = Vec::new();
        let mut queue = VecDeque::new();

        visited.insert(start_v);
        queue.push_back(start_v);

        while let Some(curr) = queue.pop_front() {
            component.push(curr);

            if let Some(neighbors) = g.adjacency_list.get(&curr) {
                for &next_v in neighbors {
                    if !all_hubs.contains(&next_v) && !visited.contains(&next_v) {
                        visited.insert(next_v);
                        queue.push_back(next_v);
                    }
                }
            }
        }

        component.sort_unstable();
        strips.push(component);
    }

    // Sort strips by discovery order (which is deterministic because non_hub_vertices is sorted)

    // 4. Build Strip-Hub and Hub-Strip bipartite adjacency maps
    let mut strip_adj_hubs: HashMap<usize, HashSet<i32>> = HashMap::new();
    let mut hub_adj_strips: HashMap<i32, HashSet<usize>> = HashMap::new();

    for &h in &all_hubs {
        hub_adj_strips.insert(h, HashSet::new());
    }

    for (si, strip) in strips.iter().enumerate() {
        let mut adj_hubs = HashSet::new();
        for &v in strip {
            if let Some(neighbors) = g.adjacency_list.get(&v) {
                for &nb in neighbors {
                    if all_hubs.contains(&nb) {
                        adj_hubs.insert(nb);
                        hub_adj_strips.entry(nb).or_default().insert(si);
                    }
                }
            }
        }
        strip_adj_hubs.insert(si, adj_hubs);
    }

    DecompositionResult {
        s_hubs,
        b_hubs,
        m_hubs,
        all_hubs,
        hh_edges,
        strips,
        strip_adj_hubs,
        hub_adj_strips,
    }
}

/// Helper function to load a graph from a file and decompose it.
pub fn decompose_from_file(filename: &str) -> DecompositionResult {
    let g = input_to_graph(filename);
    decompose_graph(&g)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synthetic_small_graph() {
        let mut g = Graph::new();
        // Hub 1: degree 25 -> M-Hub
        // Non-hub strip 1: {101, 102} connected to each other and to hub 1
        // Non-hub strip 2: {201, 202, 203} connected as path and to hub 1
        for i in 2..=24 {
            g.add_edge(1, i);
        }
        g.add_edge(1, 101); // 24th neighbor
        g.add_edge(1, 201); // 25th neighbor -> degree 25 (M-hub)

        // Strip 1: 101-102
        g.add_edge(101, 102);

        // Strip 2: 201-202-203
        g.add_edge(201, 202);
        g.add_edge(202, 203);

        let decomp = decompose_graph(&g);
        assert_eq!(decomp.m_hubs, vec![1]);
        assert_eq!(decomp.s_hubs.len(), 0);
        assert_eq!(decomp.b_hubs.len(), 0);
        assert_eq!(decomp.all_hubs.len(), 1);
        assert!(decomp.all_hubs.contains(&1));

        // Hub-hub edges: none since only 1 hub
        assert_eq!(decomp.hh_edges.len(), 0);

        // Non-hubs: 2..=24 (each is singleton strip since no edges between them),
        // plus {101, 102}, plus {201, 202, 203}.
        // Check that {101, 102} and {201, 202, 203} are separate strips
        let strip_101 = decomp.strips.iter().find(|s| s.contains(&101)).unwrap();
        assert_eq!(strip_101, &vec![101, 102]);

        let strip_201 = decomp.strips.iter().find(|s| s.contains(&201)).unwrap();
        assert_eq!(strip_201, &vec![201, 202, 203]);

        // strip_adj_hubs must contain Hub 1 for both strips
        let si_101 = decomp.strips.iter().position(|s| s.contains(&101)).unwrap();
        assert!(decomp.strip_adj_hubs.get(&si_101).unwrap().contains(&1));

        let si_201 = decomp.strips.iter().position(|s| s.contains(&201)).unwrap();
        assert!(decomp.strip_adj_hubs.get(&si_201).unwrap().contains(&1));
    }

    #[test]
    fn test_hub_hub_edges() {
        let mut g = Graph::new();
        // Hub 1: degree 20 (M-hub)
        // Hub 2: degree 20 (M-hub)
        // Edge between 1 and 2
        for i in 10..29 {
            g.add_edge(1, i);
        }
        for i in 30..49 {
            g.add_edge(2, i);
        }
        g.add_edge(1, 2); // 1 has 20 edges, 2 has 20 edges

        let decomp = decompose_graph(&g);
        assert_eq!(decomp.m_hubs, vec![1, 2]);
        assert_eq!(decomp.all_hubs.len(), 2);
        assert_eq!(decomp.hh_edges, vec![(1, 2)]);
    }
}

