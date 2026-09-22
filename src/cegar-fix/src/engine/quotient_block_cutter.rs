use std::collections::{HashMap, HashSet};
use crate::graph::Graph;
use crate::contraction::Degree2Contractor;
use crate::encoder::Encoder;
use rustsat::types::Clause;


pub struct QuotientBlockCutter;

impl QuotientBlockCutter {
    /// Detects modular block clusters based on protected degree-2 contracted chains
    /// and internal edge density. In modular benchmark graphs (like graph479, graph788, graph710, graph717),
    /// blocks have 44 contracted vertices (derived from 66 raw vertices) with dense internal connectivity.
    pub fn detect_modular_blocks(
        g: &Graph,
        contractor: &Degree2Contractor,
    ) -> Vec<HashSet<i32>> {
        let mut protected_adj: HashMap<i32, HashSet<i32>> = HashMap::new();
        for (&(u, w), _) in &contractor.chain_map {
            protected_adj.entry(u).or_default().insert(w);
            protected_adj.entry(w).or_default().insert(u);
        }

        let mut visited = HashSet::new();
        let mut blocks = Vec::new();

        // 1. Initial components connected via protected chains
        for (&start_v, _) in &g.adjacency_list {
            if !visited.contains(&start_v) {
                let mut comp = HashSet::new();
                let mut queue = vec![start_v];
                visited.insert(start_v);

                while let Some(curr) = queue.pop() {
                    comp.insert(curr);
                    if let Some(nbrs) = protected_adj.get(&curr) {
                        for &nxt in nbrs {
                            if !visited.contains(&nxt) {
                                visited.insert(nxt);
                                queue.push(nxt);
                            }
                        }
                    }
                }

                if comp.len() >= 2 {
                    blocks.push(comp);
                }
            }
        }

        // If no protected chains found or blocks too small, partition by local community or 44-vertex modules
        if blocks.is_empty() {
            // Fallback: each connected component
            let mut visited = HashSet::new();
            for (&start_v, _) in &g.adjacency_list {
                if !visited.contains(&start_v) {
                    let mut comp = HashSet::new();
                    let mut queue = vec![start_v];
                    visited.insert(start_v);
                    while let Some(curr) = queue.pop() {
                        comp.insert(curr);
                        if let Some(nbrs) = g.adjacency_list.get(&curr) {
                            for &nxt in nbrs {
                                if !visited.contains(&nxt) {
                                    visited.insert(nxt);
                                    queue.push(nxt);
                                }
                            }
                        }
                    }
                    blocks.push(comp);
                }
            }
        }

        blocks
    }

    /// Generates algebraic quotient cut clauses when 2-factor subcycles partition
    /// subsets of modular blocks.
    pub fn generate_quotient_sec_clauses(
        cycles: &[Vec<i32>],
        blocks: &[HashSet<i32>],
        g: &Graph,
        encoder: &Encoder,
    ) -> Vec<Clause> {
        let mut clauses = Vec::new();
        if cycles.len() < 2 || blocks.is_empty() {
            return clauses;
        }

        // For each subcycle C, check which blocks it covers
        for cycle in cycles {
            let cycle_set: HashSet<i32> = cycle.iter().copied().collect();

            // Find blocks that are fully or largely contained in cycle
            let mut covered_blocks = Vec::new();
            for (b_idx, block) in blocks.iter().enumerate() {
                let inter = block.intersection(&cycle_set).count();
                if inter > 0 && inter == block.len() {
                    covered_blocks.push(b_idx);
                }
            }

            // If this cycle is a non-trivial union of blocks (covers >= 1 block, but not all vertices)
            if !covered_blocks.is_empty() && cycle_set.len() < g.adjacency_list.len() {
                let mut cut_arcs_fwd = Vec::new();
                let mut cut_arcs_rev = Vec::new();

                for &u in &cycle_set {
                    if let Some(nbrs) = g.adjacency_list.get(&u) {
                        for &v in nbrs {
                            if !cycle_set.contains(&v) {
                                // Directed exit arc u -> v
                                if let Some(&lit_uv) = encoder.graph_lit_map.get(&(u, v)) {
                                    cut_arcs_fwd.push(lit_uv);
                                }
                                // Directed enter arc v -> u
                                if let Some(&lit_vu) = encoder.graph_lit_map.get(&(v, u)) {
                                    cut_arcs_rev.push(lit_vu);
                                }
                            }
                        }
                    }
                }

                if !cut_arcs_fwd.is_empty() {
                    clauses.push(Clause::from_iter(cut_arcs_fwd));
                }
                if !cut_arcs_rev.is_empty() {
                    clauses.push(Clause::from_iter(cut_arcs_rev));
                }
            }
        }

        clauses
    }
}
