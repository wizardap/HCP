use std::collections::HashSet;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::graph::Graph;
use cegar_fix::boundary_alternating_patcher::BoundaryAlternatingPatcher;

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
fn test_multi_hop_alternating_patch() {
    let mut g = Graph::new();
    // Cycle 1: 1 - 2 - 3 - 4 - 5 - 6 - 1
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 5);
    g.add_edge(5, 6);
    g.add_edge(6, 1);

    // Cycle 2: 7 - 8 - 9 - 10 - 11 - 12 - 7
    g.add_edge(7, 8);
    g.add_edge(8, 9);
    g.add_edge(9, 10);
    g.add_edge(10, 11);
    g.add_edge(11, 12);
    g.add_edge(12, 7);

    // Multi-hop cross edges and chord:
    // Cross edges: (1, 7) and (3, 8) [offset by 2 hops in Cycle 1: 1 -> 2 -> 3]
    // Internal chord in Cycle 1: (2, 6)
    // Alternating cycle A:
    // + (1, 7) [cross]
    // - (7, 8) [in Cycle 2]
    // + (8, 3) [cross]
    // - (3, 2) [in Cycle 1]
    // + (2, 6) [chord in Cycle 1]
    // - (6, 1) [in Cycle 1]
    g.add_edge(1, 7);
    g.add_edge(8, 3);
    g.add_edge(2, 6);

    let contractor = Degree2Contractor::new();
    let cycles = vec![vec![1, 2, 3, 4, 5, 6], vec![7, 8, 9, 10, 11, 12]];

    let result = BoundaryAlternatingPatcher::try_patch_macro_hemispheres(&cycles, &g, &contractor, 4);
    assert!(result.is_some(), "Multi-hop alternating patch should succeed");
    let merged_cycles = result.unwrap();
    assert_eq!(merged_cycles.len(), 1, "Expected single merged cycle");
    let expected_nodes: Vec<i32> = (1..=12).collect();
    verify_valid_cycle(&merged_cycles[0], &g, &expected_nodes);
}

#[test]
fn test_multi_hop_4opt_alternating_patch() {
    let mut g = Graph::new();
    // Cycle 1: 1 - 2 - 3 - 4 - 5 - 6 - 1
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 5);
    g.add_edge(5, 6);
    g.add_edge(6, 1);

    // Cycle 2: 7 - 8 - 9 - 10 - 11 - 12 - 7
    g.add_edge(7, 8);
    g.add_edge(8, 9);
    g.add_edge(9, 10);
    g.add_edge(10, 11);
    g.add_edge(11, 12);
    g.add_edge(12, 7);

    // 4-opt alternating cycle between the two cycles:
    // + (1, 7), - (7, 8), + (8, 2), - (2, 3), + (3, 9), - (9, 10), + (10, 6), - (6, 1)
    g.add_edge(1, 7);
    g.add_edge(8, 2);
    g.add_edge(3, 9);
    g.add_edge(10, 6);

    let contractor = Degree2Contractor::new();
    let cycles = vec![vec![1, 2, 3, 4, 5, 6], vec![7, 8, 9, 10, 11, 12]];

    let result = BoundaryAlternatingPatcher::try_patch_macro_hemispheres(&cycles, &g, &contractor, 4);
    assert!(result.is_some(), "4-opt alternating patch should succeed");
    let merged_cycles = result.unwrap();
    assert_eq!(merged_cycles.len(), 1, "Expected single merged cycle");
    let expected_nodes: Vec<i32> = (1..=12).collect();
    verify_valid_cycle(&merged_cycles[0], &g, &expected_nodes);
}

#[test]
fn test_protected_chain_preservation() {
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

    // Protect edge (1, 2)
    let mut contractor = Degree2Contractor::new();
    contractor.chain_map.insert((1, 2), vec![100]);
    contractor.chain_map.insert((2, 1), vec![100]);

    let cycles = vec![vec![1, 2, 3, 4, 5], vec![6, 7, 8, 9, 10]];

    // Should fail because (1, 2) cannot be broken
    let result = BoundaryAlternatingPatcher::try_patch_macro_hemispheres(&cycles, &g, &contractor, 4);
    assert!(result.is_none(), "Should not break protected edge (1, 2)");

    // With unprotected contractor, it should succeed
    let free_contractor = Degree2Contractor::new();
    let result_free = BoundaryAlternatingPatcher::try_patch_macro_hemispheres(&cycles, &g, &free_contractor, 4);
    assert!(result_free.is_some(), "Unprotected patch should succeed");
    let merged = result_free.unwrap();
    assert_eq!(merged.len(), 1);
    let expected: Vec<i32> = (1..=10).collect();
    verify_valid_cycle(&merged[0], &g, &expected);
}

#[test]
fn test_three_macro_cycles_merge() {
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

    // 3-opt alternating cycle crossing all 3 cycles:
    // + (1, 5), - (5, 6), + (6, 9), - (9, 10), + (10, 2), - (2, 1)
    g.add_edge(1, 5);
    g.add_edge(6, 9);
    g.add_edge(10, 2);

    let contractor = Degree2Contractor::new();
    let cycles = vec![
        vec![1, 2, 3, 4],
        vec![5, 6, 7, 8],
        vec![9, 10, 11, 12],
    ];

    let result = BoundaryAlternatingPatcher::try_patch_macro_hemispheres(&cycles, &g, &contractor, 4);
    assert!(result.is_some(), "3-cycle alternating merge should succeed");
    let merged = result.unwrap();
    assert_eq!(merged.len(), 1, "Expected all 3 cycles merged into 1");
    let expected: Vec<i32> = (1..=12).collect();
    verify_valid_cycle(&merged[0], &g, &expected);
}

#[test]
fn test_macro_cycle_bounds() {
    let mut g = Graph::new();
    g.add_edge(1, 2);
    g.add_edge(2, 1);

    let contractor = Degree2Contractor::new();

    // k = 1 (should return None)
    let single = vec![vec![1, 2]];
    assert!(BoundaryAlternatingPatcher::try_patch_macro_hemispheres(&single, &g, &contractor, 4).is_none());

    // k = 5 (should return None)
    let five = vec![vec![1], vec![2], vec![3], vec![4], vec![5]];
    assert!(BoundaryAlternatingPatcher::try_patch_macro_hemispheres(&five, &g, &contractor, 4).is_none());
}
