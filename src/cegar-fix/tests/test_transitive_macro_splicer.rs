use std::collections::HashSet;
use cegar_fix::graph::Graph;
use cegar_fix::transitive_macro_splicer::TransitiveMacroSplicer;

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
fn test_transitive_four_cycle_chain() {
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

    // Cycle 3: 13 - 14 - 15 - 16 - 13
    g.add_edge(13, 14);
    g.add_edge(14, 15);
    g.add_edge(15, 16);
    g.add_edge(16, 13);

    // Cross edges between C0 and C1: (2, 5) and (3, 6)
    // Bridge b01 removes (2, 3) in C0 and (5, 6) in C1
    g.add_edge(2, 5);
    g.add_edge(3, 6);

    // Cross edges between C1 and C2: (7, 9) and (8, 10)
    // Bridge b12 removes (7, 8) in C1 and (9, 10) in C2
    g.add_edge(7, 9);
    g.add_edge(8, 10);

    // Cross edges between C2 and C3: (11, 13) and (12, 14)
    // Bridge b23 removes (11, 12) in C2 and (13, 14) in C3
    g.add_edge(11, 13);
    g.add_edge(12, 14);

    // Notice: C0 and C3 share zero cross edges!
    let cycles = vec![
        vec![1, 2, 3, 4],
        vec![5, 6, 7, 8],
        vec![9, 10, 11, 12],
        vec![13, 14, 15, 16],
    ];
    let protected_edges = HashSet::new();

    let result = TransitiveMacroSplicer::splice_transitive_macro_graph(&cycles, &g, &protected_edges);
    assert_eq!(result.len(), 1, "Expected all 4 cycles in chain to be spliced into a single cycle");

    let expected_nodes: Vec<i32> = (1..=16).collect();
    verify_valid_cycle(&result[0], &g, &expected_nodes);
}

#[test]
fn test_transitive_protected_edges() {
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

    // Primary cross edges: (2, 5) and (3, 6), removing (2, 3) and (5, 6)
    g.add_edge(2, 5);
    g.add_edge(3, 6);

    let cycles = vec![vec![1, 2, 3, 4], vec![5, 6, 7, 8]];

    // Case 1: Protect (2, 3) in C0. Splicing should fail because no alternative cross edges exist.
    let mut protected_edges = HashSet::new();
    protected_edges.insert((2, 3));

    let res1 = TransitiveMacroSplicer::splice_transitive_macro_graph(&cycles, &g, &protected_edges);
    assert_eq!(res1.len(), 2, "Should not splice when essential bridge edge is protected");

    // Case 2: Add alternative cross edges (4, 7) and (1, 8), removing (4, 1) and (7, 8)
    g.add_edge(4, 7);
    g.add_edge(1, 8);

    let res2 = TransitiveMacroSplicer::splice_transitive_macro_graph(&cycles, &g, &protected_edges);
    assert_eq!(res2.len(), 1, "Should find alternative bridge avoiding protected edge (2, 3)");

    let expected_nodes: Vec<i32> = (1..=8).collect();
    verify_valid_cycle(&res2[0], &g, &expected_nodes);

    // Verify protected edge (2, 3) is strictly preserved in the merged cycle
    let tour = &res2[0];
    let n = tour.len();
    let mut preserved = false;
    for i in 0..n {
        let u = tour[i];
        let v = tour[(i + 1) % n];
        if (u == 2 && v == 3) || (u == 3 && v == 2) {
            preserved = true;
            break;
        }
    }
    assert!(preserved, "Protected edge (2, 3) must be preserved in the spliced tour");
}

#[test]
fn test_already_single_cycle() {
    let mut g = Graph::new();
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 1);

    let cycles = vec![vec![1, 2, 3]];
    let protected = HashSet::new();

    let result = TransitiveMacroSplicer::splice_transitive_macro_graph(&cycles, &g, &protected);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], vec![1, 2, 3]);
}

#[test]
fn test_star_macro_graph() {
    let mut g = Graph::new();

    // Center Cycle 0: 1 - 2 - 3 - 4 - 5 - 6 - 1
    for i in 1..=6 {
        g.add_edge(i, if i == 6 { 1 } else { i + 1 });
    }

    // Satellite Cycle 1: 11 - 12 - 13 - 11
    g.add_edge(11, 12);
    g.add_edge(12, 13);
    g.add_edge(13, 11);
    // Bridge to C0 via (1, 2) in C0 and (11, 12) in C1
    g.add_edge(1, 11);
    g.add_edge(2, 12);

    // Satellite Cycle 2: 21 - 22 - 23 - 21
    g.add_edge(21, 22);
    g.add_edge(22, 23);
    g.add_edge(23, 21);
    // Bridge to C0 via (3, 4) in C0 and (21, 22) in C2
    g.add_edge(3, 21);
    g.add_edge(4, 22);

    // Satellite Cycle 3: 31 - 32 - 33 - 31
    g.add_edge(31, 32);
    g.add_edge(32, 33);
    g.add_edge(33, 31);
    // Bridge to C0 via (5, 6) in C0 and (31, 32) in C3
    g.add_edge(5, 31);
    g.add_edge(6, 32);

    let cycles = vec![
        vec![1, 2, 3, 4, 5, 6],
        vec![11, 12, 13],
        vec![21, 22, 23],
        vec![31, 32, 33],
    ];
    let protected = HashSet::new();

    let result = TransitiveMacroSplicer::splice_transitive_macro_graph(&cycles, &g, &protected);
    assert_eq!(result.len(), 1, "Star macro-graph should splice all 3 satellites into center cycle");

    let expected_nodes: Vec<i32> = vec![1, 2, 3, 4, 5, 6, 11, 12, 13, 21, 22, 23, 31, 32, 33];
    verify_valid_cycle(&result[0], &g, &expected_nodes);
}

#[test]
fn test_disconnected_macro_components() {
    let mut g = Graph::new();

    // Component 1: C0 and C1
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 1);

    g.add_edge(5, 6);
    g.add_edge(6, 7);
    g.add_edge(7, 8);
    g.add_edge(8, 5);

    g.add_edge(1, 5);
    g.add_edge(2, 6);

    // Component 2: C2 and C3 (completely disconnected from C0/C1)
    g.add_edge(101, 102);
    g.add_edge(102, 103);
    g.add_edge(103, 104);
    g.add_edge(104, 101);

    g.add_edge(105, 106);
    g.add_edge(106, 107);
    g.add_edge(107, 108);
    g.add_edge(108, 105);

    g.add_edge(101, 105);
    g.add_edge(102, 106);

    let cycles = vec![
        vec![1, 2, 3, 4],
        vec![5, 6, 7, 8],
        vec![101, 102, 103, 104],
        vec![105, 106, 107, 108],
    ];
    let protected = HashSet::new();

    let result = TransitiveMacroSplicer::splice_transitive_macro_graph(&cycles, &g, &protected);
    assert_eq!(result.len(), 2, "Two independent macro components should reduce from 4 cycles to 2 cycles");

    let comp1_nodes: Vec<i32> = (1..=8).collect();
    let comp2_nodes: Vec<i32> = (101..=108).collect();

    // Result should contain one cycle with comp1_nodes and one with comp2_nodes
    let mut found_c1 = false;
    let mut found_c2 = false;
    for c in &result {
        if c.contains(&1) {
            verify_valid_cycle(c, &g, &comp1_nodes);
            found_c1 = true;
        } else if c.contains(&101) {
            verify_valid_cycle(c, &g, &comp2_nodes);
            found_c2 = true;
        }
    }
    assert!(found_c1 && found_c2, "Both components must be successfully formed");
}

#[test]
fn test_transitive_ten_cycle_chain() {
    let mut g = Graph::new();
    let num_cycles = 10;
    let mut cycles = Vec::new();

    // Create 10 cycles, each of 4 nodes: [4*k + 1, 4*k + 2, 4*k + 3, 4*k + 4]
    for k in 0..num_cycles {
        let base = (k * 4) as i32;
        let c = vec![base + 1, base + 2, base + 3, base + 4];
        g.add_edge(c[0], c[1]);
        g.add_edge(c[1], c[2]);
        g.add_edge(c[2], c[3]);
        g.add_edge(c[3], c[0]);
        cycles.push(c);
    }

    // Connect adjacent cycles C_k and C_{k+1}
    for k in 0..(num_cycles - 1) {
        let base_k = (k * 4) as i32;
        let base_next = ((k + 1) * 4) as i32;

        // Bridge uses (base_k + 2, base_k + 3) in C_k and (base_next + 1, base_next + 4) in C_{next}
        // Wait, C_k uses (base_k + 2, base_k + 3) for right bridge, and (base_k + 4, base_k + 1) for left bridge
        if k % 2 == 0 {
            g.add_edge(base_k + 2, base_next + 1);
            g.add_edge(base_k + 3, base_next + 2);
        } else {
            g.add_edge(base_k + 3, base_next + 4);
            g.add_edge(base_k + 4, base_next + 1);
        }
    }

    let protected = HashSet::new();
    let start = std::time::Instant::now();
    let result = TransitiveMacroSplicer::splice_transitive_macro_graph(&cycles, &g, &protected);
    let elapsed = start.elapsed();

    assert_eq!(result.len(), 1, "Expected all 10 cycles in chain to splice into 1 tour");
    let total_nodes = (num_cycles * 4) as i32;
    let expected_nodes: Vec<i32> = (1..=total_nodes).collect();
    verify_valid_cycle(&result[0], &g, &expected_nodes);
    assert!(elapsed.as_millis() < 50, "10-cycle chain splicing took {:?}, expected < 50ms", elapsed);
}
