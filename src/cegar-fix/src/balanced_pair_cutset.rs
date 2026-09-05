use std::collections::HashSet;
use crate::graph::Graph;
use crate::twin_giant_splicer::LitMap;
use rustsat::types::{Clause, Lit};

pub struct BalancedPairCutset;

impl BalancedPairCutset {
    /// Partitions a collection of subcycles into two balanced subsets (Left vs Right)
    /// using greedy rank balancing (inspired by Hcup / FabianTUW's BalancedPairCutSet),
    /// and generates bidirectional boundary cut clauses across the partition.
    pub fn generate_balanced_cutset_clauses<M: LitMap>(
        cycles: &[Vec<i32>],
        g: &Graph,
        lit_map: M,
    ) -> Vec<Clause> {
        if cycles.len() < 3 {
            return Vec::new();
        }

        // Sort cycles by length descending
        let mut indexed_cycles: Vec<(usize, usize)> = cycles.iter().enumerate().map(|(i, c)| (i, c.len())).collect();
        indexed_cycles.sort_by(|a, b| b.1.cmp(&a.1));

        let mut left_cycles: Vec<usize> = Vec::new();
        let mut right_cycles: Vec<usize> = Vec::new();
        let mut left_size = 0;
        let mut right_size = 0;

        for (idx, len) in indexed_cycles {
            if left_size <= right_size {
                left_cycles.push(idx);
                left_size += len;
            } else {
                right_cycles.push(idx);
                right_size += len;
            }
        }

        let left_set: HashSet<i32> = left_cycles
            .iter()
            .flat_map(|&idx| cycles[idx].iter().copied())
            .collect();

        let right_set: HashSet<i32> = right_cycles
            .iter()
            .flat_map(|&idx| cycles[idx].iter().copied())
            .collect();

        let mut cut_edges: HashSet<(i32, i32)> = HashSet::new();
        for &u in &left_set {
            if let Some(nbrs) = g.adjacency_list.get(&u) {
                for &v in nbrs {
                    if right_set.contains(&v) {
                        cut_edges.insert((u, v));
                    }
                }
            }
        }

        if cut_edges.is_empty() {
            return Vec::new();
        }

        let mut clauses = Vec::new();

        // 1. Forward cut: Left -> Right
        let mut fwd_lits: Vec<Lit> = Vec::new();
        for &(u, v) in &cut_edges {
            if let Some(lit) = lit_map.get_lit(u, v) {
                fwd_lits.push(lit);
            }
        }
        if !fwd_lits.is_empty() {
            clauses.push(Clause::from_iter(fwd_lits));
        }

        // 2. Backward cut: Right -> Left
        let mut bwd_lits: Vec<Lit> = Vec::new();
        for &(u, v) in &cut_edges {
            if let Some(lit) = lit_map.get_lit(v, u) {
                bwd_lits.push(lit);
            }
        }
        if !bwd_lits.is_empty() {
            clauses.push(Clause::from_iter(bwd_lits));
        }

        clauses
    }
}
