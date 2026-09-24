use cegar_fix::core::tour_verifier::TourVerifier;
use cegar_fix::pipeline::solver_pipeline::solve_single_graph;
use std::fs::File;
use std::io::Write;

#[test]
fn test_petersen_graph_is_unsat() {
    std::fs::create_dir_all("scratch").unwrap();
    let tmp_path = "scratch/test_petersen.col";
    let mut f = File::create(tmp_path).unwrap();
    writeln!(f, "p edge 10 15").unwrap();
    // Outer 5-cycle: 1-2, 2-3, 3-4, 4-5, 5-1
    writeln!(f, "e 1 2\ne 2 3\ne 3 4\ne 4 5\ne 5 1").unwrap();
    // Spokes: 1-6, 2-7, 3-8, 4-9, 5-10
    writeln!(f, "e 1 6\ne 2 7\ne 3 8\ne 4 9\ne 5 10").unwrap();
    // Inner star: 6-8, 8-10, 10-7, 7-9, 9-6
    writeln!(f, "e 6 8\ne 8 10\ne 10 7\ne 7 9\ne 9 6").unwrap();

    let res = solve_single_graph(tmp_path, 10.0, None);
    assert!(res.is_err(), "Petersen graph must be UNSAT");
    assert!(res.unwrap_err().message.contains("UNSAT"));
}

#[test]
fn test_vertex_permutation_invariance() {
    std::fs::create_dir_all("scratch").unwrap();
    let tmp_orig = "scratch/test_invar_orig.col";
    let tmp_perm = "scratch/test_invar_perm.col";
    let n = 8;
    let pi = |x: i32| -> i32 { (x * 3) % n + 1 };

    let mut f1 = File::create(tmp_orig).unwrap();
    let mut f2 = File::create(tmp_perm).unwrap();
    writeln!(f1, "p edge {} {}", n, n).unwrap();
    writeln!(f2, "p edge {} {}", n, n).unwrap();

    for i in 1..=n {
        let nxt = if i == n { 1 } else { i + 1 };
        writeln!(f1, "e {} {}", i, nxt).unwrap();
        writeln!(f2, "e {} {}", pi(i), pi(nxt)).unwrap();
    }

    let res1 = solve_single_graph(tmp_orig, 5.0, None).expect("Original solves");
    let res2 = solve_single_graph(tmp_perm, 5.0, None).expect("Permuted solves");
    assert_eq!(res1.0.len(), n as usize);
    assert_eq!(res2.0.len(), n as usize);

    let g1 = cegar_fix::core::file_operations::parse_graph_from_file(tmp_orig).unwrap();
    let (valid1, err1) = TourVerifier::verify(&g1, &res1.0);
    assert!(valid1, "Original tour must be valid: {}", err1);

    let g2 = cegar_fix::core::file_operations::parse_graph_from_file(tmp_perm).unwrap();
    let (valid2, err2) = TourVerifier::verify(&g2, &res2.0);
    assert!(valid2, "Permuted tour must be valid: {}", err2);
}

#[test]
fn test_permutation_invariance_2cut_graph() {
    std::fs::create_dir_all("scratch").unwrap();
    let tmp_orig = "scratch/test_invar_2cut_orig.col";
    let tmp_perm = "scratch/test_invar_2cut_perm.col";

    // 8 vertices, 2-cut between {1, 2}
    let edges = [
        (1, 3), (3, 4), (4, 2), (1, 4),
        (1, 5), (5, 6), (6, 7), (7, 8), (8, 2), (5, 8), (2, 1)
    ];
    let n = 8;
    // Permutation: map x -> (x * 5) % 8 + 1
    let pi = |x: i32| -> i32 { ((x - 1) * 3 + 4) % n + 1 };

    let mut f1 = File::create(tmp_orig).unwrap();
    let mut f2 = File::create(tmp_perm).unwrap();
    writeln!(f1, "p edge {} {}", n, edges.len()).unwrap();
    writeln!(f2, "p edge {} {}", n, edges.len()).unwrap();

    for &(u, v) in &edges {
        writeln!(f1, "e {} {}", u, v).unwrap();
        writeln!(f2, "e {} {}", pi(u), pi(v)).unwrap();
    }

    let res1 = solve_single_graph(tmp_orig, 10.0, None).expect("Original 2-cut solves");
    let res2 = solve_single_graph(tmp_perm, 10.0, None).expect("Permuted 2-cut solves");
    assert_eq!(res1.0.len(), n as usize);
    assert_eq!(res2.0.len(), n as usize);

    let g1 = cegar_fix::core::file_operations::parse_graph_from_file(tmp_orig).unwrap();
    let (valid1, err1) = TourVerifier::verify(&g1, &res1.0);
    assert!(valid1, "Original 2-cut tour must be valid: {}", err1);

    let g2 = cegar_fix::core::file_operations::parse_graph_from_file(tmp_perm).unwrap();
    let (valid2, err2) = TourVerifier::verify(&g2, &res2.0);
    assert!(valid2, "Permuted 2-cut tour must be valid: {}", err2);
}
