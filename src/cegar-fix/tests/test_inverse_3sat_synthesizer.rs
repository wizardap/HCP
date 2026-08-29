use cegar_fix::graph::Graph;
use cegar_fix::inverse_3sat_synthesizer::{DeReducedClause, DeReducedVariable, Inverse3SatSynthesizer};
use cegar_fix::tour_verifier::TourVerifier;

/// Helper to construct the 3-SAT reduction graph for (x1 \/ x2) /\ (~x1 \/ x2)
fn build_synthetic_2var_formula_graph() -> Graph {
    let mut g = Graph::new();

    // Variable x1 gadget: vertices 1..=6
    // Ports: A1 = 1, B1 = 6
    // Rung 1 (positive for C1): (2, 3)
    // Rung 2 (negative for C2): (4, 5)
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 5);
    g.add_edge(5, 6);

    // Variable x2 gadget: vertices 7..=12
    // Ports: A2 = 7, B2 = 12
    // Rung 1 (positive for C1): (8, 9)
    // Rung 2 (positive for C2): (10, 11)
    g.add_edge(7, 8);
    g.add_edge(8, 9);
    g.add_edge(9, 10);
    g.add_edge(10, 11);
    g.add_edge(11, 12);

    // Clause 1 (x1 \/ x2): clause node 13
    // Literal x1 (positive): hooks (2, 13) and (13, 3)
    // Literal x2 (positive): hooks (8, 13) and (13, 9)
    g.add_edge(2, 13);
    g.add_edge(13, 3);
    g.add_edge(8, 13);
    g.add_edge(13, 9);

    // Clause 2 (~x1 \/ x2): clause node 14
    // Literal ~x1 (negative): hooks (5, 14) and (14, 4)
    // Literal x2 (positive): hooks (10, 14) and (14, 11)
    g.add_edge(5, 14);
    g.add_edge(14, 4);
    g.add_edge(10, 14);
    g.add_edge(14, 11);

    // Boundary connector edges between V1 {1, 6} and V2 {7, 12}
    g.add_edge(1, 7);
    g.add_edge(1, 12);
    g.add_edge(6, 7);
    g.add_edge(6, 12);

    g
}

/// Helper to construct a 3-variable 3-SAT reduction graph for:
/// (x1 \/ x2 \/ ~x3) /\ (~x1 \/ ~x2 \/ x3)
fn build_synthetic_3var_3sat_graph() -> Graph {
    let mut g = Graph::new();

    // V1 (x1): vertices 1..=6, ports {1, 6}
    // Rung 1 (pos for C1): (2, 3)
    // Rung 2 (neg for C2): (4, 5)
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 5);
    g.add_edge(5, 6);

    // V2 (x2): vertices 7..=12, ports {7, 12}
    // Rung 1 (pos for C1): (8, 9)
    // Rung 2 (neg for C2): (10, 11)
    g.add_edge(7, 8);
    g.add_edge(8, 9);
    g.add_edge(9, 10);
    g.add_edge(10, 11);
    g.add_edge(11, 12);

    // V3 (x3): vertices 13..=18, ports {13, 18}
    // Rung 1 (neg for C1): (14, 15) -> enter 15, exit 14
    // Rung 2 (pos for C2): (16, 17) -> enter 16, exit 17
    g.add_edge(13, 14);
    g.add_edge(14, 15);
    g.add_edge(15, 16);
    g.add_edge(16, 17);
    g.add_edge(17, 18);

    // Clause 1 (x1 \/ x2 \/ ~x3): clause node 19
    g.add_edge(2, 19);
    g.add_edge(19, 3);
    g.add_edge(8, 19);
    g.add_edge(19, 9);
    g.add_edge(15, 19);
    g.add_edge(19, 14);

    // Clause 2 (~x1 \/ ~x2 \/ x3): clause node 20
    g.add_edge(5, 20);
    g.add_edge(20, 4);
    g.add_edge(11, 20);
    g.add_edge(20, 10);
    g.add_edge(16, 20);
    g.add_edge(20, 17);

    // Boundary edges forming ring V1 -> V2 -> V3 -> V1
    // V1 <-> V2
    g.add_edge(1, 7);
    g.add_edge(1, 12);
    g.add_edge(6, 7);
    g.add_edge(6, 12);

    // V2 <-> V3
    g.add_edge(7, 13);
    g.add_edge(7, 18);
    g.add_edge(12, 13);
    g.add_edge(12, 18);

    // V3 <-> V1
    g.add_edge(13, 1);
    g.add_edge(13, 6);
    g.add_edge(18, 1);
    g.add_edge(18, 6);

    g
}

#[test]
fn test_dereduced_structs_api() {
    let var = DeReducedVariable {
        var_id: 0,
        vertices: vec![1, 2, 3, 4, 5, 6],
        port_in: 1,
        port_out: 6,
        true_path: vec![1, 2, 3, 4, 5, 6],
        false_path: vec![6, 5, 4, 3, 2, 1],
    };
    assert_eq!(var.var_id, 0);
    assert_eq!(var.port_in, 1);
    assert_eq!(var.port_out, 6);
    assert_eq!(var.true_path.len(), 6);
    assert_eq!(var.false_path.len(), 6);

    let clause = DeReducedClause {
        clause_id: 0,
        clause_vertices: vec![13],
        literal_hooks: vec![(0, true, 2, 3), (1, true, 8, 9)],
    };
    assert_eq!(clause.clause_id, 0);
    assert_eq!(clause.clause_vertices, vec![13]);
    assert_eq!(clause.literal_hooks.len(), 2);
}

#[test]
fn test_inverse_3sat_on_synthetic_formula() {
    let g = build_synthetic_2var_formula_graph();
    assert_eq!(g.adjacency_list.len(), 14);

    let tour_opt = Inverse3SatSynthesizer::try_solve_via_inverse_3sat(&g);
    assert!(tour_opt.is_some(), "Synthesizer failed to solve synthetic 2-var 3-SAT graph");

    let tour = tour_opt.unwrap();
    assert_eq!(tour.len(), 14, "Tour length must equal total vertices in graph");

    let verify_res = TourVerifier::verify_raw_tour(&tour, &g);
    assert!(
        verify_res.is_ok(),
        "Tour verification failed: {:?}",
        verify_res.err()
    );
}

#[test]
fn test_inverse_3sat_on_synthetic_3var_formula() {
    let g = build_synthetic_3var_3sat_graph();
    assert_eq!(g.adjacency_list.len(), 20);

    let tour_opt = Inverse3SatSynthesizer::try_solve_via_inverse_3sat(&g);
    assert!(tour_opt.is_some(), "Synthesizer failed to solve synthetic 3-var 3-SAT graph");

    let tour = tour_opt.unwrap();
    assert_eq!(tour.len(), 20, "Tour length must equal total vertices in graph");

    let verify_res = TourVerifier::verify_raw_tour(&tour, &g);
    assert!(
        verify_res.is_ok(),
        "Tour verification failed: {:?}",
        verify_res.err()
    );
}

#[test]
fn test_inverse_3sat_on_non_3sat_graph() {
    let mut g = Graph::new();
    // A simple 4-cycle graph (not a 3-SAT reduction graph)
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 1);

    let tour_opt = Inverse3SatSynthesizer::try_solve_via_inverse_3sat(&g);
    // Should return None gracefully without panicking
    assert!(tour_opt.is_none());
}
