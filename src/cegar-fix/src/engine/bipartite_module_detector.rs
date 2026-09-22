use std::collections::{HashMap, HashSet};
use crate::graph::Graph;
use crate::contraction::Degree2Contractor;

#[derive(Debug, Clone)]
pub struct BipartiteModule {
    pub id: usize,
    pub vertices: Vec<i32>,
    pub virtual_edges: Vec<(i32, i32)>,
    pub boundary_ports: Vec<i32>,
}

pub struct BipartiteModuleDetector;

impl BipartiteModuleDetector {
    /// Detects canonical modules (such as the 44-vertex modules in 3-SAT reductions)
    /// using Greedy Modular Expansion on the Virtual Edge Quotient Graph.
    pub fn detect_44_modules(
        g: &Graph,
        contractor: &Degree2Contractor,
    ) -> Vec<BipartiteModule> {
        let total_v = g.adjacency_list.len();
        if total_v < 44 {
            return Vec::new();
        }

        // Expected number of modules: k = total_v / 44
        let expected_k = total_v / 44;
        if expected_k == 0 {
            return Vec::new();
        }

        // Collect canonical virtual edges (u < w)
        let mut canonical_virtual: Vec<(i32, i32)> = Vec::new();
        let mut vertex_to_ve: HashMap<i32, usize> = HashMap::new();

        for (&(u, w), _) in &contractor.chain_map {
            if u < w {
                let ve_idx = canonical_virtual.len();
                canonical_virtual.push((u, w));
                vertex_to_ve.insert(u, ve_idx);
                vertex_to_ve.insert(w, ve_idx);
            }
        }

        let num_ve = canonical_virtual.len();
        if num_ve == 0 {
            return Vec::new();
        }

        // Build quotient graph between virtual edges with connection frequencies
        let mut ve_adj: Vec<HashMap<usize, usize>> = vec![HashMap::new(); num_ve];
        for (ve_idx, &(u, w)) in canonical_virtual.iter().enumerate() {
            for &endpoint in &[u, w] {
                if let Some(nbrs) = g.adjacency_list.get(&endpoint) {
                    for &nbr in nbrs {
                        if let Some(&other_ve) = vertex_to_ve.get(&nbr) {
                            if other_ve != ve_idx {
                                *ve_adj[ve_idx].entry(other_ve).or_insert(0) += 1;
                            }
                        }
                    }
                }
            }
        }

        // Partition virtual edges into k clusters of target size = num_ve / expected_k
        let target_ve_per_mod = (num_ve / expected_k).max(1);
        let mut unassigned: HashSet<usize> = (0..num_ve).collect();
        let mut ve_clusters: Vec<Vec<usize>> = Vec::new();

        while !unassigned.is_empty() {
            if ve_clusters.len() + 1 == expected_k {
                let mut last_chunk: Vec<usize> = unassigned.drain().collect();
                last_chunk.sort_unstable();
                ve_clusters.push(last_chunk);
                break;
            }

            let start = *unassigned.iter().next().unwrap();
            let mut chunk = Vec::with_capacity(target_ve_per_mod);
            chunk.push(start);
            unassigned.remove(&start);

            let mut cand_conn: HashMap<usize, usize> = HashMap::new();
            for (&nbr_ve, &w) in &ve_adj[start] {
                if unassigned.contains(&nbr_ve) {
                    *cand_conn.entry(nbr_ve).or_insert(0) += w;
                }
            }

            while chunk.len() < target_ve_per_mod && !unassigned.is_empty() {
                let best_cand = if !cand_conn.is_empty() {
                    cand_conn.iter().max_by_key(|(_, &conn)| conn).map(|(&cand, _)| cand).unwrap()
                } else {
                    *unassigned.iter().next().unwrap()
                };

                chunk.push(best_cand);
                unassigned.remove(&best_cand);
                cand_conn.remove(&best_cand);

                for (&nbr_ve, &w) in &ve_adj[best_cand] {
                    if unassigned.contains(&nbr_ve) {
                        *cand_conn.entry(nbr_ve).or_insert(0) += w;
                    }
                }
            }

            chunk.sort_unstable();
            ve_clusters.push(chunk);
        }

        let mut final_modules = Vec::new();
        for (m_idx, cluster) in ve_clusters.into_iter().enumerate() {
            let mut mod_vertices = Vec::with_capacity(cluster.len() * 2);
            let mut mod_ve = Vec::with_capacity(cluster.len());

            for &ve_idx in &cluster {
                let (u, w) = canonical_virtual[ve_idx];
                mod_vertices.push(u);
                mod_vertices.push(w);
                mod_ve.push((u, w));
            }

            mod_vertices.sort_unstable();
            mod_vertices.dedup();
            let mod_set: HashSet<i32> = mod_vertices.iter().copied().collect();

            let mut boundary_ports = Vec::new();
            for &v in &mod_vertices {
                if let Some(nbrs) = g.adjacency_list.get(&v) {
                    if nbrs.iter().any(|nxt| !mod_set.contains(nxt)) {
                        boundary_ports.push(v);
                    }
                }
            }

            final_modules.push(BipartiteModule {
                id: m_idx,
                vertices: mod_vertices,
                virtual_edges: mod_ve,
                boundary_ports,
            });
        }

        final_modules
    }
}
