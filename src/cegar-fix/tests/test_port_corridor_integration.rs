use cegar_fix::graph::Graph;
use cegar_fix::hybrid_orchestrator::{HybridOptions, HybridOrchestrator};
use cegar_fix::tour_verifier::TourVerifier;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::encoder::Encoder;
use cegar_fix::port_corridor_lns::PortCorridorLns;
use rustsat::instances::Cnf;

#[test]
fn test_port_corridor_cli_options_wiring() {
    let mut opts = HybridOptions::default();
    assert!(opts.alternating_engine, "alternating_engine should be enabled by default");

    opts.alternating_engine = false;
    assert!(!opts.alternating_engine, "alternating_engine should be disableable");
}

#[test]
fn test_port_corridor_repair_edge_cases() {
    let g = Graph::new();
    let (_, contractor) = Degree2Contractor::contract(&g);
    let encoder = Encoder::new();
    let cnf = Cnf::new();

    // 0 cycles
    let empty: Vec<Vec<i32>> = vec![];
    let res = PortCorridorLns::repair(&empty, &g, &contractor, &encoder, &cnf);
    assert!(res.is_empty());

    // 1 cycle
    let single = vec![vec![1, 2, 3]];
    let res = PortCorridorLns::repair(&single, &g, &contractor, &encoder, &cnf);
    assert_eq!(res, single);
}

#[test]
fn test_port_corridor_integration_solves_cycle_graph() {
    // 8-cycle graph
    let mut g = Graph::new();
    for i in 0..8 {
        g.add_edge(i, (i + 1) % 8);
    }

    let mut opts = HybridOptions::default();
    opts.alternating_engine = true;
    opts.timeout_secs = 10.0;

    let tour = HybridOrchestrator::solve(&g, &opts);
    assert!(tour.is_some(), "Hamiltonian tour should be found for 8-cycle");
    let t = tour.unwrap();
    assert_eq!(t.len(), 8);
    assert!(TourVerifier::verify_raw_tour(&t, &g).is_ok());
}

#[test]
fn test_port_corridor_integration_with_degree2_chains() {
    // Graph with degree-2 paths to ensure Degree2Contractor contracts edges into chain_map
    // 3 blocks of degree-2 paths:
    // 0 - 1 - 2 - 3 - 0 (block 0)
    // with cross links to other blocks:
    // 0 - 1 - 2 - 3
    // 4 - 5 - 6 - 7
    // 8 - 9 - 10 - 11
    // Connect into a big cycle with chords:
    let n = 12;
    let mut g = Graph::new();
    for i in 0..n {
        g.add_edge(i, (i + 1) % n);
    }
    // Add non-trivial chords to make it 3-regular where possible
    g.add_edge(0, 6);
    g.add_edge(2, 8);
    g.add_edge(4, 10);

    let mut opts = HybridOptions::default();
    opts.alternating_engine = true;
    opts.timeout_secs = 15.0;

    let tour = HybridOrchestrator::solve(&g, &opts);
    assert!(tour.is_some(), "Hamiltonian tour should be found for contracted chordal ring");
    let t = tour.unwrap();
    assert_eq!(t.len(), n as usize);
    assert!(TourVerifier::verify_raw_tour(&t, &g).is_ok());
}

#[test]
fn test_port_corridor_preserves_disable_flag() {
    // Verify when alternating_engine is false, standard CEGAR solving functions normally
    let mut g = Graph::new();
    for i in 0..6 {
        g.add_edge(i, (i + 1) % 6);
    }

    let mut opts = HybridOptions::default();
    opts.alternating_engine = false;
    opts.timeout_secs = 10.0;

    let tour = HybridOrchestrator::solve(&g, &opts);
    assert!(tour.is_some(), "Hamiltonian tour should be found even when alternating_engine is disabled");
    let t = tour.unwrap();
    assert_eq!(t.len(), 6);
    assert!(TourVerifier::verify_raw_tour(&t, &g).is_ok());
}
