use cegar_fix::file_operations::input_to_graph;
use cegar_fix::global_demand_coordinator::GlobalDemandCoordinator;
use cegar_fix::graph::Graph;
use cegar_fix::two_tier_decomposer::decompose_graph;
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[test]
fn test_synthetic_coordinator_assignment() {
    let mut g = Graph::new();
    // Auxiliary hubs 10..35 form a cycle to satisfy degree 2
    for i in 10..35 {
        g.add_edge(i, i + 1);
    }
    g.add_edge(35, 10);

    // Make auxiliary hubs have degree >= 20 by adding clique-like chords
    for i in 10..35 {
        for j in (i + 2)..=35 {
            if !(i == 10 && j == 35) {
                g.add_edge(i, j);
            }
        }
    }

    // 4 Hubs: 1, 2, 3, 4 connected to auxiliary hubs to have degree >= 20
    for i in 10..30 {
        g.add_edge(1, i);
        g.add_edge(2, i);
        g.add_edge(3, i);
        g.add_edge(4, i);
    }
    // HH edges: (1, 2), (3, 4)
    g.add_edge(1, 2);
    g.add_edge(3, 4);

    // Strip 1: 101-102 connected to Hub 1 and Hub 3
    g.add_edge(101, 102);
    g.add_edge(1, 101);
    g.add_edge(3, 102);

    // Strip 2: 201-202 connected to Hub 2 and Hub 4
    g.add_edge(201, 202);
    g.add_edge(2, 201);
    g.add_edge(4, 202);

    let decomp = decompose_graph(&g);
    assert!(decomp.all_hubs.contains(&1));
    assert!(decomp.all_hubs.contains(&2));
    assert!(decomp.all_hubs.contains(&3));
    assert!(decomp.all_hubs.contains(&4));
    assert_eq!(decomp.strips.len(), 2);

    let mut coord = GlobalDemandCoordinator::new(&g, &decomp);
    let res = coord.solve_assignment();
    assert!(res.is_some(), "Expected SAT for synthetic coordinator");

    let (hh_edges, strip_demands) = res.unwrap();

    // Verify degree == 2 on all hubs
    let mut deg_map = HashMap::new();
    for &h in &decomp.all_hubs {
        deg_map.insert(h, 0);
    }

    for (u, v) in &hh_edges {
        if decomp.all_hubs.contains(u) {
            *deg_map.get_mut(u).unwrap() += 1;
        }
        if decomp.all_hubs.contains(v) {
            *deg_map.get_mut(v).unwrap() += 1;
        }
    }

    for (_si, d_map) in &strip_demands {
        for (h, &d) in d_map {
            if decomp.all_hubs.contains(h) {
                *deg_map.get_mut(h).unwrap() += d;
            }
        }
    }

    for (&h, &d) in &deg_map {
        assert_eq!(d, 2, "Hub {} must have degree exactly 2, got {}", h, d);
    }
}

#[test]
fn test_synthetic_coordinator_conflict_clause() {
    let mut g = Graph::new();
    // Auxiliary hubs 10..35
    for i in 10..35 {
        for j in (i + 1)..=35 {
            g.add_edge(i, j);
        }
    }
    for i in 10..30 {
        g.add_edge(1, i);
        g.add_edge(2, i);
    }
    g.add_edge(1, 2);

    // Strip 1: 101-102
    g.add_edge(101, 102);
    g.add_edge(1, 101);
    g.add_edge(2, 102);

    let decomp = decompose_graph(&g);
    let mut coord = GlobalDemandCoordinator::new(&g, &decomp);

    let si = decomp.strips.iter().position(|s| s.contains(&101)).unwrap();
    let mut seen_demands = HashSet::new();

    for _ in 0..10 {
        let res = coord.solve_assignment();
        if let Some((_hh, strip_demands)) = res {
            let current_dem = strip_demands.get(&si).unwrap().clone();
            let d1 = current_dem.get(&1).copied().unwrap_or(0);
            let d2 = current_dem.get(&2).copied().unwrap_or(0);
            assert!(
                seen_demands.insert((d1, d2)),
                "Demand assignment ({}, {}) repeated after blocking",
                d1,
                d2
            );

            let failed_hubs: Vec<i32> = current_dem.keys().copied().collect();
            coord.add_conflict_clause(si, &current_dem, &failed_hubs);
        } else {
            // Reached UNSAT after exhausting all demand assignments
            assert!(
                !seen_demands.is_empty(),
                "Must have explored at least one demand before UNSAT"
            );
            break;
        }
    }

    // After exhausting demands, must be UNSAT
    let final_res = coord.solve_assignment();
    assert!(
        final_res.is_none(),
        "Expected UNSAT after exhausting all feasible demand assignments"
    );
}

#[test]
fn test_synthetic_coordinator_macro_cut() {
    let mut g = Graph::new();
    // Auxiliary hubs 10..35
    for i in 10..35 {
        for j in (i + 1)..=35 {
            g.add_edge(i, j);
        }
    }
    // 4 Hubs: 1, 2, 3, 4
    for i in 10..30 {
        g.add_edge(1, i);
        g.add_edge(2, i);
        g.add_edge(3, i);
        g.add_edge(4, i);
    }
    // Disconnected hub components: (1-2) and (3-4)
    g.add_edge(1, 2);
    g.add_edge(3, 4);

    // Strips: 101-102 (between 1 and 2), 201-202 (between 3 and 4)
    // 301-302 (between 2 and 3)
    g.add_edge(101, 102);
    g.add_edge(1, 101);
    g.add_edge(2, 102);

    g.add_edge(201, 202);
    g.add_edge(3, 201);
    g.add_edge(4, 202);

    g.add_edge(301, 302);
    g.add_edge(2, 301);
    g.add_edge(3, 302);

    let decomp = decompose_graph(&g);
    let mut coord = GlobalDemandCoordinator::new(&g, &decomp);

    // Add macro cut between {1, 2} and all other hubs
    let mut cyc = HashSet::new();
    cyc.insert(1);
    cyc.insert(2);
    coord.add_macro_cut(&cyc);

    let res = coord.solve_assignment();
    assert!(res.is_some(), "Expected SAT with macro cut");
    let (_hh, strip_demands) = res.unwrap();

    // Check that bridging strip 301-302 (connecting 2 and 3) has active demands or crossing HH
    let si_bridge = decomp.strips.iter().position(|s| s.contains(&301)).unwrap();
    let bridge_dem = strip_demands.get(&si_bridge).unwrap();
    assert!(bridge_dem.get(&2).copied().unwrap_or(0) >= 1);
    assert!(bridge_dem.get(&3).copied().unwrap_or(0) >= 1);
}

#[test]
fn test_graph950_coordinator_solving_and_bounds() {
    let candidate_paths = [
        "../../FHCPCS-col/graph950.col",
        "../FHCPCS-col/graph950.col",
        "/home/ubuntu/HCP/FHCPCS-col/graph950.col",
    ];

    let path_str = match candidate_paths.iter().find(|p| Path::new(p).exists()) {
        Some(p) => *p,
        None => {
            eprintln!("Skipping graph950 integration test: FHCPCS-col/graph950.col not found on disk.");
            return;
        }
    };

    let g = input_to_graph(path_str);
    let decomp = decompose_graph(&g);
    assert_eq!(decomp.all_hubs.len(), 310);

    let mut coord = GlobalDemandCoordinator::new(&g, &decomp);
    let res = coord.solve_assignment();
    assert!(res.is_some(), "Expected SAT for graph950 global demand coordinator");

    let (hh_edges, strip_demands) = res.unwrap();

    // 1. Check exact-2 degree on ALL 310 Hubs
    let mut deg_map = HashMap::new();
    for &h in &decomp.all_hubs {
        deg_map.insert(h, 0);
    }

    for (u, v) in &hh_edges {
        if decomp.all_hubs.contains(u) {
            *deg_map.get_mut(u).unwrap() += 1;
        }
        if decomp.all_hubs.contains(v) {
            *deg_map.get_mut(v).unwrap() += 1;
        }
    }

    for (_si, d_map) in &strip_demands {
        for (h, &d) in d_map {
            if decomp.all_hubs.contains(h) {
                *deg_map.get_mut(h).unwrap() += d;
            }
        }
    }

    for (&h, &d) in &deg_map {
        assert_eq!(d, 2, "Hub {} degree must be exactly 2, got {}", h, d);
    }

    // 2. Check Parity & Endpoint bounds for all strips
    for (si, strip) in decomp.strips.iter().enumerate() {
        let d_map = strip_demands.get(&si).expect("Demands must exist for strip");
        let total_dem: usize = d_map.values().sum();

        if strip.len() < 10 {
            // Small strip: must have total demand == 2 (K=1)
            assert_eq!(
                total_dem, 2,
                "Small strip {} (len {}) must have total demand 2, got {}",
                si,
                strip.len(),
                total_dem
            );
        } else {
            // Large strip: must have total demand in {4, 6, 8, 10}
            assert!(
                total_dem == 4 || total_dem == 6 || total_dem == 8 || total_dem == 10,
                "Large strip {} (len {}) must have total demand in {{4, 6, 8, 10}}, got {}",
                si,
                strip.len(),
                total_dem
            );
        }
    }

    // 3. Test adding a conflict clause and re-solving
    let test_si = 0;
    let dem0 = strip_demands.get(&test_si).unwrap().clone();
    let failed: Vec<i32> = dem0.keys().copied().filter(|&h| dem0.get(&h).copied().unwrap_or(0) > 0).collect();
    coord.add_conflict_clause(test_si, &dem0, &failed);

    let res_after = coord.solve_assignment();
    assert!(res_after.is_some(), "Expected SAT for alternative global assignment");
    let (_, strip_demands_after) = res_after.unwrap();
    let dem0_after = strip_demands_after.get(&test_si).unwrap();
    
    // The same demand on strip 0 must not be assigned again if failed hubs were blocked
    let is_identical = failed.iter().all(|&h| dem0.get(&h) == dem0_after.get(&h));
    assert!(!is_identical, "Strip 0 demand should have changed after conflict clause");
}
