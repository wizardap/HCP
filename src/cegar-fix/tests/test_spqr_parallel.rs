use cegar_fix::core::graph::Graph;
use cegar_fix::decomp::spqr_parallel::{extract_subcomponent_graph, find_separation_pairs};

#[test]
fn test_find_separation_pair_two_blocks() {
    let mut g = Graph::new();
    // Block A: vertices {1, 2, 3, 4} with ports 1, 2
    g.add_edge(1, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 2);
    g.add_edge(1, 4);
    // Block B: vertices {1, 2, 5, 6} with ports 1, 2
    g.add_edge(1, 5);
    g.add_edge(5, 6);
    g.add_edge(6, 2);
    g.add_edge(2, 5);

    let pairs = find_separation_pairs(&g);
    assert!(!pairs.is_empty(), "Must find separation pair {{1, 2}}");
    let sep = pairs
        .iter()
        .find(|p| (p.u == 1 && p.v == 2) || (p.u == 2 && p.v == 1))
        .unwrap();
    assert_eq!(
        sep.components.len(),
        2,
        "Must partition into 2 disjoint components"
    );

    let sub_g = extract_subcomponent_graph(&g, &sep.components[0], sep.u, sep.v);
    assert!(sub_g.adjacency_list.contains_key(&sep.u));
    assert!(sub_g.adjacency_list.contains_key(&sep.v));
}

#[test]
fn test_find_separation_pairs_k5_none() {
    let mut g = Graph::new();
    for i in 1..=5 {
        for j in (i + 1)..=5 {
            g.add_edge(i, j);
        }
    }
    let pairs = find_separation_pairs(&g);
    assert!(
        pairs.is_empty(),
        "3-connected K5 has no 2-cut separation pairs"
    );
}

#[test]
fn test_find_separation_pairs_c6() {
    let mut g = Graph::new();
    for i in 1..=6 {
        let nxt = if i == 6 { 1 } else { i + 1 };
        g.add_edge(i, nxt);
    }
    let pairs = find_separation_pairs(&g);
    // For 6-cycle, opposite pairs {1,4}, {2,5}, {3,6} partition into two components of size 2
    // Pairs at distance 2 (like {1, 3}) have one isolated vertex {2} and one component {4, 5, 6}
    // Each component connects to both cut vertices.
    assert!(!pairs.is_empty());
    let op = pairs
        .iter()
        .find(|p| (p.u == 1 && p.v == 4) || (p.u == 4 && p.v == 1));
    assert!(op.is_some());
    let sep = op.unwrap();
    assert_eq!(sep.components.len(), 2);
}

#[test]
fn test_extract_subcomponent_graph_structure() {
    let mut g = Graph::new();
    // Block A: 1-3, 3-4, 4-2, 1-4
    g.add_edge(1, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 2);
    g.add_edge(1, 4);
    // Block B: 1-5, 5-6, 6-2
    g.add_edge(1, 5);
    g.add_edge(5, 6);
    g.add_edge(6, 2);

    let comp = vec![3, 4];
    let sub = extract_subcomponent_graph(&g, &comp, 1, 2);
    assert_eq!(sub.adjacency_list.len(), 4);
    assert!(sub.adjacency_list.get(&1).unwrap().contains(&3));
    assert!(sub.adjacency_list.get(&1).unwrap().contains(&4));
    assert!(sub.adjacency_list.get(&3).unwrap().contains(&4));
    assert!(sub.adjacency_list.get(&4).unwrap().contains(&2));
    // No edges from Block B
    assert!(!sub.adjacency_list.contains_key(&5));
    assert!(!sub.adjacency_list.contains_key(&6));
}
