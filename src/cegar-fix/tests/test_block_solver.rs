use cegar_fix::core::graph::Graph;
use cegar_fix::core::tour_verifier::TourVerifier;
use cegar_fix::solver::block_solver::{solve_hamiltonian_cycle, solve_hamiltonian_path};

#[test]
fn test_solve_cycle_triangle() {
    let mut g = Graph::new();
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 1);
    let tour = solve_hamiltonian_cycle(&g, 5.0).expect("Triangle is Hamiltonian");
    let (valid, err) = TourVerifier::verify(&g, &tour);
    assert!(valid, "Tour must be certified: {}", err);
}

#[test]
fn test_solve_path_between_ports() {
    let mut g = Graph::new();
    // 4-cycle 1-2-3-4-1
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 1);
    // Path from 1 to 4 visiting all vertices: 1 -> 2 -> 3 -> 4
    let path = solve_hamiltonian_path(&g, 1, 4, 5.0).expect("Hamiltonian path must exist");
    assert_eq!(path.len(), 4);
    assert_eq!(*path.first().unwrap(), 1);
    assert_eq!(*path.last().unwrap(), 4);
}

#[test]
fn test_solve_cycle_unsat() {
    let mut g = Graph::new();
    // Claw graph K_{1,3}: center 0 connected to 1, 2, 3. Cannot have Hamiltonian cycle.
    g.add_edge(0, 1);
    g.add_edge(0, 2);
    g.add_edge(0, 3);
    let res = solve_hamiltonian_cycle(&g, 5.0);
    assert!(res.is_err(), "Claw graph should be UNSAT");
}

#[test]
fn test_solve_path_unsat() {
    let mut g = Graph::new();
    // 1-2, 3-4 (disconnected)
    g.add_edge(1, 2);
    g.add_edge(3, 4);
    let res = solve_hamiltonian_path(&g, 1, 4, 5.0);
    assert!(res.is_err(), "Disconnected graph should have no Hamiltonian path");
}

#[test]
fn test_dfj_cut_separation_prism() {
    // 3-prism graph (K_3 x K_2):
    // Two triangles: (1,2,3) and (4,5,6)
    // Vertical rungs: (1,4), (2,5), (3,6)
    // Degree-2 assignment could pick the two disjoint triangles,
    // so DFJ cuts are necessary to eliminate subcycles and find the 6-cycle.
    let mut g = Graph::new();
    g.add_edge(1, 2); g.add_edge(2, 3); g.add_edge(3, 1);
    g.add_edge(4, 5); g.add_edge(5, 6); g.add_edge(6, 4);
    g.add_edge(1, 4); g.add_edge(2, 5); g.add_edge(3, 6);

    let tour = solve_hamiltonian_cycle(&g, 5.0).expect("3-prism is Hamiltonian");
    let (valid, err) = TourVerifier::verify(&g, &tour);
    assert!(valid, "Tour must be certified: {}", err);
}

#[test]
fn test_solve_cycle_petersen_unsat() {
    // Petersen graph is hypohamiltonian: 10 vertices, 15 edges, no Hamiltonian cycle.
    let mut g = Graph::new();
    // Outer 5-cycle: 0-1-2-3-4-0
    for i in 0..5 {
        g.add_edge(i, (i + 1) % 5);
    }
    // Inner 5-star: 5-7-9-6-8-5
    g.add_edge(5, 7); g.add_edge(7, 9); g.add_edge(9, 6); g.add_edge(6, 8); g.add_edge(8, 5);
    // Spokes: (i, i + 5)
    for i in 0..5 {
        g.add_edge(i, i + 5);
    }

    let res = solve_hamiltonian_cycle(&g, 5.0);
    assert!(res.is_err(), "Petersen graph must be UNSAT");
}

#[test]
fn test_solve_path_prism() {
    // 3-prism: path from 1 to 6 visiting all 6 vertices:
    // e.g. 1 -> 4 -> 5 -> 2 -> 3 -> 6
    let mut g = Graph::new();
    g.add_edge(1, 2); g.add_edge(2, 3); g.add_edge(3, 1);
    g.add_edge(4, 5); g.add_edge(5, 6); g.add_edge(6, 4);
    g.add_edge(1, 4); g.add_edge(2, 5); g.add_edge(3, 6);

    let path = solve_hamiltonian_path(&g, 1, 6, 5.0).expect("Hamiltonian path exists");
    assert_eq!(path.len(), 6);
    assert_eq!(*path.first().unwrap(), 1);
    assert_eq!(*path.last().unwrap(), 6);

    // Verify all edges in path exist in g
    for i in 0..path.len() - 1 {
        let u = path[i];
        let v = path[i + 1];
        assert!(g.adjacency_list[&u].contains(&v), "Edge ({}, {}) must exist in g", u, v);
    }
}
