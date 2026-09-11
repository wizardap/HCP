use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::graph::Graph;
use cegar_fix::hcp_solver;
use cegar_fix::hub_registry::HubRegistry;
use cegar_fix::hybrid_orchestrator::{HybridOptions, HybridOrchestrator};
use cegar_fix::tour_verifier::TourVerifier;
use std::time::Instant;

fn make_prism8() -> Graph {
    let mut g = Graph::new();
    // 8-vertex prism: outer 1-2-3-4, inner 5-6-7-8, rungs 1-5, 2-6, 3-7, 4-8
    // All vertices have degree 3, preventing trivial degree-2 contraction
    for i in 1..=4 {
        g.add_edge(i, if i == 4 { 1 } else { i + 1 });
        g.add_edge(i + 4, if i == 4 { 5 } else { i + 5 });
        g.add_edge(i, i + 4);
    }
    g
}

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

#[test]
fn test_macro_gadget_toggle_non_trivial_graph() {
    let g = make_prism8();

    // Verify macro_gadget = false (Baseline / C0) routes and solves via CEGAR
    let mut opts_off = HybridOptions::default();
    opts_off.macro_gadget = false;
    assert!(!opts_off.macro_gadget);
    let tour_off = HybridOrchestrator::solve(&g, &opts_off);
    assert!(tour_off.is_some(), "Expected tour with macro_gadget disabled (C0)");
    let t_off = tour_off.unwrap();
    assert_eq!(t_off.len(), 8);
    assert!(TourVerifier::verify_raw_tour(&t_off, &g).is_ok());

    // Verify macro_gadget = true (H1 enabled) routes and solves via CEGAR
    let mut opts_on = HybridOptions::default();
    opts_on.macro_gadget = true;
    assert!(opts_on.macro_gadget);
    let tour_on = HybridOrchestrator::solve(&g, &opts_on);
    assert!(tour_on.is_some(), "Expected tour with macro_gadget enabled (H1)");
    let t_on = tour_on.unwrap();
    assert_eq!(t_on.len(), 8);
    assert!(TourVerifier::verify_raw_tour(&t_on, &g).is_ok());
}

#[test]
fn test_solve_hamilton_macro_gadget_parameter_gating() {
    let g = make_prism8();
    let (contracted_g, contractor) = Degree2Contractor::contract(&g);
    let hub_reg = HubRegistry::new(&contracted_g);

    // Test direct solve_hamilton with macro_gadget = 0 (baseline gating off)
    let start0 = Instant::now();
    let tour_gated_off = hcp_solver::solve_hamilton(
        contracted_g.clone(),
        &contractor,
        &hub_reg,
        0, 1, 3, 2, 3, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 60, 200,
        10.0,
        start0,
        "",
        0, // macro_gadget = 0
    );
    assert!(tour_gated_off.is_some(), "Direct solve_hamilton with macro_gadget=0 must find tour");
    let t0 = tour_gated_off.unwrap();
    assert!(TourVerifier::verify_raw_tour(&t0, &g).is_ok());

    // Test direct solve_hamilton with macro_gadget = 1 (H1 gating on)
    let start1 = Instant::now();
    let tour_gated_on = hcp_solver::solve_hamilton(
        contracted_g,
        &contractor,
        &hub_reg,
        0, 1, 3, 2, 3, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 60, 200,
        10.0,
        start1,
        "",
        1, // macro_gadget = 1
    );
    assert!(tour_gated_on.is_some(), "Direct solve_hamilton with macro_gadget=1 must find tour");
    let t1 = tour_gated_on.unwrap();
    assert!(TourVerifier::verify_raw_tour(&t1, &g).is_ok());
}
