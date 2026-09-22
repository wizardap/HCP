use crate::contraction::Degree2Contractor;
use crate::graph::Graph;
use crate::hub_registry::HubRegistry;
use std::collections::{HashMap, HashSet};

/// Multi-Cycle Alternating Chain Splicer & Absorber
/// Greedily chains small disjoint subcycles into composite cycles and alternating chains
/// before performing 2-point and 3-point absorption into the giant cycle,
/// preserving degree-2 contracted chain protection.
pub struct CycleChainAbsorber;

impl CycleChainAbsorber {
    /// Greedily chains small disjoint subcycles and absorbs them into the giant cycle.
    pub fn absorb_all(
        cycles: &[Vec<i32>],
        g: &Graph,
        contractor: &Degree2Contractor,
        _hub_registry: &HubRegistry,
    ) -> Vec<Vec<i32>> {
        if cycles.len() <= 1 {
            return cycles.to_vec();
        }

        // Build protected edge lookup for degree-2 contraction safety
        let mut is_protected: HashSet<(i32, i32)> = HashSet::new();
        for (&(u, v), _) in &contractor.chain_map {
            is_protected.insert((u, v));
            is_protected.insert((v, u));
        }

        // Find the giant cycle index
        let mut max_len = 0;
        let mut giant_idx = 0;
        for (i, c) in cycles.iter().enumerate() {
            if c.len() > max_len {
                max_len = c.len();
                giant_idx = i;
            }
        }

        let mut giant_cycle = cycles[giant_idx].clone();
        let mut small_cycles: Vec<Vec<i32>> = cycles
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != giant_idx)
            .map(|(_, c)| c.clone())
            .collect();

        let mut overall_progress = true;
        while overall_progress && !small_cycles.is_empty() {
            overall_progress = false;

            // Stage 1: Try alternating subcycle chain absorption directly into giant cycle
            let (new_giant, rem_smalls, progress1) =
                Self::absorb_chains(giant_cycle, small_cycles, g, &is_protected);
            giant_cycle = new_giant;
            small_cycles = rem_smalls;
            if progress1 {
                overall_progress = true;
                continue;
            }

            // Stage 2: Greedily merge small cycles together into composite cycles
            let prev_len = small_cycles.len();
            let merged_smalls = Self::chain_small_cycles(small_cycles, g, &is_protected);
            if merged_smalls.len() < prev_len {
                overall_progress = true;
            }
            small_cycles = merged_smalls;

            // Stage 3: Try standard 2-point rotation splicing of small/composite cycles into giant
            let (new_giant, rem_smalls, progress3) =
                Self::splice_small_cycles_into_giant(giant_cycle, small_cycles, g, &is_protected);
            giant_cycle = new_giant;
            small_cycles = rem_smalls;
            if progress3 {
                overall_progress = true;
                continue;
            }

            // Stage 4: Try 3-point absorption into giant cycle
            let (new_giant, rem_smalls, progress4) =
                Self::try_3opt_absorb(giant_cycle, small_cycles, g, &is_protected);
            giant_cycle = new_giant;
            small_cycles = rem_smalls;
            if progress4 {
                overall_progress = true;
                continue;
            }
        }

        let mut result = Vec::with_capacity(1 + small_cycles.len());
        result.push(giant_cycle);
        result.extend(small_cycles);
        result
    }

    /// Search for alternating chains of 1 or more small subcycles connecting to two adjacent
    /// vertices (u1, u2) along the giant cycle.
    fn absorb_chains(
        mut giant: Vec<i32>,
        mut smalls: Vec<Vec<i32>>,
        g: &Graph,
        is_protected: &HashSet<(i32, i32)>,
    ) -> (Vec<i32>, Vec<Vec<i32>>, bool) {
        if smalls.is_empty() || giant.len() < 3 {
            return (giant, smalls, false);
        }

        let mut progress = false;
        let mut loop_again = true;

        while loop_again && !smalls.is_empty() {
            loop_again = false;
            let n_giant = giant.len();

            // Build fast mapping of vertex -> small cycle index
            let mut v_to_cycle: HashMap<i32, usize> = HashMap::new();
            for (c_idx, cycle) in smalls.iter().enumerate() {
                for &v in cycle {
                    v_to_cycle.insert(v, c_idx);
                }
            }

            let max_depth = smalls.len().min(8);

            // Try all edges (p, p+1) in giant cycle
            'edge_loop: for p in 0..n_giant {
                let u1 = giant[p];
                let u2 = giant[(p + 1) % n_giant];

                if is_protected.contains(&(u1, u2)) || is_protected.contains(&(u2, u1)) {
                    continue;
                }

                // Case 1: forward direction (u1 -> chain -> u2)
                if let Some(u1_nbrs) = g.adjacency_list.get(&u1) {
                    for &start_v in u1_nbrs {
                        if let Some(&start_c_idx) = v_to_cycle.get(&start_v) {
                            let mut visited = vec![false; smalls.len()];
                            visited[start_c_idx] = true;

                            let paths = Self::get_hamiltonian_paths(&smalls[start_c_idx], start_v, is_protected);
                            for (path, exit_v) in paths {
                                let mut current_path = path;
                                if Self::dfs_chain(
                                    exit_v,
                                    u2,
                                    &mut visited,
                                    &mut current_path,
                                    &smalls,
                                    &v_to_cycle,
                                    g,
                                    is_protected,
                                    1,
                                    max_depth,
                                ) {
                                    // Splicing forward path into giant between p and p+1
                                    let mut new_giant = Vec::with_capacity(n_giant + current_path.len());
                                    new_giant.extend_from_slice(&giant[0..=p]);
                                    new_giant.extend(current_path);
                                    if p + 1 < n_giant {
                                        new_giant.extend_from_slice(&giant[(p + 1)..n_giant]);
                                    }
                                    giant = new_giant;

                                    // Remove visited cycles from smalls
                                    smalls = smalls
                                        .into_iter()
                                        .enumerate()
                                        .filter(|(idx, _)| !visited[*idx])
                                        .map(|(_, c)| c)
                                        .collect();

                                    progress = true;
                                    loop_again = true;
                                    break 'edge_loop;
                                }
                            }
                        }
                    }
                }

                // Case 2: reverse direction (u2 -> chain -> u1)
                if let Some(u2_nbrs) = g.adjacency_list.get(&u2) {
                    for &start_v in u2_nbrs {
                        if let Some(&start_c_idx) = v_to_cycle.get(&start_v) {
                            let mut visited = vec![false; smalls.len()];
                            visited[start_c_idx] = true;

                            let paths = Self::get_hamiltonian_paths(&smalls[start_c_idx], start_v, is_protected);
                            for (path, exit_v) in paths {
                                let mut current_path = path;
                                if Self::dfs_chain(
                                    exit_v,
                                    u1,
                                    &mut visited,
                                    &mut current_path,
                                    &smalls,
                                    &v_to_cycle,
                                    g,
                                    is_protected,
                                    1,
                                    max_depth,
                                ) {
                                    // Splicing reverse path into giant between p and p+1
                                    current_path.reverse();
                                    let mut new_giant = Vec::with_capacity(n_giant + current_path.len());
                                    new_giant.extend_from_slice(&giant[0..=p]);
                                    new_giant.extend(current_path);
                                    if p + 1 < n_giant {
                                        new_giant.extend_from_slice(&giant[(p + 1)..n_giant]);
                                    }
                                    giant = new_giant;

                                    smalls = smalls
                                        .into_iter()
                                        .enumerate()
                                        .filter(|(idx, _)| !visited[*idx])
                                        .map(|(_, c)| c)
                                        .collect();

                                    progress = true;
                                    loop_again = true;
                                    break 'edge_loop;
                                }
                            }
                        }
                    }
                }
            }
        }

        (giant, smalls, progress)
    }

    /// DFS to extend alternating subcycle chain until target giant vertex is reached.
    fn dfs_chain(
        current_exit: i32,
        target_u: i32,
        visited: &mut Vec<bool>,
        current_path: &mut Vec<i32>,
        smalls: &[Vec<i32>],
        v_to_cycle: &HashMap<i32, usize>,
        g: &Graph,
        is_protected: &HashSet<(i32, i32)>,
        depth: usize,
        max_depth: usize,
    ) -> bool {
        if let Some(nbrs) = g.adjacency_list.get(&current_exit) {
            if nbrs.contains(&target_u) {
                return true;
            }
        }

        if depth >= max_depth {
            return false;
        }

        if let Some(nbrs) = g.adjacency_list.get(&current_exit) {
            for &next_v in nbrs {
                if let Some(&c_idx) = v_to_cycle.get(&next_v) {
                    if !visited[c_idx] {
                        visited[c_idx] = true;
                        let paths = Self::get_hamiltonian_paths(&smalls[c_idx], next_v, is_protected);
                        for (path, exit) in paths {
                            let path_len = path.len();
                            current_path.extend(path);
                            if Self::dfs_chain(
                                exit,
                                target_u,
                                visited,
                                current_path,
                                smalls,
                                v_to_cycle,
                                g,
                                is_protected,
                                depth + 1,
                                max_depth,
                            ) {
                                return true;
                            }
                            current_path.truncate(current_path.len() - path_len);
                        }
                        visited[c_idx] = false;
                    }
                }
            }
        }

        false
    }

    /// Returns all valid Hamiltonian paths in a small cycle starting at `entry_v`.
    fn get_hamiltonian_paths(
        cycle: &[i32],
        entry_v: i32,
        is_protected: &HashSet<(i32, i32)>,
    ) -> Vec<(Vec<i32>, i32)> {
        let m = cycle.len();
        if m == 0 {
            return Vec::new();
        }
        if m == 1 {
            return vec![(vec![entry_v], entry_v)];
        }
        if m == 2 {
            let other = if cycle[0] == entry_v { cycle[1] } else { cycle[0] };
            if !is_protected.contains(&(entry_v, other)) && !is_protected.contains(&(other, entry_v)) {
                return vec![(vec![entry_v, other], other)];
            }
            return Vec::new();
        }

        let Some(idx) = cycle.iter().position(|&v| v == entry_v) else {
            return Vec::new();
        };

        let mut results = Vec::new();

        // Option 1: Forward traversal around cycle
        let exit_fwd = cycle[(idx + m - 1) % m];
        let broken_fwd = (entry_v, exit_fwd);
        if !is_protected.contains(&broken_fwd) && !is_protected.contains(&(exit_fwd, entry_v)) {
            let path_fwd: Vec<i32> = (0..m).map(|offset| cycle[(idx + offset) % m]).collect();
            results.push((path_fwd, exit_fwd));
        }

        // Option 2: Reverse traversal around cycle
        let exit_rev = cycle[(idx + 1) % m];
        let broken_rev = (entry_v, exit_rev);
        if !is_protected.contains(&broken_rev) && !is_protected.contains(&(exit_rev, entry_v)) {
            let path_rev: Vec<i32> = (0..m).map(|offset| cycle[(idx + m - (offset % m)) % m]).collect();
            results.push((path_rev, exit_rev));
        }

        results
    }

    /// Greedily merges pairs of small cycles using 2 cross edges into composite cycles.
    fn chain_small_cycles(
        mut smalls: Vec<Vec<i32>>,
        g: &Graph,
        is_protected: &HashSet<(i32, i32)>,
    ) -> Vec<Vec<i32>> {
        let mut merged = true;
        while merged && smalls.len() > 1 {
            merged = false;
            let mut best_merge = None;

            'outer: for i in 0..smalls.len() {
                for j in (i + 1)..smalls.len() {
                    if let Some(combined) =
                        Self::try_merge_two_cycles(&smalls[i], &smalls[j], g, is_protected)
                    {
                        best_merge = Some((i, j, combined));
                        break 'outer;
                    }
                }
            }

            if let Some((i, j, combined)) = best_merge {
                smalls.remove(j);
                smalls.remove(i);
                smalls.push(combined);
                merged = true;
            }
        }
        smalls
    }

    /// Merges two cycles into one composite cycle if two cross edges exist.
    fn try_merge_two_cycles(
        c1: &[i32],
        c2: &[i32],
        g: &Graph,
        is_protected: &HashSet<(i32, i32)>,
    ) -> Option<Vec<i32>> {
        let n1 = c1.len();
        let n2 = c2.len();
        if n1 == 0 || n2 == 0 {
            return None;
        }

        for i in 0..n1 {
            let u1 = c1[i];
            let u2 = c1[(i + 1) % n1];
            if is_protected.contains(&(u1, u2)) || is_protected.contains(&(u2, u1)) {
                continue;
            }

            let Some(u1_nbrs) = g.adjacency_list.get(&u1) else {
                continue;
            };
            let Some(u2_nbrs) = g.adjacency_list.get(&u2) else {
                continue;
            };

            for j in 0..n2 {
                let v1 = c2[j];
                let v2 = c2[(j + 1) % n2];
                if is_protected.contains(&(v1, v2)) || is_protected.contains(&(v2, v1)) {
                    continue;
                }

                // Case A: (u1, v1) and (u2, v2)
                if u1_nbrs.contains(&v1) && u2_nbrs.contains(&v2) {
                    let mut res = Vec::with_capacity(n1 + n2);
                    res.extend_from_slice(&c1[0..=i]);
                    for k in (0..=j).rev() {
                        res.push(c2[k]);
                    }
                    for k in ((j + 1)..n2).rev() {
                        res.push(c2[k]);
                    }
                    if i + 1 < n1 {
                        res.extend_from_slice(&c1[(i + 1)..n1]);
                    }
                    return Some(res);
                }

                // Case B: (u1, v2) and (u2, v1)
                if u1_nbrs.contains(&v2) && u2_nbrs.contains(&v1) {
                    let mut res = Vec::with_capacity(n1 + n2);
                    res.extend_from_slice(&c1[0..=i]);
                    if j + 1 < n2 {
                        res.extend_from_slice(&c2[(j + 1)..n2]);
                    }
                    res.extend_from_slice(&c2[0..=j]);
                    if i + 1 < n1 {
                        res.extend_from_slice(&c1[(i + 1)..n1]);
                    }
                    return Some(res);
                }
            }
        }
        None
    }

    /// Splicing small/composite cycles into giant cycle using standard 2-point rotation splicing.
    fn splice_small_cycles_into_giant(
        mut giant: Vec<i32>,
        smalls: Vec<Vec<i32>>,
        g: &Graph,
        is_protected: &HashSet<(i32, i32)>,
    ) -> (Vec<i32>, Vec<Vec<i32>>, bool) {
        let mut progress = false;
        let mut remaining_smalls = Vec::new();

        for s in smalls {
            let mut giant_pos: HashMap<i32, usize> = HashMap::new();
            for (idx, &v) in giant.iter().enumerate() {
                giant_pos.insert(v, idx);
            }

            if let Some(spliced) = Self::try_splice_into_giant(&giant, &giant_pos, &s, g, is_protected) {
                giant = spliced;
                progress = true;
            } else {
                remaining_smalls.push(s);
            }
        }

        (giant, remaining_smalls, progress)
    }

    /// Tries all rotations of small into giant.
    fn try_splice_into_giant(
        giant: &[i32],
        giant_pos: &HashMap<i32, usize>,
        small: &[i32],
        g: &Graph,
        is_protected: &HashSet<(i32, i32)>,
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
                        if !is_protected.contains(&(u1, u2))
                            && !is_protected.contains(&(u2, u1))
                            && nbrs.contains(&u2)
                        {
                            let mut new_giant = Vec::with_capacity(n_giant + 1);
                            new_giant.extend_from_slice(&giant[0..=p1]);
                            new_giant.push(w);
                            if p1 + 1 < n_giant {
                                new_giant.extend_from_slice(&giant[(p1 + 1)..n_giant]);
                            }
                            return Some(new_giant);
                        }
                    }
                }
            }
            return None;
        }

        // Try forward rotations
        for rot in 0..m {
            let v_start = small[rot];
            let v_end = small[(rot + m - 1) % m];
            if is_protected.contains(&(v_start, v_end)) || is_protected.contains(&(v_end, v_start)) {
                continue;
            }

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
                    if !is_protected.contains(&(u1, u2))
                        && !is_protected.contains(&(u2, u1))
                        && end_nbrs.contains(&u2)
                    {
                        let mut new_giant = Vec::with_capacity(n_giant + m);
                        new_giant.extend_from_slice(&giant[0..=p1]);
                        for offset in 0..m {
                            new_giant.push(small[(rot + offset) % m]);
                        }
                        if p1 + 1 < n_giant {
                            new_giant.extend_from_slice(&giant[(p1 + 1)..n_giant]);
                        }
                        return Some(new_giant);
                    }
                }
            }
        }

        // Try reverse rotations
        for rot in 0..m {
            let v_start = small[rot];
            let v_end = small[(rot + 1) % m];
            if is_protected.contains(&(v_start, v_end)) || is_protected.contains(&(v_end, v_start)) {
                continue;
            }

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
                    if !is_protected.contains(&(u1, u2))
                        && !is_protected.contains(&(u2, u1))
                        && end_nbrs.contains(&u2)
                    {
                        let mut new_giant = Vec::with_capacity(n_giant + m);
                        new_giant.extend_from_slice(&giant[0..=p1]);
                        for offset in 0..m {
                            new_giant.push(small[(rot + m - (offset % m)) % m]);
                        }
                        if p1 + 1 < n_giant {
                            new_giant.extend_from_slice(&giant[(p1 + 1)..n_giant]);
                        }
                        return Some(new_giant);
                    }
                }
            }
        }

        None
    }

    /// 3-point absorption into giant cycle
    fn try_3opt_absorb(
        mut giant: Vec<i32>,
        smalls: Vec<Vec<i32>>,
        g: &Graph,
        is_protected: &HashSet<(i32, i32)>,
    ) -> (Vec<i32>, Vec<Vec<i32>>, bool) {
        let mut progress = false;
        let mut remaining_smalls = Vec::new();

        for s in smalls {
            let n_giant = giant.len();
            let mut giant_pos: HashMap<i32, usize> = HashMap::new();
            for (idx, &v) in giant.iter().enumerate() {
                giant_pos.insert(v, idx);
            }

            let mut spliced_giant = None;

            // Try all starting vertices and Hamiltonian paths in s
            's_loop: for &start_v in &s {
                let paths = Self::get_hamiltonian_paths(&s, start_v, is_protected);
                for (path, exit_v) in paths {
                    let Some(start_nbrs) = g.adjacency_list.get(&start_v) else {
                        continue;
                    };
                    let Some(exit_nbrs) = g.adjacency_list.get(&exit_v) else {
                        continue;
                    };

                    for &u1 in start_nbrs {
                        let Some(&p1) = giant_pos.get(&u1) else {
                            continue;
                        };
                        let u2 = giant[(p1 + 1) % n_giant];
                        if is_protected.contains(&(u1, u2)) || is_protected.contains(&(u2, u1)) {
                            continue;
                        }

                        for &u3 in exit_nbrs {
                            let Some(&p3) = giant_pos.get(&u3) else {
                                continue;
                            };
                            if p3 == p1 || p3 == (p1 + 1) % n_giant {
                                continue;
                            }
                            let u4 = giant[(p3 + 1) % n_giant];
                            if is_protected.contains(&(u3, u4)) || is_protected.contains(&(u4, u3)) {
                                continue;
                            }

                            // Check cross edge (u2, u4)
                            if let Some(u2_nbrs) = g.adjacency_list.get(&u2) {
                                if u2_nbrs.contains(&u4) {
                                    // Valid 3-opt:
                                    // Reconstruct tour when p1 < p3
                                    if p1 < p3 {
                                        let mut new_g = Vec::with_capacity(n_giant + path.len());
                                        new_g.extend_from_slice(&giant[0..=p1]);
                                        new_g.extend(path.clone());
                                        // Reverse segment from p3 down to p1+1
                                        for idx in ((p1 + 1)..=p3).rev() {
                                            new_g.push(giant[idx]);
                                        }
                                        if p3 + 1 < n_giant {
                                            new_g.extend_from_slice(&giant[(p3 + 1)..n_giant]);
                                        }
                                        spliced_giant = Some(new_g);
                                        break 's_loop;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(new_g) = spliced_giant {
                giant = new_g;
                progress = true;
            } else {
                remaining_smalls.push(s);
            }
        }

        (giant, remaining_smalls, progress)
    }
}
