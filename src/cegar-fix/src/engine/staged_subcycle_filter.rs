use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subcycle {
    pub vertices: Vec<i32>,
    pub edges: Vec<(i32, i32)>,
}

pub struct StagedSubcycleFilter {
    pub k_stage: usize,
    pub max_batch_size: usize,
}

impl StagedSubcycleFilter {
    pub fn new(max_batch_size: usize) -> Self {
        Self {
            k_stage: 2,
            max_batch_size,
        }
    }

    pub fn extract_subcycles(active_arcs: &[(i32, i32)]) -> Vec<Subcycle> {
        let mut next_map: HashMap<i32, i32> = HashMap::new();
        for &(u, v) in active_arcs {
            next_map.insert(u, v);
        }

        let mut visited: HashSet<i32> = HashSet::new();
        let mut subcycles = Vec::new();

        let mut sorted_vertices: Vec<i32> = next_map.keys().copied().collect();
        sorted_vertices.sort_unstable();

        for &start_v in &sorted_vertices {
            if visited.contains(&start_v) {
                continue;
            }

            let mut curr = start_v;
            let mut cycle_verts = Vec::new();
            let mut cycle_edges = Vec::new();

            while !visited.contains(&curr) {
                visited.insert(curr);
                cycle_verts.push(curr);
                if let Some(&nxt) = next_map.get(&curr) {
                    cycle_edges.push((curr, nxt));
                    curr = nxt;
                } else {
                    break;
                }
            }

            if !cycle_verts.is_empty() {
                subcycles.push(Subcycle {
                    vertices: cycle_verts,
                    edges: cycle_edges,
                });
            }
        }

        subcycles.sort_by_key(|c| c.vertices.len());
        subcycles
    }

    pub fn filter_active_cycles<'a>(
        &mut self,
        cycles: &'a [Subcycle],
        n_total: usize,
    ) -> Vec<&'a Subcycle> {
        if cycles.is_empty() || (cycles.len() == 1 && cycles[0].vertices.len() == n_total) {
            return Vec::new();
        }

        loop {
            let mut matches: Vec<&'a Subcycle> = cycles
                .iter()
                .filter(|c| c.vertices.len() <= self.k_stage)
                .collect();

            if !matches.is_empty() {
                if matches.len() > self.max_batch_size {
                    matches.truncate(self.max_batch_size);
                }
                return matches;
            }

            if self.k_stage >= n_total {
                // If stage exceeded N, return the smallest available cycles
                let mut all: Vec<&'a Subcycle> = cycles.iter().collect();
                all.sort_by_key(|c| c.vertices.len());
                if all.len() > self.max_batch_size {
                    all.truncate(self.max_batch_size);
                }
                return all;
            }

            // Advance to next power of 2 stage
            self.k_stage = std::cmp::min(self.k_stage * 2, n_total);
        }
    }
}
