use std::collections::HashSet;
use cegar_fix::graph::Graph;
use cegar_fix::macro_cycle_stitcher::MacroCycleStitcher;

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
fn test_stitch_two_cycles() {
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

    // Cross edges: (1, 5) and (2, 6)
    g.add_edge(1, 5);
    g.add_edge(2, 6);

    let cycles = vec![vec![1, 2, 3, 4], vec![5, 6, 7, 8]];
    let protected_edges = HashSet::new();

    let result = MacroCycleStitcher::stitch_cycles(&cycles, &g, &protected_edges, 2);
    assert!(result.is_some(), "Expected 2-cycle stitch to succeed");
    let merged = result.unwrap();
    assert_eq!(merged.len(), 1, "Expected exactly 1 merged cycle");
    let expected_nodes: Vec<i32> = (1..=8).collect();
    verify_valid_cycle(&merged[0], &g, &expected_nodes);

    // Also test stitch_until_fixed_point
    let fixed_result = MacroCycleStitcher::stitch_until_fixed_point(&cycles, &g, &protected_edges);
    assert_eq!(fixed_result.len(), 1);
    verify_valid_cycle(&fixed_result[0], &g, &expected_nodes);
}

#[test]
fn test_stitch_three_cycles() {
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

    // Cycle 3: 9 - 10 - 11 - 12 - 9
    g.add_edge(9, 10);
    g.add_edge(10, 11);
    g.add_edge(11, 12);
    g.add_edge(12, 9);

    // Exactly 3 cross edges connecting C1 -> C2 -> C3 -> C1:
    // (2, 5), (6, 9), (10, 1)
    // No pair of cycles has 2 cross edges, so 2-opt CANNOT merge any two cycles.
    g.add_edge(2, 5);
    g.add_edge(6, 9);
    g.add_edge(10, 1);

    let cycles = vec![vec![1, 2, 3, 4], vec![5, 6, 7, 8], vec![9, 10, 11, 12]];
    let protected_edges = HashSet::new();

    // With max_swaps = 2, 3-cycle stitch should fail
    let res_2opt = MacroCycleStitcher::stitch_cycles(&cycles, &g, &protected_edges, 2);
    assert!(res_2opt.is_none(), "2-opt should not be able to merge 3 cycles with only 1 cross-edge per pair");

    // With max_swaps = 3, 3-cycle alternating swap should succeed
    let res_3opt = MacroCycleStitcher::stitch_cycles(&cycles, &g, &protected_edges, 3);
    assert!(res_3opt.is_some(), "3-opt alternating swap should merge all 3 cycles");
    let merged = res_3opt.unwrap();
    assert_eq!(merged.len(), 1, "Expected single merged cycle of all 3 cycles");
    let expected_nodes: Vec<i32> = (1..=12).collect();
    verify_valid_cycle(&merged[0], &g, &expected_nodes);

    // stitch_until_fixed_point should also succeed
    let fixed_result = MacroCycleStitcher::stitch_until_fixed_point(&cycles, &g, &protected_edges);
    assert_eq!(fixed_result.len(), 1);
    verify_valid_cycle(&fixed_result[0], &g, &expected_nodes);
}

#[test]
fn test_protected_edge_preservation() {
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

    // Cross edges: (1, 5) and (2, 6) - would require breaking (1, 2) and (5, 6)
    g.add_edge(1, 5);
    g.add_edge(2, 6);

    let cycles = vec![vec![1, 2, 3, 4], vec![5, 6, 7, 8]];

    // Case 1: Protect edge (1, 2). Stitching should fail because no other cross edges exist.
    let mut protected_edges = HashSet::new();
    protected_edges.insert((1, 2));
    let res = MacroCycleStitcher::stitch_cycles(&cycles, &g, &protected_edges, 2);
    assert!(res.is_none(), "Should not stitch when necessary tour edge is protected");

    // Case 2: Also add alternative cross edges (3, 7) and (4, 8).
    // Now stitcher should find the alternative swap using (3, 4) and (7, 8), leaving (1, 2) intact!
    g.add_edge(3, 7);
    g.add_edge(4, 8);

    let res2 = MacroCycleStitcher::stitch_cycles(&cycles, &g, &protected_edges, 2);
    assert!(res2.is_some(), "Should find alternative stitch that preserves protected edge");
    let merged = res2.unwrap();
    assert_eq!(merged.len(), 1);
    let expected_nodes: Vec<i32> = (1..=8).collect();
    verify_valid_cycle(&merged[0], &g, &expected_nodes);

    // Verify protected edge (1, 2) is still in the merged cycle
    let tour = &merged[0];
    let n = tour.len();
    let mut contains_protected = false;
    for i in 0..n {
        let u = tour[i];
        let v = tour[(i + 1) % n];
        if (u == 1 && v == 2) || (u == 2 && v == 1) {
            contains_protected = true;
            break;
        }
    }
    assert!(contains_protected, "Protected edge (1, 2) must be preserved in merged cycle");
}

#[test]
fn test_already_single_cycle() {
    let mut g = Graph::new();
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 1);

    let cycles = vec![vec![1, 2, 3]];
    let protected = HashSet::new();

    let res = MacroCycleStitcher::stitch_cycles(&cycles, &g, &protected, 4);
    assert!(res.is_none(), "stitch_cycles on single cycle should return None");

    let fixed = MacroCycleStitcher::stitch_until_fixed_point(&cycles, &g, &protected);
    assert_eq!(fixed.len(), 1);
    assert_eq!(fixed[0], vec![1, 2, 3]);
}
