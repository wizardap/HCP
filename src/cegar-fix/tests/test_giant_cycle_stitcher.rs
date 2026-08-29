use std::collections::HashSet;
use cegar_fix::graph::Graph;
use cegar_fix::giant_cycle_stitcher::GiantCycleStitcher;

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
fn test_absorb_multiple_small_cycles_into_giant() {
    let mut g = Graph::new();
    // Giant cycle: 50 vertices (1..=50)
    for i in 1..=50 {
        g.add_edge(i, if i == 50 { 1 } else { i + 1 });
    }

    // Small cycle 1: 3 vertices (51..=53)
    g.add_edge(51, 52);
    g.add_edge(52, 53);
    g.add_edge(53, 51);
    // Connect to giant cycle at (5, 6)
    g.add_edge(5, 51);
    g.add_edge(6, 52);

    // Small cycle 2: 4 vertices (54..=57)
    g.add_edge(54, 55);
    g.add_edge(55, 56);
    g.add_edge(56, 57);
    g.add_edge(57, 54);
    // Connect to giant cycle at (15, 16)
    g.add_edge(15, 54);
    g.add_edge(16, 55);

    // Small cycle 3: 8 vertices (58..=65)
    for i in 58..=65 {
        g.add_edge(i, if i == 65 { 58 } else { i + 1 });
    }
    // Connect to giant cycle at (25, 26)
    g.add_edge(25, 58);
    g.add_edge(26, 59);

    let cycles = vec![
        (1..=50).collect::<Vec<i32>>(),
        vec![51, 52, 53],
        vec![54, 55, 56, 57],
        (58..=65).collect::<Vec<i32>>(),
    ];
    let protected_edges = HashSet::new();

    let result = GiantCycleStitcher::absorb_into_giant_cycle(&cycles, &g, &protected_edges, 4);
    assert_eq!(result.len(), 1, "Expected all 3 small cycles to be absorbed into 1 giant cycle");
    let expected: Vec<i32> = (1..=65).collect();
    verify_valid_cycle(&result[0], &g, &expected);

    let fixed_result = GiantCycleStitcher::repair_until_fixed_point(&cycles, &g, &protected_edges);
    assert_eq!(fixed_result.len(), 1);
    verify_valid_cycle(&fixed_result[0], &g, &expected);
}

#[test]
fn test_absorb_16_cycle_gadgets() {
    let mut g = Graph::new();
    // Giant cycle: 50 vertices (1..=50)
    for i in 1..=50 {
        g.add_edge(i, if i == 50 { 1 } else { i + 1 });
    }

    // Gadget 1: 16-cycle (51..=66)
    for i in 51..=66 {
        g.add_edge(i, if i == 66 { 51 } else { i + 1 });
    }
    // Cross edges to giant (10, 11)
    g.add_edge(10, 51);
    g.add_edge(11, 52);

    // Gadget 2: 16-cycle (67..=82)
    for i in 67..=82 {
        g.add_edge(i, if i == 82 { 67 } else { i + 1 });
    }
    // Cross edges to giant (30, 31)
    g.add_edge(30, 67);
    g.add_edge(31, 68);

    let cycles = vec![
        (1..=50).collect::<Vec<i32>>(),
        (51..=66).collect::<Vec<i32>>(),
        (67..=82).collect::<Vec<i32>>(),
    ];
    let protected_edges = HashSet::new();

    let result = GiantCycleStitcher::absorb_into_giant_cycle(&cycles, &g, &protected_edges, 4);
    assert_eq!(result.len(), 1, "Expected both 16-cycle gadgets to be absorbed into giant cycle");
    let expected: Vec<i32> = (1..=82).collect();
    verify_valid_cycle(&result[0], &g, &expected);
}

#[test]
fn test_protected_edge_preservation() {
    let mut g = Graph::new();
    // Giant cycle: 30 vertices (1..=30)
    for i in 1..=30 {
        g.add_edge(i, if i == 30 { 1 } else { i + 1 });
    }

    // Small cycle: 4 vertices (31..=34)
    g.add_edge(31, 32);
    g.add_edge(32, 33);
    g.add_edge(33, 34);
    g.add_edge(34, 31);

    // First pair of cross-edges at (5, 6) in giant
    g.add_edge(5, 31);
    g.add_edge(6, 32);

    let cycles = vec![
        (1..=30).collect::<Vec<i32>>(),
        vec![31, 32, 33, 34],
    ];

    // Case 1: Protect (5, 6). Since only this cross connection exists, absorption must fail.
    let mut protected_edges = HashSet::new();
    protected_edges.insert((5, 6));

    let res1 = GiantCycleStitcher::absorb_into_giant_cycle(&cycles, &g, &protected_edges, 4);
    assert_eq!(res1.len(), 2, "Absorption should not break protected edge (5, 6)");

    // Case 2: Add alternative cross edges at (15, 16) in giant
    g.add_edge(15, 33);
    g.add_edge(16, 34);

    let res2 = GiantCycleStitcher::absorb_into_giant_cycle(&cycles, &g, &protected_edges, 4);
    assert_eq!(res2.len(), 1, "Should absorb using alternative cross edges while preserving protected edge");

    let tour = &res2[0];
    let n = tour.len();
    let mut contains_protected = false;
    for i in 0..n {
        let u = tour[i];
        let v = tour[(i + 1) % n];
        if (u == 5 && v == 6) || (u == 6 && v == 5) {
            contains_protected = true;
            break;
        }
    }
    assert!(contains_protected, "Protected edge (5, 6) must remain intact in merged cycle");
    let expected: Vec<i32> = (1..=34).collect();
    verify_valid_cycle(tour, &g, &expected);
}

#[test]
fn test_repair_until_fixed_point_chain() {
    let mut g = Graph::new();
    // Giant cycle: 40 vertices (1..=40)
    for i in 1..=40 {
        g.add_edge(i, if i == 40 { 1 } else { i + 1 });
    }

    // Small cycle 1: 4 vertices (41..=44)
    g.add_edge(41, 42);
    g.add_edge(42, 43);
    g.add_edge(43, 44);
    g.add_edge(44, 41);

    // Small cycle 2: 4 vertices (45..=48)
    g.add_edge(45, 46);
    g.add_edge(46, 47);
    g.add_edge(47, 48);
    g.add_edge(48, 45);

    // Cross edges between S1 and S2:
    g.add_edge(42, 45);
    g.add_edge(43, 46);

    // Cross edges between S2 and Giant cycle (at 20, 21):
    g.add_edge(20, 47);
    g.add_edge(21, 48);

    let cycles = vec![
        (1..=40).collect::<Vec<i32>>(),
        vec![41, 42, 43, 44],
        vec![45, 46, 47, 48],
    ];
    let protected = HashSet::new();

    let res = GiantCycleStitcher::repair_until_fixed_point(&cycles, &g, &protected);
    assert_eq!(res.len(), 1, "repair_until_fixed_point should chain S1 into S2 and absorb into Giant");
    let expected: Vec<i32> = (1..=48).collect();
    verify_valid_cycle(&res[0], &g, &expected);
}

#[test]
fn test_fallback_small_giant_cycle() {
    let mut g = Graph::new();
    // Giant cycle: 10 vertices (< 20)
    for i in 1..=10 {
        g.add_edge(i, if i == 10 { 1 } else { i + 1 });
    }

    // Small cycle: 4 vertices (11..=14)
    g.add_edge(11, 12);
    g.add_edge(12, 13);
    g.add_edge(13, 14);
    g.add_edge(14, 11);

    // Cross edges: (2, 11) and (3, 12)
    g.add_edge(2, 11);
    g.add_edge(3, 12);

    let cycles = vec![
        (1..=10).collect::<Vec<i32>>(),
        vec![11, 12, 13, 14],
    ];
    let protected = HashSet::new();

    let res = GiantCycleStitcher::repair_until_fixed_point(&cycles, &g, &protected);
    assert_eq!(res.len(), 1, "Fallback to MacroCycleStitcher should stitch small giant cycle");
    let expected: Vec<i32> = (1..=14).collect();
    verify_valid_cycle(&res[0], &g, &expected);
}

#[test]
fn test_absorb_cluster_into_giant_simultaneous_multi_swap() {
    let mut g = Graph::new();
    // Giant cycle: 70 vertices (1..=70 >= 50)
    for i in 1..=70 {
        g.add_edge(i, if i == 70 { 1 } else { i + 1 });
    }

    // Small cycle 1: 3 vertices (71..=73)
    g.add_edge(71, 72);
    g.add_edge(72, 73);
    g.add_edge(73, 71);
    g.add_edge(10, 71);
    g.add_edge(11, 72);

    // Small cycle 2: 3 vertices (74..=76)
    g.add_edge(74, 75);
    g.add_edge(75, 76);
    g.add_edge(76, 74);
    g.add_edge(25, 74);
    g.add_edge(26, 75);

    // Small cycle 3: 4 vertices (77..=80)
    g.add_edge(77, 78);
    g.add_edge(78, 79);
    g.add_edge(79, 80);
    g.add_edge(80, 77);
    g.add_edge(40, 77);
    g.add_edge(41, 78);

    // Small cycle 4: 4 vertices (81..=84)
    g.add_edge(81, 82);
    g.add_edge(82, 83);
    g.add_edge(83, 84);
    g.add_edge(84, 81);
    g.add_edge(55, 81);
    g.add_edge(56, 82);

    let cycles = vec![
        (1..=70).collect::<Vec<i32>>(),
        vec![71, 72, 73],
        vec![74, 75, 76],
        vec![77, 78, 79, 80],
        vec![81, 82, 83, 84],
    ];
    let protected = HashSet::new();

    // 1. Direct absorb_cluster_into_giant with max_swaps = 16
    let result = GiantCycleStitcher::absorb_cluster_into_giant(&cycles, &g, &protected, 16);
    assert_eq!(result.len(), 1, "Expected all 4 subcycles to be absorbed simultaneously into giant cycle");
    let expected: Vec<i32> = (1..=84).collect();
    verify_valid_cycle(&result[0], &g, &expected);

    // 2. repair_until_fixed_point
    let fixed_result = GiantCycleStitcher::repair_until_fixed_point(&cycles, &g, &protected);
    assert_eq!(fixed_result.len(), 1);
    verify_valid_cycle(&fixed_result[0], &g, &expected);
}

#[test]
fn test_absorb_cluster_into_giant_with_max_swaps_32() {
    let mut g = Graph::new();
    // Giant cycle: 100 vertices (1..=100 >= 50)
    for i in 1..=100 {
        g.add_edge(i, if i == 100 { 1 } else { i + 1 });
    }

    // 6 small cycles connected at different points along the giant cycle
    let offsets = [
        (10, 11, vec![101, 102, 103]),
        (25, 26, vec![104, 105, 106, 107]),
        (40, 41, vec![108, 109, 110]),
        (55, 56, vec![111, 112, 113, 114]),
        (70, 71, vec![115, 116, 117]),
        (85, 86, vec![118, 119, 120, 121]),
    ];

    let mut cycles = vec![(1..=100).collect::<Vec<i32>>()];

    for (g_u, g_v, sub) in &offsets {
        let n = sub.len();
        for i in 0..n {
            g.add_edge(sub[i], sub[(i + 1) % n]);
        }
        g.add_edge(*g_u, sub[0]);
        g.add_edge(*g_v, sub[1]);
        cycles.push(sub.clone());
    }

    let protected = HashSet::new();

    // Simultaneous multi-swap with max_swaps = 32
    let result = GiantCycleStitcher::absorb_cluster_into_giant(&cycles, &g, &protected, 32);
    assert_eq!(result.len(), 1, "Expected all 6 subcycles to be absorbed simultaneously with max_swaps=32");
    let expected: Vec<i32> = (1..=121).collect();
    verify_valid_cycle(&result[0], &g, &expected);

    let fixed_result = GiantCycleStitcher::repair_until_fixed_point(&cycles, &g, &protected);
    assert_eq!(fixed_result.len(), 1);
    verify_valid_cycle(&fixed_result[0], &g, &expected);
}

