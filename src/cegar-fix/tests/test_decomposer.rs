use cegar_fix::file_operations::input_to_graph;
use cegar_fix::two_tier_decomposer::{decompose_graph, DecompositionResult};
use std::collections::HashSet;
use std::path::Path;

#[test]
fn test_graph950_decomposition() {
    let candidate_paths = [
        "../../FHCPCS-col/graph950.col",
        "../FHCPCS-col/graph950.col",
        "/home/ubuntu/HCP/FHCPCS-col/graph950.col",
    ];

    let path_str = candidate_paths
        .iter()
        .find(|p| Path::new(p).exists())
        .expect("graph950.col file must exist");

    let g = input_to_graph(path_str);
    let n = g.adjacency_list.len();
    assert_eq!(n, 6620, "graph950 must have exactly 6620 vertices");

    let decomp: DecompositionResult = decompose_graph(&g);

    // 1. Check Hub classifications
    assert_eq!(decomp.s_hubs.len(), 10, "Expected exactly 10 S-Hubs (degree >= 500)");
    assert_eq!(decomp.b_hubs.len(), 50, "Expected exactly 50 B-Hubs (degree 100..499)");
    assert_eq!(decomp.m_hubs.len(), 250, "Expected exactly 250 M-Hubs (degree 20..99)");
    assert_eq!(decomp.all_hubs.len(), 310, "Expected exactly 310 total Hubs");

    // Check union consistency
    let mut union_hubs = HashSet::new();
    for &h in &decomp.s_hubs {
        assert!(union_hubs.insert(h), "Duplicate S-hub: {}", h);
        let deg = g.adjacency_list.get(&h).map_or(0, |adj| adj.len());
        assert!(deg >= 500, "S-hub {} has degree {} < 500", h, deg);
    }
    for &h in &decomp.b_hubs {
        assert!(union_hubs.insert(h), "Duplicate B-hub: {}", h);
        let deg = g.adjacency_list.get(&h).map_or(0, |adj| adj.len());
        assert!((100..500).contains(&deg), "B-hub {} has degree {} not in 100..499", h, deg);
    }
    for &h in &decomp.m_hubs {
        assert!(union_hubs.insert(h), "Duplicate M-hub: {}", h);
        let deg = g.adjacency_list.get(&h).map_or(0, |adj| adj.len());
        assert!((20..100).contains(&deg), "M-hub {} has degree {} not in 20..99", h, deg);
    }
    assert_eq!(decomp.all_hubs, union_hubs, "all_hubs must match S + B + M hubs");

    // 2. Check HH Edges
    assert_eq!(decomp.hh_edges.len(), 650, "Expected exactly 650 HH edges");
    for &(u, v) in &decomp.hh_edges {
        assert!(u < v, "HH edge ({}, {}) must be sorted u < v", u, v);
        assert!(decomp.all_hubs.contains(&u), "Vertex {} in HH edge must be a hub", u);
        assert!(decomp.all_hubs.contains(&v), "Vertex {} in HH edge must be a hub", v);
        let u_adj = g.adjacency_list.get(&u).expect("u must be in graph");
        assert!(u_adj.contains(&v), "Edge ({}, {}) must exist in G", u, v);
    }

    // 3. Check Strips
    assert_eq!(decomp.strips.len(), 74, "Expected exactly 74 strips");
    let mut large_strips_count = 0;
    let mut small_strips_count = 0;
    let mut all_strip_vertices = HashSet::new();

    for (si, strip) in decomp.strips.iter().enumerate() {
        assert!(!strip.is_empty(), "Strip {} cannot be empty", si);
        match strip.len() {
            125 => large_strips_count += 1,
            2 | 3 => small_strips_count += 1,
            other => panic!("Unexpected strip {} length: {}", si, other),
        }
        for &v in strip {
            assert!(!decomp.all_hubs.contains(&v), "Strip vertex {} must not be a hub", v);
            assert!(all_strip_vertices.insert(v), "Vertex {} appears in multiple strips", v);
        }
    }
    assert_eq!(large_strips_count, 50, "Expected exactly 50 large strips of size 125");
    assert_eq!(small_strips_count, 24, "Expected exactly 24 small strips of size 2-3");

    // 4. Exact Partition check
    assert_eq!(
        all_strip_vertices.len() + decomp.all_hubs.len(),
        6620,
        "Hubs and strips must partition all 6620 vertices"
    );

    // 5. Adjacency consistency
    assert_eq!(decomp.strip_adj_hubs.len(), decomp.strips.len(), "strip_adj_hubs size mismatch");
    for (si, strip) in decomp.strips.iter().enumerate() {
        let adj_hubs = decomp.strip_adj_hubs.get(&si).expect("Strip must have entry in strip_adj_hubs");
        let mut expected_hubs = HashSet::new();
        for &v in strip {
            if let Some(neighbors) = g.adjacency_list.get(&v) {
                for &nb in neighbors {
                    if decomp.all_hubs.contains(&nb) {
                        expected_hubs.insert(nb);
                    }
                }
            }
        }
        assert_eq!(adj_hubs, &expected_hubs, "strip_adj_hubs[{}] mismatch with actual graph edges", si);

        // Check dual mapping in hub_adj_strips
        for &h in adj_hubs {
            let hub_strips = decomp.hub_adj_strips.get(&h).expect("Hub must have entry in hub_adj_strips");
            assert!(hub_strips.contains(&si), "hub_adj_strips[{}] must contain strip {}", h, si);
        }
    }

    // Check every hub's reported adjacent strips
    for (&h, strip_set) in &decomp.hub_adj_strips {
        assert!(decomp.all_hubs.contains(&h), "Key in hub_adj_strips must be a hub");
        for &si in strip_set {
            let strip_hubs = decomp.strip_adj_hubs.get(&si).expect("Strip must exist");
            assert!(strip_hubs.contains(&h), "strip_adj_hubs[{}] must contain hub {}", si, h);
        }
    }
}
