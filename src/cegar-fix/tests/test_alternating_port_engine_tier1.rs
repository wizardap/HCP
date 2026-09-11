use cegar_fix::graph::Graph;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::alternating_port_engine::{AlternatingPortEngine, Port};
use cegar_fix::tour_verifier::TourVerifier;

#[test]
fn test_port_creation_and_properties() {
    let p0 = Port { block: 0, end: 0 };
    let p1 = Port { block: 0, end: 1 };
    assert_ne!(p0, p1);
    assert_eq!(p0.block, p1.block);
}

#[test]
fn test_synthetic_bipartite_alternating_merge() {
    // Build a graph with 2 disjoint 4-cycles on 8 vertices (0..8)
    // contracted into 4 blocks of size 2.
    // Block 0: (0, 1), Block 1: (2, 3), Block 2: (4, 5), Block 3: (6, 7)
    // Cycle 1: 0 - 1 - 2 - 3 - 0
    // Cycle 2: 4 - 5 - 6 - 7 - 4
    // Cross edges available: (1, 4), (5, 2), (3, 6), (7, 0)
    let mut g = Graph::new();
    // Block internal edges
    g.add_edge(0, 1);
    g.add_edge(2, 3);
    g.add_edge(4, 5);
    g.add_edge(6, 7);

    // Cycle 1 external edges
    g.add_edge(1, 2);
    g.add_edge(3, 0);

    // Cycle 2 external edges
    g.add_edge(5, 6);
    g.add_edge(7, 4);

    // Cross inactive edges that allow alternating merge
    g.add_edge(1, 4);
    g.add_edge(5, 2);

    let (contracted_g, contractor) = Degree2Contractor::contract(&g);

    let initial_cycles = vec![
        vec![0, 1, 2, 3],
        vec![4, 5, 6, 7],
    ];

    let merged = AlternatingPortEngine::repair_tier1(&initial_cycles, &contracted_g, &contractor);
    assert_eq!(merged.len(), 1, "Tier 1 alternating flips should merge the 2 cycles into 1");
    assert_eq!(merged[0].len(), 8, "Merged cycle should contain all 8 vertices");
    assert!(TourVerifier::verify_raw_tour(&merged[0], &g).is_ok(), "Merged tour must be valid on raw graph g");
}



