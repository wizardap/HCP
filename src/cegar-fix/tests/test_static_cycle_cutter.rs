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
