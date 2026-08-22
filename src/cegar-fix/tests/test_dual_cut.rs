use cegar_fix::dual_cut_generator::DualCutGenerator;
use cegar_fix::encoder::Encoder;
use cegar_fix::graph::Graph;
use cegar_fix::staged_subcycle_filter::Subcycle;

#[test]
fn test_direct_exclusion_and_boundary_cut_generation() {
    let mut g = Graph::new();
    // 4-node graph: Triangle 1-2-3-1 connected to node 4 via (3,4) and (4,1)
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 1);
    g.add_edge(3, 4);
    g.add_edge(4, 1);

    let mut encoder = Encoder::new();
    let _ = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let subcycle = Subcycle {
        vertices: vec![1, 2, 3],
        edges: vec![(1, 2), (2, 3), (3, 1)],
    };

    // Direct exclusion clause: ¬x_{1->2} ∨ ¬x_{2->3} ∨ ¬x_{3->1}
    let direct_cut = DualCutGenerator::generate_direct_exclusion_clause(&subcycle, &encoder)
        .expect("Direct cut should be generated");

    let lit_1_2 = encoder.graph_lit_map[&(1, 2)];
    let lit_2_3 = encoder.graph_lit_map[&(2, 3)];
    let lit_3_1 = encoder.graph_lit_map[&(3, 1)];

    let mut expected_direct_lits = vec![!lit_1_2, !lit_2_3, !lit_3_1];
    expected_direct_lits.sort_unstable();
    expected_direct_lits.dedup();

    let direct_lits: Vec<_> = direct_cut.into_iter().collect();
    assert_eq!(direct_lits, expected_direct_lits);

    // Boundary cut clause: outgoing arcs from {1, 2, 3} to {4} => (1, 4) and (3, 4)
    let boundary_cut = DualCutGenerator::generate_boundary_cut_clause(&subcycle, &g, &encoder)
        .expect("Boundary cut should be generated");

    let lit_1_4 = encoder.graph_lit_map[&(1, 4)];
    let lit_3_4 = encoder.graph_lit_map[&(3, 4)];
    let mut expected_boundary_lits = vec![lit_1_4, lit_3_4];
    expected_boundary_lits.sort_unstable();
    expected_boundary_lits.dedup();

    let boundary_lits: Vec<_> = boundary_cut.into_iter().collect();
    assert_eq!(boundary_lits, expected_boundary_lits);

    // Dual cuts should return both clauses
    let dual_cuts = DualCutGenerator::generate_dual_cuts(&subcycle, &g, &encoder);
    assert_eq!(dual_cuts.len(), 2);
}

#[test]
fn test_boundary_cut_multiple_outgoing_edges() {
    let mut g = Graph::new();
    // Subcycle {1, 2} with multiple outgoing edges to {3, 4}
    g.add_edge(1, 2);
    g.add_edge(1, 3);
    g.add_edge(2, 4);
    g.add_edge(3, 4);

    let mut encoder = Encoder::new();
    let _ = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let subcycle = Subcycle {
        vertices: vec![1, 2],
        edges: vec![(1, 2), (2, 1)],
    };

    let boundary_cut = DualCutGenerator::generate_boundary_cut_clause(&subcycle, &g, &encoder)
        .expect("Boundary cut should exist");

    let lit_1_3 = encoder.graph_lit_map[&(1, 3)];
    let lit_2_4 = encoder.graph_lit_map[&(2, 4)];

    let mut expected_lits = vec![lit_1_3, lit_2_4];
    expected_lits.sort_unstable();
    expected_lits.dedup();

    let boundary_lits: Vec<_> = boundary_cut.into_iter().collect();
    assert_eq!(boundary_lits, expected_lits);
}

#[test]
fn test_dual_cut_edge_cases() {
    let mut g = Graph::new();
    // Disconnected component: triangle 1-2-3-1 and edge 4-5
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 1);
    g.add_edge(4, 5);

    let mut encoder = Encoder::new();
    let _ = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let subcycle = Subcycle {
        vertices: vec![1, 2, 3],
        edges: vec![(1, 2), (2, 3), (3, 1)],
    };

    // Subcycle has no outgoing edges to rest of graph => boundary cut is None
    let boundary_cut = DualCutGenerator::generate_boundary_cut_clause(&subcycle, &g, &encoder);
    assert!(boundary_cut.is_none());

    let direct_cut = DualCutGenerator::generate_direct_exclusion_clause(&subcycle, &encoder);
    assert!(direct_cut.is_some());

    let dual_cuts = DualCutGenerator::generate_dual_cuts(&subcycle, &g, &encoder);
    assert_eq!(dual_cuts.len(), 1);

    // Empty subcycle
    let empty_subcycle = Subcycle {
        vertices: vec![],
        edges: vec![],
    };
    assert!(DualCutGenerator::generate_direct_exclusion_clause(&empty_subcycle, &encoder).is_none());
    assert!(DualCutGenerator::generate_boundary_cut_clause(&empty_subcycle, &g, &encoder).is_none());
    assert!(DualCutGenerator::generate_dual_cuts(&empty_subcycle, &g, &encoder).is_empty());
}
