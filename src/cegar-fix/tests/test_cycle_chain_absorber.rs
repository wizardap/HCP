use cegar_fix::graph::Graph;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::hub_registry::HubRegistry;
use cegar_fix::cycle_chain_absorber::CycleChainAbsorber;

#[test]
fn test_multi_cycle_chain_and_absorb() {
    let mut g = Graph::new();
    // Giant cycle: 1 - 2 - 3 - 4 - 5 - 6 - 1
    for i in 1..=6 {
        g.add_edge(i, if i == 6 { 1 } else { i + 1 });
    }
    // Small cycle 1: 10 - 11 - 12 - 10
    g.add_edge(10, 11); g.add_edge(11, 12); g.add_edge(12, 10);
    // Small cycle 2: 20 - 21 - 22 - 20
    g.add_edge(20, 21); g.add_edge(21, 22); g.add_edge(22, 20);

    // Chaining edges between small cycle 1 and small cycle 2:
    // (11, 20) and (12, 21)
    g.add_edge(11, 20);
    g.add_edge(12, 21);

    // Absorption edges into Giant Cycle: (10, 2) and (22, 3)
    g.add_edge(10, 2);
    g.add_edge(22, 3);

    let cycles = vec![
        vec![1, 2, 3, 4, 5, 6],
        vec![10, 11, 12],
        vec![20, 21, 22],
    ];

    let contractor = Degree2Contractor::new();
    let hubs = HubRegistry::new(&g);
    let result = CycleChainAbsorber::absorb_all(&cycles, &g, &contractor, &hubs);

    assert_eq!(result.len(), 1, "Expected all cycles to be chained and absorbed into 1 cycle");
    assert_eq!(result[0].len(), 12, "Total cycle length must equal 12 vertices");
}

#[test]
fn test_single_small_cycle_absorb() {
    let mut g = Graph::new();
    // Giant cycle: 1 - 2 - 3 - 4 - 5 - 6 - 1
    for i in 1..=6 {
        g.add_edge(i, if i == 6 { 1 } else { i + 1 });
    }
    // Small cycle: 7 - 8 - 9 - 7
    g.add_edge(7, 8); g.add_edge(8, 9); g.add_edge(9, 7);
    // Bridge edges: (2, 7) and (9, 3)
    g.add_edge(2, 7);
    g.add_edge(9, 3);

    let cycles = vec![
        vec![1, 2, 3, 4, 5, 6],
        vec![7, 8, 9],
    ];

    let contractor = Degree2Contractor::new();
    let hubs = HubRegistry::new(&g);
    let result = CycleChainAbsorber::absorb_all(&cycles, &g, &contractor, &hubs);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].len(), 9);
}

#[test]
fn test_three_cycle_chain_absorb() {
    let mut g = Graph::new();
    // Giant cycle: 1 - 2 - 3 - 4 - 5 - 6 - 1
    for i in 1..=6 {
        g.add_edge(i, if i == 6 { 1 } else { i + 1 });
    }
    // Small cycle 1: 10 - 11 - 12 - 10
    g.add_edge(10, 11); g.add_edge(11, 12); g.add_edge(12, 10);
    // Small cycle 2: 20 - 21 - 22 - 20
    g.add_edge(20, 21); g.add_edge(21, 22); g.add_edge(22, 20);
    // Small cycle 3: 30 - 31 - 32 - 30
    g.add_edge(30, 31); g.add_edge(31, 32); g.add_edge(32, 30);

    // Chaining edges:
    // S1 to S2: (12, 20)
    g.add_edge(12, 20);
    // S2 to S3: (22, 30)
    g.add_edge(22, 30);

    // Absorption edges into Giant Cycle: (10, 2) and (32, 3)
    g.add_edge(10, 2);
    g.add_edge(32, 3);

    let cycles = vec![
        vec![1, 2, 3, 4, 5, 6],
        vec![10, 11, 12],
        vec![20, 21, 22],
        vec![30, 31, 32],
    ];

    let contractor = Degree2Contractor::new();
    let hubs = HubRegistry::new(&g);
    let result = CycleChainAbsorber::absorb_all(&cycles, &g, &contractor, &hubs);

    assert_eq!(result.len(), 1, "Expected all 3 small cycles to be chained and absorbed into 1 cycle");
    assert_eq!(result[0].len(), 15, "Total cycle length must equal 15 vertices");
}

#[test]
fn test_protected_edge_preservation() {
    let mut g = Graph::new();
    // Giant cycle: 1 - 2 - 3 - 4 - 5 - 6 - 1
    for i in 1..=6 {
        g.add_edge(i, if i == 6 { 1 } else { i + 1 });
    }
    // Small cycle: 10 - 11 - 12 - 10
    g.add_edge(10, 11); g.add_edge(11, 12); g.add_edge(12, 10);
    // Bridge edges to (2, 3)
    g.add_edge(10, 2);
    g.add_edge(12, 3);

    let cycles = vec![
        vec![1, 2, 3, 4, 5, 6],
        vec![10, 11, 12],
    ];

    // Protect edge (2, 3) so it cannot be broken
    let mut contractor = Degree2Contractor::new();
    contractor.chain_map.insert((2, 3), vec![99]);

    let hubs = HubRegistry::new(&g);
    let result = CycleChainAbsorber::absorb_all(&cycles, &g, &contractor, &hubs);

    // Since (2, 3) is protected, absorption should not break (2, 3)
    assert_eq!(result.len(), 2, "Absorption should be rejected because edge (2, 3) is protected");
}

#[test]
fn test_trivial_single_or_empty_cycles() {
    let g = Graph::new();
    let contractor = Degree2Contractor::new();
    let hubs = HubRegistry::new(&g);

    let empty: Vec<Vec<i32>> = Vec::new();
    assert_eq!(CycleChainAbsorber::absorb_all(&empty, &g, &contractor, &hubs).len(), 0);

    let single = vec![vec![1, 2, 3, 4]];
    let res = CycleChainAbsorber::absorb_all(&single, &g, &contractor, &hubs);
    assert_eq!(res.len(), 1);
    assert_eq!(res[0], vec![1, 2, 3, 4]);
}

#[test]
fn test_unabsorbable_disconnected_cycles() {
    let mut g = Graph::new();
    // Giant cycle 1-2-3-4-1
    g.add_edge(1, 2); g.add_edge(2, 3); g.add_edge(3, 4); g.add_edge(4, 1);
    // Small cycle 10-11-12-10 disconnected
    g.add_edge(10, 11); g.add_edge(11, 12); g.add_edge(12, 10);

    let cycles = vec![
        vec![1, 2, 3, 4],
        vec![10, 11, 12],
    ];

    let contractor = Degree2Contractor::new();
    let hubs = HubRegistry::new(&g);
    let res = CycleChainAbsorber::absorb_all(&cycles, &g, &contractor, &hubs);
    assert_eq!(res.len(), 2, "Disconnected cycles should not be absorbed");
}

#[test]
fn test_composite_cycle_merge_then_absorb() {
    let mut g = Graph::new();
    // Giant cycle: 1 - 2 - 3 - 4 - 5 - 6 - 7 - 8 - 1
    for i in 1..=8 {
        g.add_edge(i, if i == 8 { 1 } else { i + 1 });
    }
    // Small cycle 1: 10 - 11 - 12 - 10
    g.add_edge(10, 11); g.add_edge(11, 12); g.add_edge(12, 10);
    // Small cycle 2: 20 - 21 - 22 - 20
    g.add_edge(20, 21); g.add_edge(21, 22); g.add_edge(22, 20);

    // 2 cross edges between S1 and S2:
    g.add_edge(11, 20);
    g.add_edge(12, 21);

    // Bridge edges from S1 into Giant:
    g.add_edge(10, 2);
    g.add_edge(11, 3);

    let cycles = vec![
        vec![1, 2, 3, 4, 5, 6, 7, 8],
        vec![10, 11, 12],
        vec![20, 21, 22],
    ];

    let contractor = Degree2Contractor::new();
    let hubs = HubRegistry::new(&g);
    let res = CycleChainAbsorber::absorb_all(&cycles, &g, &contractor, &hubs);
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].len(), 14);
}

#[test]
fn test_3opt_absorption() {
    let mut g = Graph::new();
    // Giant cycle: 1 - 2 - 3 - 4 - 5 - 6 - 7 - 8 - 1
    for i in 1..=8 {
        g.add_edge(i, if i == 8 { 1 } else { i + 1 });
    }
    // Small cycle: 10 - 11 - 12 - 10
    g.add_edge(10, 11); g.add_edge(11, 12); g.add_edge(12, 10);

    // 3-opt connections:
    // u1 = 2 (p1=1), u2 = 3. u1 connected to 10 (start_v).
    // u3 = 6 (p3=5), u4 = 7. u3 connected to 12 (exit_v).
    // cross edge between u2 (3) and u4 (7)
    g.add_edge(2, 10);
    g.add_edge(12, 6);
    g.add_edge(3, 7);

    let cycles = vec![
        vec![1, 2, 3, 4, 5, 6, 7, 8],
        vec![10, 11, 12],
    ];

    let contractor = Degree2Contractor::new();
    let hubs = HubRegistry::new(&g);
    let res = CycleChainAbsorber::absorb_all(&cycles, &g, &contractor, &hubs);
    assert_eq!(res.len(), 1, "Expected 3-opt absorption to absorb small cycle into 1 giant cycle");
    assert_eq!(res[0].len(), 11);
}
