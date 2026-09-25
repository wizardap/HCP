use cegar_fix::core::tour_verifier::TourVerifier;
use cegar_fix::pipeline::solver_pipeline::solve_single_graph;
use std::fs::File;
use std::io::Write;

#[test]
fn test_pipeline_on_synthetic_2cut_graph() {
    std::fs::create_dir_all("scratch").unwrap();
    let tmp_path = "scratch/test_pipeline_2cut.col";
    let mut f = File::create(tmp_path).unwrap();
    writeln!(f, "p edge 8 11").unwrap();
    // Block A: 1-3, 3-4, 4-2, 1-4
    writeln!(f, "e 1 3\ne 3 4\ne 4 2\ne 1 4").unwrap();
    // Block B: 1-5, 5-6, 6-7, 7-8, 8-2, 5-8
    writeln!(f, "e 1 5\ne 5 6\ne 6 7\ne 7 8\ne 8 2\ne 5 8\ne 2 1").unwrap();

    let res = solve_single_graph(tmp_path, 10.0, None);
    assert!(res.is_ok(), "Principled pipeline must solve 2-cut graph: {:?}", res.err());
    let (tour, _elapsed, v_count) = res.unwrap();
    assert_eq!(v_count, 8);
    assert_eq!(tour.len(), 8);

    let g = cegar_fix::core::file_operations::parse_graph_from_file(tmp_path).unwrap();
    let (valid, err) = TourVerifier::verify(&g, &tour);
    assert!(valid, "Tour must be verified by TourVerifier: {}", err);
}

#[test]
fn test_pipeline_fast_filter_rejection() {
    std::fs::create_dir_all("scratch").unwrap();
    let tmp_path = "scratch/test_pipeline_unsat.col";
    let mut f = File::create(tmp_path).unwrap();
    writeln!(f, "p edge 5 4").unwrap();
    // Star graph with cut-vertex at 1
    writeln!(f, "e 1 2\ne 1 3\ne 1 4\ne 1 5").unwrap();

    let res = solve_single_graph(tmp_path, 5.0, None);
    assert!(res.is_err(), "Must be rejected as UNSAT");
    let err = res.err().unwrap();
    assert!(err.message.contains("UNSAT"), "Error must indicate UNSAT: {}", err.message);
}

#[test]
fn test_pipeline_series_cycle() {
    std::fs::create_dir_all("scratch").unwrap();
    let tmp_path = "scratch/test_pipeline_cycle10.col";
    let mut f = File::create(tmp_path).unwrap();
    let n = 10;
    writeln!(f, "p edge {} {}", n, n).unwrap();
    for i in 1..=n {
        let nxt = if i == n { 1 } else { i + 1 };
        writeln!(f, "e {} {}", i, nxt).unwrap();
    }

    let res = solve_single_graph(tmp_path, 5.0, None);
    assert!(res.is_ok(), "Cycle 10 must be solved: {:?}", res.err());
    let (tour, _elapsed, v_count) = res.unwrap();
    assert_eq!(v_count, 10);
    assert_eq!(tour.len(), 10);

    let g = cegar_fix::core::file_operations::parse_graph_from_file(tmp_path).unwrap();
    let (valid, err) = TourVerifier::verify(&g, &tour);
    assert!(valid, "Cycle tour must be verified: {}", err);
}

use cegar_fix::pipeline::solver_pipeline::find_graph_file;

#[test]
fn test_pipeline_zero_budget_dispatch() {
    let p = find_graph_file(1).expect("graph1.col must be found");
    let res = solve_single_graph(&p, 10.0, None);
    assert!(res.is_ok(), "graph1 should solve cleanly: {:?}", res.err());
}
