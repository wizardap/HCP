use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use cegar_fix::file_operations;
use cegar_fix::modular_ring_dp_solver::RingDpSolver;

#[test]
fn test_solve_graph868_modular_dp() {
    let graph_path = "FHCPCS-col/graph868.col";
    let alt_graph_path = "../../FHCPCS-col/graph868.col";
    let path = if std::path::Path::new(graph_path).exists() {
        graph_path
    } else if std::path::Path::new(alt_graph_path).exists() {
        alt_graph_path
    } else {
        "/home/ubuntu/HCP/FHCPCS-col/graph868.col"
    };

    let raw_g = file_operations::input_to_graph(path);

    let tour = RingDpSolver::solve(&raw_g)
        .expect("RingDpSolver must find a certified Hamiltonian tour for graph868");

    assert_eq!(tour.len(), 5544, "Tour must visit all 5,544 vertices");

    let unique: HashSet<i32> = tour.iter().copied().collect();
    assert_eq!(unique.len(), 5544, "All vertices must be unique");

    // Verify every edge in tour exists in raw_g.adjacency_list
    for i in 0..5544 {
        let u = tour[i];
        let v = tour[(i + 1) % 5544];
        let nbrs = raw_g.adjacency_list.get(&u).expect("Vertex must exist");
        assert!(nbrs.contains(&v), "Edge ({}, {}) must exist in raw graph!", u, v);
    }

    // Write tour to scratch file for independent python verifier
    let tour_path = "/home/ubuntu/HCP/scratch/graph868_dp_found.tour";
    cegar_fix::tour_verifier::TourVerifier::write_tsplib_hcp(&tour, "graph868", tour_path)
        .expect("Unable to write TSPLIB tour file");
}
