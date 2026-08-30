use std::collections::HashSet;
use std::time::Instant;
use cegar_fix::graph::Graph;
use cegar_fix::sat_macro_patcher::SatMacroPatcher;

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

    let result = SatMacroPatcher::try_patch_all_cycles(&cycles, &g, &protected);
    assert!(result.is_some(), "Expected result for single cycle");
    let tour = result.unwrap();
    assert_eq!(tour.len(), 6);
    verify_valid_cycle(&tour, &g, &[1, 2, 3, 4, 5, 6]);
}

#[test]
fn test_simultaneous_4_cycle_merge() {
    let mut g = Graph::new();

    // 4 cycles:
    // C0: 1 - 2 - 3 - 4 - 1
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 1);

    // C1: 5 - 6 - 7 - 8 - 5
    g.add_edge(5, 6);
    g.add_edge(6, 7);
    g.add_edge(7, 8);
    g.add_edge(8, 5);

    // C2: 9 - 10 - 11 - 12 - 9
    g.add_edge(9, 10);
    g.add_edge(10, 11);
    g.add_edge(11, 12);
    g.add_edge(12, 9);

    // C3: 13 - 14 - 15 - 16 - 13
    g.add_edge(13, 14);
    g.add_edge(14, 15);
    g.add_edge(15, 16);
    g.add_edge(16, 13);

    // Bridge 1 (C0 - C1): cut (1, 2) in C0, (5, 6) in C1; add (1, 5), (2, 6)
    g.add_edge(1, 5);
    g.add_edge(2, 6);

    // Bridge 2 (C1 - C2): cut (7, 8) in C1, (9, 10) in C2; add (7, 9), (8, 10)
    g.add_edge(7, 9);
    g.add_edge(8, 10);

    // Bridge 3 (C2 - C3): cut (11, 12) in C2, (13, 14) in C3; add (11, 13), (12, 14)
    g.add_edge(11, 13);
    g.add_edge(12, 14);

    let cycles = vec![
        vec![1, 2, 3, 4],
        vec![5, 6, 7, 8],
        vec![9, 10, 11, 12],
        vec![13, 14, 15, 16],
    ];
    let protected = HashSet::new();

    let result = SatMacroPatcher::try_patch_all_cycles(&cycles, &g, &protected);
    assert!(result.is_some(), "Expected 4 cycles merged into 1 cycle");
    let tour = result.unwrap();
    let expected_nodes: Vec<i32> = (1..=16).collect();
    verify_valid_cycle(&tour, &g, &expected_nodes);
}

#[test]
fn test_simultaneous_10_cycle_merge() {
    let mut g = Graph::new();
    let mut cycles = Vec::new();

    // 10 cycles of length 4: C_i on vertices 4*i + 1 .. 4*i + 4
    for i in 0..10 {
        let base = (i * 4 + 1) as i32;
        let v1 = base;
        let v2 = base + 1;
        let v3 = base + 2;
        let v4 = base + 3;

        g.add_edge(v1, v2);
        g.add_edge(v2, v3);
        g.add_edge(v3, v4);
        g.add_edge(v4, v1);

        cycles.push(vec![v1, v2, v3, v4]);
    }

    // Connect C_i to C_{i+1} for i in 0..9 in a line
    for i in 0..9 {
        let base_i = (i * 4 + 1) as i32;
        let base_next = ((i + 1) * 4 + 1) as i32;

        // In C_i, cut (base_i + 2, base_i + 3) = (v3, v4)
        // In C_{i+1}, cut (base_next, base_next + 1) = (v1, v2)
        // Add cross edges: (base_i + 2, base_next) and (base_i + 3, base_next + 1)
        g.add_edge(base_i + 2, base_next);
        g.add_edge(base_i + 3, base_next + 1);
    }

    let protected = HashSet::new();

    let start = Instant::now();
    let result = SatMacroPatcher::try_patch_all_cycles(&cycles, &g, &protected);
    let elapsed = start.elapsed();

    assert!(result.is_some(), "Expected 10 cycles merged into 1 cycle");
    let tour = result.unwrap();
    let expected_nodes: Vec<i32> = (1..=40).collect();
    verify_valid_cycle(&tour, &g, &expected_nodes);

    println!("10-cycle merge solved in: {:?}", elapsed);
    assert!(
        elapsed.as_millis() < 50,
        "Expected SAT spanning tree solve in < 50ms, took {:?}",
        elapsed
    );
}

#[test]
fn test_protected_edge_preservation() {
    let mut g = Graph::new();

    // 4 cycles in a line
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

    g.add_edge(13, 14);
    g.add_edge(14, 15);
    g.add_edge(15, 16);
    g.add_edge(16, 13);

    // Bridge 1 (C0 - C1)
    g.add_edge(1, 5);
    g.add_edge(2, 6);

    // Bridge 2 (C1 - C2)
    g.add_edge(7, 9);
    g.add_edge(8, 10);

    // Bridge 3 (C2 - C3)
    g.add_edge(11, 13);
    g.add_edge(12, 14);

    let cycles = vec![
        vec![1, 2, 3, 4],
        vec![5, 6, 7, 8],
        vec![9, 10, 11, 12],
        vec![13, 14, 15, 16],
    ];

    // Protect edge (7, 8) which is needed for Bridge 2
    let mut protected = HashSet::new();
    protected.insert((7, 8));
    protected.insert((8, 7));

    let result = SatMacroPatcher::try_patch_all_cycles(&cycles, &g, &protected);
    assert!(
        result.is_none(),
        "Expected None when an essential bridge cuts a protected edge"
    );
}

#[test]
fn test_try_patch_components_partial_merge() {
    let mut g = Graph::new();

    // 6 cycles: C0..C2 in cluster A, C3..C5 in cluster B
    let mut cycles = Vec::new();
    for i in 0..6 {
        let base = (i * 4 + 1) as i32;
        let v1 = base;
        let v2 = base + 1;
        let v3 = base + 2;
        let v4 = base + 3;

        g.add_edge(v1, v2);
        g.add_edge(v2, v3);
        g.add_edge(v3, v4);
        g.add_edge(v4, v1);

        cycles.push(vec![v1, v2, v3, v4]);
    }

    // Cluster A: C0 - C1 - C2
    // C0 - C1: cut (1, 2) in C0, (5, 6) in C1; add (1, 5), (2, 6)
    g.add_edge(1, 5);
    g.add_edge(2, 6);
    // C1 - C2: cut (7, 8) in C1, (9, 10) in C2; add (7, 9), (8, 10)
    g.add_edge(7, 9);
    g.add_edge(8, 10);

    // Cluster B: C3 - C4 - C5
    // C3 - C4: cut (13, 14) in C3, (17, 18) in C4; add (13, 17), (14, 18)
    g.add_edge(13, 17);
    g.add_edge(14, 18);
    // C4 - C5: cut (19, 20) in C4, (21, 22) in C5; add (19, 21), (20, 22)
    g.add_edge(19, 21);
    g.add_edge(20, 22);

    let protected = HashSet::new();

    let result = SatMacroPatcher::try_patch_components(&cycles, &g, &protected);
    assert_eq!(result.len(), 2, "Expected 6 cycles to merge into 2 cycles (2 clusters of 3)");

    let mut all_nodes = Vec::new();
    for cycle in &result {
        assert_eq!(cycle.len(), 12, "Each merged cluster cycle should have 12 vertices");
        all_nodes.extend_from_slice(cycle);
        let cycle_nodes: Vec<i32> = cycle.clone();
        verify_valid_cycle(cycle, &g, &cycle_nodes);
    }
    let expected_all: HashSet<i32> = (1..=24).collect();
    let result_all: HashSet<i32> = all_nodes.into_iter().collect();
    assert_eq!(result_all, expected_all, "All 24 vertices must be present in output cycles");
}

#[test]
fn test_try_patch_components_no_merge() {
    let mut g = Graph::new();
    let mut cycles = Vec::new();

    for i in 0..3 {
        let base = (i * 4 + 1) as i32;
        let v1 = base;
        let v2 = base + 1;
        let v3 = base + 2;
        let v4 = base + 3;

        g.add_edge(v1, v2);
        g.add_edge(v2, v3);
        g.add_edge(v3, v4);
        g.add_edge(v4, v1);

        cycles.push(vec![v1, v2, v3, v4]);
    }

    let protected = HashSet::new();

    let result = SatMacroPatcher::try_patch_components(&cycles, &g, &protected);
    assert_eq!(result.len(), 3, "Isolated cycles without bridges should remain unchanged");
    for cycle in &result {
        assert_eq!(cycle.len(), 4);
    }
}

#[test]
fn test_try_patch_components_all_merge() {
    let mut g = Graph::new();
    let mut cycles = Vec::new();

    for i in 0..6 {
        let base = (i * 4 + 1) as i32;
        let v1 = base;
        let v2 = base + 1;
        let v3 = base + 2;
        let v4 = base + 3;

        g.add_edge(v1, v2);
        g.add_edge(v2, v3);
        g.add_edge(v3, v4);
        g.add_edge(v4, v1);

        cycles.push(vec![v1, v2, v3, v4]);
    }

    // Connect C0-C1, C1-C2, C2-C3, C3-C4, C4-C5
    g.add_edge(1, 5);
    g.add_edge(2, 6);

    g.add_edge(7, 9);
    g.add_edge(8, 10);

    g.add_edge(11, 13);
    g.add_edge(12, 14);

    g.add_edge(15, 17);
    g.add_edge(16, 18);

    g.add_edge(19, 21);
    g.add_edge(20, 22);

    let protected = HashSet::new();

    let result = SatMacroPatcher::try_patch_components(&cycles, &g, &protected);
    assert_eq!(result.len(), 1, "All 6 cycles should merge into 1 single cycle");
    let expected_nodes: Vec<i32> = (1..=24).collect();
    verify_valid_cycle(&result[0], &g, &expected_nodes);
}

