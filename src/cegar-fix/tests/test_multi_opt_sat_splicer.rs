use std::collections::HashSet;
use cegar_fix::graph::Graph;
use cegar_fix::multi_opt_sat_splicer::MultiOptSatSplicer;

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
fn test_already_single_cycle() {
    let mut g = Graph::new();
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 5);
    g.add_edge(5, 6);
    g.add_edge(6, 1);

    let cycles = vec![vec![1, 2, 3, 4, 5, 6]];
    let protected = HashSet::new();

    let result = MultiOptSatSplicer::splice_multi_opt_cycles(&cycles, &g, &protected);
    assert_eq!(result.len(), 1, "Expected already-single cycle to be returned unchanged");
    verify_valid_cycle(&result[0], &g, &[1, 2, 3, 4, 5, 6]);
}

#[test]
fn test_3opt_triangle_merge() {
    let mut g = Graph::new();

    // Cycle 0: 1 - 2 - 3 - 4 - 1
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 1);

    // Cycle 1: 5 - 6 - 7 - 8 - 5
    g.add_edge(5, 6);
    g.add_edge(6, 7);
    g.add_edge(7, 8);
    g.add_edge(8, 5);

    // Cycle 2: 9 - 10 - 11 - 12 - 9
    g.add_edge(9, 10);
    g.add_edge(10, 11);
    g.add_edge(11, 12);
    g.add_edge(12, 9);

    // Cross edges creating exactly a 3-opt triangle:
    // C0 removed (1, 2), C1 removed (5, 6), C2 removed (9, 10)
    // Added cross-edges: (1, 6) between C0 and C1, (5, 10) between C1 and C2, (9, 2) between C2 and C0.
    // Note: there are NO 2-opt bridges between any pair (only 1 cross edge per pair)!
    g.add_edge(1, 6);
    g.add_edge(5, 10);
    g.add_edge(9, 2);

    let cycles = vec![
        vec![1, 2, 3, 4],
        vec![5, 6, 7, 8],
        vec![9, 10, 11, 12],
    ];
    let protected = HashSet::new();

    let result = MultiOptSatSplicer::splice_multi_opt_cycles(&cycles, &g, &protected);
    assert_eq!(result.len(), 1, "Expected 3-opt triangle to merge all 3 cycles into 1 cycle");

    let expected_nodes: Vec<i32> = (1..=12).collect();
    verify_valid_cycle(&result[0], &g, &expected_nodes);
}

#[test]
fn test_mixed_2opt_and_3opt() {
    let mut g = Graph::new();

    // 6 cycles of length 4:
    // C0: 1..4
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 1);

    // C1: 5..8
    g.add_edge(5, 6);
    g.add_edge(6, 7);
    g.add_edge(7, 8);
    g.add_edge(8, 5);

    // C2: 9..12
    g.add_edge(9, 10);
    g.add_edge(10, 11);
    g.add_edge(11, 12);
    g.add_edge(12, 9);

    // C3: 13..16
    g.add_edge(13, 14);
    g.add_edge(14, 15);
    g.add_edge(15, 16);
    g.add_edge(16, 13);

    // C4: 17..20
    g.add_edge(17, 18);
    g.add_edge(18, 19);
    g.add_edge(19, 20);
    g.add_edge(20, 17);

    // C5: 21..24
    g.add_edge(21, 22);
    g.add_edge(22, 23);
    g.add_edge(23, 24);
    g.add_edge(24, 21);

    // 1. 3-opt triangle on C0, C1, C2:
    // Removals: (1, 2) in C0, (5, 6) in C1, (9, 10) in C2
    // Additions: (1, 6), (5, 10), (9, 2)
    g.add_edge(1, 6);
    g.add_edge(5, 10);
    g.add_edge(9, 2);

    // 2. 2-opt bridge between C2 and C3:
    // Removals: (11, 12) in C2, (13, 14) in C3
    // Additions: (11, 13), (12, 14)
    g.add_edge(11, 13);
    g.add_edge(12, 14);

    // 3. 3-opt triangle on C3, C4, C5:
    // Removals: (15, 16) in C3, (17, 18) in C4, (21, 22) in C5
    // Additions: (15, 18), (17, 22), (21, 16)
    g.add_edge(15, 18);
    g.add_edge(17, 22);
    g.add_edge(21, 16);

    let cycles = vec![
        vec![1, 2, 3, 4],
        vec![5, 6, 7, 8],
        vec![9, 10, 11, 12],
        vec![13, 14, 15, 16],
        vec![17, 18, 19, 20],
        vec![21, 22, 23, 24],
    ];
    let protected = HashSet::new();

    let result = MultiOptSatSplicer::splice_multi_opt_cycles(&cycles, &g, &protected);
    assert_eq!(result.len(), 1, "Expected all 6 cycles merged via combination of 2-opt and 3-opt");

    let expected_nodes: Vec<i32> = (1..=24).collect();
    verify_valid_cycle(&result[0], &g, &expected_nodes);
}

#[test]
fn test_protected_edge_preservation() {
    let mut g = Graph::new();

    // 3 cycles with 3-opt triangle configuration
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 1);

    g.add_edge(5, 6);
    g.add_edge(6, 7);
    g.add_edge(7, 8);
    g.add_edge(8, 5);

    g.add_edge(9, 10);
    g.add_edge(10, 11);
    g.add_edge(11, 12);
    g.add_edge(12, 9);

    g.add_edge(1, 6);
    g.add_edge(5, 10);
    g.add_edge(9, 2);

    let cycles = vec![
        vec![1, 2, 3, 4],
        vec![5, 6, 7, 8],
        vec![9, 10, 11, 12],
    ];

    // Protect edge (1, 2)
    let mut protected = HashSet::new();
    protected.insert((1, 2));
    protected.insert((2, 1));

    let result = MultiOptSatSplicer::splice_multi_opt_cycles(&cycles, &g, &protected);
    assert_eq!(
        result.len(),
        3,
        "Expected 3 cycles returned when required 3-opt edge is protected"
    );

    // Verify protected edge (1, 2) is intact in cycle 0
    let mut found_protected = false;
    for c in &result {
        let n = c.len();
        for i in 0..n {
            let u = c[i];
            let v = c[(i + 1) % n];
            if (u == 1 && v == 2) || (u == 2 && v == 1) {
                found_protected = true;
            }
        }
    }
    assert!(found_protected, "Protected edge (1, 2) must be preserved in result");
}
