use cegar_fix::core::graph::Graph;
use cegar_fix::core::tour_verifier::TourVerifier;
use cegar_fix::decomp::spqr_series::{contract_series_chains, expand_series_tour};

#[test]
fn test_contract_and_expand_cycle_10() {
    let mut g = Graph::new();
    let n = 10;
    for i in 1..=n {
        let nxt = if i == n { 1 } else { i + 1 };
        g.add_edge(i, nxt);
    }

    let sc = contract_series_chains(&g);
    assert!(sc.contracted_count > 0);

    let skeleton_tour = sc.contracted_g.adjacency_list.keys().copied().collect::<Vec<_>>();
    let full_tour = expand_series_tour(&skeleton_tour, &sc.chain_map);

    let (valid, err) = TourVerifier::verify(&g, &full_tour);
    assert!(valid, "Expanded tour must be certified: {}", err);
}

#[test]
fn test_contract_theta_graph() {
    let mut g = Graph::new();
    // Theta graph: poles 1 and 2 connected by 3 disjoint paths:
    // Path 1: 1 - 3 - 4 - 2
    // Path 2: 1 - 5 - 6 - 2
    // Path 3: 1 - 7 - 8 - 2
    g.add_edge(1, 3); g.add_edge(3, 4); g.add_edge(4, 2);
    g.add_edge(1, 5); g.add_edge(5, 6); g.add_edge(6, 2);
    g.add_edge(1, 7); g.add_edge(7, 8); g.add_edge(8, 2);

    let sc = contract_series_chains(&g);
    assert_eq!(sc.contracted_count, 6);
    assert_eq!(sc.contracted_g.adjacency_list.len(), 2);
}

#[test]
fn test_no_degree_2() {
    let mut g = Graph::new();
    // Complete graph K4
    for u in 1..=4 {
        for v in (u + 1)..=4 {
            g.add_edge(u, v);
        }
    }
    let sc = contract_series_chains(&g);
    assert_eq!(sc.contracted_count, 0);
    assert_eq!(sc.contracted_g.adjacency_list.len(), 4);
}

#[test]
fn test_triangle_k3() {
    let mut g = Graph::new();
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 1);
    let sc = contract_series_chains(&g);
    // Cycle of 3 is already minimal, no contraction
    assert_eq!(sc.contracted_count, 0);
    assert_eq!(sc.contracted_g.adjacency_list.len(), 3);
}

#[test]
fn test_cycle_4_and_5_both_directions() {
    for n in [4, 5, 20] {
        let mut g = Graph::new();
        for i in 1..=n {
            let nxt = if i == n { 1 } else { i + 1 };
            g.add_edge(i, nxt);
        }
        let sc = contract_series_chains(&g);
        assert_eq!(sc.contracted_count, (n - 3) as usize);
        assert_eq!(sc.contracted_g.adjacency_list.len(), 3);

        let mut skeleton = sc.contracted_g.adjacency_list.keys().copied().collect::<Vec<_>>();
        let tour_fwd = expand_series_tour(&skeleton, &sc.chain_map);
        let (valid1, err1) = TourVerifier::verify(&g, &tour_fwd);
        assert!(valid1, "Forward expanded tour invalid for n={}: {}", n, err1);

        skeleton.reverse();
        let tour_rev = expand_series_tour(&skeleton, &sc.chain_map);
        let (valid2, err2) = TourVerifier::verify(&g, &tour_rev);
        assert!(valid2, "Reverse expanded tour invalid for n={}: {}", n, err2);
    }
}

#[test]
fn test_self_loop_guard() {
    let mut g = Graph::new();
    // Node 1 connected to a degree-2 loop: 1 - 2 - 3 - 1
    // and an external node 4: 1 - 4
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 1);
    g.add_edge(1, 4);

    let sc = contract_series_chains(&g);
    // 2 and 3 form a chain between 1 and 1. Contraction must NOT create self-loop (1, 1).
    assert_eq!(sc.contracted_count, 0);
}

#[test]
fn test_bridge_chain_between_cliques() {
    let mut g = Graph::new();
    // Clique 1: 1, 2, 3, 4 (K4)
    for u in 1..=4 {
        for v in (u + 1)..=4 {
            g.add_edge(u, v);
        }
    }
    // Clique 2: 10, 11, 12, 13 (K4)
    for u in 10..=13 {
        for v in (u + 1)..=13 {
            g.add_edge(u, v);
        }
    }
    // Degree-2 chain connecting node 4 to node 10: 4 - 5 - 6 - 7 - 10
    g.add_edge(4, 5);
    g.add_edge(5, 6);
    g.add_edge(6, 7);
    g.add_edge(7, 10);

    let sc = contract_series_chains(&g);
    // 5, 6, 7 must be contracted
    assert_eq!(sc.contracted_count, 3);
    // contracted edge (4, 10) must exist
    assert!(sc.contracted_g.adjacency_list[&4].contains(&10));
    assert!(sc.contracted_g.adjacency_list[&10].contains(&4));

    let key = (4, 10);
    assert_eq!(sc.chain_map.get(&key), Some(&vec![5, 6, 7]));
}
