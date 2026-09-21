use std::collections::{HashMap, HashSet};

/// Merges two cycles if there exist vertices u1, u2 on c1 and v1, v2 on c2
/// such that deleting (u1, u2) and (v1, v2) and reconnecting forms a single cycle.
/// Never deletes edges present in `forbidden_delete`.
pub fn merge_two_cycles(
    c1: &[i32],
    c2: &[i32],
    adj: &HashMap<i32, HashSet<i32>>,
    forbidden_delete: &HashSet<(i32, i32)>,
) -> Option<Vec<i32>> {
    let n1 = c1.len();
    let n2 = c2.len();
    if n1 == 0 || n2 == 0 {
        return None;
    }

    for i in 0..n1 {
        let u1 = c1[i];
        let u2 = c1[(i + 1) % n1];
        let e1 = (u1.min(u2), u1.max(u2));
        if forbidden_delete.contains(&e1) {
            continue;
        }

        let u1_nbrs = match adj.get(&u1) {
            Some(s) => s,
            None => continue,
        };
        let u2_nbrs = match adj.get(&u2) {
            Some(s) => s,
            None => continue,
        };

        for j in 0..n2 {
            let v1 = c2[j];
            let v2 = c2[(j + 1) % n2];
            let e2 = (v1.min(v2), v1.max(v2));
            if forbidden_delete.contains(&e2) {
                continue;
            }

            // Case 1: (u1, v1) and (u2, v2)
            if u1_nbrs.contains(&v1) && u2_nbrs.contains(&v2) {
                let mut tour = Vec::with_capacity(n1 + n2);
                for k in 1..=n1 {
                    tour.push(c1[(i + k) % n1]);
                }
                for k in 0..n2 {
                    let idx = (j + n2 - (k % n2)) % n2;
                    tour.push(c2[idx]);
                }
                return Some(tour);
            }

            // Case 2: (u1, v2) and (u2, v1)
            if u1_nbrs.contains(&v2) && u2_nbrs.contains(&v1) {
                let mut tour = Vec::with_capacity(n1 + n2);
                for k in 1..=n1 {
                    tour.push(c1[(i + k) % n1]);
                }
                for k in 0..n2 {
                    let idx = (j + 1 + k) % n2;
                    tour.push(c2[idx]);
                }
                return Some(tour);
            }
        }
    }

    None
}

/// Attempts greedy 2-opt pairwise merge on disjoint cycles.
/// Never deletes any edge in `forbidden_delete`.
pub fn safe_2opt_merge(
    cycles: &[Vec<i32>],
    adj: &HashMap<i32, HashSet<i32>>,
    forbidden_delete: &HashSet<(i32, i32)>,
) -> Option<Vec<i32>> {
    let mut curr = cycles.to_vec();
    let mut merged_any = true;

    while merged_any && curr.len() > 1 {
        merged_any = false;
        curr.sort_by(|a, b| b.len().cmp(&a.len()));

        let num_cycles = curr.len();
        let mut merge_step = None;

        'search: for i in 0..num_cycles {
            for j in (i + 1)..num_cycles {
                if let Some(res) = merge_two_cycles(&curr[i], &curr[j], adj, forbidden_delete) {
                    merge_step = Some((i, j, res));
                    break 'search;
                }
            }
        }

        if let Some((i, j, res)) = merge_step {
            curr.remove(j);
            curr[i] = res;
            merged_any = true;
        }
    }

    if curr.len() == 1 {
        Some(curr.into_iter().next().unwrap())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_adj(edges: &[(i32, i32)]) -> HashMap<i32, HashSet<i32>> {
        let mut adj: HashMap<i32, HashSet<i32>> = HashMap::new();
        for &(u, v) in edges {
            adj.entry(u).or_default().insert(v);
            adj.entry(v).or_default().insert(u);
        }
        adj
    }

    #[test]
    fn test_merge_two_cycles_success() {
        // Cycle 1: 1 - 2 - 3 - 1
        // Cycle 2: 4 - 5 - 6 - 4
        // Cross edges: (1, 4) and (2, 5)
        // Merge should cut (1, 2) and (4, 5) and connect 1-4 and 2-5
        let c1 = vec![1, 2, 3];
        let c2 = vec![4, 5, 6];
        let edges = vec![
            (1, 2), (2, 3), (3, 1),
            (4, 5), (5, 6), (6, 4),
            (1, 4), (2, 5),
        ];
        let adj = build_adj(&edges);
        let forbidden = HashSet::new();

        let merged = safe_2opt_merge(&[c1, c2], &adj, &forbidden);
        assert!(merged.is_some());
        let tour = merged.unwrap();
        assert_eq!(tour.len(), 6);
    }

    #[test]
    fn test_merge_forbidden_delete() {
        // Cycle 1: 1 - 2 - 3 - 1
        // Cycle 2: 4 - 5 - 6 - 4
        // Cross edges: (1, 4) and (2, 5)
        // If (1, 2) is forbidden to delete, merge should fail (since that is the only cross-pair)
        let c1 = vec![1, 2, 3];
        let c2 = vec![4, 5, 6];
        let edges = vec![
            (1, 2), (2, 3), (3, 1),
            (4, 5), (5, 6), (6, 4),
            (1, 4), (2, 5),
        ];
        let adj = build_adj(&edges);
        let mut forbidden = HashSet::new();
        forbidden.insert((1, 2));

        let merged = safe_2opt_merge(&[c1, c2], &adj, &forbidden);
        assert!(merged.is_none());
    }

    #[test]
    fn test_merge_no_cross_edges() {
        let c1 = vec![1, 2, 3];
        let c2 = vec![4, 5, 6];
        let edges = vec![
            (1, 2), (2, 3), (3, 1),
            (4, 5), (5, 6), (6, 4),
        ];
        let adj = build_adj(&edges);
        let forbidden = HashSet::new();

        let merged = safe_2opt_merge(&[c1, c2], &adj, &forbidden);
        assert!(merged.is_none());
    }
}
