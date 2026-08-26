use std::collections::{HashMap, VecDeque};
use crate::graph::Graph;

#[derive(Debug, Clone)]
pub struct ComponentMetaGraph {
    pub num_components: usize,
    pub cross_edges: HashMap<(usize, usize), Vec<(i32, i32)>>,
    pub meta_adj: Vec<Vec<usize>>,
    pub meta_components: Vec<Vec<usize>>,
}

impl ComponentMetaGraph {
    /// Builds the meta-graph of cycle components from a set of subtour cycles and the base graph.
    pub fn build(cycles: &[Vec<i32>], g: &Graph) -> Self {
        let num_components = cycles.len();
        let mut vertex_to_cycle: HashMap<i32, usize> = HashMap::new();

        for (cycle_idx, cycle) in cycles.iter().enumerate() {
            for &v in cycle {
                vertex_to_cycle.insert(v, cycle_idx);
            }
        }

        let mut cross_edges: HashMap<(usize, usize), Vec<(i32, i32)>> = HashMap::new();

        for (&u, neighbors) in &g.adjacency_list {
            for &v in neighbors {
                if u < v {
                    if let (Some(&c_u), Some(&c_v)) = (vertex_to_cycle.get(&u), vertex_to_cycle.get(&v)) {
                        if c_u != c_v {
                            let key = if c_u < c_v { (c_u, c_v) } else { (c_v, c_u) };
                            cross_edges.entry(key).or_default().push((u, v));
                        }
                    }
                }
            }
        }

        let mut meta_adj: Vec<Vec<usize>> = vec![Vec::new(); num_components];
        for (&(c1, c2), edges) in &cross_edges {
            if !edges.is_empty() {
                meta_adj[c1].push(c2);
                meta_adj[c2].push(c1);
            }
        }

        for neighbors in &mut meta_adj {
            neighbors.sort_unstable();
            neighbors.dedup();
        }

        let mut visited = vec![false; num_components];
        let mut meta_components = Vec::new();

        for i in 0..num_components {
            if !visited[i] {
                let mut comp = Vec::new();
                let mut queue = VecDeque::new();
                visited[i] = true;
                queue.push_back(i);

                while let Some(u) = queue.pop_front() {
                    comp.push(u);
                    for &next in &meta_adj[u] {
                        if !visited[next] {
                            visited[next] = true;
                            queue.push_back(next);
                        }
                    }
                }
                meta_components.push(comp);
            }
        }

        Self {
            num_components,
            cross_edges,
            meta_adj,
            meta_components,
        }
    }

    /// Returns true iff there are at least 2 cross-edges between component c1 and c2 (structural 2-opt merge potential).
    pub fn has_merge_potential(&self, c1: usize, c2: usize) -> bool {
        if c1 == c2 {
            return false;
        }
        let key = if c1 < c2 { (c1, c2) } else { (c2, c1) };
        self.cross_edges.get(&key).map_or(false, |edges| edges.len() >= 2)
    }

    /// Returns true iff all components belong to at most one connected meta-component (or cycles is empty).
    pub fn is_connected(&self) -> bool {
        self.num_components == 0 || self.meta_components.len() <= 1
    }

    /// Returns the computed connected meta-components.
    pub fn get_meta_components(&self) -> &[Vec<usize>] {
        &self.meta_components
    }
}
