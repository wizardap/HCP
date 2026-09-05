use std::collections::{BTreeMap, HashSet, VecDeque};
use std::time::Instant;
use crate::encoder::Encoder;
use crate::graph::Graph;
use rustsat::instances::Cnf;
use rustsat::solvers::{Solve, SolveIncremental, SolverResult};
use rustsat::types::{Clause, Lit, TernaryVal};
use rustsat_cadical::CaDiCaL;

pub struct LocalizedSatRepair;

impl LocalizedSatRepair {
    /// Attempts two-tier Large Neighborhood Search (LNS) repair:
    /// Tier 1: Per-cycle incremental absorption (absorbing small subcycles one by one in < 50ms).
    /// Tier 2: Strict last-mile corridor repair (when remaining subcycles <= 8 or <= 350 sat vertices).
    pub fn try_repair(
        cycles: &[Vec<i32>],
        g: &Graph,
        working_cnf: &Cnf,
        encoder: &Encoder,
        max_hops: usize,
    ) -> Option<Vec<i32>> {
        if cycles.len() < 2 {
            return None;
        }

        let total_v = g.adjacency_list.len();

        // 1. Find giant cycle
        let mut giant_idx = 0;
        let mut giant_len = 0;
        for (i, c) in cycles.iter().enumerate() {
            if c.len() > giant_len {
                giant_len = c.len();
                giant_idx = i;
            }
        }

        let mut current_cycles = cycles.to_vec();

        // Tier 1: Try sequential per-cycle absorption for small subcycles (len <= 64)
        if giant_len >= total_v / 2 {
            let absorbed_cycles = Self::try_absorb_subcycles_sequentially(
                &current_cycles,
                giant_idx,
                g,
                working_cnf,
                encoder,
            );
            if absorbed_cycles.len() == 1 && absorbed_cycles[0].len() == total_v {
                println!("LocalizedSatRepair (Tier 1): all subcycles sequentially absorbed into full tour!");
                return Some(absorbed_cycles[0].clone());
            }
            if absorbed_cycles.len() < current_cycles.len() {
                println!(
                    "LocalizedSatRepair (Tier 1): sequentially absorbed subcycles from {} down to {} cycles",
                    current_cycles.len(),
                    absorbed_cycles.len()
                );
                current_cycles = absorbed_cycles;
                // Recompute giant
                giant_len = 0;
                for (i, c) in current_cycles.iter().enumerate() {
                    if c.len() > giant_len {
                        giant_len = c.len();
                        giant_idx = i;
                    }
                }
            }
        }

        // Tier 2: Strict Last-Mile Full Corridor LNS
        // Only trigger when: giant >= 80% OR <= 8 cycles OR <= 350 sat vertices remain
        let sat_vertices_count: usize = current_cycles
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != giant_idx)
            .map(|(_, c)| c.len())
            .sum();

        let is_strict_last_mile = giant_len >= (total_v * 80) / 100
            || current_cycles.len() <= 8
            || sat_vertices_count <= 350;

        if !is_strict_last_mile {
            return None;
        }

        Self::try_full_corridor_lns(
            &current_cycles,
            giant_idx,
            g,
            working_cnf,
            encoder,
            max_hops,
        )
    }

    /// Tier 1: Attempts to absorb individual small subcycles into the giant cycle one-by-one.
    fn try_absorb_subcycles_sequentially(
        cycles: &[Vec<i32>],
        giant_idx: usize,
        g: &Graph,
        working_cnf: &Cnf,
        encoder: &Encoder,
    ) -> Vec<Vec<i32>> {
        let total_v = g.adjacency_list.len();
        let mut active_cycles = cycles.to_vec();
        let mut current_giant = active_cycles[giant_idx].clone();

        // Sort satellite cycles by length (smallest first)
        let mut sat_indices: Vec<usize> = (0..active_cycles.len()).filter(|&i| i != giant_idx).collect();
        sat_indices.sort_by_key(|&i| active_cycles[i].len());

        let mut absorbed_any = false;
        let mut absorbed_indices = HashSet::new();

        for &sat_idx in &sat_indices {
            let sat_cycle = &active_cycles[sat_idx];
            if sat_cycle.len() > 64 {
                continue; // Focus on small subcycles for localized fast absorption
            }

            // Precompute giant cycle neighbor maps
            let n_giant = current_giant.len();
            let mut giant_next = BTreeMap::new();
            let mut giant_prev = BTreeMap::new();
            for pos in 0..n_giant {
                let u = current_giant[pos];
                let v_next = current_giant[(pos + 1) % n_giant];
                let v_prev = current_giant[(pos + n_giant - 1) % n_giant];
                giant_next.insert(u, v_next);
                giant_prev.insert(u, v_prev);
            }

            let sat_set: HashSet<i32> = sat_cycle.iter().copied().collect();

            // Try small hops: 1, 2, 3
            let mut merged_giant = None;
            for hop in 1..=3 {
                let mut visited: HashSet<i32> = sat_set.clone();
                let mut queue: VecDeque<(i32, usize)> = sat_set.iter().map(|&v| (v, 0)).collect();

                while let Some((curr, d)) = queue.pop_front() {
                    if d < hop {
                        if let Some(nbrs) = g.adjacency_list.get(&curr) {
                            for &nxt in nbrs {
                                if visited.insert(nxt) {
                                    queue.push_back((nxt, d + 1));
                                }
                            }
                        }
                    }
                }

                // If corridor is too large, stop expanding for this cycle
                if visited.len() > 300 {
                    break;
                }

                let corridor_v = visited;

                // Build assumptions:
                // 1. All vertices in giant NOT in corridor_v are frozen
                let mut assumptions: Vec<Lit> = Vec::new();
                let mut assumed_vars: HashSet<rustsat::types::Var> = HashSet::new();

                for (&u, &v_next) in &giant_next {
                    if !corridor_v.contains(&u) {
                        if let Some(&lit) = encoder.graph_lit_map.get(&(u, v_next)) {
                            if assumed_vars.insert(lit.var()) {
                                assumptions.push(lit);
                            }
                        }
                        if let Some(nbrs) = g.adjacency_list.get(&u) {
                            for &w in nbrs {
                                if w != v_next {
                                    if let Some(&lit) = encoder.graph_lit_map.get(&(u, w)) {
                                        if assumed_vars.insert(lit.var()) {
                                            assumptions.push(!lit);
                                        }
                                    }
                                }
                            }
                        }
                        let v_prev = giant_prev[&u];
                        if let Some(nbrs) = g.adjacency_list.get(&u) {
                            for &z in nbrs {
                                if z != v_prev {
                                    if let Some(&lit) = encoder.graph_lit_map.get(&(z, u)) {
                                        if assumed_vars.insert(lit.var()) {
                                            assumptions.push(!lit);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // 2. All other satellite cycles NOT in this pair have their edges frozen
                for (other_idx, other_cycle) in active_cycles.iter().enumerate() {
                    if other_idx != giant_idx && other_idx != sat_idx && !absorbed_indices.contains(&other_idx) {
                        let n_o = other_cycle.len();
                        for p in 0..n_o {
                            let u = other_cycle[p];
                            let v = other_cycle[(p + 1) % n_o];
                            if !corridor_v.contains(&u) && !corridor_v.contains(&v) {
                                if let Some(&lit) = encoder.graph_lit_map.get(&(u, v)) {
                                    if assumed_vars.insert(lit.var()) {
                                        assumptions.push(lit);
                                    }
                                }
                            }
                        }
                    }
                }

                // Solve locally with quick timeout (0.5s)
                let mut local_solver = CaDiCaL::default();
                for cl in working_cnf.iter() {
                    let _ = local_solver.add_clause(cl.clone());
                }

                let t_start = Instant::now();
                let mut sat_success = false;
                let mut combined_cycle = Vec::new();

                while t_start.elapsed().as_secs_f64() < 0.5 {
                    match local_solver.solve_assumps(&assumptions) {
                        Ok(SolverResult::Sat) => {
                            let sol = match local_solver.full_solution() {
                                Ok(s) => s,
                                Err(_) => break,
                            };
                            let mut sol_arcs = Vec::new();
                            for (&arc, &lit) in &encoder.graph_lit_map {
                                if sol.lit_value(lit) == TernaryVal::True {
                                    sol_arcs.push(arc);
                                }
                            }
                            let sub_cycles = extract_cycles_from_arcs(sol_arcs);
                            // Check if giant and sat_cycle merged
                            let expected_len = current_giant.len() + sat_cycle.len();
                            if let Some(merged) = sub_cycles.iter().find(|c| c.len() == expected_len) {
                                combined_cycle = merged.clone();
                                sat_success = true;
                                break;
                            }

                            // Otherwise add subcycle cut for any cycle strictly inside corridor
                            let mut added_cut = false;
                            for cyc in &sub_cycles {
                                if cyc.len() < total_v && cyc.len() <= corridor_v.len() {
                                    let cyc_set: HashSet<i32> = cyc.iter().copied().collect();
                                    let mut cut_lits = Vec::new();
                                    for &u in &cyc_set {
                                        if let Some(nbrs) = g.adjacency_list.get(&u) {
                                            for &v in nbrs {
                                                if !cyc_set.contains(&v) {
                                                    if let Some(&lit) = encoder.graph_lit_map.get(&(u, v)) {
                                                        cut_lits.push(lit);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    if !cut_lits.is_empty() {
                                        let _ = local_solver.add_clause(Clause::from_iter(cut_lits));
                                        added_cut = true;
                                    }
                                }
                            }
                            if !added_cut {
                                break;
                            }
                        }
                        _ => break,
                    }
                }

                if sat_success {
                    println!(
                        "  [Tier 1 LocalizedSatRepair]: absorbed subcycle len {} into giant -> new giant len {}",
                        sat_cycle.len(),
                        combined_cycle.len()
                    );
                    merged_giant = Some(combined_cycle);
                    break;
                }
            }

            if let Some(new_g) = merged_giant {
                current_giant = new_g;
                absorbed_indices.insert(sat_idx);
                absorbed_any = true;
            }
        }

        if absorbed_any {
            let mut result = vec![current_giant];
            for (idx, c) in active_cycles.into_iter().enumerate() {
                if idx != giant_idx && !absorbed_indices.contains(&idx) {
                    result.push(c);
                }
            }
            result
        } else {
            active_cycles
        }
    }

    /// Tier 2: Strict Last-Mile Full Corridor LNS (when problem is truly near completion).
    fn try_full_corridor_lns(
        cycles: &[Vec<i32>],
        giant_idx: usize,
        g: &Graph,
        working_cnf: &Cnf,
        encoder: &Encoder,
        max_hops: usize,
    ) -> Option<Vec<i32>> {
        let total_v = g.adjacency_list.len();
        let giant_cycle = &cycles[giant_idx];

        let mut sat_vertices: HashSet<i32> = HashSet::new();
        for (i, c) in cycles.iter().enumerate() {
            if i != giant_idx {
                for &v in c {
                    sat_vertices.insert(v);
                }
            }
        }

        println!(
            "LocalizedSatRepair (Tier 2 Full LNS): Giant = {}/{} ({:.1}%), Sat vertices = {}, Cycles = {}",
            giant_cycle.len(),
            total_v,
            (giant_cycle.len() as f64 / total_v as f64) * 100.0,
            sat_vertices.len(),
            cycles.len()
        );

        let n_giant = giant_cycle.len();
        let mut giant_next = BTreeMap::new();
        let mut giant_prev = BTreeMap::new();
        for pos in 0..n_giant {
            let u = giant_cycle[pos];
            let v_next = giant_cycle[(pos + 1) % n_giant];
            let v_prev = giant_cycle[(pos + n_giant - 1) % n_giant];
            giant_next.insert(u, v_next);
            giant_prev.insert(u, v_prev);
        }

        for hop in 1..=max_hops {
            let t_hop_start = Instant::now();
            let mut visited: HashSet<i32> = sat_vertices.clone();
            let mut queue: VecDeque<(i32, usize)> = sat_vertices.iter().map(|&v| (v, 0)).collect();

            while let Some((curr, d)) = queue.pop_front() {
                if d < hop {
                    if let Some(nbrs) = g.adjacency_list.get(&curr) {
                        for &nxt in nbrs {
                            if visited.insert(nxt) {
                                queue.push_back((nxt, d + 1));
                            }
                        }
                    }
                }
            }

            let corridor_v = visited;
            // Cap corridor size to 1,000 vertices to prevent thrashing
            if corridor_v.len() > 1000 || corridor_v.len() >= total_v {
                println!("  hop {}: corridor too wide ({} vertices), skipping", hop, corridor_v.len());
                continue;
            }

            let frozen_count = total_v - corridor_v.len();
            println!(
                "  hop {}: corridor vertices = {}, frozen backbone = {} ({:.1}%)",
                hop,
                corridor_v.len(),
                frozen_count,
                (frozen_count as f64 / total_v as f64) * 100.0
            );

            let mut assumptions: Vec<Lit> = Vec::new();
            let mut assumed_vars: HashSet<rustsat::types::Var> = HashSet::new();

            for (&u, &v_next) in &giant_next {
                if !corridor_v.contains(&u) {
                    if let Some(&lit) = encoder.graph_lit_map.get(&(u, v_next)) {
                        if assumed_vars.insert(lit.var()) {
                            assumptions.push(lit);
                        }
                    }
                    if let Some(nbrs) = g.adjacency_list.get(&u) {
                        for &w in nbrs {
                            if w != v_next {
                                if let Some(&lit) = encoder.graph_lit_map.get(&(u, w)) {
                                    if assumed_vars.insert(lit.var()) {
                                        assumptions.push(!lit);
                                    }
                                }
                            }
                        }
                    }
                    let v_prev = giant_prev[&u];
                    if let Some(nbrs) = g.adjacency_list.get(&u) {
                        for &z in nbrs {
                            if z != v_prev {
                                if let Some(&lit) = encoder.graph_lit_map.get(&(z, u)) {
                                    if assumed_vars.insert(lit.var()) {
                                        assumptions.push(!lit);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let mut local_solver = CaDiCaL::default();
            for cl in working_cnf.iter() {
                let _ = local_solver.add_clause(cl.clone());
            }

            let mut local_iter = 0;
            let mut local_success = false;
            let mut found_tour = Vec::new();

            while local_iter < 40 && t_hop_start.elapsed().as_secs_f64() < 5.0 {
                local_iter += 1;
                match local_solver.solve_assumps(&assumptions) {
                    Ok(SolverResult::Sat) => {
                        let sol = match local_solver.full_solution() {
                            Ok(s) => s,
                            Err(_) => break,
                        };

                        let mut sol_arcs: Vec<(i32, i32)> = Vec::new();
                        for (&arc, &lit) in &encoder.graph_lit_map {
                            if sol.lit_value(lit) == TernaryVal::True {
                                sol_arcs.push(arc);
                            }
                        }

                        let sol_cycles = extract_cycles_from_arcs(sol_arcs);
                        if sol_cycles.len() == 1 && sol_cycles[0].len() == total_v {
                            println!(
                                "  >>> LocalizedSatRepair SUCCESS at hop {} (iter {}, {:.3}s)! Full Hamiltonian tour found! <<<",
                                hop,
                                local_iter,
                                t_hop_start.elapsed().as_secs_f64()
                            );
                            local_success = true;
                            found_tour = sol_cycles[0].clone();
                            break;
                        }

                        let mut cuts_added = 0;
                        for cyc in &sol_cycles {
                            if cyc.len() < total_v {
                                let cyc_set: HashSet<i32> = cyc.iter().copied().collect();
                                let mut cut_lits = Vec::new();
                                for &u in &cyc_set {
                                    if let Some(nbrs) = g.adjacency_list.get(&u) {
                                        for &v in nbrs {
                                            if !cyc_set.contains(&v) {
                                                if let Some(&lit) = encoder.graph_lit_map.get(&(u, v)) {
                                                    cut_lits.push(lit);
                                                }
                                            }
                                        }
                                    }
                                }
                                if !cut_lits.is_empty() {
                                    let _ = local_solver.add_clause(Clause::from_iter(cut_lits));
                                    cuts_added += 1;
                                }
                            }
                        }

                        if cuts_added == 0 {
                            break;
                        }
                    }
                    Ok(SolverResult::Unsat) => {
                        println!("    hop {} is UNSAT (parity barrier at this width) in {:.3}s", hop, t_hop_start.elapsed().as_secs_f64());
                        break;
                    }
                    _ => break,
                }
            }

            if local_success {
                return Some(found_tour);
            } else if t_hop_start.elapsed().as_secs_f64() >= 5.0 {
                println!("    hop {} reached 5.0s local limit, advancing to next hop", hop);
            }
        }

        println!("LocalizedSatRepair: all hops exhausted without repair, continuing main CEGAR.");
        None
    }
}

fn extract_cycles_from_arcs(sol_arcs: Vec<(i32, i32)>) -> Vec<Vec<i32>> {
    let mut arcs: BTreeMap<i32, i32> = BTreeMap::new();
    for (u, v) in sol_arcs {
        arcs.insert(u, v);
    }

    let mut visited: HashSet<i32> = HashSet::new();
    let mut cycles = Vec::new();

    for &node in arcs.keys() {
        if visited.contains(&node) {
            continue;
        }
        let mut cycle = Vec::new();
        let mut curr = node;
        loop {
            visited.insert(curr);
            cycle.push(curr);
            curr = match arcs.get(&curr) {
                Some(&nxt) => nxt,
                None => break,
            };
            if visited.contains(&curr) {
                break;
            }
        }
        cycles.push(cycle);
    }
    cycles
}
