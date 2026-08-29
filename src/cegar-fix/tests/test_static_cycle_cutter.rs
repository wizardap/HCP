use cegar_fix::graph::Graph;
use cegar_fix::encoder::Encoder;
use cegar_fix::static_cycle_cutter::StaticCycleCutter;

#[test]
fn test_detects_triangles_and_squares() {
    let mut g = Graph::new();
    // Triangle: 1 - 2 - 3 - 1
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 1);

    // Square: 3 - 4 - 5 - 6 - 3
    g.add_edge(3, 4);
    g.add_edge(4, 5);
    g.add_edge(5, 6);
    g.add_edge(6, 3);

    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let cuts = StaticCycleCutter::generate_static_small_cycle_cuts(&g, &encoder);

    // 1 triangle generates 2 directional clauses (length 3)
    // 1 square generates 2 directional clauses (length 4)
    // Total 4 clauses
    assert_eq!(cuts.len(), 4, "Expected 4 clauses (2 for triangle, 2 for square)");

    let l_12 = *encoder.graph_lit_map.get(&(1, 2)).unwrap();
    let l_23 = *encoder.graph_lit_map.get(&(2, 3)).unwrap();
    let l_31 = *encoder.graph_lit_map.get(&(3, 1)).unwrap();
    let l_13 = *encoder.graph_lit_map.get(&(1, 3)).unwrap();
    let l_32 = *encoder.graph_lit_map.get(&(3, 2)).unwrap();
    let l_21 = *encoder.graph_lit_map.get(&(2, 1)).unwrap();

    let l_34 = *encoder.graph_lit_map.get(&(3, 4)).unwrap();
    let l_45 = *encoder.graph_lit_map.get(&(4, 5)).unwrap();
    let l_56 = *encoder.graph_lit_map.get(&(5, 6)).unwrap();
    let l_63 = *encoder.graph_lit_map.get(&(6, 3)).unwrap();
    let l_36 = *encoder.graph_lit_map.get(&(3, 6)).unwrap();
    let l_65 = *encoder.graph_lit_map.get(&(6, 5)).unwrap();
    let l_54 = *encoder.graph_lit_map.get(&(5, 4)).unwrap();
    let l_43 = *encoder.graph_lit_map.get(&(4, 3)).unwrap();

    let clauses: Vec<Vec<rustsat::types::Lit>> = cuts
        .into_iter()
        .map(|c| c.into_iter().collect())
        .collect();

    let expected_tri_1 = vec![!l_12, !l_23, !l_31];
    let expected_tri_2 = vec![!l_13, !l_32, !l_21];
    let expected_sq_1 = vec![!l_34, !l_45, !l_56, !l_63];
    let expected_sq_2 = vec![!l_36, !l_65, !l_54, !l_43];

    assert!(clauses.contains(&expected_tri_1), "Missing triangle cut 1");
    assert!(clauses.contains(&expected_tri_2), "Missing triangle cut 2");
    assert!(clauses.contains(&expected_sq_1), "Missing square cut 1");
    assert!(clauses.contains(&expected_sq_2), "Missing square cut 2");
}

#[test]
fn test_bypasses_small_graphs() {
    let mut g = Graph::new();
    // 4-cycle graph
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 1);

    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let cuts = StaticCycleCutter::generate_static_small_cycle_cuts(&g, &encoder);
    assert_eq!(cuts.len(), 0, "Graphs with <= 4 vertices should produce 0 cuts");
}

#[test]
fn test_detects_6_cycles() {
    let mut g = Graph::new();
    // 6-cycle: 1 - 2 - 3 - 4 - 5 - 6 - 1
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 5);
    g.add_edge(5, 6);
    g.add_edge(6, 1);

    // Extra vertex 7 to make total_v = 7 > 6
    // Connect 7 to 1 and 4 (forming two 5-cycles 1-7-4-3-2-1 and 1-7-4-5-6-1, but no 3-cycles, 4-cycles, or other 6-cycles)
    g.add_edge(1, 7);
    g.add_edge(7, 4);

    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let cuts = StaticCycleCutter::generate_static_small_cycle_cuts(&g, &encoder);

    // Should detect the 6-cycle and generate 2 directional clauses
    assert_eq!(cuts.len(), 2, "Expected 2 clauses for the 6-cycle");

    let l_12 = *encoder.graph_lit_map.get(&(1, 2)).unwrap();
    let l_23 = *encoder.graph_lit_map.get(&(2, 3)).unwrap();
    let l_34 = *encoder.graph_lit_map.get(&(3, 4)).unwrap();
    let l_45 = *encoder.graph_lit_map.get(&(4, 5)).unwrap();
    let l_56 = *encoder.graph_lit_map.get(&(5, 6)).unwrap();
    let l_61 = *encoder.graph_lit_map.get(&(6, 1)).unwrap();

    let l_16 = *encoder.graph_lit_map.get(&(1, 6)).unwrap();
    let l_65 = *encoder.graph_lit_map.get(&(6, 5)).unwrap();
    let l_54 = *encoder.graph_lit_map.get(&(5, 4)).unwrap();
    let l_43 = *encoder.graph_lit_map.get(&(4, 3)).unwrap();
    let l_32 = *encoder.graph_lit_map.get(&(3, 2)).unwrap();
    let l_21 = *encoder.graph_lit_map.get(&(2, 1)).unwrap();

    let clauses: Vec<Vec<rustsat::types::Lit>> = cuts
        .into_iter()
        .map(|c| c.into_iter().collect())
        .collect();

    let expected_6_1 = vec![!l_12, !l_23, !l_34, !l_45, !l_56, !l_61];
    let expected_6_2 = vec![!l_16, !l_65, !l_54, !l_43, !l_32, !l_21];

    assert!(clauses.contains(&expected_6_1), "Missing 6-cycle cut direction 1");
    assert!(clauses.contains(&expected_6_2), "Missing 6-cycle cut direction 2");
}

#[test]
fn test_bypasses_6_cycle_in_6_vertex_graph() {
    let mut g = Graph::new();
    // 6-cycle graph with exactly 6 vertices
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 5);
    g.add_edge(5, 6);
    g.add_edge(6, 1);

    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let cuts = StaticCycleCutter::generate_static_small_cycle_cuts(&g, &encoder);
    assert_eq!(cuts.len(), 0, "6-cycle in a 6-vertex graph should produce 0 cuts (total_v <= 6)");
}

#[test]
fn test_6_cycle_cap() {
    let mut g = Graph::new();
    // Create 2,100 disjoint chordless 6-cycles to test 4,000 clause cap
    // 2,100 * 2 directional clauses = 4,200 clauses, capped at 4,000
    for k in 0..2100 {
        let base = 1 + k * 6;
        let (a, b, c, d, e, f) = (base, base + 1, base + 2, base + 3, base + 4, base + 5);
        g.add_edge(a, b);
        g.add_edge(b, c);
        g.add_edge(c, d);
        g.add_edge(d, e);
        g.add_edge(e, f);
        g.add_edge(f, a);
    }

    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let cuts = StaticCycleCutter::generate_static_small_cycle_cuts(&g, &encoder);
    assert_eq!(cuts.len(), 4000, "Expected 4000 capped 6-cycle clauses");
}

#[test]
fn test_detects_7_and_8_cycles() {
    let mut g = Graph::new();
    // 7-cycle (1..7) and 8-cycle (10..17) in a 20-vertex graph
    for i in 1..=7 {
        let nxt = if i == 7 { 1 } else { i + 1 };
        g.add_edge(i, nxt);
    }
    for i in 10..=17 {
        let nxt = if i == 17 { 10 } else { i + 1 };
        g.add_edge(i, nxt);
    }
    // Extra vertices to ensure total_v > 8
    g.add_edge(18, 19);
    g.add_edge(19, 20);

    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let cuts = StaticCycleCutter::generate_static_small_cycle_cuts(&g, &encoder);
    // 1 7-cycle * 2 directional cuts + 1 8-cycle * 2 directional cuts = 4 cuts
    assert_eq!(cuts.len(), 4, "Expected 2 directional cuts for 7-cycle and 2 for 8-cycle");
}

#[test]
fn test_detects_12_and_16_cycles() {
    let mut g = Graph::new();
    // Induced 12-cycle: 1..=12
    for i in 1..=12 {
        let nxt = if i == 12 { 1 } else { i + 1 };
        g.add_edge(i, nxt);
    }
    // Induced 16-cycle: 20..=35
    for i in 20..=35 {
        let nxt = if i == 35 { 20 } else { i + 1 };
        g.add_edge(i, nxt);
    }
    // Extra vertices to make total_v > 35
    g.add_edge(40, 41);

    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let cuts = StaticCycleCutter::generate_static_small_cycle_cuts(&g, &encoder);

    // Should detect the 12-cycle (2 clauses) and 16-cycle (2 clauses)
    let clauses: Vec<Vec<rustsat::types::Lit>> = cuts
        .into_iter()
        .map(|c| c.into_iter().collect())
        .collect();

    // Check 12-cycle clauses
    let l_1_2 = *encoder.graph_lit_map.get(&(1, 2)).unwrap();
    let l_12_1 = *encoder.graph_lit_map.get(&(12, 1)).unwrap();
    let l_1_12 = *encoder.graph_lit_map.get(&(1, 12)).unwrap();
    let l_2_1 = *encoder.graph_lit_map.get(&(2, 1)).unwrap();

    let has_12_fwd = clauses.iter().any(|c| c.len() == 12 && c.contains(&!l_1_2) && c.contains(&!l_12_1));
    let has_12_rev = clauses.iter().any(|c| c.len() == 12 && c.contains(&!l_1_12) && c.contains(&!l_2_1));
    assert!(has_12_fwd, "Missing 12-cycle forward clause");
    assert!(has_12_rev, "Missing 12-cycle reverse clause");

    // Check 16-cycle clauses
    let l_20_21 = *encoder.graph_lit_map.get(&(20, 21)).unwrap();
    let l_35_20 = *encoder.graph_lit_map.get(&(35, 20)).unwrap();
    let l_20_35 = *encoder.graph_lit_map.get(&(20, 35)).unwrap();
    let l_21_20 = *encoder.graph_lit_map.get(&(21, 20)).unwrap();

    let has_16_fwd = clauses.iter().any(|c| c.len() == 16 && c.contains(&!l_20_21) && c.contains(&!l_35_20));
    let has_16_rev = clauses.iter().any(|c| c.len() == 16 && c.contains(&!l_20_35) && c.contains(&!l_21_20));
    assert!(has_16_fwd, "Missing 16-cycle forward clause");
    assert!(has_16_rev, "Missing 16-cycle reverse clause");
}

#[test]
fn test_detects_all_lengths_9_to_16() {
    let mut g = Graph::new();
    let mut base = 100;
    // Add one chordless cycle for each length L in 9..=16
    for len in 9..=16 {
        for i in 0..len {
            let u = base + i;
            let v = base + ((i + 1) % len);
            g.add_edge(u, v);
        }
        base += len + 10;
    }
    // Extra edge to ensure total_v > 200
    g.add_edge(base, base + 1);

    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let cuts = StaticCycleCutter::generate_static_small_cycle_cuts(&g, &encoder);

    for len in 9..=16 {
        let count_len = cuts.iter().filter(|c| c.len() == len).count();
        assert_eq!(count_len, 2, "Expected 2 directional clauses for {}-cycle, got {}", len, count_len);
    }
}

#[test]
fn test_chord_prevents_extended_cycle() {
    let mut g = Graph::new();
    // 16-cycle: 1..=16
    for i in 1..=16 {
        let nxt = if i == 16 { 1 } else { i + 1 };
        g.add_edge(i, nxt);
    }
    // Add chord between 1 and 8 (splitting into an 8-cycle and a 10-cycle)
    g.add_edge(1, 8);
    // Extra vertex
    g.add_edge(20, 21);

    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let cuts = StaticCycleCutter::generate_static_small_cycle_cuts(&g, &encoder);

    // The 16-cycle is not chordless, so no length-16 clauses should be generated
    let count_16 = cuts.iter().filter(|c| c.len() == 16).count();
    assert_eq!(count_16, 0, "Chorded 16-cycle should NOT generate 16-cycle clauses");

    // But the chordless 8-cycle (1-2-3-4-5-6-7-8-1) and 10-cycle (1-8-9-10-11-12-13-14-15-16-1) should be found!
    let count_8 = cuts.iter().filter(|c| c.len() == 8).count();
    let count_10 = cuts.iter().filter(|c| c.len() == 10).count();
    assert_eq!(count_8, 2, "Expected 2 clauses for the induced 8-cycle");
    assert_eq!(count_10, 2, "Expected 2 clauses for the induced 10-cycle");
}

#[test]
fn test_extended_cycle_cap() {
    let mut g = Graph::new();
    // Create 1,500 disjoint chordless 10-cycles (3,000 clauses)
    // and 1,500 disjoint chordless 16-cycles (3,000 clauses)
    for k in 0..1500 {
        let base = 1 + k * 20;
        for i in 0..10 {
            g.add_edge(base + i, base + ((i + 1) % 10));
        }
    }

    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let cuts = StaticCycleCutter::generate_static_small_cycle_cuts(&g, &encoder);
    // Budget cap per length is 2000 (or total extended cap <= 15000)
    let count_10 = cuts.iter().filter(|c| c.len() == 10).count();
    assert!(count_10 <= 2000, "10-cycle clauses ({}) exceeded cap 2000", count_10);
    assert_eq!(count_10, 2000, "10-cycle clauses should hit cap of 2000");
}

#[test]
fn test_performance_on_large_graph() {
    use std::time::Instant;

    let mut g = Graph::new();
    // Build a 3000-vertex cubic-like ring with cross-chords
    let n = 3000;
    for i in 1..=n {
        let nxt = if i == n { 1 } else { i + 1 };
        g.add_edge(i, nxt);
        // Add a cross edge every 16 vertices forming various cycles
        if i % 16 == 0 {
            let target = if i + 15 <= n { i + 15 } else { (i + 15) % n + 1 };
            g.add_edge(i, target);
        }
    }

    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let start = Instant::now();
    let cuts = StaticCycleCutter::generate_static_small_cycle_cuts(&g, &encoder);
    let elapsed = start.elapsed();

    println!("Generated {} cuts in {:?}", cuts.len(), elapsed);
    assert!(elapsed.as_millis() < 250, "Static cut generation took {:?}, exceeding 250ms limit", elapsed);
}
