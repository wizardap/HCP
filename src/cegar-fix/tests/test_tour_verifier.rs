use cegar_fix::graph::Graph;
use cegar_fix::tour_verifier::TourVerifier;
use std::fs;

#[test]
fn test_tour_verifier_soundness() {
    let mut g = Graph::new();
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 1);

    // Valid cycle
    assert!(TourVerifier::verify_raw_tour(&[1, 2, 3, 4], &g).is_ok());
    assert!(TourVerifier::verify_raw_tour(&[4, 3, 2, 1], &g).is_ok());

    // Invalid length
    assert!(TourVerifier::verify_raw_tour(&[1, 2, 3], &g).is_err());
    assert!(TourVerifier::verify_raw_tour(&[1, 2, 3, 4, 1], &g).is_err());

    // Non-existent edge
    assert!(TourVerifier::verify_raw_tour(&[1, 3, 2, 4], &g).is_err());

    // Duplicate vertex
    assert!(TourVerifier::verify_raw_tour(&[1, 2, 2, 4], &g).is_err());

    // Non-existent vertex
    assert!(TourVerifier::verify_raw_tour(&[1, 2, 3, 5], &g).is_err());
}

#[test]
fn test_write_tsplib_hcp() {
    let tour = vec![1, 2, 3, 4];
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("test_tour_output.hcp");
    let temp_path = temp_file.to_str().unwrap();

    let res = TourVerifier::write_tsplib_hcp(&tour, "test_cycle_4", temp_path);
    assert!(res.is_ok());

    let content = fs::read_to_string(temp_path).unwrap();
    let lines: Vec<&str> = content.lines().collect();

    assert_eq!(lines[0], "NAME : test_cycle_4");
    assert_eq!(lines[1], "TYPE : TOUR");
    assert_eq!(lines[2], "DIMENSION : 4");
    assert_eq!(lines[3], "TOUR_SECTION");
    assert_eq!(lines[4], "1");
    assert_eq!(lines[5], "2");
    assert_eq!(lines[6], "3");
    assert_eq!(lines[7], "4");
    assert_eq!(lines[8], "-1");
    assert_eq!(lines[9], "EOF");

    let _ = fs::remove_file(temp_path);
}
