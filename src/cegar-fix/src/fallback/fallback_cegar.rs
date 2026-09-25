use crate::core::graph::Graph;
use crate::core::tour_verifier::TourVerifier;
use crate::fallback::contraction::Degree2Contractor;
use crate::solver::cegar_engine;
use std::time::Instant;

/// Pure Rust CDCL CEGAR fallback solver delegating core solving to `cegar_engine`.
#[deprecated(note = "Use crate::solver::cegar_engine directly")]
#[allow(deprecated)]
pub fn solve_with_contraction(g: &Graph, timeout_secs: f64) -> Result<Vec<i32>, String> {
    let t_start = Instant::now();

    // 1. Contract degree-2 chains
    let (contracted_g, contractor) = Degree2Contractor::contract(g);

    if contractor.is_infeasible {
        return Err("Infeasible".to_string());
    }

    if let Some(cycle) = contractor.is_direct_cycle {
        let (ok, err) = TourVerifier::verify(g, &cycle);
        if ok {
            return Ok(cycle);
        } else {
            return Err(format!("Direct cycle verification failed: {}", err));
        }
    }

    let contracted_v = contracted_g.adjacency_list.len();
    if contracted_v < 3 {
        if contracted_v == 0 {
            return Ok(Vec::new());
        }
        return Err("Infeasible: contracted graph has fewer than 3 vertices".to_string());
    }

    let rem = timeout_secs - t_start.elapsed().as_secs_f64();
    if rem <= 0.0 {
        return Err("Timeout".to_string());
    }

    let contracted_tour = cegar_engine::solve_cycle_with_forced_edges(&contracted_g, rem, &contractor.forced_edges)?;
    let expanded_tour = contractor.expand_tour(&contracted_tour);

    let (ok, err) = TourVerifier::verify(g, &expanded_tour);
    if ok {
        Ok(expanded_tour)
    } else {
        Err(format!("Tour verification failed: {}", err))
    }
}
