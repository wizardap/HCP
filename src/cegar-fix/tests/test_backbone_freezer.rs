use cegar_fix::graph::Graph;
use cegar_fix::encoder::Encoder;
use cegar_fix::backbone_freezer::BackboneFreezer;

#[test]
fn test_backbone_freezer_extraction() {
    let mut g = Graph::new();
    // Giant cycle: 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7 -> 8 -> 1
    for i in 1..=7 {
        g.add_edge(i, i + 1);
    }
    g.add_edge(8, 1);

    // Small cycle: 9 -> 10 -> 9
    g.add_edge(9, 10);

    // Connection from small cycle to giant cycle only at node 1
    g.add_edge(1, 9);

    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let giant = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let small = vec![9, 10];
    let cycles = vec![giant, small];

    // Nodes 1, 2, 8 are near the boundary (connected to 9).
    // Nodes 4 -> 5 -> 6 should be deep in the internal backbone.
    let assumps = BackboneFreezer::extract_backbone_assumptions(&cycles, &g, &encoder, 0.5, 25);

    assert!(!assumps.is_empty(), "Internal backbone edges should be extracted");
}

#[test]
fn test_backbone_freezer_giant_cycle_preservation_6_cycles() {
    let mut g = Graph::new();

    // 1 giant cycle of 80 nodes: 1 -> 2 -> ... -> 80 -> 1
    for i in 1..80 {
        g.add_edge(i, i + 1);
    }
    g.add_edge(80, 1);

    // 5 small cycles of 4 nodes each:
    // Cycle 1: 81..=84
    // Cycle 2: 85..=88
    // Cycle 3: 89..=92
    // Cycle 4: 93..=96
    // Cycle 5: 97..=100
    let mut cycles = Vec::new();
    let giant: Vec<i32> = (1..=80).collect();
    cycles.push(giant);

    for base in [81, 85, 89, 93, 97] {
        for i in 0..3 {
            g.add_edge(base + i, base + i + 1);
        }
        g.add_edge(base + 3, base);
        cycles.push((base..base + 4).collect());
    }

    assert_eq!(cycles.len(), 6, "Must have exactly 6 cycles");

    // Add cross edges connecting each small cycle to specific nodes on the giant cycle
    g.add_edge(1, 81);
    g.add_edge(10, 85);
    g.add_edge(20, 89);
    g.add_edge(30, 93);
    g.add_edge(40, 97);

    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    // When max_cycle_count_trigger is 25 and min_giant_ratio is 0.50,
    // the 80-node giant cycle (80% >= 50%) with 6 cycles (<= 25) should yield assumptions.
    let assumps_25 = BackboneFreezer::extract_backbone_assumptions(&cycles, &g, &encoder, 0.50, 25);
    assert!(!assumps_25.is_empty(), "Internal backbone edges should be extracted for 6 cycles when trigger is 25");

    // Verify that internal edges far from boundary nodes (e.g. 50 -> 51, 60 -> 61, 70 -> 71) are locked
    let lit_50_51 = encoder.graph_lit_map.get(&(50, 51)).copied();
    assert!(lit_50_51.is_some());
    assert!(assumps_25.contains(&lit_50_51.unwrap()), "Edge (50, 51) should be in assumptions");

    // When max_cycle_count_trigger is 5, but giant cycle is 80% (>= 50%),
    // the giant cycle ratio should override the count trigger and yield assumptions!
    let assumps_giant_override = BackboneFreezer::extract_backbone_assumptions(&cycles, &g, &encoder, 0.50, 5);
    assert!(!assumps_giant_override.is_empty(), "Giant cycle >= 50% must override max_cycle_count_trigger");

    // When neither condition is met (min_giant_ratio = 0.90 > 80% AND trigger = 5 < 6 cycles),
    // it should return empty.
    let assumps_none = BackboneFreezer::extract_backbone_assumptions(&cycles, &g, &encoder, 0.90, 5);
    assert!(assumps_none.is_empty(), "No assumptions when neither giant ratio nor cycle count threshold is met");
}

#[test]
fn test_backbone_freezer_boundary_exclusion() {
    let mut g = Graph::new();
    // Giant cycle: 1 -> 2 -> ... -> 10 -> 1
    for i in 1..10 {
        g.add_edge(i, i + 1);
    }
    g.add_edge(10, 1);

    // Small cycle: 11 -> 12 -> 13 -> 14 -> 11
    for i in 11..14 {
        g.add_edge(i, i + 1);
    }
    g.add_edge(14, 11);

    // Cross edge connecting node 5 on giant cycle to node 11 on small cycle
    g.add_edge(5, 11);

    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let giant: Vec<i32> = (1..=10).collect();
    let small: Vec<i32> = (11..=14).collect();
    let cycles = vec![giant, small];

    let assumps = BackboneFreezer::extract_backbone_assumptions(&cycles, &g, &encoder, 0.50, 25);

    // Boundary buffer of 1 around node 5 means nodes 4, 5, 6 are boundary.
    // Edges touching boundary nodes: (3, 4), (4, 5), (5, 6), (6, 7) must be excluded.
    let lit_3_4 = encoder.graph_lit_map.get(&(3, 4)).copied().unwrap();
    let lit_4_5 = encoder.graph_lit_map.get(&(4, 5)).copied().unwrap();
    let lit_5_6 = encoder.graph_lit_map.get(&(5, 6)).copied().unwrap();
    let lit_6_7 = encoder.graph_lit_map.get(&(6, 7)).copied().unwrap();

    assert!(!assumps.contains(&lit_3_4), "Edge (3, 4) should be excluded due to boundary buffer");
    assert!(!assumps.contains(&lit_4_5), "Edge (4, 5) should be excluded due to boundary buffer");
    assert!(!assumps.contains(&lit_5_6), "Edge (5, 6) should be excluded due to boundary buffer");
    assert!(!assumps.contains(&lit_6_7), "Edge (6, 7) should be excluded due to boundary buffer");

    // Internal edges: (1, 2), (2, 3), (7, 8), (8, 9), (9, 10), (10, 1) must be included.
    let lit_1_2 = encoder.graph_lit_map.get(&(1, 2)).copied().unwrap();
    let lit_2_3 = encoder.graph_lit_map.get(&(2, 3)).copied().unwrap();
    let lit_7_8 = encoder.graph_lit_map.get(&(7, 8)).copied().unwrap();
    let lit_8_9 = encoder.graph_lit_map.get(&(8, 9)).copied().unwrap();
    let lit_9_10 = encoder.graph_lit_map.get(&(9, 10)).copied().unwrap();
    let lit_10_1 = encoder.graph_lit_map.get(&(10, 1)).copied().unwrap();

    assert!(assumps.contains(&lit_1_2), "Edge (1, 2) should be included");
    assert!(assumps.contains(&lit_2_3), "Edge (2, 3) should be included");
    assert!(assumps.contains(&lit_7_8), "Edge (7, 8) should be included");
    assert!(assumps.contains(&lit_8_9), "Edge (8, 9) should be included");
    assert!(assumps.contains(&lit_9_10), "Edge (9, 10) should be included");
    assert!(assumps.contains(&lit_10_1), "Edge (10, 1) should be included");
}
