use std::collections::{HashMap, HashSet};
use crate::graph::Graph;

#[derive(Clone, Debug)]
pub struct HubRegistry {
    pub is_hub: Vec<bool>,
    pub hub_vertices: Vec<i32>,
    pub hub_neighbors: HashMap<i32, HashSet<i32>>,
    pub min_hub_degree: usize,
}

impl HubRegistry {
    pub fn new(g: &Graph) -> Self {
        let total_v = g.adjacency_list.len();
        let total_deg: usize = g.adjacency_list.values().map(|v| v.len()).sum();
        let avg_deg = if total_v > 0 { total_deg as f64 / total_v as f64 } else { 0.0 };
        let max_deg = g.adjacency_list.values().map(|v| v.len()).max().unwrap_or(0);
        
        // A vertex is a hub if its degree is significantly above average and exceeds threshold
        let min_hub_degree = (max_deg / 2).max(20).min(50);
        
        let max_v = g.adjacency_list.keys().copied().max().unwrap_or(0).max(total_v as i32) as usize;
        let mut is_hub = vec![false; max_v + 1];
        let mut hub_vertices = Vec::new();
        let mut hub_neighbors = HashMap::new();
        
        for (&u, neighbors) in &g.adjacency_list {
            if neighbors.len() >= min_hub_degree && (neighbors.len() as f64) >= avg_deg * 3.0 {
                if (u as usize) < is_hub.len() {
                    is_hub[u as usize] = true;
                }
                hub_vertices.push(u);
                hub_neighbors.insert(u, neighbors.iter().cloned().collect());
            }
        }
        
        hub_vertices.sort_unstable_by(|&a, &b| {
            g.adjacency_list[&b].len().cmp(&g.adjacency_list[&a].len())
        });
        
        HubRegistry {
            is_hub,
            hub_vertices,
            hub_neighbors,
            min_hub_degree,
        }
    }
    
    pub fn is_hub_vertex(&self, v: i32) -> bool {
        if v >= 0 && (v as usize) < self.is_hub.len() {
            self.is_hub[v as usize]
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Graph;

    fn build_test_graph(edges: &[(i32, i32)]) -> Graph {
        let mut g = Graph::new();
        for &(u, v) in edges {
            g.add_edge(u, v);
        }
        g
    }

    #[test]
    fn test_hub_detection_star_graph() {
        // Hub 1 connected to 30 nodes (deg=30), nodes 2..31 have deg 2 or 3
        let mut edges = Vec::new();
        for v in 2..=31 {
            edges.push((1, v));
            let next_v = if v == 31 { 2 } else { v + 1 };
            edges.push((v, next_v));
        }
        let g = build_test_graph(&edges);
        let registry = HubRegistry::new(&g);
        assert!(registry.is_hub_vertex(1));
        assert_eq!(registry.hub_vertices, vec![1]);
        assert!(!registry.is_hub_vertex(2));
        assert_eq!(registry.min_hub_degree, 20);
        assert!(registry.hub_neighbors.contains_key(&1));
        assert_eq!(registry.hub_neighbors.get(&1).unwrap().len(), 30);
    }

    #[test]
    fn test_hub_detection_regular_graph() {
        // In a 3-regular graph, no vertex should be classified as a hub
        let mut edges = Vec::new();
        let n = 40;
        for i in 1..=n {
            let next1 = if i == n { 1 } else { i + 1 };
            let next2 = if (i + n / 2) > n { (i + n / 2) - n } else { i + n / 2 };
            edges.push((i, next1));
            if i <= n / 2 {
                edges.push((i, next2));
            }
        }
        let g = build_test_graph(&edges);
        let registry = HubRegistry::new(&g);
        assert!(registry.hub_vertices.is_empty());
        for i in 1..=n {
            assert!(!registry.is_hub_vertex(i));
        }
    }
}
