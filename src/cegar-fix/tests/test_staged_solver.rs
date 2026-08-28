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

#[test]
fn test_cegar_solver_reseeder_integration() {
    use cegar_fix::hcp_solver::solve_hamilton;
    use cegar_fix::contraction::Degree2Contractor;
    use cegar_fix::hub_registry::HubRegistry;
    use std::time::Instant;

    let mut g = Graph::new();
    // 30-node cycle with cross-connections
    for i in 1..30 {
        g.add_edge(i, i + 1);
    }
    g.add_edge(30, 1);
    g.add_edge(1, 15);
    g.add_edge(15, 30);
    g.add_edge(5, 20);
    g.add_edge(10, 25);

    let (contracted_g, contractor) = Degree2Contractor::contract(&g);
    let hub_reg = HubRegistry::new(&contracted_g);
    let start = Instant::now();

    let tour = solve_hamilton(
        contracted_g,
        &contractor,
        &hub_reg,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 100, 10.0, start, "default"
    );
    assert!(tour.is_some(), "Graph must be solved with CEGAR and solver reseeder wired");
    let t = tour.unwrap();
    assert_eq!(t.len(), 30);
}

#[test]
fn test_cegar_hemisphere_splicer_integration() {
    use cegar_fix::hcp_solver::solve_hamilton;
    use cegar_fix::contraction::Degree2Contractor;
    use cegar_fix::hub_registry::HubRegistry;
    use std::time::Instant;

    let mut g = Graph::new();
    // Two 10-node components with chords (all degree >= 3)
    // Component 1: 1..=10
    for i in 1..10 {
        g.add_edge(i, i + 1);
    }
    g.add_edge(10, 1);
    g.add_edge(1, 4);
    g.add_edge(2, 5);
    g.add_edge(3, 6);
    g.add_edge(4, 7);
    g.add_edge(5, 8);
    g.add_edge(6, 9);
    g.add_edge(7, 10);
    g.add_edge(8, 1);
    g.add_edge(9, 2);
    g.add_edge(10, 3);

    // Component 2: 11..=20
    for i in 11..20 {
        g.add_edge(i, i + 1);
    }
    g.add_edge(20, 11);
    g.add_edge(11, 14);
    g.add_edge(12, 15);
    g.add_edge(13, 16);
    g.add_edge(14, 17);
    g.add_edge(15, 18);
    g.add_edge(16, 19);
    g.add_edge(17, 20);
    g.add_edge(18, 11);
    g.add_edge(19, 12);
    g.add_edge(20, 13);

    // Cross-hemisphere connecting edges for 2-opt splice
    g.add_edge(1, 11);
    g.add_edge(2, 12);

    let (contracted_g, contractor) = Degree2Contractor::contract(&g);
    let hub_reg = HubRegistry::new(&contracted_g);
    let start = Instant::now();

    let tour = solve_hamilton(
        contracted_g,
        &contractor,
        &hub_reg,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 100, 10.0, start, "default"
    );
    assert!(tour.is_some(), "Graph with two hemispheres must be solved via HemisphereSplicer/CEGAR");
    let t = tour.unwrap();
    assert_eq!(t.len(), 20);

    // Verify validity of Hamiltonian cycle on g
    let mut seen = std::collections::HashSet::new();
    for &v in &t {
        assert!(seen.insert(v), "Duplicate vertex {} in tour", v);
    }
    for i in 0..t.len() {
        let u = t[i];
        let v = t[(i + 1) % t.len()];
        assert!(
            g.adjacency_list.get(&u).map_or(false, |nbrs| nbrs.contains(&v)),
            "Edge ({}, {}) must exist in graph",
            u,
            v
        );
    }
}

#[test]
fn test_cegar_static_cycle_cutter_integration() {
    use cegar_fix::hcp_solver::solve_hamilton;
    use cegar_fix::contraction::Degree2Contractor;
    use cegar_fix::hub_registry::HubRegistry;
    use cegar_fix::static_cycle_cutter::StaticCycleCutter;
    use cegar_fix::encoder::Encoder;
    use std::time::Instant;

    let mut g = Graph::new();
    // 8-vertex 3-regular ladder / 3-cube graph with 6 4-cycles:
    // Cycle A: 1-2-3-4-1
    // Cycle B: 5-6-7-8-5
    // Rungs: (1,5), (2,6), (3,7), (4,8)
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 1);

    g.add_edge(5, 6);
    g.add_edge(6, 7);
    g.add_edge(7, 8);
    g.add_edge(8, 5);

    g.add_edge(1, 5);
    g.add_edge(2, 6);
    g.add_edge(3, 7);
    g.add_edge(4, 8);

    let (contracted_g, contractor) = Degree2Contractor::contract(&g);
    let hub_reg = HubRegistry::new(&contracted_g);

    // Verify static cuts generated
    let mut encoder = Encoder::new();
    let _ = encoder.encode(&contracted_g, 0, 0, 0, 0, 0, 0);
    let static_cuts = StaticCycleCutter::generate_static_small_cycle_cuts(&contracted_g, &encoder);
    assert!(!static_cuts.is_empty(), "Static cuts should detect 4-cycles in 3-cube graph");

    let start = Instant::now();
    let tour = solve_hamilton(
        contracted_g,
        &contractor,
        &hub_reg,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 100, 10.0, start, "default"
    );
    assert!(tour.is_some(), "Graph must be solved with CEGAR and static cycle cutter wired");
    let t = tour.unwrap();
    assert_eq!(t.len(), 8);

    // Verify validity of Hamiltonian cycle on g
    let mut seen = std::collections::HashSet::new();
    for &v in &t {
        assert!(seen.insert(v), "Duplicate vertex {} in tour", v);
    }
    for i in 0..t.len() {
        let u = t[i];
        let v = t[(i + 1) % t.len()];
        assert!(
            g.adjacency_list.get(&u).map_or(false, |nbrs| nbrs.contains(&v)),
            "Edge ({}, {}) must exist in graph",
            u,
            v
        );
    }
}

