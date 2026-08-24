use crate::auto_classifier::{AutoTopologyClassifier, TargetTrack};
use crate::contraction::Degree2Contractor;
use crate::graph::Graph;
use crate::hcp_solver;
use crate::hub_registry::HubRegistry;
use crate::tour_verifier::TourVerifier;
use crate::two_tier_orchestrator::{TwoTierOptions, TwoTierOrchestrator};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct HybridOptions {
    pub auto_mode: bool,
    pub timeout_secs: f64,
    pub output_tour: Option<String>,
}

impl Default for HybridOptions {
    fn default() -> Self {
        Self {
            auto_mode: true,
            timeout_secs: 1800.0,
            output_tour: None,
        }
    }
}

pub struct HybridOrchestrator;

impl HybridOrchestrator {
    pub fn solve(g: &Graph, options: &HybridOptions) -> Option<Vec<i32>> {
        let features = AutoTopologyClassifier::extract_features(g);
        let track = if options.auto_mode {
            AutoTopologyClassifier::classify(&features)
        } else {
            TargetTrack::B2SinzChainSMT
        };

        println!(
            "AutoClassifier: N={}, M={}, Density={:.2}, Hubs={} -> Track: {:?}",
            features.n, features.m, features.density, features.hub_count, track
        );

        let tour = match track {
            TargetTrack::B1LadderTwoTier => {
                let tt_opts = TwoTierOptions {
                    timeout_secs: options.timeout_secs,
                    output_tour: options.output_tour.clone(),
                };
                TwoTierOrchestrator::solve(g, &tt_opts)
            }
            _ => {
                // Pre-flight check: cut-vertices or disconnected components
                if g.has_articulation_points() {
                    println!("Graph has cut-vertex or is disconnected.");
                    println!("s UNSATISFIABLE");
                    return None;
                }
                let mut uncontracted_g = g.clone();
                let pruned = uncontracted_g.prune_degree2_triangles();
                if pruned > 0 {
                    println!("Pruned {} degree-2 triangle shortcut edges", pruned);
                }
                let (contracted_g, contractor) = Degree2Contractor::contract(&uncontracted_g);

                if let Some(cycle) = contractor.is_direct_cycle.clone() {
                    println!("Graph is a single 2-regular Hamiltonian cycle.");
                    let uncontracted = contractor.uncontract_cycle(&cycle);
                    let line = uncontracted
                        .iter()
                        .map(|i| i.to_string())
                        .collect::<Vec<String>>()
                        .join(" ");
                    println!();
                    println!("solution: ");
                    println!("{}\n", line);
                    println!("s SATISFIABLE");
                    if let Some(ref out_path) = options.output_tour {
                        let _ = TourVerifier::write_tsplib_hcp(&uncontracted, "tour", out_path);
                    }
                    return Some(uncontracted);
                }

                if contractor.is_infeasible {
                    println!("Infeasible degree-2 structure detected.");
                    println!("s UNSATISFIABLE");
                    return None;
                }

                if contractor.contracted_vertices_count < contractor.original_vertices_count {
                    println!(
                        "Degree-2 contraction: compressed graph from {} to {} vertices (reduced by {}%)",
                        contractor.original_vertices_count,
                        contractor.contracted_vertices_count,
                        (contractor.original_vertices_count - contractor.contracted_vertices_count) * 100
                            / contractor.original_vertices_count
                    );
                }

                let hub_reg = HubRegistry::new(&contracted_g);
                if !hub_reg.hub_vertices.is_empty() {
                    println!(
                        "Dense Hub optimization: detected {} hub vertices (sample: {:?})",
                        hub_reg.hub_vertices.len(),
                        &hub_reg.hub_vertices[..hub_reg.hub_vertices.len().min(5)]
                    );
                }

                let start = Instant::now();
                hcp_solver::solve_hamilton(
                    contracted_g,
                    &contractor,
                    &hub_reg,
                    0, 1, 3, 2, 3, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 60, 200,
                    options.timeout_secs,
                    start,
                    "",
                )
            }
        };

        if let Some(ref t) = tour {
            if let Some(ref out_path) = options.output_tour {
                if let Err(e) = TourVerifier::write_tsplib_hcp(t, "tour", out_path) {
                    eprintln!("Warning: failed to write tour to {}: {}", out_path, e);
                } else {
                    println!("Wrote certified tour to {}", out_path);
                }
            }
        }

        tour
    }
}
