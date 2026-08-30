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

#[test]
fn test_cegar_boundary_alternating_patcher_integration() {
    use cegar_fix::hcp_solver::solve_hamilton;
    use cegar_fix::contraction::Degree2Contractor;
    use cegar_fix::hub_registry::HubRegistry;
    use std::time::Instant;

    let mut g = Graph::new();
    // Cycle 1: 1-2-3-4-5-6-1
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 5);
    g.add_edge(5, 6);
    g.add_edge(6, 1);

    // Cycle 2: 7-8-9-10-11-12-7
    g.add_edge(7, 8);
    g.add_edge(8, 9);
    g.add_edge(9, 10);
    g.add_edge(10, 11);
    g.add_edge(11, 12);
    g.add_edge(12, 7);

    // Internal Chords
    g.add_edge(2, 6);
    g.add_edge(9, 11);

    // Cross-hemisphere connecting edges for multi-hop alternating patch
    g.add_edge(1, 7);
    g.add_edge(3, 8);
    g.add_edge(4, 12);
    g.add_edge(5, 10);

    let (contracted_g, contractor) = Degree2Contractor::contract(&g);
    let hub_reg = HubRegistry::new(&contracted_g);
    let start = Instant::now();

    let tour = solve_hamilton(
        contracted_g,
        &contractor,
        &hub_reg,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 100, 10.0, start, "default"
    );
    assert!(tour.is_some(), "Graph with two macro-hemispheres must be solved via BoundaryAlternatingPatcher/CEGAR");
    let t = tour.unwrap();
    assert_eq!(t.len(), 12);

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
fn test_fast_fail_assumptions_integration() {
    use cegar_fix::hcp_solver::solve_hamilton;
    use cegar_fix::contraction::Degree2Contractor;
    use cegar_fix::hub_registry::HubRegistry;
    use std::time::Instant;

    let mut g = Graph::new();
    // 24-node cycle with chords inducing subcycles and exercising assumption conflict limiting
    for i in 1..24 {
        g.add_edge(i, i + 1);
    }
    g.add_edge(24, 1);
    g.add_edge(1, 8);
    g.add_edge(8, 16);
    g.add_edge(16, 24);
    g.add_edge(4, 12);
    g.add_edge(12, 20);
    g.add_edge(20, 4);

    let (contracted_g, contractor) = Degree2Contractor::contract(&g);
    let hub_reg = HubRegistry::new(&contracted_g);
    let start = Instant::now();

    let tour = solve_hamilton(
        contracted_g,
        &contractor,
        &hub_reg,
        0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 100, 10.0, start, "default"
    );
    assert!(tour.is_some(), "Graph must be solved with fast-fail assumption conflict limiting");
    let t = tour.unwrap();
    assert_eq!(t.len(), 24);

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
fn test_cegar_metagraph_router_integration() {
    use cegar_fix::hcp_solver::solve_hamilton;
    use cegar_fix::contraction::Degree2Contractor;
    use cegar_fix::hub_registry::HubRegistry;
    use cegar_fix::metagraph_router::MetagraphRouter;
    use std::time::Instant;

    let mut g = Graph::new();
    // 4 K4 gadget modules arranged in a ring with inter-module connecting edges
    // Module 0: 1, 2, 3, 4
    // Module 1: 5, 6, 7, 8
    // Module 2: 9, 10, 11, 12
    // Module 3: 13, 14, 15, 16
    for m in 0..4 {
        let base = m * 4 + 1;
        for i in 0..4 {
            for j in (i + 1)..4 {
                g.add_edge(base + i, base + j);
            }
        }
    }

    // Inter-module ring connections
    g.add_edge(4, 5);
    g.add_edge(8, 9);
    g.add_edge(12, 13);
    g.add_edge(16, 1);

    // Cross chords between modules to provide alternative paths and test MTZ pruning
    g.add_edge(3, 6);
    g.add_edge(7, 10);
    g.add_edge(11, 14);
    g.add_edge(15, 2);

    let (contracted_g, contractor) = Degree2Contractor::contract(&g);
    let hub_reg = HubRegistry::new(&contracted_g);

    let modules = MetagraphRouter::detect_gadget_modules(&contracted_g);
    assert_eq!(modules.len(), 4, "Expected 4 gadget modules in contracted graph");

    let start = Instant::now();
    let tour = solve_hamilton(
        contracted_g,
        &contractor,
        &hub_reg,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 100, 10.0, start, "default"
    );
    assert!(tour.is_some(), "Graph with 4 gadget modules must be solved via MetagraphRouter and CEGAR");
    let t = tour.unwrap();
    assert_eq!(t.len(), 16);

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
fn test_cegar_dual_channel_router_integration() {
    use cegar_fix::hcp_solver::solve_hamilton;
    use cegar_fix::contraction::Degree2Contractor;
    use cegar_fix::hub_registry::HubRegistry;
    use cegar_fix::metagraph_router::MetagraphRouter;
    use std::time::Instant;

    let mut g = Graph::new();
    // 3 gadget modules with 14 vertices each (total 42 vertices)
    // Module vertices > 12 will trigger splitting into 2 channels per module (6 channels total)
    // C0: 1..=7, C1: 8..=14
    // C2: 15..=21, C3: 22..=28
    // C4: 29..=35, C5: 36..=42
    for m in 0..3 {
        let base = m * 14;
        // Channel A vertices (all degree >= 3 to avoid contraction)
        for i in 1..=6 {
            g.add_edge(base + i, base + i + 1);
        }
        for i in 1..=5 {
            g.add_edge(base + i, base + i + 2);
        }

        // Channel B vertices (all degree >= 3 to avoid contraction)
        for i in 8..=13 {
            g.add_edge(base + i, base + i + 1);
        }
        for i in 8..=12 {
            g.add_edge(base + i, base + i + 2);
        }

        // Internal intra-module bridge between Channel A and Channel B
        g.add_edge(base + 7, base + 8);
        g.add_edge(base + 14, base + 1);
    }

    // Inter-module bridges forming 2-pass tour across 6 channels:
    // Pass 1: C0 -> C2 -> C4
    g.add_edge(7, 15);
    g.add_edge(21, 29);
    g.add_edge(35, 8);
    // Pass 2: C1 -> C3 -> C5 -> C0
    g.add_edge(14, 22);
    g.add_edge(28, 36);
    g.add_edge(42, 1);

    let (contracted_g, contractor) = Degree2Contractor::contract(&g);
    let hub_reg = HubRegistry::new(&contracted_g);

    let channels = MetagraphRouter::detect_dual_channels(&contracted_g);
    assert_eq!(channels.len(), 6, "Expected 6 dual-channel modules (3 gadgets * 2 channels)");

    let start = Instant::now();
    let tour = solve_hamilton(
        contracted_g,
        &contractor,
        &hub_reg,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 100, 10.0, start, "default"
    );
    assert!(tour.is_some(), "Graph with dual-channel modules must be solved via DualChannelRouter and CEGAR");
    let t = tour.unwrap();
    assert_eq!(t.len(), 42);

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
fn test_cegar_parallel_sat_portfolio_integration() {
    use cegar_fix::hcp_solver::{solve_hamilton, get_solution_arcs_from_lits};
    use cegar_fix::contraction::Degree2Contractor;
    use cegar_fix::hub_registry::HubRegistry;
    use rustsat::types::Lit;
    use std::collections::BTreeMap;
    use std::time::Instant;

    // 1. Test helper get_solution_arcs_from_lits
    let mut lit_map = BTreeMap::new();
    let lit1 = Lit::positive(0);
    let lit2 = Lit::positive(1);
    let lit3 = Lit::positive(2);
    lit_map.insert((1, 2), lit1);
    lit_map.insert((2, 3), lit2);
    lit_map.insert((3, 1), lit3);

    let active_lits = vec![lit1, lit3];
    let arcs = get_solution_arcs_from_lits(&active_lits, &lit_map);
    assert_eq!(arcs.len(), 2);
    assert!(arcs.contains(&(1, 2)));
    assert!(arcs.contains(&(3, 1)));
    assert!(!arcs.contains(&(2, 3)));

    // 2. Test CEGAR solving on graph with multiple subcycles / chords
    let mut g = Graph::new();
    // 30-node cycle with multiple chords forcing CEGAR subcycle cuts
    for i in 1..30 {
        g.add_edge(i, i + 1);
    }
    g.add_edge(30, 1);
    g.add_edge(1, 10);
    g.add_edge(10, 20);
    g.add_edge(20, 30);
    g.add_edge(5, 15);
    g.add_edge(15, 25);

    let (contracted_g, contractor) = Degree2Contractor::contract(&g);
    let hub_reg = HubRegistry::new(&contracted_g);
    let start = Instant::now();

    let tour = solve_hamilton(
        contracted_g,
        &contractor,
        &hub_reg,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 100, 10.0, start, "default"
    );
    assert!(tour.is_some(), "Graph must be solved with ParallelSatPortfolio in CEGAR loop");
    let t = tour.unwrap();
    assert_eq!(t.len(), 30);

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
fn test_cegar_macro_cycle_stitcher_integration() {
    use cegar_fix::macro_cycle_stitcher::MacroCycleStitcher;
    use cegar_fix::hcp_solver::solve_hamilton;
    use cegar_fix::contraction::Degree2Contractor;
    use cegar_fix::hub_registry::HubRegistry;
    use std::collections::HashSet;
    use std::time::Instant;

    // 1. Direct unit test of MacroCycleStitcher on 3-cycle stitching
    // Cycle A: 1-2-3-4-1
    // Cycle B: 5-6-7-8-5
    // Cycle C: 9-10-11-12-9
    let mut g_direct = Graph::new();
    let cycle_a = vec![1, 2, 3, 4];
    let cycle_b = vec![5, 6, 7, 8];
    let cycle_c = vec![9, 10, 11, 12];

    for c in &[&cycle_a, &cycle_b, &cycle_c] {
        let n = c.len();
        for i in 0..n {
            g_direct.add_edge(c[i], c[(i + 1) % n]);
        }
    }
    // Add cross edges allowing 3-cycle alternating symmetric difference stitching
    // (2, 5), (6, 9), (10, 1) connects C1 -> C2 -> C3 -> C1
    g_direct.add_edge(2, 5);
    g_direct.add_edge(6, 9);
    g_direct.add_edge(10, 1);

    let protected = HashSet::new();
    let initial_cycles = vec![cycle_a.clone(), cycle_b.clone(), cycle_c.clone()];
    let stitched = MacroCycleStitcher::stitch_until_fixed_point(&initial_cycles, &g_direct, &protected);
    assert_eq!(stitched.len(), 1, "MacroCycleStitcher should stitch 3 cycles into 1");
    assert_eq!(stitched[0].len(), 12, "Stitched tour must contain all 12 vertices");

    // 2. CEGAR end-to-end solve test with MacroCycleStitcher wired in
    let mut g_cegar = Graph::new();
    // 24-node graph with multiple sub-cycle chords
    for i in 1..24 {
        g_cegar.add_edge(i, i + 1);
    }
    g_cegar.add_edge(24, 1);
    g_cegar.add_edge(1, 8);
    g_cegar.add_edge(8, 16);
    g_cegar.add_edge(16, 24);
    g_cegar.add_edge(4, 12);
    g_cegar.add_edge(12, 20);

    let (contracted_g, contractor) = Degree2Contractor::contract(&g_cegar);
    let hub_reg = HubRegistry::new(&contracted_g);
    let start = Instant::now();

    let tour = solve_hamilton(
        contracted_g,
        &contractor,
        &hub_reg,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 100, 10.0, start, "default"
    );
    assert!(tour.is_some(), "Graph must be solved with MacroCycleStitcher in CEGAR loop");
    let t = tour.unwrap();
    assert_eq!(t.len(), 24);

    let mut seen = HashSet::new();
    for &v in &t {
        assert!(seen.insert(v), "Duplicate vertex {} in tour", v);
    }
    for i in 0..t.len() {
        let u = t[i];
        let v = t[(i + 1) % t.len()];
        assert!(
            g_cegar.adjacency_list.get(&u).map_or(false, |nbrs| nbrs.contains(&v)),
            "Edge ({}, {}) must exist in graph",
            u,
            v
        );
    }
}

#[test]
fn test_cegar_giant_cycle_stitcher_integration() {
    use cegar_fix::giant_cycle_stitcher::GiantCycleStitcher;
    use cegar_fix::hcp_solver::solve_hamilton;
    use cegar_fix::contraction::Degree2Contractor;
    use cegar_fix::hub_registry::HubRegistry;
    use std::collections::HashSet;
    use std::time::Instant;

    // 1. Direct unit test of GiantCycleStitcher targeted giant cycle absorption
    // Giant cycle (len = 24): 1..24
    // Subcycle (len = 4): 25..28
    let mut g_direct = Graph::new();
    let giant_cycle: Vec<i32> = (1..=24).collect();
    let subcycle: Vec<i32> = vec![25, 26, 27, 28];

    let n_g = giant_cycle.len();
    for i in 0..n_g {
        g_direct.add_edge(giant_cycle[i], giant_cycle[(i + 1) % n_g]);
    }
    let n_s = subcycle.len();
    for i in 0..n_s {
        g_direct.add_edge(subcycle[i], subcycle[(i + 1) % n_s]);
    }

    // Add cross edges enabling 2-swap absorption between giant (edge 1-2) and subcycle (edge 25-26)
    g_direct.add_edge(1, 25);
    g_direct.add_edge(2, 26);

    let protected = HashSet::new();
    let initial_cycles = vec![giant_cycle.clone(), subcycle.clone()];
    let stitched = GiantCycleStitcher::repair_until_fixed_point(&initial_cycles, &g_direct, &protected);
    assert_eq!(stitched.len(), 1, "GiantCycleStitcher should absorb subcycle into giant cycle");
    assert_eq!(stitched[0].len(), 28, "Stitched tour must contain all 28 vertices");

    let mut direct_seen = HashSet::new();
    for &v in &stitched[0] {
        assert!(direct_seen.insert(v), "Duplicate vertex {} in direct stitched tour", v);
    }
    for i in 0..stitched[0].len() {
        let u = stitched[0][i];
        let v = stitched[0][(i + 1) % stitched[0].len()];
        assert!(
            g_direct.adjacency_list.get(&u).map_or(false, |nbrs| nbrs.contains(&v)),
            "Direct stitched tour edge ({}, {}) must exist in graph",
            u,
            v
        );
    }

    // 2. CEGAR end-to-end solve test with GiantCycleStitcher wired in
    let mut g_cegar = Graph::new();
    // 28-node graph with subcycle chords
    for i in 1..28 {
        g_cegar.add_edge(i, i + 1);
    }
    g_cegar.add_edge(28, 1);
    // Add subcycle chords
    g_cegar.add_edge(1, 8);
    g_cegar.add_edge(8, 16);
    g_cegar.add_edge(16, 24);
    g_cegar.add_edge(24, 28);
    g_cegar.add_edge(4, 12);
    g_cegar.add_edge(12, 20);

    let (contracted_g, contractor) = Degree2Contractor::contract(&g_cegar);
    let hub_reg = HubRegistry::new(&contracted_g);
    let start = Instant::now();

    let tour = solve_hamilton(
        contracted_g,
        &contractor,
        &hub_reg,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 100, 10.0, start, "default"
    );
    assert!(tour.is_some(), "Graph must be solved with GiantCycleStitcher in CEGAR loop");
    let t = tour.unwrap();
    assert_eq!(t.len(), 28);

    let mut seen = HashSet::new();
    for &v in &t {
        assert!(seen.insert(v), "Duplicate vertex {} in tour", v);
    }
    for i in 0..t.len() {
        let u = t[i];
        let v = t[(i + 1) % t.len()];
        assert!(
            g_cegar.adjacency_list.get(&u).map_or(false, |nbrs| nbrs.contains(&v)),
            "Edge ({}, {}) must exist in graph",
            u,
            v
        );
    }
}

#[test]
fn test_cegar_extended_static_cycle_cutter_integration() {
    use cegar_fix::hcp_solver::solve_hamilton;
    use cegar_fix::contraction::Degree2Contractor;
    use cegar_fix::hub_registry::HubRegistry;
    use cegar_fix::static_cycle_cutter::StaticCycleCutter;
    use cegar_fix::encoder::Encoder;
    use std::time::Instant;

    let mut g = Graph::new();
    // 32-vertex 16-rung ladder graph (inner 16-cycle and outer 16-cycle)
    // Inner ring: 1..=16
    for i in 1..=16 {
        let nxt = if i == 16 { 1 } else { i + 1 };
        g.add_edge(i, nxt);
    }
    // Outer ring: 17..=32
    for i in 17..=32 {
        let nxt = if i == 32 { 17 } else { i + 1 };
        g.add_edge(i, nxt);
    }
    // 16 rungs connecting inner to outer ring
    for i in 1..=16 {
        g.add_edge(i, i + 16);
    }

    let (contracted_g, contractor) = Degree2Contractor::contract(&g);
    let hub_reg = HubRegistry::new(&contracted_g);

    // Verify static cuts include extended 16-cycles
    let mut encoder = Encoder::new();
    let _ = encoder.encode(&contracted_g, 0, 0, 0, 0, 0, 0);
    let static_cuts = StaticCycleCutter::generate_static_small_cycle_cuts(&contracted_g, &encoder);
    let has_16_cuts = static_cuts.iter().any(|c| c.len() == 16);
    assert!(has_16_cuts, "Static cuts must include 16-cycle cuts for 32-vertex ladder");

    let start = Instant::now();
    let tour = solve_hamilton(
        contracted_g,
        &contractor,
        &hub_reg,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 100, 10.0, start, "default"
    );
    assert!(tour.is_some(), "Graph must be solved with CEGAR and extended static cuts wired");
    let t = tour.unwrap();
    assert_eq!(t.len(), 32);

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
fn test_cegar_multi_swap_stitcher_integration() {
    use cegar_fix::giant_cycle_stitcher::GiantCycleStitcher;
    use cegar_fix::hcp_solver::solve_hamilton;
    use cegar_fix::contraction::Degree2Contractor;
    use cegar_fix::hub_registry::HubRegistry;
    use std::collections::HashSet;
    use std::time::Instant;

    let mut g = Graph::new();
    // 1. Giant ring: 60 vertices (1..=60)
    let giant_cycle: Vec<i32> = (1..=60).collect();
    for i in 1..=60 {
        let nxt = if i == 60 { 1 } else { i + 1 };
        g.add_edge(i, nxt);
    }
    // Giant ring chords to maintain vertex degrees >= 3
    for i in 1..=30 {
        g.add_edge(i, i + 30);
    }

    // 2. Three 8-vertex satellite rings:
    // Satellite 1: 61..=68
    let sat1: Vec<i32> = (61..=68).collect();
    for i in 61..=68 {
        let nxt = if i == 68 { 61 } else { i + 1 };
        g.add_edge(i, nxt);
    }
    for i in 61..=64 {
        g.add_edge(i, i + 4);
    }

    // Satellite 2: 69..=76
    let sat2: Vec<i32> = (69..=76).collect();
    for i in 69..=76 {
        let nxt = if i == 76 { 69 } else { i + 1 };
        g.add_edge(i, nxt);
    }
    for i in 69..=72 {
        g.add_edge(i, i + 4);
    }

    // Satellite 3: 77..=84
    let sat3: Vec<i32> = (77..=84).collect();
    for i in 77..=84 {
        let nxt = if i == 84 { 77 } else { i + 1 };
        g.add_edge(i, nxt);
    }
    for i in 77..=80 {
        g.add_edge(i, i + 4);
    }

    // 3. Cross-edges enabling simultaneous multi-swap absorption into giant ring:
    // Satellite 1 connected at (10, 11) <-> (61, 62)
    g.add_edge(10, 61);
    g.add_edge(11, 62);

    // Satellite 2 connected at (30, 31) <-> (69, 70)
    g.add_edge(30, 69);
    g.add_edge(31, 70);

    // Satellite 3 connected at (50, 51) <-> (77, 78)
    g.add_edge(50, 77);
    g.add_edge(51, 78);

    // Step A: Direct unit test of GiantCycleStitcher multi-swap simultaneous absorption
    let protected = HashSet::new();
    let initial_cycles = vec![giant_cycle.clone(), sat1.clone(), sat2.clone(), sat3.clone()];
    let stitched = GiantCycleStitcher::repair_until_fixed_point(&initial_cycles, &g, &protected);
    assert_eq!(stitched.len(), 1, "GiantCycleStitcher must stitch 60-vertex giant ring and 3 satellite rings into 1 cycle");
    assert_eq!(stitched[0].len(), 84, "Stitched tour must contain all 84 vertices");

    let mut direct_seen = HashSet::new();
    for &v in &stitched[0] {
        assert!(direct_seen.insert(v), "Duplicate vertex {} in direct stitched tour", v);
    }
    for i in 0..stitched[0].len() {
        let u = stitched[0][i];
        let v = stitched[0][(i + 1) % stitched[0].len()];
        assert!(
            g.adjacency_list.get(&u).map_or(false, |nbrs| nbrs.contains(&v)),
            "Direct stitched edge ({}, {}) must exist in graph",
            u,
            v
        );
    }

    // Step B: Full CEGAR end-to-end solve via solve_hamilton
    let (contracted_g, contractor) = Degree2Contractor::contract(&g);
    let hub_reg = HubRegistry::new(&contracted_g);
    let start = Instant::now();

    let tour = solve_hamilton(
        contracted_g,
        &contractor,
        &hub_reg,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 100, 10.0, start, "default"
    );
    assert!(tour.is_some(), "Graph with 60-vertex giant ring and three 8-vertex satellites must be solved via multi-swap CEGAR stitcher");
    let t = tour.unwrap();
    assert_eq!(t.len(), 84, "Tour length must equal total vertices (60 + 8 * 3 = 84)");

    // Verify uniqueness and validity of Hamiltonian cycle
    let mut seen = HashSet::new();
    for &v in &t {
        assert!(seen.insert(v), "Duplicate vertex {} in tour", v);
    }
    assert_eq!(seen.len(), 84, "Tour must visit all 84 distinct vertices");

    for i in 0..t.len() {
        let u = t[i];
        let v = t[(i + 1) % t.len()];
        assert!(
            g.adjacency_list.get(&u).map_or(false, |nbrs| nbrs.contains(&v)),
            "Edge ({}, {}) must exist in original graph",
            u,
            v
        );
    }
}

#[test]
fn test_cegar_global_supernode_mtz_integration() {
    use cegar_fix::hcp_solver::solve_hamilton;
    use cegar_fix::contraction::Degree2Contractor;
    use cegar_fix::hub_registry::HubRegistry;
    use cegar_fix::metagraph_router::MetagraphRouter;
    use std::collections::HashSet;
    use std::time::Instant;

    let mut g = Graph::new();
    // 64-vertex 8-module cycle-of-ladders graph (N=64 >= 50, K=8 in [4, 24])
    // 8 ladder gadget modules, each containing 8 vertices (base = m * 8, vertices base+1..=base+8)
    for m in 0..8 {
        let base = m * 8;
        // Top rail: base+1..base+4
        for i in 1..4 {
            g.add_edge(base + i, base + i + 1);
        }
        // Bottom rail: base+5..base+8
        for i in 5..8 {
            g.add_edge(base + i, base + i + 1);
        }
        // Rungs
        for i in 1..=4 {
            g.add_edge(base + i, base + i + 4);
        }
        // Internal diagonals to ensure strong components and degree >= 4
        g.add_edge(base + 1, base + 6);
        g.add_edge(base + 2, base + 5);
        g.add_edge(base + 2, base + 7);
        g.add_edge(base + 3, base + 6);
        g.add_edge(base + 3, base + 8);
        g.add_edge(base + 4, base + 7);
    }

    // Inter-module ring connections connecting the 8 modules into a cycle-of-ladders
    for m in 0..8 {
        let base = m * 8;
        let next_base = ((m + 1) % 8) * 8;
        // Top rail bridge
        g.add_edge(base + 4, next_base + 1);
        // Bottom rail bridge
        g.add_edge(base + 8, next_base + 5);
    }

    // Verify MetagraphRouter detects 8 modules on G
    let (contracted_g, contractor) = Degree2Contractor::contract(&g);
    assert_eq!(contracted_g.adjacency_list.len(), 64, "Contracted graph must retain all 64 vertices");

    let modules = MetagraphRouter::detect_gadget_modules_with_size(&contracted_g, 25);
    assert_eq!(modules.len(), 8, "Expected 8 supernode modules in cycle-of-ladders");

    let hub_reg = HubRegistry::new(&contracted_g);
    let start = Instant::now();

    // Solve via CEGAR pipeline with GlobalSupernodeMTZ enabled at Round 0
    let tour = solve_hamilton(
        contracted_g,
        &contractor,
        &hub_reg,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 100, 10.0, start, "default"
    );

    assert!(tour.is_some(), "64-vertex 8-module cycle-of-ladders must be solved via GlobalSupernodeMTZ and CEGAR");
    let t = tour.unwrap();
    assert_eq!(t.len(), 64, "Tour length must equal 64");

    // Verify distinct vertex visitation
    let mut seen = HashSet::new();
    for &v in &t {
        assert!(seen.insert(v), "Duplicate vertex {} in tour", v);
    }
    assert_eq!(seen.len(), 64, "Tour must visit all 64 distinct vertices");

    // Verify validity of all consecutive edges
    for i in 0..t.len() {
        let u = t[i];
        let v = t[(i + 1) % t.len()];
        assert!(
            g.adjacency_list.get(&u).map_or(false, |nbrs| nbrs.contains(&v)),
            "Edge ({}, {}) must exist in original graph",
            u,
            v
        );
    }
}

#[test]
fn test_cegar_transitive_macro_splicer_integration() {
    use cegar_fix::transitive_macro_splicer::TransitiveMacroSplicer;
    use cegar_fix::hcp_solver::solve_hamilton;
    use cegar_fix::contraction::Degree2Contractor;
    use cegar_fix::hub_registry::HubRegistry;
    use std::collections::HashSet;
    use std::time::Instant;

    let mut g = Graph::new();

    // 4 cycles forming a transitive chain: C0 <-> C1 <-> C2 <-> C3
    // C0: 1..=6
    for i in 1..=6 {
        let nxt = if i == 6 { 1 } else { i + 1 };
        g.add_edge(i, nxt);
    }
    g.add_edge(3, 5);
    g.add_edge(4, 6);

    // C1: 7..=12
    for i in 7..=12 {
        let nxt = if i == 12 { 7 } else { i + 1 };
        g.add_edge(i, nxt);
    }
    g.add_edge(9, 12);

    // C2: 13..=18
    for i in 13..=18 {
        let nxt = if i == 18 { 13 } else { i + 1 };
        g.add_edge(i, nxt);
    }
    g.add_edge(15, 18);

    // C3: 19..=24
    for i in 19..=24 {
        let nxt = if i == 24 { 19 } else { i + 1 };
        g.add_edge(i, nxt);
    }
    g.add_edge(21, 23);
    g.add_edge(22, 24);

    // Transitive bridge cross-edges:
    // C0 <-> C1
    g.add_edge(1, 7);
    g.add_edge(2, 8);

    // C1 <-> C2
    g.add_edge(10, 13);
    g.add_edge(11, 14);

    // C2 <-> C3
    g.add_edge(16, 19);
    g.add_edge(17, 20);

    // Verify 0 direct cross-edges between C0 (1..=6) and C3 (19..=24)
    for u in 1..=6 {
        if let Some(nbrs) = g.adjacency_list.get(&u) {
            for v in 19..=24 {
                assert!(!nbrs.contains(&v), "No direct cross-edges allowed between C0 and C3");
            }
        }
    }

    // Step 1: Direct test of TransitiveMacroSplicer on 4-cycle decomposition
    let c0: Vec<i32> = (1..=6).collect();
    let c1: Vec<i32> = (7..=12).collect();
    let c2: Vec<i32> = (13..=18).collect();
    let c3: Vec<i32> = (19..=24).collect();

    let initial_cycles = vec![c0, c1, c2, c3];
    let protected = HashSet::new();
    let spliced = TransitiveMacroSplicer::splice_transitive_macro_graph(&initial_cycles, &g, &protected);
    assert_eq!(spliced.len(), 1, "TransitiveMacroSplicer must splice transitive 4-cycle chain into 1 cycle");
    assert_eq!(spliced[0].len(), 24, "Spliced cycle must contain all 24 vertices");

    // Step 2: Full CEGAR end-to-end solve via solve_hamilton
    let (contracted_g, contractor) = Degree2Contractor::contract(&g);
    let hub_reg = HubRegistry::new(&contracted_g);
    let start = Instant::now();

    let tour = solve_hamilton(
        contracted_g,
        &contractor,
        &hub_reg,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 100, 10.0, start, "default"
    );

    assert!(tour.is_some(), "Graph with transitive 4-cycle chain must be solved via CEGAR with TransitiveMacroSplicer");
    let t = tour.unwrap();
    assert_eq!(t.len(), 24, "Tour length must equal 24");

    // Step 3: Verify tour validity
    let mut seen = HashSet::new();
    for &v in &t {
        assert!(seen.insert(v), "Duplicate vertex {} in tour", v);
    }
    assert_eq!(seen.len(), 24, "Tour must visit all 24 distinct vertices");

    for i in 0..t.len() {
        let u = t[i];
        let v = t[(i + 1) % t.len()];
        assert!(
            g.adjacency_list.get(&u).map_or(false, |nbrs| nbrs.contains(&v)),
            "Edge ({}, {}) must exist in original graph",
            u,
            v
        );
    }
}

#[test]
fn test_cegar_interface_port_synchronizer_integration() {
    use cegar_fix::hcp_solver::solve_hamilton;
    use cegar_fix::contraction::Degree2Contractor;
    use cegar_fix::hub_registry::HubRegistry;
    use cegar_fix::interface_port_synchronizer::InterfacePortSynchronizer;
    use std::collections::HashSet;
    use std::time::Instant;

    let mut g = Graph::new();

    // 4 ladder gadget modules arranged in a ring (8 vertices each, 32 vertices total)
    // Module 0: 1..=8
    // Module 1: 9..=16
    // Module 2: 17..=24
    // Module 3: 25..=32
    for m in 0..4 {
        let base = m * 8;
        // Top rail
        for i in 1..4 {
            g.add_edge(base + i, base + i + 1);
        }
        // Bottom rail
        for i in 5..8 {
            g.add_edge(base + i, base + i + 1);
        }
        // Rungs
        for i in 1..=4 {
            g.add_edge(base + i, base + i + 4);
        }
        // Diagonals inside 2x2 blocks
        g.add_edge(base + 1, base + 6);
        g.add_edge(base + 2, base + 5);
        g.add_edge(base + 2, base + 7);
        g.add_edge(base + 3, base + 6);
        g.add_edge(base + 3, base + 8);
        g.add_edge(base + 4, base + 7);
    }

    // Inter-module ring connections connecting the 4 modules:
    // b_0 (4) -> a_1 (9)
    // b_1 (12) -> a_2 (17)
    // b_2 (20) -> a_3 (25)
    // b_3 (28) -> a_0 (1)
    for m in 0..4 {
        let base = m * 8;
        let next_base = ((m + 1) % 4) * 8;
        g.add_edge(base + 4, next_base + 1);
    }

    let (contracted_g, contractor) = Degree2Contractor::contract(&g);
    assert_eq!(contracted_g.adjacency_list.len(), 32, "Contracted graph must retain all 32 vertices");

    let dual_paths = InterfacePortSynchronizer::extract_gadget_dual_paths(&contracted_g, 32);
    assert_eq!(dual_paths.len(), 4, "Expected 4 gadget modules with dual T/F paths");

    let hub_reg = HubRegistry::new(&contracted_g);
    let start = Instant::now();

    // Solve via CEGAR pipeline with InterfacePortSynchronizer wired at Round 0
    let tour = solve_hamilton(
        contracted_g,
        &contractor,
        &hub_reg,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 100, 10.0, start, "default"
    );

    assert!(tour.is_some(), "32-vertex 4-gadget ring graph must be solved via InterfacePortSynchronizer and CEGAR");
    let t = tour.unwrap();
    assert_eq!(t.len(), 32, "Tour length must equal 32");

    // Verify distinct vertex visitation
    let mut seen = HashSet::new();
    for &v in &t {
        assert!(seen.insert(v), "Duplicate vertex {} in tour", v);
    }
    assert_eq!(seen.len(), 32, "Tour must visit all 32 distinct vertices");

    // Verify validity of all consecutive edges
    for i in 0..t.len() {
        let u = t[i];
        let v = t[(i + 1) % t.len()];
        assert!(
            g.adjacency_list.get(&u).map_or(false, |nbrs| nbrs.contains(&v)),
            "Edge ({}, {}) must exist in original graph",
            u,
            v
        );
    }
}

#[test]
fn test_cegar_inverse_3sat_synthesizer_integration() {
    use cegar_fix::hcp_solver::solve_hamilton;
    use cegar_fix::contraction::Degree2Contractor;
    use cegar_fix::hub_registry::HubRegistry;
    use cegar_fix::tour_verifier::TourVerifier;
    use std::collections::HashSet;
    use std::time::Instant;

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

    let (contracted_g, contractor) = Degree2Contractor::contract(&g);
    assert_eq!(contracted_g.adjacency_list.len(), 14, "Contracted graph must retain all 14 vertices");

    let hub_reg = HubRegistry::new(&contracted_g);
    let start = Instant::now();

    // Solve via CEGAR pipeline with Inverse3SatSynthesizer fast track wired at Round 0
    let tour = solve_hamilton(
        contracted_g,
        &contractor,
        &hub_reg,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 100, 10.0, start, "default"
    );

    assert!(tour.is_some(), "14-vertex 2-variable 3-SAT reduction graph must be solved via Inverse3SatSynthesizer");
    let t = tour.unwrap();
    assert_eq!(t.len(), 14, "Tour length must equal 14");

    // Verify distinct vertex visitation
    let mut seen = HashSet::new();
    for &v in &t {
        assert!(seen.insert(v), "Duplicate vertex {} in tour", v);
    }
    assert_eq!(seen.len(), 14, "Tour must visit all 14 distinct vertices");

    // Verify tour validity with TourVerifier
    let verify_res = TourVerifier::verify_raw_tour(&t, &g);
    assert!(verify_res.is_ok(), "Tour verification failed: {:?}", verify_res.err());

    // Verify validity of all consecutive edges
    for i in 0..t.len() {
        let u = t[i];
        let v = t[(i + 1) % t.len()];
        assert!(
            g.adjacency_list.get(&u).map_or(false, |nbrs| nbrs.contains(&v)),
            "Edge ({}, {}) must exist in original graph",
            u,
            v
        );
    }
}

#[test]
fn test_cegar_hub_hierarchical_decomposer_integration() {
    use cegar_fix::hcp_solver::solve_hamilton;
    use cegar_fix::contraction::Degree2Contractor;
    use cegar_fix::hub_registry::HubRegistry;
    use cegar_fix::tour_verifier::TourVerifier;
    use std::collections::HashSet;
    use std::time::Instant;

    let mut g = Graph::new();

    // 3 hub clusters with degree-2 chains
    // Module 0: Hub 1 (deg 6), Satellites 2..=7, degree-2 chains 101 and 102
    for v in 2..=7 {
        g.add_edge(1, v);
    }
    // Subdivided chain 2 - 101 - 3
    g.add_edge(2, 101);
    g.add_edge(101, 3);
    // Intermediate edges and chords
    g.add_edge(3, 4);
    g.add_edge(2, 4);
    // Subdivided chain 5 - 102 - 6
    g.add_edge(5, 102);
    g.add_edge(102, 6);
    g.add_edge(6, 7);
    g.add_edge(5, 7);

    // Module 1: Hub 11 (deg 6), Satellites 12..=17, degree-2 chains 103 and 104
    for v in 12..=17 {
        g.add_edge(11, v);
    }
    // Subdivided chain 12 - 103 - 13
    g.add_edge(12, 103);
    g.add_edge(103, 13);
    g.add_edge(13, 14);
    g.add_edge(12, 14);
    // Subdivided chain 15 - 104 - 16
    g.add_edge(15, 104);
    g.add_edge(104, 16);
    g.add_edge(16, 17);
    g.add_edge(15, 17);

    // Module 2: Hub 21 (deg 6), Satellites 22..=27, degree-2 chains 105 and 106
    for v in 22..=27 {
        g.add_edge(21, v);
    }
    // Subdivided chain 22 - 105 - 23
    g.add_edge(22, 105);
    g.add_edge(105, 23);
    g.add_edge(23, 24);
    g.add_edge(22, 24);
    // Subdivided chain 25 - 106 - 26
    g.add_edge(25, 106);
    g.add_edge(106, 26);
    g.add_edge(26, 27);
    g.add_edge(25, 27);

    // Inter-module ring connections
    g.add_edge(7, 12);
    g.add_edge(17, 22);
    g.add_edge(27, 2);

    let (contracted_g, contractor) = Degree2Contractor::contract(&g);
    assert_eq!(contractor.original_vertices_count, 27, "Original graph must have 27 vertices");
    assert_eq!(contracted_g.adjacency_list.len(), 21, "Contracted graph must have 21 vertices");

    let hub_reg = HubRegistry::new(&contracted_g);
    let start = Instant::now();

    // Solve via CEGAR pipeline with HubHierarchicalDecomposer fast track wired at Round 0
    let tour = solve_hamilton(
        contracted_g,
        &contractor,
        &hub_reg,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 100, 10.0, start, "default"
    );

    assert!(tour.is_some(), "27-vertex multi-hub graph must be solved via HubHierarchicalDecomposer and expanded via Degree2Contractor");
    let t = tour.unwrap();
    assert_eq!(t.len(), 27, "Tour length must equal 27");

    // Verify distinct vertex visitation
    let mut seen = HashSet::new();
    for &v in &t {
        assert!(seen.insert(v), "Duplicate vertex {} in tour", v);
    }
    assert_eq!(seen.len(), 27, "Tour must visit all 27 distinct vertices");

    // Verify tour validity with TourVerifier
    let verify_res = TourVerifier::verify_raw_tour(&t, &g);
    assert!(verify_res.is_ok(), "Tour verification failed: {:?}", verify_res.err());

    // Verify validity of all consecutive edges
    for i in 0..t.len() {
        let u = t[i];
        let v = t[(i + 1) % t.len()];
        assert!(
            g.adjacency_list.get(&u).map_or(false, |nbrs| nbrs.contains(&v)),
            "Edge ({}, {}) must exist in original graph",
            u,
            v
        );
    }
}

#[test]
fn test_cegar_multi_opt_sat_splicer_integration() {
    use cegar_fix::multi_opt_sat_splicer::MultiOptSatSplicer;
    use cegar_fix::giant_cycle_stitcher::GiantCycleStitcher;
    use cegar_fix::hcp_solver::solve_hamilton;
    use cegar_fix::contraction::Degree2Contractor;
    use cegar_fix::hub_registry::HubRegistry;
    use cegar_fix::tour_verifier::TourVerifier;
    use std::collections::HashSet;
    use std::time::Instant;

    let mut g = Graph::new();

    // 3 cycle gadgets C0, C1, C2 arranged in a 3-opt triangle configuration
    // with degree-2 subdivision vertices (101, 102, 103)

    // C0: vertices 1, 2, 3, 4 with subdivision node 101 between 4 and 1
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 101);
    g.add_edge(101, 1);
    // Internal chords in C0 to ensure all non-subdivided vertices have degree >= 3
    g.add_edge(1, 3);
    g.add_edge(2, 4);

    // C1: vertices 5, 6, 7, 8 with subdivision node 102 between 8 and 5
    g.add_edge(5, 6);
    g.add_edge(6, 7);
    g.add_edge(7, 8);
    g.add_edge(8, 102);
    g.add_edge(102, 5);
    // Internal chords in C1
    g.add_edge(5, 7);
    g.add_edge(6, 8);

    // C2: vertices 9, 10, 11, 12 with subdivision node 103 between 12 and 9
    g.add_edge(9, 10);
    g.add_edge(10, 11);
    g.add_edge(11, 12);
    g.add_edge(12, 103);
    g.add_edge(103, 9);
    // Internal chords in C2
    g.add_edge(9, 11);
    g.add_edge(10, 12);

    // 3-opt triangle cross-edges (exactly 1 cross-edge per cycle pair, no 2-opt bridges possible):
    // C0 <-> C1: (1, 6)
    // C1 <-> C2: (5, 10)
    // C2 <-> C0: (9, 2)
    g.add_edge(1, 6);
    g.add_edge(5, 10);
    g.add_edge(9, 2);

    // Step 1: Verify Degree-2 contraction
    let (contracted_g, contractor) = Degree2Contractor::contract(&g);
    assert_eq!(contractor.original_vertices_count, 15, "Original graph must have 15 vertices");
    assert_eq!(contracted_g.adjacency_list.len(), 12, "Contracted graph must have 12 vertices");

    // Step 2: Direct test of MultiOptSatSplicer and GiantCycleStitcher on contracted cycles
    let c0 = vec![1, 2, 3, 4];
    let c1 = vec![5, 6, 7, 8];
    let c2 = vec![9, 10, 11, 12];
    let initial_cycles = vec![c0, c1, c2];
    let protected = HashSet::new();

    let multi_opt_spliced = MultiOptSatSplicer::splice_multi_opt_cycles(&initial_cycles, &contracted_g, &protected);
    assert_eq!(multi_opt_spliced.len(), 1, "MultiOptSatSplicer must splice 3-cycle triangle into 1 cycle");
    assert_eq!(multi_opt_spliced[0].len(), 12, "Spliced cycle must cover all 12 contracted vertices");

    let stitcher_repaired = GiantCycleStitcher::repair_until_fixed_point(&initial_cycles, &contracted_g, &protected);
    assert_eq!(stitcher_repaired.len(), 1, "GiantCycleStitcher repair_until_fixed_point must splice 3-cycle triangle into 1 cycle");
    assert_eq!(stitcher_repaired[0].len(), 12, "Repaired cycle must cover all 12 contracted vertices");

    // Step 3: Full CEGAR end-to-end solve via solve_hamilton
    let hub_reg = HubRegistry::new(&contracted_g);
    let start = Instant::now();

    let tour = solve_hamilton(
        contracted_g,
        &contractor,
        &hub_reg,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 100, 10.0, start, "default"
    );

    assert!(tour.is_some(), "Graph with 3-cycle triangle configuration must be solved via CEGAR with MultiOptSatSplicer");
    let t = tour.unwrap();
    assert_eq!(t.len(), 15, "Final uncontracted tour length must equal 15");

    // Step 4: Verify distinct vertex visitation and edge validity on original graph
    let mut seen = HashSet::new();
    for &v in &t {
        assert!(seen.insert(v), "Duplicate vertex {} in tour", v);
    }
    assert_eq!(seen.len(), 15, "Tour must visit all 15 distinct vertices");

    let verify_res = TourVerifier::verify_raw_tour(&t, &g);
    assert!(verify_res.is_ok(), "Tour verification failed: {:?}", verify_res.err());

    for i in 0..t.len() {
        let u = t[i];
        let v = t[(i + 1) % t.len()];
        assert!(
            g.adjacency_list.get(&u).map_or(false, |nbrs| nbrs.contains(&v)),
            "Edge ({}, {}) must exist in original graph",
            u,
            v
        );
    }
}

#[test]
fn test_cegar_empirical_backbone_cutter_integration() {
    use cegar_fix::empirical_backbone_cutter::{EmpiricalBackboneTracker, EmpiricalBackboneCutter};
    use cegar_fix::hcp_solver::solve_hamilton;
    use cegar_fix::contraction::Degree2Contractor;
    use cegar_fix::hub_registry::HubRegistry;
    use cegar_fix::tour_verifier::TourVerifier;
    use cegar_fix::encoder::Encoder;
    use std::collections::HashSet;
    use std::time::Instant;

    // 1. Direct validation of EmpiricalBackboneCutter & EmpiricalBackboneTracker
    let mut tracker = EmpiricalBackboneTracker::new(5);
    let cycle1 = vec![1, 2, 3, 4];
    let cycle2 = vec![5, 6, 7, 8];
    tracker.record_solution_edges(&[cycle1.clone(), cycle2.clone()]);
    assert_eq!(tracker.total_rounds_recorded, 1);
    let freq_edges = tracker.get_frequent_backbone_edges(1.0);
    assert_eq!(freq_edges.len(), 8);

    // 2. Build graph with multiple sub-cycle chords requiring CEGAR & SEC cutting
    let mut g = Graph::new();
    // 32-node ring with chords inducing subcycles
    for i in 1..32 {
        g.add_edge(i, i + 1);
    }
    g.add_edge(32, 1);

    // Internal chords creating multiple competing 4-cycles and 8-cycles
    g.add_edge(1, 8);
    g.add_edge(8, 16);
    g.add_edge(16, 24);
    g.add_edge(24, 32);
    g.add_edge(4, 12);
    g.add_edge(12, 20);
    g.add_edge(20, 28);
    g.add_edge(28, 4);

    let (contracted_g, contractor) = Degree2Contractor::contract(&g);
    let hub_reg = HubRegistry::new(&contracted_g);

    // Verify comprehensive SEC clauses generation on encoder
    let mut encoder = Encoder::new();
    let _ = encoder.encode(&contracted_g, 0, 0, 0, 0, 0, 0);
    let sec_clauses = EmpiricalBackboneCutter::generate_comprehensive_sec_clauses(
        &[vec![4, 12, 20, 28]],
        5,
        &encoder.graph_lit_map,
    );
    assert_eq!(sec_clauses.len(), 2, "Comprehensive SEC clauses (forward + reverse) should be generated for 4-cycle");

    // 3. Full CEGAR end-to-end solve with EmpiricalBackboneTracker and SEC Cutter wired
    let start = Instant::now();
    let tour = solve_hamilton(
        contracted_g,
        &contractor,
        &hub_reg,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 100, 10.0, start, "default"
    );

    assert!(tour.is_some(), "Graph must be solved with EmpiricalBackboneCutter wired into CEGAR loop");
    let t = tour.unwrap();
    assert_eq!(t.len(), 32, "Tour length must equal 32 vertices");

    // 4. Verify valid Hamiltonian tour on g
    let mut seen = HashSet::new();
    for &v in &t {
        assert!(seen.insert(v), "Duplicate vertex {} in tour", v);
    }
    assert_eq!(seen.len(), 32, "Tour must visit all 32 distinct vertices");

    let verify_res = TourVerifier::verify_raw_tour(&t, &g);
    assert!(verify_res.is_ok(), "Tour verification failed: {:?}", verify_res.err());

    for i in 0..t.len() {
        let u = t[i];
        let v = t[(i + 1) % t.len()];
        assert!(
            g.adjacency_list.get(&u).map_or(false, |nbrs| nbrs.contains(&v)),
            "Edge ({}, {}) must exist in original graph",
            u,
            v
        );
    }
}

#[test]
fn test_cegar_cnf_subsumer_integration() {
    use cegar_fix::hcp_solver::solve_hamilton;
    use cegar_fix::contraction::Degree2Contractor;
    use cegar_fix::hub_registry::HubRegistry;
    use cegar_fix::cnf_subsumer::CnfSubsumer;
    use cegar_fix::tour_verifier::TourVerifier;
    use rustsat::instances::Cnf;
    use rustsat::types::Lit;
    use rustsat::clause;
    use std::collections::HashSet;
    use std::time::Instant;

    // 1. Direct unit verification of CnfSubsumer::prune_and_subsume_cuts
    let l1 = Lit::positive(1);
    let l2 = Lit::positive(2);
    let l3 = Lit::positive(3);
    let l4 = Lit::positive(4);

    let mut cnf1 = Cnf::new();
    cnf1.add_clause(clause!(l1, l2)); // Short clause
    cnf1.add_clause(clause!(l1, l2, l3)); // Subsumed by (l1, l2)
    cnf1.add_clause(clause!(l1, !l1)); // Tautology

    let mut cnf2 = Cnf::new();
    cnf2.add_clause(clause!(l1, l2)); // Duplicate
    cnf2.add_clause(clause!(l3, l4)); // Distinct clause
    cnf2.add_clause(clause!(l1, l2, l4)); // Subsumed by (l1, l2)

    let pruned = CnfSubsumer::prune_and_subsume_cuts(&[cnf1, cnf2]);
    assert_eq!(pruned.len(), 2, "Pruned CNF should contain exactly (l1, l2) and (l3, l4)");

    // 2. CEGAR end-to-end solve on 30-vertex graph with chords
    let mut g = Graph::new();
    for i in 1..30 {
        g.add_edge(i, i + 1);
    }
    g.add_edge(30, 1);
    g.add_edge(1, 15);
    g.add_edge(15, 30);
    g.add_edge(5, 20);
    g.add_edge(10, 25);
    g.add_edge(3, 18);
    g.add_edge(8, 23);

    let (contracted_g, contractor) = Degree2Contractor::contract(&g);
    let hub_reg = HubRegistry::new(&contracted_g);
    let start = Instant::now();

    let tour = solve_hamilton(
        contracted_g,
        &contractor,
        &hub_reg,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 100, 10.0, start, "default"
    );

    assert!(tour.is_some(), "Graph must be solved with CnfSubsumer wired into CEGAR loop");
    let t = tour.unwrap();
    assert_eq!(t.len(), 30, "Tour length must equal 30 vertices");

    let verify_res = TourVerifier::verify_raw_tour(&t, &g);
    assert!(verify_res.is_ok(), "Tour verification failed: {:?}", verify_res.err());

    let mut seen = HashSet::new();
    for &v in &t {
        assert!(seen.insert(v), "Duplicate vertex {} in tour", v);
    }
    assert_eq!(seen.len(), 30, "Tour must visit all 30 distinct vertices");
}

#[test]
fn test_cegar_twin_giant_splicer_integration() {
    use cegar_fix::giant_cycle_stitcher::GiantCycleStitcher;
    use cegar_fix::twin_giant_splicer::TwinGiantSplicer;
    use cegar_fix::hcp_solver::solve_hamilton;
    use cegar_fix::contraction::Degree2Contractor;
    use cegar_fix::hub_registry::HubRegistry;
    use cegar_fix::tour_verifier::TourVerifier;
    use cegar_fix::encoder::Encoder;
    use std::collections::HashSet;
    use std::time::Instant;

    let mut g = Graph::new();
    let c1: Vec<i32> = (1..=12).collect();
    let c2: Vec<i32> = (13..=24).collect();

    // Giant cycle 1: 1..=12 with internal chords (all degree >= 3)
    for i in 0..12 {
        g.add_edge(c1[i], c1[(i + 1) % 12]);
    }
    g.add_edge(1, 4);
    g.add_edge(2, 5);
    g.add_edge(3, 6);
    g.add_edge(4, 7);
    g.add_edge(5, 8);
    g.add_edge(6, 9);
    g.add_edge(7, 10);
    g.add_edge(8, 11);
    g.add_edge(9, 12);
    g.add_edge(10, 1);
    g.add_edge(11, 2);
    g.add_edge(12, 3);

    // Giant cycle 2: 13..=24 with internal chords (all degree >= 3)
    for i in 0..12 {
        g.add_edge(c2[i], c2[(i + 1) % 12]);
    }
    g.add_edge(13, 16);
    g.add_edge(14, 17);
    g.add_edge(15, 18);
    g.add_edge(16, 19);
    g.add_edge(17, 20);
    g.add_edge(18, 21);
    g.add_edge(19, 22);
    g.add_edge(20, 23);
    g.add_edge(21, 24);
    g.add_edge(22, 13);
    g.add_edge(23, 14);
    g.add_edge(24, 15);

    // Cross-giant connecting edges enabling 2-opt splice and bicomponent cuts
    g.add_edge(1, 13);
    g.add_edge(2, 14);

    // 1. Direct unit verification: GiantCycleStitcher using TwinGiantSplicer
    let protected = HashSet::new();
    let initial_cycles = vec![c1.clone(), c2.clone()];
    let stitched = GiantCycleStitcher::repair_until_fixed_point(&initial_cycles, &g, &protected);
    assert_eq!(stitched.len(), 1, "GiantCycleStitcher should stitch two twin giant cycles into 1");
    assert_eq!(stitched[0].len(), 24, "Stitched tour must contain all 24 vertices");

    let verify_direct = TourVerifier::verify_raw_tour(&stitched[0], &g);
    assert!(verify_direct.is_ok(), "Direct stitched tour verification failed: {:?}", verify_direct.err());

    // 2. Direct verification of TwinGiantSplicer bicomponent cut generation with Encoder
    let mut encoder = Encoder::new();
    let _ = encoder.encode(&g, 0, 0, 0, 0, 0, 0);
    let cuts = TwinGiantSplicer::generate_bicomponent_cut_clauses(&c1, &c2, &g, &encoder.graph_lit_map);
    assert_eq!(cuts.len(), 2, "Expected 2 bicomponent cut clauses: delta+(C1 -> C2) and delta+(C2 -> C1)");

    // 3. Full CEGAR end-to-end solve
    let (contracted_g, contractor) = Degree2Contractor::contract(&g);
    let hub_reg = HubRegistry::new(&contracted_g);
    let start = Instant::now();

    let tour = solve_hamilton(
        contracted_g,
        &contractor,
        &hub_reg,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 100, 10.0, start, "default"
    );

    assert!(tour.is_some(), "Graph must be solved with TwinGiantSplicer wired into CEGAR loop");
    let t = tour.unwrap();
    assert_eq!(t.len(), 24, "Tour length must equal 24 vertices");

    let verify_res = TourVerifier::verify_raw_tour(&t, &g);
    assert!(verify_res.is_ok(), "Tour verification failed: {:?}", verify_res.err());

    let mut seen = HashSet::new();
    for &v in &t {
        assert!(seen.insert(v), "Duplicate vertex {} in tour", v);
    }
    assert_eq!(seen.len(), 24, "Tour must visit all 24 distinct vertices");
}

#[test]
fn test_cegar_macro_component_splicer_integration() {
    use cegar_fix::giant_cycle_stitcher::GiantCycleStitcher;
    use cegar_fix::macro_component_splicer::MacroComponentSplicer;
    use cegar_fix::hcp_solver::solve_hamilton;
    use cegar_fix::contraction::Degree2Contractor;
    use cegar_fix::hub_registry::HubRegistry;
    use cegar_fix::tour_verifier::TourVerifier;
    use std::collections::HashSet;
    use std::time::Instant;

    let mut g = Graph::new();
    let c1: Vec<i32> = (1..=8).collect();
    let c2: Vec<i32> = (9..=16).collect();
    let c3: Vec<i32> = (17..=24).collect();

    // Cycle 1: 1..=8 with chords (all degree >= 3)
    for i in 0..8 {
        g.add_edge(c1[i], c1[(i + 1) % 8]);
    }
    g.add_edge(1, 4);
    g.add_edge(2, 5);
    g.add_edge(3, 6);
    g.add_edge(7, 2);
    g.add_edge(8, 3);

    // Cycle 2: 9..=16 with chords (all degree >= 3)
    for i in 0..8 {
        g.add_edge(c2[i], c2[(i + 1) % 8]);
    }
    g.add_edge(9, 12);
    g.add_edge(10, 13);
    g.add_edge(11, 14);
    g.add_edge(15, 10);
    g.add_edge(16, 11);

    // Cycle 3: 17..=24 with chords (all degree >= 3)
    for i in 0..8 {
        g.add_edge(c3[i], c3[(i + 1) % 8]);
    }
    g.add_edge(17, 20);
    g.add_edge(18, 21);
    g.add_edge(19, 22);
    g.add_edge(23, 18);
    g.add_edge(24, 19);

    // Cross-component connecting bridges:
    // C1 <-> C2 via cross-edges (1, 9) and (2, 10), replacing (1, 2) in C1 and (9, 10) in C2
    g.add_edge(1, 9);
    g.add_edge(2, 10);

    // C2 <-> C3 via cross-edges (15, 17) and (16, 18), replacing (15, 16) in C2 and (17, 18) in C3
    g.add_edge(15, 17);
    g.add_edge(16, 18);

    // 1. Direct unit verification: MacroComponentSplicer directly
    let protected = HashSet::new();
    let initial_cycles = vec![c1.clone(), c2.clone(), c3.clone()];
    let spliced = MacroComponentSplicer::splice_spanning_components(&initial_cycles, &g, &protected);
    assert_eq!(spliced.len(), 1, "MacroComponentSplicer should splice 3-component spanning tree into 1");
    assert_eq!(spliced[0].len(), 24, "Spliced tour must contain all 24 vertices");

    let verify_spliced = TourVerifier::verify_raw_tour(&spliced[0], &g);
    assert!(verify_spliced.is_ok(), "Direct spliced tour verification failed: {:?}", verify_spliced.err());

    // 2. Direct unit verification: GiantCycleStitcher using MacroComponentSplicer
    let stitched = GiantCycleStitcher::repair_until_fixed_point(&initial_cycles, &g, &protected);
    assert_eq!(stitched.len(), 1, "GiantCycleStitcher should stitch 3-component spanning tree into 1");
    assert_eq!(stitched[0].len(), 24, "Stitched tour must contain all 24 vertices");

    let verify_stitched = TourVerifier::verify_raw_tour(&stitched[0], &g);
    assert!(verify_stitched.is_ok(), "Direct stitched tour verification failed: {:?}", verify_stitched.err());

    // 3. Full CEGAR end-to-end solve
    let (contracted_g, contractor) = Degree2Contractor::contract(&g);
    let hub_reg = HubRegistry::new(&contracted_g);
    let start = Instant::now();

    let tour = solve_hamilton(
        contracted_g,
        &contractor,
        &hub_reg,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 100, 10.0, start, "default"
    );

    assert!(tour.is_some(), "Graph must be solved with MacroComponentSplicer wired into CEGAR loop");
    let t = tour.unwrap();
    assert_eq!(t.len(), 24, "Tour length must equal 24 vertices");

    let verify_res = TourVerifier::verify_raw_tour(&t, &g);
    assert!(verify_res.is_ok(), "Tour verification failed: {:?}", verify_res.err());

    let mut seen = HashSet::new();
    for &v in &t {
        assert!(seen.insert(v), "Duplicate vertex {} in tour", v);
    }
    assert_eq!(seen.len(), 24, "Tour must visit all 24 distinct vertices");
}

#[test]
fn test_cegar_sat_macro_patcher_integration() {
    use cegar_fix::graph::Graph;
    use cegar_fix::giant_cycle_stitcher::GiantCycleStitcher;
    use cegar_fix::sat_macro_patcher::SatMacroPatcher;
    use cegar_fix::hcp_solver::solve_hamilton;
    use cegar_fix::contraction::Degree2Contractor;
    use cegar_fix::hub_registry::HubRegistry;
    use cegar_fix::tour_verifier::TourVerifier;
    use std::collections::HashSet;
    use std::time::Instant;

    let mut g = Graph::new();
    let c1: Vec<i32> = (1..=6).collect();
    let c2: Vec<i32> = (7..=12).collect();
    let c3: Vec<i32> = (13..=18).collect();
    let c4: Vec<i32> = (19..=24).collect();

    // Cycle 1: 1..=6 with chords (all degree >= 3)
    for i in 0..6 {
        g.add_edge(c1[i], c1[(i + 1) % 6]);
    }
    g.add_edge(1, 4);
    g.add_edge(2, 5);
    g.add_edge(3, 6);

    // Cycle 2: 7..=12 with chords (all degree >= 3)
    for i in 0..6 {
        g.add_edge(c2[i], c2[(i + 1) % 6]);
    }
    g.add_edge(7, 10);
    g.add_edge(8, 11);
    g.add_edge(9, 12);

    // Cycle 3: 13..=18 with chords (all degree >= 3)
    for i in 0..6 {
        g.add_edge(c3[i], c3[(i + 1) % 6]);
    }
    g.add_edge(13, 16);
    g.add_edge(14, 17);
    g.add_edge(15, 18);

    // Cycle 4: 19..=24 with chords (all degree >= 3)
    for i in 0..6 {
        g.add_edge(c4[i], c4[(i + 1) % 6]);
    }
    g.add_edge(19, 22);
    g.add_edge(20, 23);
    g.add_edge(21, 24);

    // Spanning tree 2-opt bridges:
    // C1 <-> C2 via cross-edges (1, 7) and (2, 8), replacing (1, 2) and (7, 8)
    g.add_edge(1, 7);
    g.add_edge(2, 8);

    // C1 <-> C3 via cross-edges (4, 13) and (5, 14), replacing (4, 5) and (13, 14)
    g.add_edge(4, 13);
    g.add_edge(5, 14);

    // C3 <-> C4 via cross-edges (16, 19) and (17, 20), replacing (16, 17) and (19, 20)
    g.add_edge(16, 19);
    g.add_edge(17, 20);

    let protected = HashSet::new();
    let initial_cycles = vec![c1.clone(), c2.clone(), c3.clone(), c4.clone()];

    // 1. Direct unit verification: SatMacroPatcher directly
    let patched = SatMacroPatcher::try_patch_all_cycles(&initial_cycles, &g, &protected);
    assert!(patched.is_some(), "SatMacroPatcher should patch 4 cycles into 1");
    let patch_tour = patched.unwrap();
    assert_eq!(patch_tour.len(), 24, "Patched tour must contain 24 vertices");
    let verify_patch = TourVerifier::verify_raw_tour(&patch_tour, &g);
    assert!(verify_patch.is_ok(), "Direct patched tour verification failed: {:?}", verify_patch.err());

    // 2. Direct unit verification: GiantCycleStitcher using SatMacroPatcher
    let stitched = GiantCycleStitcher::repair_until_fixed_point(&initial_cycles, &g, &protected);
    assert_eq!(stitched.len(), 1, "GiantCycleStitcher should stitch 4 cycles into 1");
    assert_eq!(stitched[0].len(), 24, "Stitched tour must contain 24 vertices");
    let verify_stitched = TourVerifier::verify_raw_tour(&stitched[0], &g);
    assert!(verify_stitched.is_ok(), "Direct stitched tour verification failed: {:?}", verify_stitched.err());

    // 3. Full CEGAR end-to-end solve
    let (contracted_g, contractor) = Degree2Contractor::contract(&g);
    let hub_reg = HubRegistry::new(&contracted_g);
    let start = Instant::now();

    let tour = solve_hamilton(
        contracted_g,
        &contractor,
        &hub_reg,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 100, 10.0, start, "default"
    );

    assert!(tour.is_some(), "Graph must be solved with SatMacroPatcher wired into CEGAR loop");
    let t = tour.unwrap();
    assert_eq!(t.len(), 24, "Tour length must equal 24 vertices");

    let verify_res = TourVerifier::verify_raw_tour(&t, &g);
    assert!(verify_res.is_ok(), "Tour verification failed: {:?}", verify_res.err());

    let mut seen = HashSet::new();
    for &v in &t {
        assert!(seen.insert(v), "Duplicate vertex {} in tour", v);
    }
    assert_eq!(seen.len(), 24, "Tour must visit all 24 distinct vertices");
}

#[test]
fn test_cegar_sat_macro_patcher_components_integration() {
    use cegar_fix::graph::Graph;
    use cegar_fix::giant_cycle_stitcher::GiantCycleStitcher;
    use cegar_fix::sat_macro_patcher::SatMacroPatcher;
    use cegar_fix::hcp_solver::solve_hamilton;
    use cegar_fix::contraction::Degree2Contractor;
    use cegar_fix::hub_registry::HubRegistry;
    use cegar_fix::tour_verifier::TourVerifier;
    use std::collections::HashSet;
    use std::time::Instant;

    let mut g = Graph::new();
    let c1: Vec<i32> = (1..=6).collect();
    let c2: Vec<i32> = (7..=12).collect();
    let c3: Vec<i32> = (13..=18).collect();
    let c4: Vec<i32> = (19..=24).collect();
    let c5: Vec<i32> = (25..=30).collect();
    let c6: Vec<i32> = (31..=36).collect();

    let cycles = vec![c1.clone(), c2.clone(), c3.clone(), c4.clone(), c5.clone(), c6.clone()];

    // Add ring edges and chords for all 6 cycles (ensures min-degree >= 3)
    for c in &cycles {
        for i in 0..6 {
            g.add_edge(c[i], c[(i + 1) % 6]);
        }
        g.add_edge(c[0], c[3]);
        g.add_edge(c[1], c[4]);
        g.add_edge(c[2], c[5]);
    }

    // Component 1 bridges: C1 <-> C2 and C2 <-> C3
    // C1 <-> C2 via cross-edges (1, 7) and (2, 8), replacing (1, 2) and (7, 8)
    g.add_edge(1, 7);
    g.add_edge(2, 8);

    // C2 <-> C3 via cross-edges (9, 13) and (10, 14), replacing (9, 10) and (13, 14)
    g.add_edge(9, 13);
    g.add_edge(10, 14);

    // Component 2 bridges: C4 <-> C5 and C5 <-> C6
    // C4 <-> C5 via cross-edges (19, 25) and (20, 26), replacing (19, 20) and (25, 26)
    g.add_edge(19, 25);
    g.add_edge(20, 26);

    // C5 <-> C6 via cross-edges (27, 31) and (28, 32), replacing (27, 28) and (31, 32)
    g.add_edge(27, 31);
    g.add_edge(28, 32);

    let protected = HashSet::new();

    // 1. Verify partial component patching on 2 disconnected macro-components
    let partial_patched = SatMacroPatcher::try_patch_components(&cycles, &g, &protected);
    assert_eq!(partial_patched.len(), 2, "SatMacroPatcher should patch 6 cycles into 2 component giant cycles");
    for comp_tour in &partial_patched {
        assert_eq!(comp_tour.len(), 18, "Each component tour must have 18 vertices");
        let n = comp_tour.len();
        let mut comp_seen = HashSet::new();
        for i in 0..n {
            let u = comp_tour[i];
            let v = comp_tour[(i + 1) % n];
            assert!(comp_seen.insert(u), "Duplicate vertex {} in component tour", u);
            assert!(g.adjacency_list.get(&u).map_or(false, |nbrs| nbrs.contains(&v)), "Edge ({}, {}) must exist in graph", u, v);
        }
    }

    // 2. Add bridge connecting the two components: C3 <-> C4
    // C3 <-> C4 via cross-edges (15, 21) and (16, 22), replacing (15, 16) and (21, 22)
    g.add_edge(15, 21);
    g.add_edge(16, 22);

    // Verify full macro patching across all 6 cycles
    let full_patched = SatMacroPatcher::try_patch_components(&cycles, &g, &protected);
    assert_eq!(full_patched.len(), 1, "SatMacroPatcher should patch all 6 cycles into 1 Hamiltonian cycle");
    assert_eq!(full_patched[0].len(), 36, "Patched tour must contain 36 vertices");
    let verify_patch = TourVerifier::verify_raw_tour(&full_patched[0], &g);
    assert!(verify_patch.is_ok(), "Direct patched tour verification failed: {:?}", verify_patch.err());

    // 3. Direct verification via GiantCycleStitcher using SatMacroPatcher::try_patch_components
    let stitched = GiantCycleStitcher::repair_until_fixed_point(&cycles, &g, &protected);
    assert_eq!(stitched.len(), 1, "GiantCycleStitcher should stitch 6 cycles into 1");
    assert_eq!(stitched[0].len(), 36, "Stitched tour must contain 36 vertices");
    let verify_stitched = TourVerifier::verify_raw_tour(&stitched[0], &g);
    assert!(verify_stitched.is_ok(), "Direct stitched tour verification failed: {:?}", verify_stitched.err());

    // 4. Full CEGAR end-to-end solve
    let (contracted_g, contractor) = Degree2Contractor::contract(&g);
    let hub_reg = HubRegistry::new(&contracted_g);
    let start = Instant::now();

    let tour = solve_hamilton(
        contracted_g,
        &contractor,
        &hub_reg,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 100, 10.0, start, "default"
    );

    assert!(tour.is_some(), "Graph must be solved with SatMacroPatcher component-wise patching wired into CEGAR loop");
    let t = tour.unwrap();
    assert_eq!(t.len(), 36, "Tour length must equal 36 vertices");

    let verify_res = TourVerifier::verify_raw_tour(&t, &g);
    assert!(verify_res.is_ok(), "Tour verification failed: {:?}", verify_res.err());

    let mut seen = HashSet::new();
    for &v in &t {
        assert!(seen.insert(v), "Duplicate vertex {} in tour", v);
    }
    assert_eq!(seen.len(), 36, "Tour must visit all 36 distinct vertices");
}

#[test]
fn test_cegar_gadget_path_absorber_integration() {
    use cegar_fix::graph::Graph;
    use cegar_fix::giant_cycle_stitcher::GiantCycleStitcher;
    use cegar_fix::gadget_path_absorber::GadgetPathAbsorber;
    use cegar_fix::hcp_solver::solve_hamilton;
    use cegar_fix::contraction::Degree2Contractor;
    use cegar_fix::hub_registry::HubRegistry;
    use cegar_fix::tour_verifier::TourVerifier;
    use std::collections::HashSet;
    use std::time::Instant;

    let mut g = Graph::new();
    let c1: Vec<i32> = (1..=20).collect();
    let c2: Vec<i32> = (21..=26).collect();

    // Giant cycle C1: 1..=20 with chords (all degree >= 3)
    for i in 0..20 {
        g.add_edge(c1[i], c1[(i + 1) % 20]);
    }
    for i in 1..=20 {
        let chord_target = ((i + 2) % 20) + 1;
        g.add_edge(i, chord_target);
    }

    // Satellite gadget C2: 21..=26 with chords (all degree >= 3)
    for i in 0..6 {
        g.add_edge(c2[i], c2[(i + 1) % 6]);
    }
    g.add_edge(21, 24);
    g.add_edge(22, 25);
    g.add_edge(23, 26);

    // Cross-edges connecting endpoints of Hamiltonian path 21..26 to giant cycle edge (1, 2)
    g.add_edge(1, 21);
    g.add_edge(26, 2);

    let protected = HashSet::new();
    let initial_cycles = vec![c1.clone(), c2.clone()];

    // 1. Direct unit verification: GadgetPathAbsorber directly
    let absorbed = GadgetPathAbsorber::try_absorb_gadgets(&initial_cycles, &g, &protected);
    assert_eq!(absorbed.len(), 1, "GadgetPathAbsorber should absorb satellite gadget into 1 cycle");
    assert_eq!(absorbed[0].len(), 26, "Absorbed tour must contain 26 vertices");
    let verify_absorbed = TourVerifier::verify_raw_tour(&absorbed[0], &g);
    assert!(verify_absorbed.is_ok(), "Direct absorbed tour verification failed: {:?}", verify_absorbed.err());

    // 2. Direct unit verification: GiantCycleStitcher using GadgetPathAbsorber
    let stitched = GiantCycleStitcher::repair_until_fixed_point(&initial_cycles, &g, &protected);
    assert_eq!(stitched.len(), 1, "GiantCycleStitcher should stitch cycles into 1");
    assert_eq!(stitched[0].len(), 26, "Stitched tour must contain 26 vertices");
    let verify_stitched = TourVerifier::verify_raw_tour(&stitched[0], &g);
    assert!(verify_stitched.is_ok(), "Direct stitched tour verification failed: {:?}", verify_stitched.err());

    // 3. Full CEGAR end-to-end solve
    let (contracted_g, contractor) = Degree2Contractor::contract(&g);
    let hub_reg = HubRegistry::new(&contracted_g);
    let start = Instant::now();

    let tour = solve_hamilton(
        contracted_g,
        &contractor,
        &hub_reg,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 100, 10.0, start, "default"
    );

    assert!(tour.is_some(), "Graph must be solved with GadgetPathAbsorber wired into CEGAR loop");
    let t = tour.unwrap();
    assert_eq!(t.len(), 26, "Tour length must equal 26 vertices");

    let verify_res = TourVerifier::verify_raw_tour(&t, &g);
    assert!(verify_res.is_ok(), "Tour verification failed: {:?}", verify_res.err());

    let mut seen = HashSet::new();
    for &v in &t {
        assert!(seen.insert(v), "Duplicate vertex {} in tour", v);
    }
    assert_eq!(seen.len(), 26, "Tour must visit all 26 distinct vertices");
}

#[test]
fn test_cegar_multi_macro_gadget_integration() {
    use cegar_fix::graph::Graph;
    use cegar_fix::encoder::Encoder;
    use cegar_fix::gadget_parity::GadgetInterfaceParityEngine;
    use cegar_fix::hcp_solver::solve_hamilton;
    use cegar_fix::contraction::Degree2Contractor;
    use cegar_fix::hub_registry::HubRegistry;
    use cegar_fix::tour_verifier::TourVerifier;
    use std::collections::HashSet;
    use std::time::Instant;

    let mut g = Graph::new();
    let m1: Vec<i32> = (1..=20).collect();
    let m2: Vec<i32> = (21..=40).collect();
    let s1: Vec<i32> = (41..=44).collect();
    let s2: Vec<i32> = (45..=48).collect();

    // Macro-cycle 1: 1..=20 with chords (all degree >= 3)
    for i in 0..20 {
        g.add_edge(m1[i], m1[(i + 1) % 20]);
    }
    for i in 1..=20 {
        let chord_target = ((i + 2) % 20) + 1;
        g.add_edge(i, chord_target);
    }

    // Macro-cycle 2: 21..=40 with chords (all degree >= 3)
    for i in 0..20 {
        g.add_edge(m2[i], m2[(i + 1) % 20]);
    }
    for i in 21..=40 {
        let chord_target = 21 + ((i - 21 + 3) % 20);
        g.add_edge(i, chord_target);
    }

    // Satellite subcycle 1: 41..=44 with chords (all degree >= 3)
    for i in 0..4 {
        g.add_edge(s1[i], s1[(i + 1) % 4]);
    }
    g.add_edge(41, 43);
    g.add_edge(42, 44);

    // Satellite subcycle 2: 45..=48 with chords (all degree >= 3)
    for i in 0..4 {
        g.add_edge(s2[i], s2[(i + 1) % 4]);
    }
    g.add_edge(45, 47);
    g.add_edge(46, 48);

    // Connecting cross-edges:
    // Satellite 1 attaches to Macro-cycle 1 adjacent vertices (1, 2)
    g.add_edge(1, 41);
    g.add_edge(44, 2);

    // Satellite 2 attaches to Macro-cycle 2 adjacent vertices (21, 22)
    g.add_edge(21, 45);
    g.add_edge(48, 22);

    // Cross-macro bridges connecting M1 and M2 (e.g., between (10, 11) and (30, 31))
    g.add_edge(10, 30);
    g.add_edge(11, 31);
    g.add_edge(9, 29);
    g.add_edge(12, 32);

    // 1. Direct unit verification: GadgetInterfaceParityEngine on each macro-cycle + satellite
    let mut encoder = Encoder::new();
    let _ = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let res1 = GadgetInterfaceParityEngine::analyze_subcycle_gadget(&s1, &g, Some(&m1), &encoder);
    assert!(res1.direct_spliced_tour.is_some(), "GadgetInterfaceParityEngine should splice S1 with M1");
    let spliced1 = res1.direct_spliced_tour.unwrap();
    assert_eq!(spliced1.len(), 24, "Spliced tour of M1 + S1 must contain 24 vertices");
    let mut seen1 = HashSet::new();
    for i in 0..spliced1.len() {
        let u = spliced1[i];
        let v = spliced1[(i + 1) % spliced1.len()];
        assert!(seen1.insert(u), "Duplicate vertex {} in spliced1", u);
        assert!(g.adjacency_list.get(&u).map_or(false, |nbrs| nbrs.contains(&v)), "Edge ({}, {}) must exist in graph", u, v);
    }

    let res2 = GadgetInterfaceParityEngine::analyze_subcycle_gadget(&s2, &g, Some(&m2), &encoder);
    assert!(res2.direct_spliced_tour.is_some(), "GadgetInterfaceParityEngine should splice S2 with M2");
    let spliced2 = res2.direct_spliced_tour.unwrap();
    assert_eq!(spliced2.len(), 24, "Spliced tour of M2 + S2 must contain 24 vertices");
    let mut seen2 = HashSet::new();
    for i in 0..spliced2.len() {
        let u = spliced2[i];
        let v = spliced2[(i + 1) % spliced2.len()];
        assert!(seen2.insert(u), "Duplicate vertex {} in spliced2", u);
        assert!(g.adjacency_list.get(&u).map_or(false, |nbrs| nbrs.contains(&v)), "Edge ({}, {}) must exist in graph", u, v);
    }

    // 2. Full CEGAR end-to-end solve
    let (contracted_g, contractor) = Degree2Contractor::contract(&g);
    let hub_reg = HubRegistry::new(&contracted_g);
    let start = Instant::now();

    let tour = solve_hamilton(
        contracted_g,
        &contractor,
        &hub_reg,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 100, 10.0, start, "default"
    );

    assert!(tour.is_some(), "Multi-macro gadget graph must be solved via CEGAR");
    let t = tour.unwrap();
    assert_eq!(t.len(), 48, "Tour length must equal 48 vertices");

    let verify_res = TourVerifier::verify_raw_tour(&t, &g);
    assert!(verify_res.is_ok(), "Tour verification failed: {:?}", verify_res.err());

    let mut seen = HashSet::new();
    for &v in &t {
        assert!(seen.insert(v), "Duplicate vertex {} in tour", v);
    }
    assert_eq!(seen.len(), 48, "Tour must visit all 48 distinct vertices");
}










