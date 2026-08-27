use cegar_fix::graph::Graph;
use cegar_fix::staged_lazy_smt_solver::{solve_staged_lazy_smt, StagedLazySmtOptions};

#[test]
fn test_solve_staged_smt_on_simple_cycle() {
    let mut g = Graph::new();
    // 5-cycle graph: 1-2-3-4-5-1 with chord 1-3
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 5);
    g.add_edge(5, 1);
    g.add_edge(1, 3);

    let options = StagedLazySmtOptions {
        max_batch_size: 500,
        timeout_secs: 10.0,
        output_path: None,
    };

    let tour = solve_staged_lazy_smt(&g, &options);
    assert!(tour.is_some());
    let t = tour.unwrap();
    assert_eq!(t.len(), 5);
}

#[test]
fn test_solve_staged_smt_unsat_disconnected() {
    let mut g = Graph::new();
    // Two disconnected triangles: (1, 2, 3) and (4, 5, 6)
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 1);
    g.add_edge(4, 5);
    g.add_edge(5, 6);
    g.add_edge(6, 4);

    let options = StagedLazySmtOptions {
        max_batch_size: 500,
        timeout_secs: 5.0,
        output_path: None,
    };

    let tour = solve_staged_lazy_smt(&g, &options);
    assert!(tour.is_none(), "Disconnected graph should be UNSAT");
}

#[test]
fn test_solve_staged_smt_ladder_graph() {
    let mut g = Graph::new();
    // Ladder graph with 8 vertices (4 rungs)
    // 1-2-3-4
    // | | | |
    // 5-6-7-8
    for i in 1..4 {
        g.add_edge(i, i + 1);
        g.add_edge(i + 4, i + 5);
    }
    for i in 1..=4 {
        g.add_edge(i, i + 4);
    }

    let options = StagedLazySmtOptions {
        max_batch_size: 10,
        timeout_secs: 10.0,
        output_path: None,
    };

    let tour = solve_staged_lazy_smt(&g, &options);
    assert!(tour.is_some());
    let t = tour.unwrap();
    assert_eq!(t.len(), 8);
}

#[test]
fn test_solve_staged_smt_petersen_subgraph() {
    let mut g = Graph::new();
    // Petersen graph has no Hamiltonian cycle
    // Outer 5-cycle: 1-2-3-4-5-1
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 5);
    g.add_edge(5, 1);
    // Inner star: 6-8-10-7-9-6
    g.add_edge(6, 8);
    g.add_edge(8, 10);
    g.add_edge(10, 7);
    g.add_edge(7, 9);
    g.add_edge(9, 6);
    // Radii: 1-6, 2-7, 3-8, 4-9, 5-10
    g.add_edge(1, 6);
    g.add_edge(2, 7);
    g.add_edge(3, 8);
    g.add_edge(4, 9);
    g.add_edge(5, 10);

    let options = StagedLazySmtOptions {
        max_batch_size: 50,
        timeout_secs: 10.0,
        output_path: None,
    };

    let tour = solve_staged_lazy_smt(&g, &options);
    assert!(tour.is_none(), "Petersen graph is hypohamiltonian and must be UNSAT");
}

#[test]
fn test_cegar_adaptive_backbone_freezing_integration() {
    use cegar_fix::hcp_solver::solve_hamilton;
    use cegar_fix::contraction::Degree2Contractor;
    use cegar_fix::hub_registry::HubRegistry;
    use std::time::Instant;

    let mut g = Graph::new();
    // 20-node cycle with chords
    for i in 1..20 {
        g.add_edge(i, i + 1);
    }
    g.add_edge(20, 1);
    g.add_edge(1, 10);
    g.add_edge(10, 20);
    g.add_edge(5, 15);

    let (contracted_g, contractor) = Degree2Contractor::contract(&g);
    let hub_reg = HubRegistry::new(&contracted_g);
    let start = Instant::now();

    let tour = solve_hamilton(
        contracted_g,
        &contractor,
        &hub_reg,
        0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 100, 10.0, start, "output"
    );
    assert!(tour.is_some(), "Graph must be solvable with CEGAR and adaptive freezing");
    let t = tour.unwrap();
    assert_eq!(t.len(), 20);
}


