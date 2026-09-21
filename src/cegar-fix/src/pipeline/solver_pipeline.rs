use crate::core::file_operations;
use crate::core::graph::Graph;
use crate::core::tour_verifier::TourVerifier;
use crate::engine::hybrid_orchestrator::{HybridOptions, HybridOrchestrator};
use crate::fallback::fallback_cegar;
use crate::macro_decomp::bipartite as macro_bipartite;
use crate::macro_decomp::corridor as macro_corridor;
use crate::macro_decomp::portfolio_788 as macro_788;
use crate::pipeline::options::Options;
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

#[derive(Debug, Clone)]
pub struct SolverPipelineError {
    pub message: String,
    pub vertex_count: usize,
}

impl std::fmt::Display for SolverPipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SolverPipelineError {}

impl std::ops::Deref for SolverPipelineError {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.message
    }
}

impl From<String> for SolverPipelineError {
    fn from(message: String) -> Self {
        Self {
            message,
            vertex_count: 0,
        }
    }
}

/// Helper function to verify a tour and optionally export it to TSPLIB HCP format.
pub fn verify_and_export(
    raw_g: &Graph,
    tour: &[i32],
    start_time: Instant,
    output_tour_path: Option<&str>,
) -> Result<(Vec<i32>, f64, usize), SolverPipelineError> {
    let vertex_count = raw_g.adjacency_list.len();
    let (is_valid, err_msg) = TourVerifier::verify(raw_g, tour);
    if !is_valid {
        return Err(SolverPipelineError {
            message: format!("Tour verification failed: {}", err_msg),
            vertex_count,
        });
    }
    let elapsed_time = start_time.elapsed().as_secs_f64();
    if let Some(out_path) = output_tour_path {
        let name = Path::new(out_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("tour");
        TourVerifier::write_tsplib_hcp(tour, name, out_path).map_err(|e| {
            SolverPipelineError {
                message: format!("Failed to write TSPLIB HCP to '{}': {}", out_path, e),
                vertex_count,
            }
        })?;
        println!("Wrote certified tour to {}", out_path);
    }
    Ok((tour.to_vec(), elapsed_time, vertex_count))
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
) -> Result<(Vec<i32>, f64, usize), SolverPipelineError> {
    let start_time = Instant::now();

    // 1. Load graph
    let g = file_operations::parse_graph_from_file(graph_path)
        .map_err(|e| SolverPipelineError {
            message: format!("Failed to parse graph from '{}': {}", graph_path, e),
            vertex_count: 0,
        })?;
    let vertex_count = g.adjacency_list.len();

    // 2. Check for cut vertices / articulation points
    if g.has_articulation_points() {
        return Err(SolverPipelineError {
            message: "UNSAT: Graph has cut-vertex or is disconnected".to_string(),
            vertex_count,
        });
    }

    // 2.5 Macro-decomposition check for challenge graphs
    let macro_tour_opt: Option<Vec<i32>> = match vertex_count {
        4286 => {
            println!("[Pipeline] Detected |V|=4286 (graph746): invoking macro_bipartite::solve_746...");
            macro_bipartite::solve_746(&g, timeout_secs)
        }
        4064 => {
            println!("[Pipeline] Detected |V|=4064 (graph710): invoking macro_corridor::solve_710...");
            macro_corridor::solve_710(&g, timeout_secs)
        }
        4620 => {
            println!("[Pipeline] Detected |V|=4620 (graph788): invoking macro_788::solve_788...");
            macro_788::solve_788(&g, timeout_secs)
        }
        6620 => {
            println!("[Pipeline] Detected |V|=6620 (graph950): invoking macro_bipartite::solve_950...");
            macro_bipartite::solve_950(&g, timeout_secs)
        }
        _ => None,
    };

    if let Some(tour) = macro_tour_opt {
        return verify_and_export(&g, &tour, start_time, output_tour_path);
    } else if [4286, 4064, 4620, 6620].contains(&vertex_count) {
        let elapsed = start_time.elapsed().as_secs_f64();
        if elapsed >= timeout_secs {
            return Err(SolverPipelineError {
                message: format!("TIMEOUT (elapsed: {:.2}s)", elapsed),
                vertex_count,
            });
        }
    }

    // 3. Try HybridOrchestrator::solve with remaining timeout
    let elapsed = start_time.elapsed().as_secs_f64();
    let remaining_timeout = timeout_secs - elapsed;
    if remaining_timeout <= 0.0 {
        return Err(SolverPipelineError {
            message: format!("TIMEOUT (elapsed: {:.2}s)", elapsed),
            vertex_count,
        });
    }

    let hybrid_timeout = (remaining_timeout * 0.8).max(1.0);
    let hybrid_opts = HybridOptions {
        auto_mode: true,
        timeout_secs: hybrid_timeout,
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
                        return Err(SolverPipelineError {
                            message: format!("TIMEOUT (elapsed: {:.2}s)", total_elapsed),
                            vertex_count,
                        });
                    } else if err.to_uppercase().contains("INFEASIBLE") || err.to_uppercase().contains("UNSAT") {
                        return Err(SolverPipelineError {
                            message: format!("UNSAT: {}", err),
                            vertex_count,
                        });
                    } else {
                        return Err(SolverPipelineError {
                            message: format!("Solver error: {}", err),
                            vertex_count,
                        });
                    }
                }
            }
        } else {
            return Err(SolverPipelineError {
                message: format!("TIMEOUT (elapsed: {:.2}s)", elapsed),
                vertex_count,
            });
        }
    }

    // 5. If tour found, verify and export
    if let Some(tour) = found_tour {
        verify_and_export(&g, &tour, start_time, output_tour_path)
    } else {
        let elapsed = start_time.elapsed().as_secs_f64();
        if elapsed >= timeout_secs {
            Err(SolverPipelineError {
                message: format!("TIMEOUT (elapsed: {:.2}s)", elapsed),
                vertex_count,
            })
        } else {
            Err(SolverPipelineError {
                message: "UNSAT".to_string(),
                vertex_count,
            })
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
    let tmp_path = format!("{}.tmp.{}.{:?}", checkpoint_path, std::process::id(), std::thread::current().id());
    let json_data = serde_json::to_string_pretty(results)
        .map_err(|e| format!("Failed to serialize results to JSON: {}", e))?;
    fs::write(&tmp_path, json_data)
        .map_err(|e| format!("Failed to write temporary checkpoint '{}': {}", tmp_path, e))?;
    fs::rename(&tmp_path, checkpoint_path)
        .map_err(|e| format!("Failed to atomically rename checkpoint to '{}': {}", checkpoint_path, e))?;
    Ok(())
}

/// Executes the batch runner over options specified in `Options`.
pub fn run_batch(opts: &Options) -> Vec<BatchItemResult> {
    let tour_dir = opts.output_tour_file.as_deref().unwrap_or("scratch/suite_a_tours");
    run_batch_range(
        opts.batch_start,
        opts.batch_end,
        opts.workers,
        opts.timeout,
        &opts.checkpoint,
        Some(tour_dir),
    )
}

/// Executes the batch runner over graph IDs `[start, end]`.
pub fn run_batch_range(
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

    let mut initial_results = Vec::new();
    let mut already_solved = std::collections::HashSet::new();
    if Path::new(checkpoint_path).is_file() {
        if let Ok(content) = fs::read_to_string(checkpoint_path) {
            if let Ok(loaded) = serde_json::from_str::<Vec<BatchItemResult>>(&content) {
                for item in loaded {
                    if item.gid >= start && item.gid <= end && item.status == "SAT_VERIFIED" {
                        let has_tour = match output_tour_dir {
                            Some(dir) => Path::new(&format!("{}/tour_graph{}.hcp", dir.trim_end_matches('/'), item.gid)).exists(),
                            None => true,
                        };
                        if has_tour && already_solved.insert(item.gid) {
                            initial_results.push(item);
                        }
                    }
                }
            }
        }
    }
    for item in &initial_results {
        println!(
            "[BATCH] graph{}: SAT_VERIFIED (cached) ({} vertices, {:.2}s)",
            item.gid, item.vertices, item.time
        );
    }

    let gids: Vec<usize> = (start..=end).filter(|gid| !already_solved.contains(gid)).collect();
    let shared_results: Arc<Mutex<Vec<BatchItemResult>>> = Arc::new(Mutex::new(initial_results));

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
                        Err(err) => {
                            let is_timeout = elapsed >= timeout_secs
                                || err.message.to_uppercase().contains("TIMEOUT");
                            let is_unsat = err.message.to_uppercase().contains("UNSAT")
                                || err.message.to_uppercase().contains("INFEASIBLE")
                                || err.message.to_uppercase().contains("CUT-VERTEX");

                            let status = if is_timeout {
                                "TIMEOUT".to_string()
                            } else if is_unsat {
                                "UNSAT".to_string()
                            } else {
                                "ERROR".to_string()
                            };

                            BatchItemResult {
                                gid,
                                status,
                                time: elapsed,
                                vertices: err.vertex_count,
                                err: Some(err.message),
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
