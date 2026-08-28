use std::collections::HashSet;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::encoder::Encoder;
use cegar_fix::graph::Graph;
use cegar_fix::hemisphere_splicer::HemisphereSplicer;

fn verify_valid_cycle(cycle: &[i32], g: &Graph, expected_nodes: &[i32]) {
    assert_eq!(cycle.len(), expected_nodes.len());
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
fn test_direct_2opt_hemisphere_splice() {
    let mut g = Graph::new();
    // Cycle 1: 1 - 2 - 3 - 4 - 5 - 1
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 5);
    g.add_edge(5, 1);

    // Cycle 2: 6 - 7 - 8 - 9 - 10 - 6
    g.add_edge(6, 7);
    g.add_edge(7, 8);
    g.add_edge(8, 9);
    g.add_edge(9, 10);
    g.add_edge(10, 6);

    // Cross edges: (1, 6) and (2, 7)
    g.add_edge(1, 6);
    g.add_edge(2, 7);

    let contractor = Degree2Contractor::new();
    let cycles = vec![vec![1, 2, 3, 4, 5], vec![6, 7, 8, 9, 10]];

    let result = HemisphereSplicer::try_direct_splice_all(&cycles, &g, &contractor);
    assert!(result.is_some(), "Direct splice should succeed");
    let merged_cycles = result.unwrap();
    assert_eq!(merged_cycles.len(), 1, "Expected single merged cycle");
    let expected_nodes: Vec<i32> = (1..=10).collect();
    verify_valid_cycle(&merged_cycles[0], &g, &expected_nodes);
}

#[test]
fn test_direct_2opt_hemisphere_splice_case_b() {
    let mut g = Graph::new();
    // Cycle 1: 1 - 2 - 3 - 4 - 5 - 1
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 5);
    g.add_edge(5, 1);

    // Cycle 2: 6 - 7 - 8 - 9 - 10 - 6
    g.add_edge(6, 7);
    g.add_edge(7, 8);
    g.add_edge(8, 9);
    g.add_edge(9, 10);
    g.add_edge(10, 6);

    // Case B Cross edges: (1, 7) and (2, 6)
    g.add_edge(1, 7);
    g.add_edge(2, 6);

    let contractor = Degree2Contractor::new();
    let cycles = vec![vec![1, 2, 3, 4, 5], vec![6, 7, 8, 9, 10]];

    let result = HemisphereSplicer::try_direct_splice_all(&cycles, &g, &contractor);
    assert!(result.is_some(), "Direct splice (Case B) should succeed");
    let merged_cycles = result.unwrap();
    assert_eq!(merged_cycles.len(), 1, "Expected single merged cycle");
    let expected_nodes: Vec<i32> = (1..=10).collect();
    verify_valid_cycle(&merged_cycles[0], &g, &expected_nodes);
}

#[test]
fn test_protected_contracted_chain_skipping() {
    let mut g = Graph::new();
    // Cycle 1: 1 - 2 - 3 - 4 - 5 - 1
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 5);
    g.add_edge(5, 1);

    // Cycle 2: 6 - 7 - 8 - 9 - 10 - 6
    g.add_edge(6, 7);
    g.add_edge(7, 8);
    g.add_edge(8, 9);
    g.add_edge(9, 10);
    g.add_edge(10, 6);

    // Cross edges connecting (1,2) and (6,7)
    g.add_edge(1, 6);
    g.add_edge(2, 7);

    // Protect edge (1, 2)
    let mut contractor = Degree2Contractor::new();
    contractor.chain_map.insert((1, 2), vec![100]);
    contractor.chain_map.insert((2, 1), vec![100]);

    let cycles = vec![vec![1, 2, 3, 4, 5], vec![6, 7, 8, 9, 10]];
    let result = HemisphereSplicer::try_direct_splice_all(&cycles, &g, &contractor);
    assert!(result.is_none(), "Splice should be prevented due to protected contracted edge (1, 2)");
}

#[test]
fn test_hemisphere_crossing_cuts_generation() {
    let mut g = Graph::new();
    // Cycle 1: 1 - 2 - 3 - 4 - 1
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 1);

    // Cycle 2: 5 - 6 - 7 - 8 - 5
    g.add_edge(5, 6);
    g.add_edge(6, 7);
    g.add_edge(7, 8);
    g.add_edge(8, 5);

    // Cross edges: (2, 5) and (3, 8)
    g.add_edge(2, 5);
    g.add_edge(3, 8);

    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let cycles = vec![vec![1, 2, 3, 4], vec![5, 6, 7, 8]];
    let cuts = HemisphereSplicer::generate_hemisphere_crossing_cuts(&cycles, &g, &encoder);

    // Each of the 2 cycles should generate 1 outgoing and 1 incoming clause = 4 clauses total
    assert_eq!(cuts.len(), 4, "Expected 4 crossing cut clauses (2 per hemisphere cycle)");
    for clause in &cuts {
        assert!(!clause.is_empty(), "Crossing cut clause must not be empty");
    }
}

#[test]
fn test_macro_cycle_count_limits() {
    let mut g = Graph::new();
    g.add_edge(1, 2);
    g.add_edge(2, 1);

    let contractor = Degree2Contractor::new();
    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    // Single cycle: k = 1 (should return None / empty)
    let single_cycle = vec![vec![1, 2]];
    assert!(HemisphereSplicer::try_direct_splice_all(&single_cycle, &g, &contractor).is_none());
    assert!(HemisphereSplicer::generate_hemisphere_crossing_cuts(&single_cycle, &g, &encoder).is_empty());

    // 5 cycles: k = 5 (exceeds k in 2..=4, should return None / empty)
    let five_cycles = vec![vec![1], vec![2], vec![3], vec![4], vec![5]];
    assert!(HemisphereSplicer::try_direct_splice_all(&five_cycles, &g, &contractor).is_none());
    assert!(HemisphereSplicer::generate_hemisphere_crossing_cuts(&five_cycles, &g, &encoder).is_empty());
}
