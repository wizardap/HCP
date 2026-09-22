use crate::graph::Graph;
use crate::encoder::Encoder;
use rustsat::types::Lit;

pub struct SnarkBridgeEngine;

impl SnarkBridgeEngine {
    /// Inspects the graph degree sequence. If the graph is 3-regular with exactly two degree-4 vertices
    /// connected by an edge (the canonical Flower Snark / GP(n,2)+1 edge construction from Haythorpe),
    /// returns the two endpoint vertices and the positive literal for the directed/undirected bridge.
    pub fn detect_and_extract_key_bridge(
        g: &Graph,
        encoder: &Encoder,
    ) -> Option<(i32, i32, Lit)> {
        let n = g.adjacency_list.len();
        if n < 4 {
            return None;
        }

        let mut deg_4_vertices = Vec::new();
        let mut deg_3_count = 0;

        for (&v, neighbors) in &g.adjacency_list {
            match neighbors.len() {
                3 => deg_3_count += 1,
                4 => deg_4_vertices.push(v),
                _ => return None, // Not a Snark+1 edge structure
            }
        }

        if deg_3_count == n - 2 && deg_4_vertices.len() == 2 {
            let u = deg_4_vertices[0];
            let v = deg_4_vertices[1];

            // Check if there is an edge between u and v
            if let Some(neighbors) = g.adjacency_list.get(&u) {
                if neighbors.contains(&v) {
                    if let Some(&lit) = encoder.graph_lit_map.get(&(u, v)) {
                        return Some((u, v, lit));
                    } else if let Some(&lit) = encoder.graph_lit_map.get(&(v, u)) {
                        return Some((v, u, lit));
                    }
                }
            }
        }

        None
    }
}
