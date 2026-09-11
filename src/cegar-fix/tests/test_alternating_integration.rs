use cegar_fix::graph::Graph;
use cegar_fix::hybrid_orchestrator::{HybridOptions, HybridOrchestrator};
use cegar_fix::tour_verifier::TourVerifier;

#[test]
fn test_alternating_cli_options_wiring() {
    let mut opts = HybridOptions::default();
    assert!(opts.alternating_engine, "alternating_engine should be true by default");

    opts.alternating_engine = false;
    assert!(!opts.alternating_engine);
}

#[test]
fn test_alternating_engine_solves_small_instance() {
    // 6-cycle graph
    let mut g = Graph::new();
    g.add_edge(0, 1);
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 5);
    g.add_edge(5, 0);

    let mut opts = HybridOptions::default();
    opts.alternating_engine = true;
    opts.timeout_secs = 10.0;

    let tour = HybridOrchestrator::solve(&g, &opts);
    assert!(tour.is_some(), "Hamiltonian tour should be found for 6-cycle");
    let t = tour.unwrap();
    assert_eq!(t.len(), 6);
    assert!(TourVerifier::verify_raw_tour(&t, &g).is_ok());
}
