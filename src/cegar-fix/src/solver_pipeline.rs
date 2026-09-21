use crate::file_operations;
use crate::hybrid_orchestrator::{HybridOptions, HybridOrchestrator};
use crate::fallback_cegar;
use crate::tour_verifier::TourVerifier;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BatchItemResult {
    pub gid: usize,
    pub status: String, // "SAT_VERIFIED", "TIMEOUT", "ERROR", "UNSAT"
    pub time: f64,
    pub vertices: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub err: Option<String>,
}

/// Attempts to locate a graph file for a given graph ID across common directories.
pub fn find_graph_file(gid: usize) -> Option<String> {
    let candidates = [
        format!("FHCPCS-col/graph{}.col", gid),
        format!("../FHCPCS-col/graph{}.col", gid),
        format!("../../FHCPCS-col/graph{}.col", gid),
    ];
    for cand in &candidates {
        if Path::new(cand).is_file() {
            return Some(cand.clone());
        }
    }
    None
}

/// Unified solving pipeline function:
/// 1. Load graph from `graph_path` using `file_operations::parse_graph_from_file(graph_path)`.
/// 2. Check for cut vertices / articulation points (`g.has_articulation_points()`).
/// 3. Try `HybridOrchestrator::solve(&g, &opts)` with timeout.
/// 4. If not solved or returns None, try `fallback_cegar::solve_with_contraction(&g, remaining_timeout)`.
/// 5. If tour found, verify with `TourVerifier::verify(&g, &tour)`.
/// 6. If sound:
///    - Write TSPLIB HCP file if `output_tour_path` is specified.
///    - Return `Ok((tour, elapsed_time, vertex_count))`.
/// 7. Else return `Err(reason)`.
pub fn solve_single_graph(
    graph_path: &str,
    timeout_secs: f64,
    output_tour_path: Option<&str>,
) -> Result<(Vec<i32>, f64, usize), String> {
    let start_time = Instant::now();

    // 1. Load graph
    let g = file_operations::parse_graph_from_file(graph_path)
        .map_err(|e| format!("Failed to parse graph from '{}': {}", graph_path, e))?;
    let vertex_count = g.adjacency_list.len();

    // 2. Check for cut vertices / articulation points
    if g.has_articulation_points() {
        return Err("UNSAT: Graph has cut-vertex or is disconnected".to_string());
    }

    // 3. Try HybridOrchestrator::solve with timeout
    let hybrid_opts = HybridOptions {
        auto_mode: true,
        timeout_secs,
        output_tour: None, // Verified tour will be written at the end if output_tour_path is specified
        macro_gadget: false,
        bounded_freezer: false,
        alternating_engine: true,
    };

    let mut found_tour = HybridOrchestrator::solve(&g, &hybrid_opts);

    // 4. If not solved or returns None, try fallback_cegar::solve_with_contraction
    if found_tour.is_none() {
        let elapsed = start_time.elapsed().as_secs_f64();
        let remaining_timeout = timeout_secs - elapsed;
        if remaining_timeout > 0.0 {
            println!(
                "HybridOrchestrator returned None; invoking fallback CEGAR (remaining timeout: {:.2}s)...",
                remaining_timeout
            );
            match fallback_cegar::solve_with_contraction(&g, remaining_timeout) {
                Ok(tour) => {
                    found_tour = Some(tour);
                }
                Err(err) => {
                    let total_elapsed = start_time.elapsed().as_secs_f64();
                    if total_elapsed >= timeout_secs || err.to_uppercase().contains("TIMEOUT") {
                        return Err(format!("TIMEOUT (elapsed: {:.2}s)", total_elapsed));
                    } else if err.to_uppercase().contains("INFEASIBLE") || err.to_uppercase().contains("UNSAT") {
                        return Err(format!("UNSAT: {}", err));
                    } else {
                        return Err(format!("Solver error: {}", err));
                    }
                }
            }
        } else {
            return Err(format!("TIMEOUT (elapsed: {:.2}s)", elapsed));
        }
    }

    // 5. If tour found, verify with TourVerifier::verify
    if let Some(tour) = found_tour {
        let (is_valid, err_msg) = TourVerifier::verify(&g, &tour);
        if !is_valid {
            return Err(format!("Tour verification failed: {}", err_msg));
        }
        let elapsed_time = start_time.elapsed().as_secs_f64();

        // 6. If sound, write TSPLIB HCP file if output_tour_path is specified
        if let Some(out_path) = output_tour_path {
            let name = Path::new(graph_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("tour");
            TourVerifier::write_tsplib_hcp(&tour, name, out_path)
                .map_err(|e| format!("Failed to write TSPLIB HCP to '{}': {}", out_path, e))?;
            println!("Wrote certified tour to {}", out_path);
        }

        Ok((tour, elapsed_time, vertex_count))
    } else {
        let elapsed = start_time.elapsed().as_secs_f64();
        if elapsed >= timeout_secs {
            Err(format!("TIMEOUT (elapsed: {:.2}s)", elapsed))
        } else {
            Err("UNSAT".to_string())
        }
    }
}

/// Atomically saves the checkpoint results to a JSON file.
pub fn save_checkpoint_atomic(results: &[BatchItemResult], checkpoint_path: &str) -> Result<(), String> {
    let path = Path::new(checkpoint_path);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = fs::create_dir_all(parent);
        }
    }
    let tmp_path = format!("{}.tmp.{}", checkpoint_path, std::process::id());
    let json_data = serde_json::to_string_pretty(results)
        .map_err(|e| format!("Failed to serialize results to JSON: {}", e))?;
    fs::write(&tmp_path, json_data)
        .map_err(|e| format!("Failed to write temporary checkpoint '{}': {}", tmp_path, e))?;
    fs::rename(&tmp_path, checkpoint_path)
        .map_err(|e| format!("Failed to atomically rename checkpoint to '{}': {}", checkpoint_path, e))?;
    Ok(())
}

/// Executes the batch runner over graph IDs `[start, end]`.
pub fn run_batch(
    start: usize,
    end: usize,
    workers: usize,
    timeout_secs: f64,
    checkpoint_path: &str,
    output_tour_dir: Option<&str>,
) -> Vec<BatchItemResult> {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .build()
        .expect("Failed to create Rayon thread pool");

    let shared_results: Arc<Mutex<Vec<BatchItemResult>>> = Arc::new(Mutex::new(Vec::new()));
    let gids: Vec<usize> = (start..=end).collect();

    pool.install(|| {
        gids.into_par_iter().for_each(|gid| {
            let graph_file_opt = find_graph_file(gid);
            let item = match graph_file_opt {
                None => {
                    let res = BatchItemResult {
                        gid,
                        status: "MISSING_FILE".to_string(),
                        time: 0.0,
                        vertices: 0,
                        err: Some(format!("Graph file not found for graph{}", gid)),
                    };
                    println!("[BATCH] graph{}: MISSING_FILE (0 vertices, 0.00s)", gid);
                    res
                }
                Some(ref graph_path) => {
                    let start_t = Instant::now();
                    let tour_out = output_tour_dir.map(|dir| {
                        format!("{}/tour_graph{}.hcp", dir.trim_end_matches('/'), gid)
                    });
                    let res = solve_single_graph(graph_path, timeout_secs, tour_out.as_deref());
                    let elapsed = start_t.elapsed().as_secs_f64();

                    let item = match res {
                        Ok((_tour, solve_time, vertices)) => {
                            BatchItemResult {
                                gid,
                                status: "SAT_VERIFIED".to_string(),
                                time: solve_time,
                                vertices,
                                err: None,
                            }
                        }
                        Err(err_msg) => {
                            let is_timeout = elapsed >= timeout_secs
                                || err_msg.to_uppercase().contains("TIMEOUT");
                            let is_unsat = err_msg.to_uppercase().contains("UNSAT")
                                || err_msg.to_uppercase().contains("INFEASIBLE")
                                || err_msg.to_uppercase().contains("CUT-VERTEX");

                            let status = if is_timeout {
                                "TIMEOUT".to_string()
                            } else if is_unsat {
                                "UNSAT".to_string()
                            } else {
                                "ERROR".to_string()
                            };

                            let vertices = file_operations::parse_graph_from_file(graph_path)
                                .map(|g| g.adjacency_list.len())
                                .unwrap_or(0);

                            BatchItemResult {
                                gid,
                                status,
                                time: elapsed,
                                vertices,
                                err: Some(err_msg),
                            }
                        }
                    };

                    println!(
                        "[BATCH] graph{}: {} ({} vertices, {:.2}s)",
                        item.gid, item.status, item.vertices, item.time
                    );
                    item
                }
            };

            let snapshot_to_save = {
                let mut results = shared_results.lock().unwrap();
                results.push(item);
                let count = results.len();
                if count % 10 == 0 {
                    let mut sorted = results.clone();
                    sorted.sort_by_key(|r| r.gid);
                    Some(sorted)
                } else {
                    None
                }
            };

            if let Some(snapshot) = snapshot_to_save {
                if let Err(e) = save_checkpoint_atomic(&snapshot, checkpoint_path) {
                    eprintln!("Warning: failed to save checkpoint: {}", e);
                }
            }
        });
    });

    let final_results = {
        let mut results = shared_results.lock().unwrap();
        results.sort_by_key(|r| r.gid);
        results.clone()
    };
    let _ = save_checkpoint_atomic(&final_results, checkpoint_path);

    let total = final_results.len();
    let sat_count = final_results.iter().filter(|r| r.status == "SAT_VERIFIED").count();
    let timeout_count = final_results.iter().filter(|r| r.status == "TIMEOUT").count();
    let error_count = final_results.iter().filter(|r| r.status == "ERROR").count();
    let unsat_count = final_results.iter().filter(|r| r.status == "UNSAT").count();
    let missing_count = final_results.iter().filter(|r| r.status == "MISSING_FILE").count();
    let sat_pct = if total > 0 {
        (sat_count as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    println!("\n================ BATCH SUMMARY ================");
    println!("Total graphs tested: {}", total);
    println!("SAT_VERIFIED:        {} ({:.1}%)", sat_count, sat_pct);
    println!("TIMEOUT:             {}", timeout_count);
    println!("ERROR:               {}", error_count);
    if unsat_count > 0 {
        println!("UNSAT:               {}", unsat_count);
    }
    if missing_count > 0 {
        println!("MISSING_FILE:        {}", missing_count);
    }
    println!("Results saved to:    {}", checkpoint_path);
    println!("===============================================\n");

    final_results
}
