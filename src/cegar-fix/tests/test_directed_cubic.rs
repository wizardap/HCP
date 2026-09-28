use cegar_fix::core::graph::Graph;
use cegar_fix::core::tour_verifier::TourVerifier;
use cegar_fix::solver::directed_cegar::{can_solve_directed_cubic, solve_directed_cubic};
use std::path::Path;

fn build_cube_graph() -> Graph {
    // 8-vertex 3-regular cube graph
    // Vertices: 1..8
    // Bottom square: 1-2, 2-3, 3-4, 4-1
    // Top square: 5-6, 6-7, 7-8, 8-5
    // Pillars: 1-5, 2-6, 3-7, 4-8
    let mut g = Graph::new();
    let edges = [
        (1, 2), (2, 3), (3, 4), (4, 1),
        (5, 6), (6, 7), (7, 8), (8, 5),
        (1, 5), (2, 6), (3, 7), (4, 8),
    ];
    for (u, v) in edges {
        g.add_edge(u, v);
    }
    g
}

fn build_petersen_graph() -> Graph {
    // 10-vertex 3-regular Petersen graph (hypohamiltonian / non-Hamiltonian)
    let mut g = Graph::new();
    // Outer 5-cycle: 1-2, 2-3, 3-4, 4-5, 5-1
    let outer = [(1, 2), (2, 3), (3, 4), (4, 5), (5, 1)];
    // Inner star: 6-8, 8-10, 10-7, 7-9, 9-6
    let inner = [(6, 8), (8, 10), (10, 7), (7, 9), (9, 6)];
    // Spokes: 1-6, 2-7, 3-8, 4-9, 5-10
    let spokes = [(1, 6), (2, 7), (3, 8), (4, 9), (5, 10)];
    for (u, v) in outer.into_iter().chain(inner).chain(spokes) {
        g.add_edge(u, v);
    }
    g
}

#[test]
fn test_can_solve_directed_cubic() {
    let cube = build_cube_graph();
    assert!(can_solve_directed_cubic(&cube), "Cube is 3-regular");

    let petersen = build_petersen_graph();
    assert!(can_solve_directed_cubic(&petersen), "Petersen is 3-regular");

    // Non-cubic: C6 has all degree 2
    let mut c6 = Graph::new();
    for i in 1..=6 {
        let nxt = if i == 6 { 1 } else { i + 1 };
        c6.add_edge(i, nxt);
    }
    assert!(!can_solve_directed_cubic(&c6), "C6 is 2-regular, not cubic");
}

#[test]
fn test_directed_cubic_petersen_unsat() {
    let petersen = build_petersen_graph();
    let res = solve_directed_cubic(&petersen, 5.0);
    assert!(res.is_err(), "Petersen graph must be UNSAT");
    let err_str = res.unwrap_err();
    assert!(err_str.contains("UNSAT") || err_str.contains("INFEASIBLE"));
}

#[test]
fn test_directed_cubic_cube_hamiltonian() {
    let cube = build_cube_graph();
    let tour = solve_directed_cubic(&cube, 5.0).expect("Cube has Hamiltonian cycle");
    assert_eq!(tour.len(), 8);
    let (ok, msg) = TourVerifier::verify(&cube, &tour);
    assert!(ok, "Tour verification failed: {}", msg);
}

#[test]
fn test_directed_cubic_graph707() {
    let candidates = [
        "FHCPCS-col/graph707.col",
        "../../FHCPCS-col/graph707.col",
        "../../../FHCPCS-col/graph707.col",
    ];
    let path = candidates.iter().find(|p| Path::new(p).is_file());
    if let Some(&p) = path {
        let g = cegar_fix::core::file_operations::parse_graph_from_file(p).expect("parse graph707");
        assert!(can_solve_directed_cubic(&g));
        let start = std::time::Instant::now();
        let tour = solve_directed_cubic(&g, 10.0).expect("solve graph707 within 10s");
        let elapsed = start.elapsed().as_secs_f64();
        println!("graph707 solved in {:.3}s", elapsed);
        assert!(elapsed < 5.0, "graph707 must be solved in under 5.0s");
        assert_eq!(tour.len(), 4050);
        let (ok, msg) = TourVerifier::verify(&g, &tour);
        assert!(ok, "Tour verification failed: {}", msg);
    }
}

#[test]
fn test_directed_near_cubic_graph12_and_21() {
    let candidates_12 = [
        "FHCPCS-col/graph12.col",
        "../../FHCPCS-col/graph12.col",
        "../../../FHCPCS-col/graph12.col",
    ];
    if let Some(&p) = candidates_12.iter().find(|p| Path::new(p).is_file()) {
        let g = cegar_fix::core::file_operations::parse_graph_from_file(p).expect("parse graph12");
        assert!(can_solve_directed_cubic(&g), "graph12 must be detected as near-cubic");
        let start = std::time::Instant::now();
        let tour = solve_directed_cubic(&g, 10.0).expect("solve graph12 within 10s");
        let elapsed = start.elapsed().as_secs_f64();
        println!("graph12 solved in {:.3}s", elapsed);
        assert!(elapsed < 5.0, "graph12 must be solved in under 5.0s");
        assert_eq!(tour.len(), 132);
        let (ok, msg) = TourVerifier::verify(&g, &tour);
        assert!(ok, "Tour verification failed for graph12: {}", msg);
    }

    let candidates_21 = [
        "FHCPCS-col/graph21.col",
        "../../FHCPCS-col/graph21.col",
        "../../../FHCPCS-col/graph21.col",
    ];
    if let Some(&p) = candidates_21.iter().find(|p| Path::new(p).is_file()) {
        let g = cegar_fix::core::file_operations::parse_graph_from_file(p).expect("parse graph21");
        assert!(can_solve_directed_cubic(&g), "graph21 must be detected as near-cubic");
        let start = std::time::Instant::now();
        let tour = solve_directed_cubic(&g, 15.0).expect("solve graph21 within 15s");
        let elapsed = start.elapsed().as_secs_f64();
        println!("graph21 solved in {:.3}s", elapsed);
        assert!(elapsed < 10.0, "graph21 must be solved in under 10.0s");
        assert_eq!(tour.len(), 180);
        let (ok, msg) = TourVerifier::verify(&g, &tour);
        assert!(ok, "Tour verification failed for graph21: {}", msg);
    }
}
