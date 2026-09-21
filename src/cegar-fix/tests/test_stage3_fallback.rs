use cegar_fix::fallback_cegar;
use cegar_fix::file_operations::input_to_graph;
use cegar_fix::tour_verifier::TourVerifier;
use std::path::Path;

#[test]
fn test_stage3_fallback_graph76() {
    let graph_path = "FHCPCS-col/graph76.col";
    let alt_graph_path = "../../FHCPCS-col/graph76.col";
    let full_path = "/home/ubuntu/HCP/FHCPCS-col/graph76.col";

    let path = if Path::new(graph_path).exists() {
        graph_path
    } else if Path::new(alt_graph_path).exists() {
        alt_graph_path
    } else if Path::new(full_path).exists() {
        full_path
    } else {
        panic!("graph76.col not found in search paths");
    };

    let graph = input_to_graph(path);
    let res = fallback_cegar::solve_with_contraction(&graph, 30.0);

    assert!(res.is_ok(), "Expected Ok(tour), got Err: {:?}", res.err());
    let tour = res.unwrap();

    assert_eq!(tour.len(), 471, "Expected tour length 471, got {}", tour.len());

    let (is_valid, err_msg) = TourVerifier::verify(&graph, &tour);
    assert!(is_valid, "TourVerifier failed: {}", err_msg);
}
