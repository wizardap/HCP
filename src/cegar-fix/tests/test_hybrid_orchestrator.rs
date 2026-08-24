use cegar_fix::graph::Graph;
use cegar_fix::hybrid_orchestrator::{HybridOptions, HybridOrchestrator};
use cegar_fix::tour_verifier::TourVerifier;
use std::fs;

#[test]
fn test_hybrid_orchestrator_synthetic_solve() {
    let mut g = Graph::new();
    // 6-cycle: 1 - 2 - 3 - 4 - 5 - 6 - 1
    for i in 1..=6 {
        g.add_edge(i, if i == 6 { 1 } else { i + 1 });
    }

    let opts = HybridOptions {
        auto_mode: true,
        timeout_secs: 10.0,
        output_tour: None,
    };

    let tour = HybridOrchestrator::solve(&g, &opts);
    assert!(tour.is_some(), "Expected solver to find tour for 6-cycle");
    let t = tour.unwrap();
    assert!(
        TourVerifier::verify_raw_tour(&t, &g).is_ok(),
        "Tour must pass raw graph verification"
    );
}

#[test]
fn test_hybrid_orchestrator_b2_sinz_route_and_output() {
    let mut g = Graph::new();
    // Small 3-regular Hamiltonian graph (Petersen-like or 8-vertex cubic)
    // 8-vertex prism graph: outer 1-2-3-4, inner 5-6-7-8, rungs 1-5, 2-6, 3-7, 4-8
    for i in 1..=4 {
        g.add_edge(i, if i == 4 { 1 } else { i + 1 });
        g.add_edge(i + 4, if i == 4 { 5 } else { i + 5 });
        g.add_edge(i, i + 4);
    }

    let out_file = "scratch/test_hybrid_out.hcp";
    let opts = HybridOptions {
        auto_mode: false, // Force B2/General SMT track
        timeout_secs: 10.0,
        output_tour: Some(out_file.to_string()),
    };

    let tour = HybridOrchestrator::solve(&g, &opts);
    assert!(tour.is_some(), "Expected solver to find tour for 8-vertex prism");
    let t = tour.unwrap();
    assert_eq!(t.len(), 8);
    assert!(TourVerifier::verify_raw_tour(&t, &g).is_ok());

    // Verify output file written
    assert!(fs::metadata(out_file).is_ok(), "Output tour file should exist");
    let _ = fs::remove_file(out_file);
}

#[test]
fn test_hybrid_orchestrator_infeasible_graph() {
    let mut g = Graph::new();
    // Disconnected graph: two separate triangles
    g.add_edge(1, 2); g.add_edge(2, 3); g.add_edge(3, 1);
    g.add_edge(4, 5); g.add_edge(5, 6); g.add_edge(6, 4);

    let opts = HybridOptions {
        auto_mode: true,
        timeout_secs: 5.0,
        output_tour: None,
    };

    let tour = HybridOrchestrator::solve(&g, &opts);
    assert!(tour.is_none(), "Disconnected graph should be UNSAT");
}

#[test]
fn test_hybrid_orchestrator_default_options() {
    let opts = HybridOptions::default();
    assert!(opts.auto_mode);
    assert_eq!(opts.timeout_secs, 1800.0);
    assert!(opts.output_tour.is_none());
}

#[test]
fn test_hybrid_orchestrator_b1_ladder_synthetic() {
    let mut g = Graph::new();
    // Create ladder structure with 60 high-degree hubs and strips
    // Hubs: 1..=60. Strip vertices: 100..200
    for h in 1..=60 {
        let next_h = if h == 60 { 1 } else { h + 1 };
        g.add_edge(h, next_h);
        for v in 100..120 {
            g.add_edge(h, v);
        }
    }
    // Also inter-strip connections
    for v in 100..119 {
        g.add_edge(v, v + 1);
    }
    g.add_edge(119, 100);

    let opts = HybridOptions {
        auto_mode: true,
        timeout_secs: 10.0,
        output_tour: None,
    };

    // Features should classify as B1LadderTwoTier
    let feat = cegar_fix::auto_classifier::AutoTopologyClassifier::extract_features(&g);
    assert_eq!(
        cegar_fix::auto_classifier::AutoTopologyClassifier::classify(&feat),
        cegar_fix::auto_classifier::TargetTrack::B1LadderTwoTier
    );
}

