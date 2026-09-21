use cegar_fix::solver_pipeline::{find_graph_file, save_checkpoint_atomic, solve_single_graph, BatchItemResult};
use std::fs;
use std::path::Path;

#[test]
fn test_find_graph_file() {
    let path_opt = find_graph_file(1);
    assert!(path_opt.is_some(), "Should find graph1.col");
    let path = path_opt.unwrap();
    assert!(Path::new(&path).is_file(), "Found path must be an existing file");
}

#[test]
fn test_solve_single_graph_graph1() {
    let graph_path = find_graph_file(1).expect("graph1.col must exist");
    let tour_out = "target/test_tour_graph1.hcp";
    if Path::new(tour_out).exists() {
        let _ = fs::remove_file(tour_out);
    }

    let result = solve_single_graph(&graph_path, 30.0, Some(tour_out));
    assert!(result.is_ok(), "graph1 must be solved successfully: {:?}", result.err());

    let (tour, elapsed, vertices) = result.unwrap();
    assert_eq!(vertices, 66);
    assert_eq!(tour.len(), 66);
    assert!(elapsed < 30.0);

    // Verify tour file was written and is valid TSPLIB
    assert!(Path::new(tour_out).exists(), "Output tour file must exist");
    let content = fs::read_to_string(tour_out).expect("Must read tour file");
    assert!(content.contains("TYPE : TOUR"));
    assert!(content.contains("DIMENSION : 66"));
    assert!(content.contains("TOUR_SECTION"));
    assert!(content.contains("-1\nEOF"));

    let _ = fs::remove_file(tour_out);
}

#[test]
fn test_solve_single_graph_graph76_fallback() {
    let graph_path = find_graph_file(76).expect("graph76.col must exist");
    let tour_out = "target/test_tour_graph76.hcp";
    if Path::new(tour_out).exists() {
        let _ = fs::remove_file(tour_out);
    }

    let result = solve_single_graph(&graph_path, 30.0, Some(tour_out));
    assert!(result.is_ok(), "graph76 must be solved via fallback CEGAR: {:?}", result.err());

    let (tour, _elapsed, vertices) = result.unwrap();
    assert_eq!(vertices, 471);
    assert_eq!(tour.len(), 471);

    assert!(Path::new(tour_out).exists(), "Output tour file must exist for graph76");
    let _ = fs::remove_file(tour_out);
}

#[test]
fn test_save_checkpoint_atomic() {
    let checkpoint_path = "target/test_checkpoint.json";
    if Path::new(checkpoint_path).exists() {
        let _ = fs::remove_file(checkpoint_path);
    }

    let items = vec![
        BatchItemResult {
            gid: 1,
            status: "SAT_VERIFIED".to_string(),
            time: 0.52,
            vertices: 66,
            err: None,
        },
        BatchItemResult {
            gid: 2,
            status: "TIMEOUT".to_string(),
            time: 60.0,
            vertices: 100,
            err: Some("Timeout reached".to_string()),
        },
    ];

    let res = save_checkpoint_atomic(&items, checkpoint_path);
    assert!(res.is_ok(), "Atomic save must succeed");
    assert!(Path::new(checkpoint_path).exists(), "Checkpoint file must exist");

    let content = fs::read_to_string(checkpoint_path).unwrap();
    let loaded: Vec<BatchItemResult> = serde_json::from_str(&content).unwrap();
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded[0].gid, 1);
    assert_eq!(loaded[0].status, "SAT_VERIFIED");
    assert_eq!(loaded[1].gid, 2);
    assert_eq!(loaded[1].status, "TIMEOUT");

    let _ = fs::remove_file(checkpoint_path);
}
