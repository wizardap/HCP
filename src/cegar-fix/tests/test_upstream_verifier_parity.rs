use cegar_fix::core::file_operations;
use cegar_fix::core::tour_verifier::TourVerifier;
use cegar_fix::fallback_cegar;

#[test]
fn test_graph1_upstream_verifier_parity() {
    let graph_path = "../../FHCPCS-col/graph1.col";
    let g = file_operations::parse_graph_from_file(graph_path).expect("Failed to parse graph1");
    let tour = fallback_cegar::solve_with_contraction(&g, 30.0).expect("Failed to solve graph1");
    
    let (valid, err) = TourVerifier::verify(&g, &tour);
    assert!(valid, "Internal TourVerifier failed: {}", err);

    let upstream_valid = TourVerifier::verify_upstream_python(graph_path, &tour)
        .expect("Failed to run upstream is_hamiltonian.py");
    assert!(upstream_valid, "Upstream is_hamiltonian.py returned False");
}
