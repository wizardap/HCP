use crate::contraction::Degree2Contractor;
use crate::graph::Graph;
use crate::hub_registry::HubRegistry;
use std::collections::{HashMap, HashSet};

/// SubcycleAbsorber absorbs small disjoint subcycles directly into a dominant Giant Cycle
/// without discarding the giant cycle's topological progress.
pub struct SubcycleAbsorber;

impl SubcycleAbsorber {
    /// Attempts to absorb smaller subcycles into the dominant cycle.
    pub fn absorb_subcycles(
        cycles: &[Vec<i32>],
        g: &Graph,
        contractor: &Degree2Contractor,
        _hub_registry: &HubRegistry,
    ) -> Vec<Vec<i32>> {
        if cycles.len() <= 1 {
            return cycles.to_vec();
        }

        // Find the largest cycle index
        let mut max_len = 0;
        let mut giant_idx = 0;
        for (i, c) in cycles.iter().enumerate() {
            if c.len() > max_len {
                max_len = c.len();
                giant_idx = i;
            }
        }

        let mut giant_cycle = cycles[giant_idx].clone();
        let mut unabsorbed: Vec<Vec<i32>> = cycles
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != giant_idx)
            .map(|(_, c)| c.clone())
            .collect();

        // Build protected edge lookup for degree-2 contraction safety
        let mut is_protected_edge: HashSet<(i32, i32)> = HashSet::new();
        for (&(u, v), _) in &contractor.chain_map {
            is_protected_edge.insert((u, v));
            is_protected_edge.insert((v, u));
        }

        let mut progress = true;
        while progress && !unabsorbed.is_empty() {
            progress = false;

            // Map vertex -> index in giant_cycle
            let mut giant_pos: HashMap<i32, usize> = HashMap::new();
            for (idx, &v) in giant_cycle.iter().enumerate() {
                giant_pos.insert(v, idx);
            }

            let mut next_unabsorbed = Vec::new();

            for small_cycle in unabsorbed {
                if let Some(spliced_giant) = Self::try_splice_subcycle(
                    &giant_cycle,
                    &giant_pos,
                    &small_cycle,
                    g,
                    &is_protected_edge,
                ) {
                    giant_cycle = spliced_giant;
                    progress = true;
                    // Rebuild giant_pos
                    giant_pos.clear();
                    for (idx, &v) in giant_cycle.iter().enumerate() {
                        giant_pos.insert(v, idx);
                    }
                } else {
                    next_unabsorbed.push(small_cycle);
                }
            }

            unabsorbed = next_unabsorbed;
        }

        let mut result = Vec::with_capacity(1 + unabsorbed.len());
        result.push(giant_cycle);
        result.extend(unabsorbed);
        result
    }

    /// Tries all rotations (forward and reverse) of small_cycle to find an insertion into giant_cycle.
    fn try_splice_subcycle(
        giant: &[i32],
        giant_pos: &HashMap<i32, usize>,
        small: &[i32],
        g: &Graph,
        is_protected_edge: &HashSet<(i32, i32)>,
    ) -> Option<Vec<i32>> {
        let n_giant = giant.len();
        let m = small.len();
        if m == 0 || n_giant < 3 {
            return None;
        }

        // Try single vertex insertion if m == 1
        if m == 1 {
            let w = small[0];
            if let Some(nbrs) = g.adjacency_list.get(&w) {
                for &u1 in nbrs {
                    if let Some(&p1) = giant_pos.get(&u1) {
                        let p2 = (p1 + 1) % n_giant;
                        let u2 = giant[p2];
                        if !is_protected_edge.contains(&(u1, u2)) && nbrs.contains(&u2) {
                            let mut new_giant = Vec::with_capacity(n_giant + 1);
                            new_giant.extend_from_slice(&giant[0..=p1]);
                            new_giant.push(w);
                            new_giant.extend_from_slice(&giant[(p1 + 1)..n_giant]);
                            return Some(new_giant);
                        }
                    }
                }
            }
            return None;
        }

        // Try full path splices for all cyclic rotations of `small`
        // 1. Forward orientations
        for rot in 0..m {
            let v_start = small[rot];
            let v_end = small[(rot + m - 1) % m];

            let Some(start_nbrs) = g.adjacency_list.get(&v_start) else {
                continue;
            };
            let Some(end_nbrs) = g.adjacency_list.get(&v_end) else {
                continue;
            };

            for &u1 in start_nbrs {
                if let Some(&p1) = giant_pos.get(&u1) {
                    let p2 = (p1 + 1) % n_giant;
                    let u2 = giant[p2];

                    if !is_protected_edge.contains(&(u1, u2)) && end_nbrs.contains(&u2) {
                        // Splicing forward path small[rot..] + small[..rot] between p1 and p1+1
                        let mut new_giant = Vec::with_capacity(n_giant + m);
                        new_giant.extend_from_slice(&giant[0..=p1]);
                        for offset in 0..m {
                            new_giant.push(small[(rot + offset) % m]);
                        }
                        new_giant.extend_from_slice(&giant[(p1 + 1)..n_giant]);
                        return Some(new_giant);
                    }
                }
            }
        }

        // 2. Reverse orientations
        for rot in 0..m {
            let v_start = small[rot];
            let v_end = small[(rot + 1) % m];

            let Some(start_nbrs) = g.adjacency_list.get(&v_start) else {
                continue;
            };
            let Some(end_nbrs) = g.adjacency_list.get(&v_end) else {
                continue;
            };

            for &u1 in start_nbrs {
                if let Some(&p1) = giant_pos.get(&u1) {
                    let p2 = (p1 + 1) % n_giant;
                    let u2 = giant[p2];

                    if !is_protected_edge.contains(&(u1, u2)) && end_nbrs.contains(&u2) {
                        // Splicing reverse path between p1 and p1+1
                        let mut new_giant = Vec::with_capacity(n_giant + m);
                        new_giant.extend_from_slice(&giant[0..=p1]);
                        for offset in 0..m {
                            new_giant.push(small[(rot + m - (offset % m)) % m]);
                        }
                        new_giant.extend_from_slice(&giant[(p1 + 1)..n_giant]);
                        return Some(new_giant);
                    }
                }
            }
        }

        None
    }
}
