use std::collections::{HashMap, HashSet};
use cegar_fix::graph::Graph;
use cegar_fix::twin_giant_splicer::TwinGiantSplicer;
use rustsat::types::{Clause, Lit};

fn verify_valid_cycle(cycle: &[i32], g: &Graph, expected_nodes: &[i32]) {
    assert_eq!(cycle.len(), expected_nodes.len(), "Cycle length mismatch");
    let mut seen = HashSet::new();
    for &v in cycle {
        assert!(seen.insert(v), "Duplicate vertex {} in cycle", v);
    }
    for &expected in expected_nodes {
        assert!(seen.contains(&expected), "Missing expected vertex {}", expected);
    }
    let n = cycle.len();
    for i in 0..n {
        let u = cycle[i];
        let v = cycle[(i + 1) % n];
        let neighbors = g.adjacency_list.get(&u).expect("Vertex must exist in graph");
        assert!(
            neighbors.contains(&v),
            "Edge ({}, {}) is not present in graph",
            u,
            v
        );
    }
}

#[test]
fn test_direct_twin_giant_2opt_splice() {
    let mut g = Graph::new();

    // Giant Cycle 1: 1..=50 (50 vertices)
    for i in 1..=50 {
        let next = if i == 50 { 1 } else { i + 1 };
        g.add_edge(i, next);
    }

    // Giant Cycle 2: 51..=100 (50 vertices)
    for i in 51..=100 {
        let next = if i == 100 { 51 } else { i + 1 };
        g.add_edge(i, next);
    }

    // Direct 2-opt cross edges: (1, 51) and (2, 52)
    // Replaces (1, 2) in C1 and (51, 52) in C2
    g.add_edge(1, 51);
    g.add_edge(2, 52);

    let c1: Vec<i32> = (1..=50).collect();
    let c2: Vec<i32> = (51..=100).collect();
    let cycles = vec![c1, c2];

    let result = TwinGiantSplicer::try_splice_twin_giants(&cycles, &g, 100);
    assert!(result.is_some(), "Direct twin giant splice should succeed");

    let merged_cycles = result.unwrap();
    assert_eq!(merged_cycles.len(), 1, "Expected single merged giant cycle");

    let expected_nodes: Vec<i32> = (1..=100).collect();
    verify_valid_cycle(&merged_cycles[0], &g, &expected_nodes);
}

#[test]
fn test_direct_twin_giant_2opt_splice_case_b() {
    let mut g = Graph::new();

    // Giant Cycle 1: 1..=50
    for i in 1..=50 {
        let next = if i == 50 { 1 } else { i + 1 };
        g.add_edge(i, next);
    }

    // Giant Cycle 2: 51..=100
    for i in 51..=100 {
        let next = if i == 100 { 51 } else { i + 1 };
        g.add_edge(i, next);
    }

    // Case B cross edges: (1, 52) and (2, 51)
    g.add_edge(1, 52);
    g.add_edge(2, 51);

    let c1: Vec<i32> = (1..=50).collect();
    let c2: Vec<i32> = (51..=100).collect();
    let cycles = vec![c1, c2];

    let result = TwinGiantSplicer::try_splice_twin_giants(&cycles, &g, 100);
    assert!(result.is_some(), "Direct twin giant splice (Case B) should succeed");

    let merged_cycles = result.unwrap();
    assert_eq!(merged_cycles.len(), 1, "Expected single merged giant cycle");

    let expected_nodes: Vec<i32> = (1..=100).collect();
    verify_valid_cycle(&merged_cycles[0], &g, &expected_nodes);
}

#[test]
fn test_3way_intermediate_bridge_splice() {
    let mut g = Graph::new();

    // Giant Cycle 1: 1..=40 (40 vertices)
    for i in 1..=40 {
        let next = if i == 40 { 1 } else { i + 1 };
        g.add_edge(i, next);
    }

    // Giant Cycle 2: 41..=80 (40 vertices)
    for i in 41..=80 {
        let next = if i == 80 { 41 } else { i + 1 };
        g.add_edge(i, next);
    }

    // Intermediate Cycle 3: 81..=86 (6 vertices)
    for i in 81..=86 {
        let next = if i == 86 { 81 } else { i + 1 };
        g.add_edge(i, next);
    }

    // No direct cross edges between C1 and C2!
    // Cross edges between C1 and C3: (1, 81) and (2, 82) -> removes (1, 2) and (81, 82)
    g.add_edge(1, 81);
    g.add_edge(2, 82);

    // Cross edges between C3 and C2: (84, 41) and (85, 42) -> removes (84, 85) and (41, 42)
    g.add_edge(84, 41);
    g.add_edge(85, 42);

    let c1: Vec<i32> = (1..=40).collect();
    let c2: Vec<i32> = (41..=80).collect();
    let c3: Vec<i32> = (81..=86).collect();
    let cycles = vec![c1, c2, c3];

    let result = TwinGiantSplicer::try_splice_twin_giants(&cycles, &g, 86);
    assert!(result.is_some(), "3-way intermediate bridge splice should succeed");

    let merged_cycles = result.unwrap();
    assert_eq!(merged_cycles.len(), 1, "Expected single merged cycle from 3-way splice");

    let expected_nodes: Vec<i32> = (1..=86).collect();
    verify_valid_cycle(&merged_cycles[0], &g, &expected_nodes);
}

#[test]
fn test_3way_intermediate_with_remaining_cycles() {
    let mut g = Graph::new();

    // Giant Cycle 1: 1..=20 (20 vertices)
    for i in 1..=20 {
        let next = if i == 20 { 1 } else { i + 1 };
        g.add_edge(i, next);
    }

    // Giant Cycle 2: 21..=40 (20 vertices)
    for i in 21..=40 {
        let next = if i == 40 { 21 } else { i + 1 };
        g.add_edge(i, next);
    }

    // Intermediate Cycle 3: 41..=45 (5 vertices)
    for i in 41..=45 {
        let next = if i == 45 { 41 } else { i + 1 };
        g.add_edge(i, next);
    }

    // Disjoint Unrelated Cycle 4: 46..=50 (5 vertices)
    for i in 46..=50 {
        let next = if i == 50 { 46 } else { i + 1 };
        g.add_edge(i, next);
    }

    // Cross edges between C1 and C3
    g.add_edge(1, 41);
    g.add_edge(2, 42);

    // Cross edges between C3 and C2 (disjoint on C3: removing (43, 44))
    g.add_edge(43, 21);
    g.add_edge(44, 22);

    let c1: Vec<i32> = (1..=20).collect();
    let c2: Vec<i32> = (21..=40).collect();
    let c3: Vec<i32> = (41..=45).collect();
    let c4: Vec<i32> = (46..=50).collect();
    let cycles = vec![c1, c2, c3, c4];

    let result = TwinGiantSplicer::try_splice_twin_giants(&cycles, &g, 50);
    assert!(result.is_some(), "3-way splice with leftover cycle should succeed");

    let merged_cycles = result.unwrap();
    assert_eq!(merged_cycles.len(), 2, "Expected 2 cycles (1 merged giant of 45 nodes, 1 untouched cycle of 5 nodes)");

    let merged_nodes: Vec<i32> = (1..=45).collect();
    let untouched_nodes: Vec<i32> = (46..=50).collect();

    // Verify one cycle is the 45-node merged cycle and the other is the untouched 5-node cycle
    if merged_cycles[0].len() == 45 {
        verify_valid_cycle(&merged_cycles[0], &g, &merged_nodes);
        verify_valid_cycle(&merged_cycles[1], &g, &untouched_nodes);
    } else {
        verify_valid_cycle(&merged_cycles[0], &g, &untouched_nodes);
        verify_valid_cycle(&merged_cycles[1], &g, &merged_nodes);
    }
}

#[test]
fn test_threshold_rejection() {
    let mut g = Graph::new();

    // Two small cycles of 5 vertices
    for i in 1..=5 {
        let next = if i == 5 { 1 } else { i + 1 };
        g.add_edge(i, next);
    }
    for i in 6..=10 {
        let next = if i == 10 { 6 } else { i + 1 };
        g.add_edge(i, next);
    }

    g.add_edge(1, 6);
    g.add_edge(2, 7);

    let cycles = vec![vec![1, 2, 3, 4, 5], vec![6, 7, 8, 9, 10]];

    // total_v = 100 -> threshold = max(10, 20) = 20 > 5 -> must return None
    let result = TwinGiantSplicer::try_splice_twin_giants(&cycles, &g, 100);
    assert!(result.is_none(), "Should reject cycles smaller than threshold");

    // total_v = 10 -> threshold = max(10, 2) = 10 > 5 -> must return None
    let result = TwinGiantSplicer::try_splice_twin_giants(&cycles, &g, 10);
    assert!(result.is_none(), "Should reject cycles smaller than min threshold 10");
}

#[test]
fn test_bicomponent_cut_clauses() {
    let mut g = Graph::new();

    // Cycle 1: 1..=15
    for i in 1..=15 {
        let next = if i == 15 { 1 } else { i + 1 };
        g.add_edge(i, next);
    }

    // Cycle 2: 16..=30
    for i in 16..=30 {
        let next = if i == 30 { 16 } else { i + 1 };
        g.add_edge(i, next);
    }

    // Cross edges: (5, 20) and (10, 25)
    g.add_edge(5, 20);
    g.add_edge(10, 25);

    let c1: Vec<i32> = (1..=15).collect();
    let c2: Vec<i32> = (16..=30).collect();

    // Mock graph_lit_map
    let mut graph_lit_map = HashMap::new();
    let lit_5_20 = Lit::positive(1);
    let lit_20_5 = Lit::positive(2);
    let lit_10_25 = Lit::positive(3);
    let lit_25_10 = Lit::positive(4);

    graph_lit_map.insert((5, 20), lit_5_20);
    graph_lit_map.insert((20, 5), lit_20_5);
    graph_lit_map.insert((10, 25), lit_10_25);
    graph_lit_map.insert((25, 10), lit_25_10);

    let clauses = TwinGiantSplicer::generate_bicomponent_cut_clauses(&c1, &c2, &g, &graph_lit_map);

    assert_eq!(clauses.len(), 2, "Expected 2 cut clauses: delta+(C1 -> C2) and delta+(C2 -> C1)");

    let expected_c1_to_c2 = {
        let mut v = vec![lit_5_20, lit_10_25];
        v.sort_unstable();
        v.dedup();
        Clause::from_iter(v)
    };

    let expected_c2_to_c1 = {
        let mut v = vec![lit_20_5, lit_25_10];
        v.sort_unstable();
        v.dedup();
        Clause::from_iter(v)
    };

    assert!(clauses.contains(&expected_c1_to_c2), "Missing delta+(C1 -> C2) clause");
    assert!(clauses.contains(&expected_c2_to_c1), "Missing delta+(C2 -> C1) clause");
}
