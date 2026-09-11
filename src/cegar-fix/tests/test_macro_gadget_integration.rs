use cegar_fix::graph::Graph;
use cegar_fix::hybrid_orchestrator::{HybridOptions, HybridOrchestrator};
use cegar_fix::tour_verifier::TourVerifier;

#[test]
fn test_macro_gadget_option_activation() {
    let mut opts = HybridOptions::default();
    opts.macro_gadget = true;
    assert!(opts.macro_gadget);

    // Construct a small 6-cycle graph to verify solve works with macro_gadget enabled
    let mut g = Graph::new();
    for i in 1..=6 {
        g.add_edge(i, if i == 6 { 1 } else { i + 1 });
    }

    let tour = HybridOrchestrator::solve(&g, &opts);
    assert!(tour.is_some(), "Expected tour with macro_gadget enabled");
    let t = tour.unwrap();
    assert!(TourVerifier::verify_raw_tour(&t, &g).is_ok());
}
