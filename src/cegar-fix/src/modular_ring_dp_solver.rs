use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use crate::graph::Graph;
use crate::contraction::Degree2Contractor;

#[derive(Debug, Clone)]
pub struct ElementaryGadget {
    pub id: usize,
    pub vertices: Vec<i32>,
    pub virtual_edges: Vec<(i32, i32)>,
    pub ports: Vec<i32>,
}

#[derive(Debug, Clone)]
pub struct Module42 {
    pub id: usize,
    pub vertices: Vec<i32>,
    pub virtual_edges: Vec<(i32, i32)>,
    pub ports_in: Vec<i32>,
    pub ports_out: Vec<i32>,
}

pub struct ModularRingDecomposer;

impl ModularRingDecomposer {
    /// Extracts the 168 elementary gadgets of 22 vertices from the contracted graph $G_c$.
    ///
    /// In graph868, degree-2 vertices in $G_c$ with their neighbors and virtual edges
    /// cleanly partition the 3,696 contracted vertices into 168 isomorphic gadgets of size 22,
    /// each containing 11 virtual edges and 12 boundary ports.
    pub fn extract_elementary_gadgets(
        g: &Graph,
        contractor: &Degree2Contractor,
    ) -> Result<Vec<ElementaryGadget>, String> {
        let total_v = g.adjacency_list.len();
        if total_v == 0 {
            return Err("Contracted graph is empty".to_string());
        }

        // Map each contracted vertex to its virtual partner
        let mut v_partner: HashMap<i32, i32> = HashMap::new();
        for (&(u, w), _) in &contractor.chain_map {
            v_partner.insert(u, w);
            v_partner.insert(w, u);
        }

        if v_partner.len() != total_v {
            return Err(format!(
                "Contractor chain map endpoints ({}) do not cover all contracted vertices ({})",
                v_partner.len(),
                total_v
            ));
        }

        // Identify degree-2 vertices in G_c:
        // In G_c, vertices of real degree 2 have total degree 3 (2 real edges + 1 virtual edge)
        let d2_nodes: HashSet<i32> = g
            .adjacency_list
            .iter()
            .filter(|(_, nbrs)| nbrs.len() == 3)
            .map(|(&u, _)| u)
            .collect();

        // Build gadget connectivity graph:
        // Connect each degree-2 vertex to all its neighbors in G_c,
        // and connect each vertex to its virtual edge partner.
        let mut adj_gadget: HashMap<i32, Vec<i32>> = HashMap::new();
        for &u in &d2_nodes {
            if let Some(nbrs) = g.adjacency_list.get(&u) {
                for &v in nbrs {
                    adj_gadget.entry(u).or_default().push(v);
                    adj_gadget.entry(v).or_default().push(u);
                }
            }
        }

        for (&u, &w) in &v_partner {
            adj_gadget.entry(u).or_default().push(w);
            adj_gadget.entry(w).or_default().push(u);
        }

        // Deterministic BFS traversal to extract connected components
        let mut sorted_vertices: Vec<i32> = g.adjacency_list.keys().copied().collect();
        sorted_vertices.sort_unstable();

        let mut visited: HashSet<i32> = HashSet::new();
        let mut raw_gadgets: Vec<Vec<i32>> = Vec::new();

        for &u in &sorted_vertices {
            if !visited.contains(&u) {
                let mut comp = Vec::new();
                let mut q = VecDeque::new();
                visited.insert(u);
                q.push_back(u);

                while let Some(curr) = q.pop_front() {
                    comp.push(curr);
                    if let Some(nbrs) = adj_gadget.get(&curr) {
                        for &nxt in nbrs {
                            if !visited.contains(&nxt) {
                                visited.insert(nxt);
                                q.push_back(nxt);
                            }
                        }
                    }
                }
                comp.sort_unstable();
                raw_gadgets.push(comp);
            }
        }

        // Verify that all raw gadgets have exactly 22 vertices
        for (idx, comp) in raw_gadgets.iter().enumerate() {
            if comp.len() != 22 {
                return Err(format!(
                    "Gadget {} has {} vertices, expected 22",
                    idx,
                    comp.len()
                ));
            }
        }

        // Build ElementaryGadget structs
        let mut gadgets = Vec::with_capacity(raw_gadgets.len());
        for (g_id, vertices) in raw_gadgets.into_iter().enumerate() {
            let vert_set: HashSet<i32> = vertices.iter().copied().collect();

            let mut virtual_edges = Vec::new();
            for &u in &vertices {
                if let Some(&w) = v_partner.get(&u) {
                    if u < w && vert_set.contains(&w) {
                        virtual_edges.push((u, w));
                    }
                }
            }
            virtual_edges.sort_unstable();

            let mut ports = Vec::new();
            for &u in &vertices {
                if let Some(nbrs) = g.adjacency_list.get(&u) {
                    if nbrs.iter().any(|nxt| !vert_set.contains(nxt)) {
                        ports.push(u);
                    }
                }
            }
            ports.sort_unstable();

            gadgets.push(ElementaryGadget {
                id: g_id,
                vertices,
                virtual_edges,
                ports,
            });
        }

        Ok(gadgets)
    }

    /// Decomposes the contracted graph into 42 modules of 88 vertices,
    /// ordered cyclically $M_0 \to M_1 \to \dots \to M_{41} \to M_0$.
    pub fn decompose(
        g: &Graph,
        contractor: &Degree2Contractor,
    ) -> Result<Vec<Module42>, String> {
        let gadgets = Self::extract_elementary_gadgets(g, contractor)?;
        let num_gadgets = gadgets.len();
        let expected_k = 42;

        if num_gadgets % expected_k != 0 {
            return Err(format!(
                "Number of gadgets ({}) is not divisible by {}",
                num_gadgets, expected_k
            ));
        }

        let target_gadgets_per_mod = num_gadgets / expected_k;

        // Build node to gadget mapping
        let mut node_to_gadget: HashMap<i32, usize> = HashMap::new();
        for g_elem in &gadgets {
            for &v in &g_elem.vertices {
                node_to_gadget.insert(v, g_elem.id);
            }
        }

        // Build inter-gadget connectivity graph
        let mut gadget_adj: Vec<HashMap<usize, usize>> = vec![HashMap::new(); num_gadgets];
        for (&u, nbrs) in &g.adjacency_list {
            let g1 = node_to_gadget[&u];
            for &v in nbrs {
                let g2 = node_to_gadget[&v];
                if g1 != g2 {
                    *gadget_adj[g1].entry(g2).or_insert(0) += 1;
                }
            }
        }

        // Greedy modular expansion to cluster gadgets into 42 modules of 4 gadgets each
        let mut unassigned: BTreeSet<usize> = (0..num_gadgets).collect();
        let mut module_gadget_clusters: Vec<Vec<usize>> = Vec::with_capacity(expected_k);

        while !unassigned.is_empty() {
            if module_gadget_clusters.len() + 1 == expected_k {
                let mut last_chunk: Vec<usize> = unassigned.into_iter().collect();
                last_chunk.sort_unstable();
                module_gadget_clusters.push(last_chunk);
                break;
            }

            let start = *unassigned.iter().next().unwrap();
            let mut chunk = vec![start];
            unassigned.remove(&start);

            let mut cand_conn: BTreeMap<usize, usize> = BTreeMap::new();
            for (&nbr, &w) in &gadget_adj[start] {
                if unassigned.contains(&nbr) {
                    *cand_conn.entry(nbr).or_insert(0) += w;
                }
            }

            while chunk.len() < target_gadgets_per_mod && !unassigned.is_empty() {
                let best_cand = if !cand_conn.is_empty() {
                    let (&cand, _) = cand_conn
                        .iter()
                        .max_by(|(cand_a, &w_a), (cand_b, &w_b)| {
                            w_a.cmp(&w_b).then_with(|| cand_b.cmp(cand_a))
                        })
                        .unwrap();
                    cand
                } else {
                    *unassigned.iter().next().unwrap()
                };

                chunk.push(best_cand);
                unassigned.remove(&best_cand);
                cand_conn.remove(&best_cand);

                for (&nbr, &w) in &gadget_adj[best_cand] {
                    if unassigned.contains(&nbr) {
                        *cand_conn.entry(nbr).or_insert(0) += w;
                    }
                }
            }

            chunk.sort_unstable();
            module_gadget_clusters.push(chunk);
        }

        // Map gadget to module cluster
        let mut gadget_to_cluster: HashMap<usize, usize> = HashMap::new();
        for (c_idx, g_list) in module_gadget_clusters.iter().enumerate() {
            for &g_id in g_list {
                gadget_to_cluster.insert(g_id, c_idx);
            }
        }

        // Quotient graph between the 42 modules
        let mut mod_adj: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); expected_k];
        for g1 in 0..num_gadgets {
            let m1 = gadget_to_cluster[&g1];
            for &g2 in gadget_adj[g1].keys() {
                let m2 = gadget_to_cluster[&g2];
                if m1 != m2 {
                    mod_adj[m1].insert(m2);
                }
            }
        }

        // Find Hamiltonian cycle of modules using Warnsdorff's heuristic
        fn find_module_ring(
            curr: usize,
            path: &mut Vec<usize>,
            visited: &mut HashSet<usize>,
            mod_adj: &[BTreeSet<usize>],
            expected_k: usize,
        ) -> bool {
            if path.len() == expected_k {
                return mod_adj[curr].contains(&path[0]);
            }

            let mut candidates: Vec<usize> = mod_adj[curr]
                .iter()
                .copied()
                .filter(|nbr| !visited.contains(nbr))
                .collect();
            candidates.sort_by_key(|&nbr| (mod_adj[nbr].len(), nbr));

            for nbr in candidates {
                visited.insert(nbr);
                path.push(nbr);
                if find_module_ring(nbr, path, visited, mod_adj, expected_k) {
                    return true;
                }
                path.pop();
                visited.remove(&nbr);
            }
            false
        }

        let mut ring_order = vec![0];
        let mut visited_mods = HashSet::new();
        visited_mods.insert(0);

        if !find_module_ring(0, &mut ring_order, &mut visited_mods, &mod_adj, expected_k) {
            return Err("Failed to find circular ordering among the 42 modules".to_string());
        }

        // Re-order modules along the circular ring
        let ordered_clusters: Vec<Vec<usize>> = ring_order
            .into_iter()
            .map(|c_idx| module_gadget_clusters[c_idx].clone())
            .collect();

        // Build vertex sets for each module in circular order
        let mut module_vertices_list: Vec<Vec<i32>> = Vec::with_capacity(expected_k);
        for g_list in &ordered_clusters {
            let mut v_list = Vec::new();
            for &g_id in g_list {
                v_list.extend_from_slice(&gadgets[g_id].vertices);
            }
            v_list.sort_unstable();
            module_vertices_list.push(v_list);
        }

        let mut modules = Vec::with_capacity(expected_k);
        for i in 0..expected_k {
            let prev_i = (i + expected_k - 1) % expected_k;
            let next_i = (i + 1) % expected_k;

            let prev_vert_set: HashSet<i32> = module_vertices_list[prev_i].iter().copied().collect();
            let next_vert_set: HashSet<i32> = module_vertices_list[next_i].iter().copied().collect();

            let verts = module_vertices_list[i].clone();
            let mut virtual_edges = Vec::new();

            for &g_id in &ordered_clusters[i] {
                for &ve in &gadgets[g_id].virtual_edges {
                    virtual_edges.push(ve);
                }
            }
            virtual_edges.sort_unstable();

            let mut ports_in = Vec::new();
            let mut ports_out = Vec::new();

            for &u in &verts {
                if let Some(nbrs) = g.adjacency_list.get(&u) {
                    if nbrs.iter().any(|v| prev_vert_set.contains(v)) {
                        ports_in.push(u);
                    }
                    if nbrs.iter().any(|v| next_vert_set.contains(v)) {
                        ports_out.push(u);
                    }
                }
            }

            ports_in.sort_unstable();
            ports_in.dedup();
            ports_out.sort_unstable();
            ports_out.dedup();

            if ports_in.is_empty() || ports_out.is_empty() {
                return Err(format!(
                    "Module {} has empty ports_in ({}) or ports_out ({})",
                    i, ports_in.len(), ports_out.len()
                ));
            }

            modules.push(Module42 {
                id: i,
                vertices: verts,
                virtual_edges,
                ports_in,
                ports_out,
            });
        }

        Ok(modules)
    }
}
