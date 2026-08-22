use cegar_fix::file_operations::input_to_graph;
use cegar_fix::graph::Graph;
use cegar_fix::pinpointed_strip_solver::PinpointedStripSolver;
use cegar_fix::two_tier_decomposer::decompose_graph;
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[test]
fn test_synthetic_strip_sat() {
    let mut g = Graph::new();
    // Hub 1 and Hub 2 (degree >= 20 to be M-hubs)
    for i in 10..30 {
        g.add_edge(1, i);
        g.add_edge(2, i);
    }
    // Hub 1 connects to 101, Hub 2 connects to 102
    g.add_edge(1, 101);
    g.add_edge(2, 102);

    // Strip vertices: 101-102
    g.add_edge(101, 102);

    let decomp = decompose_graph(&g);
    let si = decomp.strips.iter().position(|s| s.contains(&101)).expect("Strip must exist");
    assert_eq!(decomp.strips[si], vec![101, 102]);

    let mut solver = PinpointedStripSolver::new(&g, &decomp);

    let mut dem = HashMap::new();
    dem.insert(1, 1);
    dem.insert(2, 1);

    let res = solver.solve_strip(si, &dem, None, None, 1);
    assert!(res.is_ok(), "Expected SAT for strip 101-102 with demand {{1: 1, 2: 1}}");

    let paths = res.unwrap();
    assert_eq!(paths.len(), 1, "Expected 1 path for K=1");
    let p = &paths[0];
    assert_eq!(p.len(), 2);
    assert!((p[0] == 101 && p[1] == 102) || (p[0] == 102 && p[1] == 101));
}

#[test]
fn test_synthetic_strip_unsat_core() {
    let mut g = Graph::new();
    // Hubs 1, 2, 3 (degree >= 20)
    for i in 10..30 {
        g.add_edge(1, i);
        g.add_edge(2, i);
        g.add_edge(3, i);
    }
    // Strip vertices: 101-102-103
    g.add_edge(101, 102);
    g.add_edge(102, 103);

    // Connections to hubs:
    g.add_edge(1, 101);
    g.add_edge(2, 102);
    g.add_edge(3, 103);

    let decomp = decompose_graph(&g);
    let si = decomp.strips.iter().position(|s| s.contains(&101)).expect("Strip must exist");

    let mut solver = PinpointedStripSolver::new(&g, &decomp);

    // Request 3 demands for K=1 (which requires exactly 2 endpoints) -> UNSAT
    let mut dem = HashMap::new();
    dem.insert(1, 1);
    dem.insert(2, 1);
    dem.insert(3, 1);

    let res = solver.solve_strip(si, &dem, None, None, 1);
    assert!(res.is_err(), "Expected UNSAT for strip with 3 demands and K=1");

    let core = res.unwrap_err();
    assert!(!core.is_empty(), "UNSAT core must not be empty");
    for h in &core {
        assert!(dem.contains_key(h), "Core hub {} must be in dem", h);
    }
}

#[test]
fn test_synthetic_strip_subtour_elimination() {
    let mut g = Graph::new();
    // Hub 1 and Hub 2 (degree >= 20)
    for i in 10..30 {
        g.add_edge(1, i);
        g.add_edge(2, i);
    }

    // Strip vertices: 101, 102, 103, 104
    // Edges: 101-102, 102-103, 103-101 (forms a 3-cycle!), plus 103-104
    g.add_edge(101, 102);
    g.add_edge(102, 103);
    g.add_edge(103, 101);
    g.add_edge(103, 104);

    // Hub 1 connects to 101, Hub 2 connects to 104
    g.add_edge(1, 101);
    g.add_edge(2, 104);

    let decomp = decompose_graph(&g);
    let si = decomp.strips.iter().position(|s| s.contains(&101)).expect("Strip must exist");

    let mut solver = PinpointedStripSolver::new(&g, &decomp);

    let mut dem = HashMap::new();
    dem.insert(1, 1);
    dem.insert(2, 1);

    let res = solver.solve_strip(si, &dem, None, None, 1);
    assert!(res.is_ok(), "Expected SAT for strip after subtour elimination");

    let paths = res.unwrap();
    assert_eq!(paths.len(), 1);
    let p = &paths[0];
    assert_eq!(p.len(), 4);
    // All 4 vertices must be covered in path order: 101-102-103-104 (or reversed)
    let is_valid_order = (p[0] == 101 && p[1] == 102 && p[2] == 103 && p[3] == 104)
        || (p[0] == 104 && p[1] == 103 && p[2] == 102 && p[3] == 101);
    assert!(is_valid_order, "Path must traverse 101-102-103-104: {:?}", p);
}

#[test]
fn test_synthetic_multi_path_cover_k2() {
    let mut g = Graph::new();
    // Hubs 1, 2, 3, 4 (degree >= 20)
    for i in 10..30 {
        g.add_edge(1, i);
        g.add_edge(2, i);
        g.add_edge(3, i);
        g.add_edge(4, i);
    }

    // Strip vertices: 101-102-103-104
    // Path 1: 101-102
    // Path 2: 103-104
    g.add_edge(101, 102);
    g.add_edge(102, 103); // bridge edge
    g.add_edge(103, 104);

    // External connections:
    // Hub 1 -> 101, Hub 2 -> 102
    // Hub 3 -> 103, Hub 4 -> 104
    g.add_edge(1, 101);
    g.add_edge(2, 102);
    g.add_edge(3, 103);
    g.add_edge(4, 104);

    let decomp = decompose_graph(&g);
    let si = decomp.strips.iter().position(|s| s.contains(&101)).expect("Strip must exist");

    let mut solver = PinpointedStripSolver::new(&g, &decomp);

    let mut dem = HashMap::new();
    dem.insert(1, 1);
    dem.insert(2, 1);
    dem.insert(3, 1);
    dem.insert(4, 1);

    // K = 2 paths (4 endpoints)
    let res = solver.solve_strip(si, &dem, None, None, 2);
    assert!(res.is_ok(), "Expected SAT for K=2 path cover");

    let paths = res.unwrap();
    assert_eq!(paths.len(), 2, "Expected exactly 2 paths for K=2");

    let mut all_v = HashSet::new();
    for p in &paths {
        for &v in p {
            assert!(all_v.insert(v), "Vertex {} duplicated across paths", v);
        }
    }
    assert_eq!(all_v.len(), 4);
}

#[test]
fn test_graph950_strip_solving() {
    let candidate_paths = [
        "../../FHCPCS-col/graph950.col",
        "../FHCPCS-col/graph950.col",
        "/home/ubuntu/HCP/FHCPCS-col/graph950.col",
    ];

    let path_str = candidate_paths
        .iter()
        .find(|p| Path::new(p).exists())
        .expect("graph950.col file must exist");

    let g = input_to_graph(path_str);
    let decomp = decompose_graph(&g);

    let mut solver = PinpointedStripSolver::new(&g, &decomp);

    // 1. Test on a small strip (size 2)
    let small_si = decomp.strips.iter().position(|s| s.len() == 2).expect("Small strip exists");
    let small_strip = &decomp.strips[small_si];
    let adj_hubs: Vec<i32> = decomp.strip_adj_hubs.get(&small_si).unwrap().iter().copied().collect();
    assert!(adj_hubs.len() >= 2, "Small strip must be adjacent to at least 2 hubs");

    let h1 = adj_hubs[0];
    let h2 = adj_hubs[1];
    let mut dem_sat = HashMap::new();
    dem_sat.insert(h1, 1);
    dem_sat.insert(h2, 1);

    let h1_has_edge = small_strip.iter().any(|&v| g.adjacency_list.get(&v).map_or(false, |adj| adj.contains(&h1)));
    let h2_has_edge = small_strip.iter().any(|&v| g.adjacency_list.get(&v).map_or(false, |adj| adj.contains(&h2)));

    if h1_has_edge && h2_has_edge {
        let res = solver.solve_strip(small_si, &dem_sat, None, None, 1);
        if let Ok(paths) = res {
            assert_eq!(paths.len(), 1);
            let mut visited = HashSet::new();
            for p in paths {
                for v in p {
                    visited.insert(v);
                }
            }
            assert_eq!(visited.len(), small_strip.len());
        }
    }

    // Test UNSAT with impossible demand (total demand 3 > 2K for K=1)
    let mut dem_unsat = HashMap::new();
    for &h in adj_hubs.iter().take(3) {
        dem_unsat.insert(h, 1);
    }
    if dem_unsat.len() == 3 {
        let res = solver.solve_strip(small_si, &dem_unsat, None, None, 1);
        assert!(res.is_err(), "Expected UNSAT when total demand 3 > 2K");
        let core = res.unwrap_err();
        assert!(!core.is_empty(), "Core must not be empty");
        for h in &core {
            assert!(dem_unsat.contains_key(h));
        }
    }

    // 2. Test on a large strip (size 125)
    let large_si = decomp.strips.iter().position(|s| s.len() == 125).expect("Large strip exists");
    let _large_strip = &decomp.strips[large_si];
    let large_adj_hubs: Vec<i32> = decomp.strip_adj_hubs.get(&large_si).unwrap().iter().copied().collect();

    // UNSAT test on large strip: total demand 1 (odd demand != 2K)
    let mut dem_large_odd = HashMap::new();
    dem_large_odd.insert(large_adj_hubs[0], 1);
    let res_odd = solver.solve_strip(large_si, &dem_large_odd, None, None, 1);
    assert!(res_odd.is_err(), "Expected UNSAT for odd demand sum with K=1");
    let core_odd = res_odd.unwrap_err();
    assert!(!core_odd.is_empty());
}
