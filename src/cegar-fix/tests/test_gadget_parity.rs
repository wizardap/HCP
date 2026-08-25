use cegar_fix::graph::Graph;
use cegar_fix::encoder::Encoder;
use cegar_fix::gadget_parity::GadgetInterfaceParityEngine;
use cegar_fix::tour_verifier::TourVerifier;

#[test]
fn test_gadget_parity_internal_hamiltonian_paths() {
    let mut g = Graph::new();
    // Giant cycle: 1 - 2 - 3 - 4 - 5 - 1
    g.add_edge(1, 2); g.add_edge(2, 3); g.add_edge(3, 4); g.add_edge(4, 5); g.add_edge(5, 1);
    
    // Gadget: 10 - 11 - 12 - 13 - 10
    g.add_edge(10, 11); g.add_edge(11, 12); g.add_edge(12, 13); g.add_edge(13, 10);
    
    // Interface ports: 10 connects to 1, 11 connects to 2
    g.add_edge(10, 1);
    g.add_edge(11, 2);
    
    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);
    
    let giant = vec![1, 2, 3, 4, 5];
    let gadget = vec![10, 11, 12, 13];
    
    let result = GadgetInterfaceParityEngine::analyze_subcycle_gadget(&gadget, &g, Some(&giant), &encoder);
    
    // Direct splice should succeed since 1 and 2 are adjacent on giant cycle
    assert!(result.direct_spliced_tour.is_some(), "Should directly splice gadget into adjacent giant cycle nodes");
    let tour = result.direct_spliced_tour.unwrap();
    assert_eq!(tour.len(), 9);
    
    // Soundness check with TourVerifier
    let verify_res = TourVerifier::verify_raw_tour(&tour, &g);
    assert!(verify_res.is_ok(), "Direct spliced tour must be a valid Hamiltonian cycle: {:?}", verify_res);
}

#[test]
fn test_gadget_infeasible_port_pruning() {
    let mut g = Graph::new();
    // Gadget with non-Hamiltonian port pair: Star-like gadget with center 20 and leaves 21, 22, 23
    g.add_edge(20, 21); g.add_edge(20, 22); g.add_edge(20, 23);
    // External connections from leaves
    g.add_edge(21, 1); g.add_edge(22, 2); g.add_edge(23, 3);
    // Complete remaining graph so encoder has all vertices
    g.add_edge(1, 2); g.add_edge(2, 3); g.add_edge(3, 1);
    
    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);
    
    let gadget = vec![20, 21, 22, 23];
    let result = GadgetInterfaceParityEngine::analyze_subcycle_gadget(&gadget, &g, None, &encoder);
    
    // Path visiting all 4 nodes must enter at one leaf and exit at another leaf passing through center, which is impossible in a star graph with 3 leaves
    assert!(!result.pruning_clauses.is_empty(), "Should generate pruning clauses for infeasible port pairs");
    assert!(!result.cut_parity_clauses.is_empty(), "Should generate cut parity boundary clauses");
}

#[test]
fn test_gadget_parity_reverse_adjacency_splice() {
    let mut g = Graph::new();
    // Giant cycle: 1 - 2 - 3 - 4 - 5 - 1
    g.add_edge(1, 2); g.add_edge(2, 3); g.add_edge(3, 4); g.add_edge(4, 5); g.add_edge(5, 1);
    
    // Gadget: 10 - 11 - 12 - 13 - 10
    g.add_edge(10, 11); g.add_edge(11, 12); g.add_edge(12, 13); g.add_edge(13, 10);
    
    // Interface ports: 10 connects to 2, 11 connects to 1
    g.add_edge(10, 2);
    g.add_edge(11, 1);
    
    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);
    
    let giant = vec![1, 2, 3, 4, 5];
    let gadget = vec![10, 11, 12, 13];
    
    let result = GadgetInterfaceParityEngine::analyze_subcycle_gadget(&gadget, &g, Some(&giant), &encoder);
    
    assert!(result.direct_spliced_tour.is_some());
    let tour = result.direct_spliced_tour.unwrap();
    assert_eq!(tour.len(), 9);
    assert!(TourVerifier::verify_raw_tour(&tour, &g).is_ok());
}

#[test]
fn test_gadget_parity_boundary_cases() {
    let mut g = Graph::new();
    g.add_edge(1, 2); g.add_edge(2, 3); g.add_edge(3, 1);
    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    // Too small gadget (< 3)
    let small_gadget = vec![1, 2];
    let res = GadgetInterfaceParityEngine::analyze_subcycle_gadget(&small_gadget, &g, None, &encoder);
    assert!(res.direct_spliced_tour.is_none());
    assert!(res.pruning_clauses.is_empty());
    assert!(res.cut_parity_clauses.is_empty());
}

#[test]
fn test_gadget_parity_wraparound_adjacency_splice() {
    let mut g = Graph::new();
    // Giant cycle: 1 - 2 - 3 - 4 - 5 - 1 (indices 0..5, where index 4 connects to index 0)
    g.add_edge(1, 2); g.add_edge(2, 3); g.add_edge(3, 4); g.add_edge(4, 5); g.add_edge(5, 1);
    
    // Gadget: 10 - 11 - 12 - 13 - 10
    g.add_edge(10, 11); g.add_edge(11, 12); g.add_edge(12, 13); g.add_edge(13, 10);
    
    // Interface ports connect to wrap-around edge: 10 connects to 5 (pos 4), 11 connects to 1 (pos 0)
    g.add_edge(10, 5);
    g.add_edge(11, 1);
    
    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);
    
    let giant = vec![1, 2, 3, 4, 5];
    let gadget = vec![10, 11, 12, 13];
    
    let result = GadgetInterfaceParityEngine::analyze_subcycle_gadget(&gadget, &g, Some(&giant), &encoder);
    
    assert!(result.direct_spliced_tour.is_some(), "Should directly splice gadget on wrap-around boundary (5 <-> 1)");
    let tour = result.direct_spliced_tour.unwrap();
    assert_eq!(tour.len(), 9, "Tour must contain exactly 9 unique vertices without duplication");
    let verify_res = TourVerifier::verify_raw_tour(&tour, &g);
    assert!(verify_res.is_ok(), "Wrap-around spliced tour must be valid: {:?}", verify_res);
}
