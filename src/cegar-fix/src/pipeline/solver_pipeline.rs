use crate::assembly::tour_stitcher::stitch_subpath;
use crate::core::file_operations;
use crate::core::graph::Graph;
use crate::core::tour_verifier::TourVerifier;
use crate::decomp::fast_filters::check_fast_invariants;
use crate::decomp::spqr_parallel::{extract_subcomponent_graph, find_separation_pairs};
use crate::decomp::spqr_series::{contract_series_chains, expand_series_tour};
use crate::macro_decomp::corridor as macro_corridor;
use crate::macro_decomp::dynamic_bipartite;
use crate::macro_decomp::portfolio_788 as macro_788;
use crate::pipeline::options::Options;
use crate::solver::cegar_engine::{solve_cycle, solve_path};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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

    // Step 1: Fast filters (connectivity, min degree, 1-cut, bipartite parity)
    if let Err(err_msg) = check_fast_invariants(&g) {
        return Err(SolverPipelineError {
            message: err_msg.to_string(),
            vertex_count,
        });
    }
    let deadline = start_time + std::time::Duration::from_secs_f64(timeout_secs);

    // Step 1.5: Topological Macro-Decomposition Cascade
    // Fast path for graphs exhibiting highly structured topologies.
    // Dispatch checks run in O(V+E) time; if matched, the solver receives the full remaining deadline.
    {
        let rem = (deadline - Instant::now()).as_secs_f64();
        if rem > 0.1 {
            let mut macro_tour_opt: Option<Vec<i32>> = None;

            if dynamic_bipartite::can_solve_bipartite(&g) {
                macro_tour_opt = dynamic_bipartite::solve_bipartite(&g, rem);
            } else if let Some((_u, _v)) = macro_corridor::can_solve_2cut(&g) {
                macro_tour_opt = macro_corridor::solve_2cut_corridor(&g, rem);
            } else if macro_788::can_solve_alternating_pairs(&g) {
                macro_tour_opt = macro_788::solve_alternating_pairs(&g, rem);
            }

            if let Some(tour) = macro_tour_opt {
                return verify_and_export(&g, &tour, start_time, output_tour_path);
            }
        }
    }

    // Step 2: Contract series chains using crate::decomp::spqr_series::contract_series_chains
    let series_decomp = contract_series_chains(&g);

    // Guard against over-contraction (if contracted graph has < 3 vertices, use original graph)
    let (work_g, chain_map) = if series_decomp.contracted_g.adjacency_list.len() >= 3 {
        // If contracted_g has no 2-cut on small graphs, but g itself does, prefer g
        let pairs = if series_decomp.contracted_g.adjacency_list.len() <= 250 {
            find_separation_pairs(&series_decomp.contracted_g)
        } else {
            Vec::new()
        };
        if pairs.is_empty() && g.adjacency_list.len() <= 100 {
            let orig_pairs = find_separation_pairs(&g);
            let has_nontrivial = orig_pairs.iter().any(|p| {
                p.components.len() == 2 && p.components[0].len() >= 2 && p.components[1].len() >= 2
            });
            if has_nontrivial {
                (g.clone(), std::collections::HashMap::new())
            } else {
                (series_decomp.contracted_g, series_decomp.chain_map)
            }
        } else {
            (series_decomp.contracted_g, series_decomp.chain_map)
        }
    } else {
        (g.clone(), std::collections::HashMap::new())
    };

    // Step 3: Check separation pairs using crate::decomp::spqr_parallel::find_separation_pairs
    let mut cur_g = work_g;
    let mut stitched_cuts: Vec<(i32, i32, Vec<i32>)> = Vec::new();

    while cur_g.adjacency_list.len() >= 4 && cur_g.adjacency_list.len() <= 250 {
        let elapsed = start_time.elapsed().as_secs_f64();
        if elapsed >= timeout_secs {
            return Err(SolverPipelineError {
                message: format!("TIMEOUT (elapsed: {:.2}s)", elapsed),
                vertex_count,
            });
        }

        let pairs = find_separation_pairs(&cur_g);
        if pairs.is_empty() {
            break;
        }

        // Fast check: If any separation pair splits G into > 2 components, G is UNSAT
        for p in &pairs {
            if p.components.len() > 2 {
                return Err(SolverPipelineError {
                    message: "UNSAT: Graph has 2-cut with >2 components".to_string(),
                    vertex_count,
                });
            }
        }

        let valid_pairs: Vec<_> = pairs
            .into_iter()
            .filter(|p| {
                p.components.len() == 2 && !p.components[0].is_empty() && !p.components[1].is_empty()
            })
            .collect();

        if valid_pairs.is_empty() {
            break;
        }

        // Pick pair with smallest subcomponent
        let best_pair = valid_pairs
            .into_iter()
            .min_by_key(|p| p.components[0].len().min(p.components[1].len()))
            .unwrap();

        let (u, v) = (best_pair.u, best_pair.v);
        let smaller_comp = if best_pair.components[0].len() <= best_pair.components[1].len() {
            &best_pair.components[0]
        } else {
            &best_pair.components[1]
        };

        // Isolate smaller component G_sub
        let sub_g = extract_subcomponent_graph(&cur_g, smaller_comp, u, v);

        let rem_time = (deadline - Instant::now()).as_secs_f64();
        if rem_time <= 0.0 {
            return Err(SolverPipelineError {
                message: format!("TIMEOUT (elapsed: {:.2}s)", start_time.elapsed().as_secs_f64()),
                vertex_count,
            });
        }
        let subpath = match solve_path(&sub_g, u, v, rem_time) {
            Ok(p) => p,
            Err(e) => {
                let is_timeout = start_time.elapsed().as_secs_f64() >= timeout_secs
                    || e.to_uppercase().contains("TIMEOUT");
                if is_timeout {
                    return Err(SolverPipelineError {
                        message: format!("TIMEOUT (elapsed: {:.2}s)", start_time.elapsed().as_secs_f64()),
                        vertex_count,
                    });
                }
                if e.to_uppercase().contains("UNSAT") || e.to_uppercase().contains("INFEASIBLE") {
                    return Err(SolverPipelineError {
                        message: format!("UNSAT: Subcomponent path infeasible: {}", e),
                        vertex_count,
                    });
                }
                return Err(SolverPipelineError {
                    message: format!("Subcomponent solver error: {}", e),
                    vertex_count,
                });
            }
        };

        // Replace subcomponent with virtual edge (u, v) in skeleton
        let smaller_set: HashSet<i32> = smaller_comp.iter().copied().collect();
        let mut next_skeleton = Graph::new();
        for &node in cur_g.adjacency_list.keys() {
            if !smaller_set.contains(&node) {
                next_skeleton.adjacency_list.entry(node).or_default();
                next_skeleton.adjacency_list_btree.entry(node).or_default();
            }
        }
        for (&node, nbrs) in &cur_g.adjacency_list {
            if !smaller_set.contains(&node) {
                for &nbr in nbrs {
                    if !smaller_set.contains(&nbr) && node < nbr {
                        next_skeleton.add_edge(node, nbr);
                    }
                }
            }
        }
        if !next_skeleton.adjacency_list.get(&u).map_or(false, |nbrs| nbrs.contains(&v)) {
            next_skeleton.add_edge(u, v);
        }

        stitched_cuts.push((u, v, subpath));
        cur_g = next_skeleton;

        if cur_g.adjacency_list.len() <= 3 {
            break;
        }
    }

    // Step 4: Solve skeleton using unified cegar_engine
    let rem_skeleton = (deadline - Instant::now()).as_secs_f64();
    if rem_skeleton <= 0.0 {
        return Err(SolverPipelineError {
            message: format!("TIMEOUT (elapsed: {:.2}s)", start_time.elapsed().as_secs_f64()),
            vertex_count,
        });
    }

    let skeleton_tour_res = solve_cycle(&cur_g, rem_skeleton);

    let mut tour = match skeleton_tour_res {
        Ok(t) => t,
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
    };

    // Step 5: Expand virtual edges and series chains using tour_stitcher and expand_series_tour
    while let Some((u, v, subpath)) = stitched_cuts.pop() {
        tour = match stitch_subpath(&tour, u, v, &subpath) {
            Ok(t) => t,
            Err(_) => {
                // If the skeleton cycle didn't traverse (u, v), solve as Hamiltonian path in skeleton \ (u, v)
                let mut cur_without_uv = cur_g.clone();
                cur_without_uv.remove_edge_if_exists(u, v);
                let rem3 = (deadline - Instant::now()).as_secs_f64();
                if rem3 <= 0.0 {
                    return Err(SolverPipelineError {
                        message: format!("TIMEOUT (elapsed: {:.2}s)", start_time.elapsed().as_secs_f64()),
                        vertex_count,
                    });
                }
                let path = solve_path(&cur_without_uv, u, v, rem3)
                    .map_err(|e| SolverPipelineError {
                        message: format!("Failed to route through ports ({}, {}): {}", u, v, e),
                        vertex_count,
                    })?;
                stitch_subpath(&path, u, v, &subpath)
                    .map_err(|e| SolverPipelineError {
                        message: format!("Failed to stitch subpath: {}", e),
                        vertex_count,
                    })?
            }
        };
    }

    if !chain_map.is_empty() {
        tour = expand_series_tour(&tour, &chain_map);
    }

    // If tour is incomplete (e.g. some contracted edges were bypassed in the skeleton cycle),
    // fall back to solving g directly with cegar_engine
    if tour.len() != vertex_count {
        let rem = (deadline - Instant::now()).as_secs_f64();
        if rem > 0.0 {
            tour = solve_cycle(&g, rem).map_err(|e| {
                SolverPipelineError {
                    message: format!("Solver fallback error: {}", e),
                    vertex_count,
                }
            })?;
        }
    }

    // Step 6: verify_and_export
    verify_and_export(&g, &tour, start_time, output_tour_path)
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
