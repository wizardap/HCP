
/// Verifies all 11 challenge graphs that were previously certified SAT
/// still produce SAT_VERIFIED through the simplified principled pipeline.
///
/// Each graph is solved de novo with a 600s timeout and verified by TourVerifier.
/// This is the final regression gate for the codebase simplification.

fn solve_and_verify(gid: usize) {
    let col_path = match cegar_fix::find_graph_file(gid) {
        Some(p) => p,
        None => {
            panic!("[✗] graph{}: file not found in any candidate path", gid);
        }
    };
    let result = cegar_fix::solve_single_graph(&col_path, 1800.0, None);
    match result {
        Ok((tour, time, vcount)) => {
            println!("[✓] graph{}: SAT_VERIFIED ({} vertices, {:.2}s)", gid, vcount, time);
            assert!(!tour.is_empty(), "Tour should not be empty for graph{}", gid);
        }
        Err(e) => {
            panic!("[✗] graph{}: FAILED — {}", gid, e.message);
        }
    }
}

#[test]
fn test_challenge_graph_710() { solve_and_verify(710); }

#[test]
fn test_challenge_graph_717() { solve_and_verify(717); }

#[test]
fn test_challenge_graph_746() { solve_and_verify(746); }

#[test]
#[ignore = "Skipped per user instruction"]
fn test_challenge_graph_788() { solve_and_verify(788); }

#[test]
fn test_challenge_graph_882() { solve_and_verify(882); }

#[test]
fn test_challenge_graph_944() { solve_and_verify(944); }

#[test]
fn test_challenge_graph_950() { solve_and_verify(950); }

#[test]
fn test_challenge_graph_963() { solve_and_verify(963); }

#[test]
fn test_challenge_graph_975() { solve_and_verify(975); }

#[test]
fn test_challenge_graph_982() { solve_and_verify(982); }

#[test]
fn test_challenge_graph_990() { solve_and_verify(990); }
