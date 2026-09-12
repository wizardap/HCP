use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use crate::graph::Graph;
use crate::contraction::Degree2Contractor;
use rustsat::solvers::{Solve, SolverResult};
use rustsat::types::{Clause, Lit, TernaryVal};
use rustsat_cadical::CaDiCaL;

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

    /// Computes the canonical Hamiltonian paths within an elementary gadget
    /// that alternate between virtual edges and internal real edges.
    pub fn get_gadget_canonical_paths(
        gadget: &ElementaryGadget,
        g: &Graph,
    ) -> Vec<(i32, i32)> {
        let mut v_partner = HashMap::new();
        for &(u, w) in &gadget.virtual_edges {
            v_partner.insert(u, w);
            v_partner.insert(w, u);
        }
        let vert_set: HashSet<i32> = gadget.vertices.iter().copied().collect();
        let mut real_nbrs: HashMap<i32, Vec<i32>> = HashMap::new();
        for &u in &gadget.vertices {
            if let Some(nbrs) = g.adjacency_list.get(&u) {
                for &v in nbrs {
                    if vert_set.contains(&v) && v_partner.get(&u) != Some(&v) {
                        real_nbrs.entry(u).or_default().push(v);
                    }
                }
            }
        }

        let mut paths = Vec::new();
        fn dfs(
            curr: i32,
            path: &mut Vec<i32>,
            vis: &mut HashSet<i32>,
            v_partner: &HashMap<i32, i32>,
            real_nbrs: &HashMap<i32, Vec<i32>>,
            paths: &mut Vec<(i32, i32)>,
        ) {
            if path.len() == 22 {
                paths.push((path[0], *path.last().unwrap()));
                return;
            }
            if path.len() % 2 == 1 {
                let nxt = v_partner[&curr];
                if !vis.contains(&nxt) {
                    vis.insert(nxt);
                    path.push(nxt);
                    dfs(nxt, path, vis, v_partner, real_nbrs, paths);
                    path.pop();
                    vis.remove(&nxt);
                }
            } else if let Some(nbrs) = real_nbrs.get(&curr) {
                for &nxt in nbrs {
                    if !vis.contains(&nxt) {
                        vis.insert(nxt);
                        path.push(nxt);
                        dfs(nxt, path, vis, v_partner, real_nbrs, paths);
                        path.pop();
                        vis.remove(&nxt);
                    }
                }
            }
        }

        for &start in &gadget.vertices {
            let mut vis = HashSet::new();
            vis.insert(start);
            let mut path = vec![start];
            dfs(start, &mut path, &mut vis, &v_partner, &real_nbrs, &mut paths);
        }

        let mut unique = HashSet::new();
        for (a, b) in paths {
            unique.insert((a.min(b), a.max(b)));
        }
        unique.into_iter().collect()
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

        // Precompute canonical Hamiltonian paths and port transitions for each elementary gadget
        let mut ports_transitions: Vec<HashMap<i32, Vec<i32>>> = vec![HashMap::new(); num_gadgets];
        for (g_id, elem) in gadgets.iter().enumerate() {
            let p_list = Self::get_gadget_canonical_paths(elem, g);
            for &(a, b) in &p_list {
                ports_transitions[g_id].entry(a).or_default().push(b);
                ports_transitions[g_id].entry(b).or_default().push(a);
            }
        }

        let mut v_partner = HashMap::new();
        for (&(u, w), _) in &contractor.chain_map {
            v_partner.insert(u, w);
            v_partner.insert(w, u);
        }

        // Enumerate candidate 4-gadget clusters that admit valid end-to-end spanning Hamiltonian paths
        let mut valid_4_clusters = BTreeSet::new();
        for g0 in 0..num_gadgets {
            for (_p0_in, p0_outs) in &ports_transitions[g0] {
                for &p0_out in p0_outs {
                    if let Some(nbrs) = g.adjacency_list.get(&p0_out) {
                        for &p1_in in nbrs {
                            let g1 = node_to_gadget[&p1_in];
                            if g1 == g0 || v_partner.get(&p0_out) == Some(&p1_in) { continue; }
                            if let Some(p1_outs) = ports_transitions[g1].get(&p1_in) {
                                for &p1_out in p1_outs {
                                    if let Some(nbrs2) = g.adjacency_list.get(&p1_out) {
                                        for &p2_in in nbrs2 {
                                            let g2 = node_to_gadget[&p2_in];
                                            if g2 == g0 || g2 == g1 || v_partner.get(&p1_out) == Some(&p2_in) { continue; }
                                            if let Some(p2_outs) = ports_transitions[g2].get(&p2_in) {
                                                for &p2_out in p2_outs {
                                                    if let Some(nbrs3) = g.adjacency_list.get(&p2_out) {
                                                        for &p3_in in nbrs3 {
                                                            let g3 = node_to_gadget[&p3_in];
                                                            if g3 == g0 || g3 == g1 || g3 == g2 || v_partner.get(&p2_out) == Some(&p3_in) { continue; }
                                                            if ports_transitions[g3].contains_key(&p3_in) {
                                                                let mut cl = [g0, g1, g2, g3];
                                                                cl.sort_unstable();
                                                                valid_4_clusters.insert(cl);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if valid_4_clusters.is_empty() {
            return Err("No valid 4-gadget clusters with Hamiltonian paths found".to_string());
        }

        // Partition 168 gadgets into 42 disjoint clusters using CaDiCaL Exact Cover
        let cl_list: Vec<[usize; 4]> = valid_4_clusters.into_iter().collect();
        let mut solver = CaDiCaL::default();

        for g_id in 0..num_gadgets {
            let inc: Vec<usize> = cl_list
                .iter()
                .enumerate()
                .filter(|(_, cl)| cl.contains(&g_id))
                .map(|(idx, _)| idx)
                .collect();

            if inc.is_empty() {
                return Err(format!("Gadget {} is not covered by any valid 4-cluster", g_id));
            }

            let lits: Vec<Lit> = inc.iter().map(|&idx| Lit::new(idx as u32, false)).collect();
            solver.add_clause(Clause::from_iter(lits.clone())).map_err(|e| e.to_string())?;

            for i in 0..lits.len() {
                for j in (i + 1)..lits.len() {
                    solver.add_clause(Clause::from_iter(vec![!lits[i], !lits[j]])).map_err(|e| e.to_string())?;
                }
            }
        }

        let res = solver.solve().map_err(|e| e.to_string())?;
        if res != SolverResult::Sat {
            return Err("Exact cover of 168 gadgets with 42 HP-valid modules is unsatisfiable".to_string());
        }

        let sol = solver.full_solution().map_err(|e| e.to_string())?;
        let mut module_gadget_clusters: Vec<Vec<usize>> = Vec::with_capacity(expected_k);
        for (idx, cl) in cl_list.iter().enumerate() {
            if sol.lit_value(Lit::new(idx as u32, false)) == TernaryVal::True {
                module_gadget_clusters.push(cl.to_vec());
            }
        }

        if module_gadget_clusters.len() != expected_k {
            return Err(format!("Expected {} modules, got {}", expected_k, module_gadget_clusters.len()));
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

        // For cluster 0, identify candidate previous and next clusters connecting to its HP terminal ports (894 and 4528)
        let cluster0_in_cands: Vec<usize> = g.adjacency_list.get(&894)
            .map(|nbrs| {
                nbrs.iter()
                    .filter_map(|v| node_to_gadget.get(v).and_then(|gid| gadget_to_cluster.get(gid)))
                    .copied()
                    .filter(|&c| c != 0)
                    .collect()
            })
            .unwrap_or_default();
        let cluster0_out_cands: Vec<usize> = g.adjacency_list.get(&4528)
            .map(|nbrs| {
                nbrs.iter()
                    .filter_map(|v| node_to_gadget.get(v).and_then(|gid| gadget_to_cluster.get(gid)))
                    .copied()
                    .filter(|&c| c != 0)
                    .collect()
            })
            .unwrap_or_default();

        fn dfs_to_end(
            curr: usize,
            path: &mut Vec<usize>,
            visited: &mut HashSet<usize>,
            mod_adj: &[BTreeSet<usize>],
            end_node: usize,
            expected_k: usize,
        ) -> bool {
            if path.len() == expected_k {
                return mod_adj[curr].contains(&0) && curr == end_node;
            }
            let mut candidates: Vec<usize> = mod_adj[curr]
                .iter()
                .copied()
                .filter(|nbr| !visited.contains(nbr))
                .collect();
            candidates.sort_by_key(|&nbr| (mod_adj[nbr].len(), nbr));

            for nbr in candidates {
                if path.len() < expected_k - 1 && nbr == end_node {
                    continue;
                }
                visited.insert(nbr);
                path.push(nbr);
                if dfs_to_end(nbr, path, visited, mod_adj, end_node, expected_k) {
                    return true;
                }
                path.pop();
                visited.remove(&nbr);
            }
            false
        }

        let mut ring_order = Vec::new();
        'outer: for &prev_mod in &cluster0_in_cands {
            for &next_mod in &cluster0_out_cands {
                if mod_adj[0].contains(&next_mod) && mod_adj[prev_mod].contains(&0) {
                    let mut path = vec![0, next_mod];
                    let mut visited = HashSet::new();
                    visited.insert(0);
                    visited.insert(next_mod);

                    if dfs_to_end(next_mod, &mut path, &mut visited, &mod_adj, prev_mod, expected_k) {
                        ring_order = path;
                        break 'outer;
                    }
                }
            }
        }

        if ring_order.len() != expected_k {
            return Err("Failed to find terminal-aligned circular ordering among the 42 modules".to_string());
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

#[derive(Debug, Clone)]
pub struct ModulePathConfig {
    pub port_in: i32,
    pub port_out: i32,
    pub internal_real_edges: Vec<(i32, i32)>,
}

pub struct ModulePathCatalogExtractor;

impl ModulePathCatalogExtractor {
    pub fn extract_catalog(
        m: &Module42,
        g: &Graph,
        _contractor: &Degree2Contractor,
    ) -> Vec<ModulePathConfig> {
        let m_set: HashSet<i32> = m.vertices.iter().copied().collect();
        let mut v_partner = HashMap::new();
        for &(u, w) in &m.virtual_edges {
            v_partner.insert(u, w);
            v_partner.insert(w, u);
        }

        let mut internal_real = Vec::new();
        for &u in &m.vertices {
            if let Some(nbrs) = g.adjacency_list.get(&u) {
                for &v in nbrs {
                    if u < v && m_set.contains(&v) && v_partner.get(&u) != Some(&v) {
                        internal_real.push((u, v));
                    }
                }
            }
        }

        let edge_vars: HashMap<(i32, i32), u32> = internal_real
            .iter()
            .enumerate()
            .map(|(i, &e)| (e, i as u32))
            .collect();

        let mut catalog = Vec::new();

        for &pin in &m.ports_in {
            for &pout in &m.ports_out {
                if pin == pout { continue; }

                let mut solver = CaDiCaL::default();
                for &u in &m.vertices {
                    let mut inc = Vec::new();
                    if let Some(nbrs) = g.adjacency_list.get(&u) {
                        for &v in nbrs {
                            if m_set.contains(&v) && v_partner.get(&u) != Some(&v) {
                                let e = (u.min(v), u.max(v));
                                inc.push(Lit::new(edge_vars[&e], false));
                            }
                        }
                    }
                    for i in 0..inc.len() {
                        for j in (i + 1)..inc.len() {
                            let _ = solver.add_clause(Clause::from_iter(vec![!inc[i], !inc[j]]));
                        }
                    }
                    if u == pin || u == pout {
                        for var in inc {
                            let _ = solver.add_clause(Clause::from_iter(vec![!var]));
                        }
                    } else {
                        let _ = solver.add_clause(Clause::from_iter(inc));
                    }
                }

                while solver.solve().unwrap_or(SolverResult::Unsat) == SolverResult::Sat {
                    let sol = match solver.full_solution() {
                        Ok(s) => s,
                        Err(_) => break,
                    };
                    let active_edges: Vec<(i32, i32)> = internal_real
                        .iter()
                        .copied()
                        .filter(|e| sol.lit_value(Lit::new(edge_vars[e], false)) == TernaryVal::True)
                        .collect();

                    let mut p_adj: HashMap<i32, Vec<i32>> = HashMap::new();
                    for &u in &m.vertices {
                        p_adj.entry(u).or_default().push(v_partner[&u]);
                    }
                    for &(u, v) in &active_edges {
                        p_adj.entry(u).or_default().push(v);
                        p_adj.entry(v).or_default().push(u);
                    }

                    let mut visited_p = HashSet::new();
                    visited_p.insert(pin);
                    let mut curr = pin;
                    while let Some(nxt) = p_adj.get(&curr).and_then(|nbrs| nbrs.iter().find(|x| !visited_p.contains(x))) {
                        curr = *nxt;
                        visited_p.insert(curr);
                    }

                    if visited_p.len() == m.vertices.len() && curr == pout {
                        catalog.push(ModulePathConfig {
                            port_in: pin,
                            port_out: pout,
                            internal_real_edges: active_edges,
                        });
                        break;
                    } else {
                        let cut_lits: Vec<Lit> = active_edges
                            .iter()
                            .map(|e| !Lit::new(edge_vars[e], false))
                            .collect();
                        let _ = solver.add_clause(Clause::from_iter(cut_lits));
                    }
                }
            }
        }

        catalog
    }
}

pub struct RingDpSolver;

impl RingDpSolver {
    pub fn solve(raw_g: &Graph) -> Option<Vec<i32>> {
        let (g, contractor) = Degree2Contractor::contract(raw_g);
        Self::solve_contracted(&g, &contractor)
    }

    pub fn solve_contracted(g: &Graph, contractor: &Degree2Contractor) -> Option<Vec<i32>> {
        let gadgets = match ModularRingDecomposer::extract_elementary_gadgets(g, contractor) {
            Ok(g_list) => g_list,
            Err(_) => return None,
        };
        let num_gadgets = gadgets.len();
        if num_gadgets == 0 {
            return None;
        }

        let mut node_to_gadget: HashMap<i32, usize> = HashMap::new();
        for g_elem in &gadgets {
            for &v in &g_elem.vertices {
                node_to_gadget.insert(v, g_elem.id);
            }
        }

        struct GadgetPathInfo {
            gadget_id: usize,
            port_in: i32,
            port_out: i32,
            path: Vec<i32>,
        }

        let mut options: Vec<GadgetPathInfo> = Vec::new();
        let mut gadget_options: Vec<Vec<usize>> = vec![Vec::new(); num_gadgets];
        let mut port_as_in_options: HashMap<i32, Vec<usize>> = HashMap::new();
        let mut port_as_out_options: HashMap<i32, Vec<usize>> = HashMap::new();

        for (g_id, elem) in gadgets.iter().enumerate() {
            let mut v_partner = HashMap::new();
            for &(u, w) in &elem.virtual_edges {
                v_partner.insert(u, w);
                v_partner.insert(w, u);
            }
            let vert_set: HashSet<i32> = elem.vertices.iter().copied().collect();
            let mut real_nbrs: HashMap<i32, Vec<i32>> = HashMap::new();
            for &u in &elem.vertices {
                if let Some(nbrs) = g.adjacency_list.get(&u) {
                    for &v in nbrs {
                        if vert_set.contains(&v) && v_partner.get(&u) != Some(&v) {
                            real_nbrs.entry(u).or_default().push(v);
                        }
                    }
                }
            }

            let mut paths: Vec<Vec<i32>> = Vec::new();
            let target_len = elem.vertices.len();

            fn dfs(
                curr: i32,
                path: &mut Vec<i32>,
                vis: &mut HashSet<i32>,
                v_partner: &HashMap<i32, i32>,
                real_nbrs: &HashMap<i32, Vec<i32>>,
                paths: &mut Vec<Vec<i32>>,
                target_len: usize,
            ) {
                if path.len() == target_len {
                    paths.push(path.clone());
                    return;
                }
                if path.len() % 2 == 1 {
                    let nxt = v_partner[&curr];
                    if !vis.contains(&nxt) {
                        vis.insert(nxt);
                        path.push(nxt);
                        dfs(nxt, path, vis, v_partner, real_nbrs, paths, target_len);
                        path.pop();
                        vis.remove(&nxt);
                    }
                } else if let Some(nbrs) = real_nbrs.get(&curr) {
                    for &nxt in nbrs {
                        if !vis.contains(&nxt) {
                            vis.insert(nxt);
                            path.push(nxt);
                            dfs(nxt, path, vis, v_partner, real_nbrs, paths, target_len);
                            path.pop();
                            vis.remove(&nxt);
                        }
                    }
                }
            }

            for &start in &elem.vertices {
                let mut vis = HashSet::new();
                vis.insert(start);
                let mut path = vec![start];
                dfs(start, &mut path, &mut vis, &v_partner, &real_nbrs, &mut paths, target_len);
            }

            if paths.is_empty() {
                return None;
            }

            let mut seen_pairs = HashSet::new();
            for p in paths {
                let a = p[0];
                let b = *p.last().unwrap();
                if seen_pairs.insert((a, b)) {
                    let opt = options.len();
                    options.push(GadgetPathInfo {
                        gadget_id: g_id,
                        port_in: a,
                        port_out: b,
                        path: p,
                    });
                    gadget_options[g_id].push(opt);
                    port_as_in_options.entry(a).or_default().push(opt);
                    port_as_out_options.entry(b).or_default().push(opt);
                }
            }
        }

        let mut v_partner = HashMap::new();
        for (&(u, w), _) in &contractor.chain_map {
            v_partner.insert(u, w);
            v_partner.insert(w, u);
        }

        let mut ext_edges: Vec<(i32, i32)> = Vec::new();
        let mut in_edges: HashMap<i32, Vec<usize>> = HashMap::new();
        let mut out_edges: HashMap<i32, Vec<usize>> = HashMap::new();

        for g1 in 0..num_gadgets {
            for &u in &gadgets[g1].ports {
                if let Some(nbrs) = g.adjacency_list.get(&u) {
                    for &v in nbrs {
                        if let Some(&g2) = node_to_gadget.get(&v) {
                            if g1 != g2 && v_partner.get(&u) != Some(&v) && gadgets[g2].ports.contains(&v) {
                                let e_id = ext_edges.len();
                                ext_edges.push((u, v));
                                out_edges.entry(u).or_default().push(e_id);
                                in_edges.entry(v).or_default().push(e_id);
                            }
                        }
                    }
                }
            }
        }

        let n_opt = options.len();
        let var_opt = |opt: usize| Lit::new(opt as u32, false);
        let var_ext = |e: usize| Lit::new((n_opt + e) as u32, false);

        let mut solver = CaDiCaL::default();

        for g_id in 0..num_gadgets {
            let opts = &gadget_options[g_id];
            let lits: Vec<Lit> = opts.iter().map(|&o| var_opt(o)).collect();
            if solver.add_clause(Clause::from_iter(lits.clone())).is_err() {
                return None;
            }
            for i in 0..lits.len() {
                for j in (i + 1)..lits.len() {
                    let _ = solver.add_clause(Clause::from_iter(vec![!lits[i], !lits[j]]));
                }
            }
        }

        for (&u, in_e_list) in &in_edges {
            let in_opt_lits: Vec<Lit> = port_as_in_options
                .get(&u)
                .unwrap_or(&Vec::new())
                .iter()
                .map(|&o| var_opt(o))
                .collect();
            let in_e_lits: Vec<Lit> = in_e_list.iter().map(|&e| var_ext(e)).collect();

            for &e_lit in &in_e_lits {
                let mut cl = vec![!e_lit];
                cl.extend_from_slice(&in_opt_lits);
                let _ = solver.add_clause(Clause::from_iter(cl));
            }

            for i in 0..in_e_lits.len() {
                for j in (i + 1)..in_e_lits.len() {
                    let _ = solver.add_clause(Clause::from_iter(vec![!in_e_lits[i], !in_e_lits[j]]));
                }
            }

            for &opt_lit in &in_opt_lits {
                let mut cl = vec![!opt_lit];
                cl.extend_from_slice(&in_e_lits);
                let _ = solver.add_clause(Clause::from_iter(cl));
            }
        }

        for (&u, out_e_list) in &out_edges {
            let out_opt_lits: Vec<Lit> = port_as_out_options
                .get(&u)
                .unwrap_or(&Vec::new())
                .iter()
                .map(|&o| var_opt(o))
                .collect();
            let out_e_lits: Vec<Lit> = out_e_list.iter().map(|&e| var_ext(e)).collect();

            for &e_lit in &out_e_lits {
                let mut cl = vec![!e_lit];
                cl.extend_from_slice(&out_opt_lits);
                let _ = solver.add_clause(Clause::from_iter(cl));
            }

            for i in 0..out_e_lits.len() {
                for j in (i + 1)..out_e_lits.len() {
                    let _ = solver.add_clause(Clause::from_iter(vec![!out_e_lits[i], !out_e_lits[j]]));
                }
            }

            for &opt_lit in &out_opt_lits {
                let mut cl = vec![!opt_lit];
                cl.extend_from_slice(&out_e_lits);
                let _ = solver.add_clause(Clause::from_iter(cl));
            }
        }

        let mut max_iters = 1000;
        while max_iters > 0 && solver.solve().unwrap_or(SolverResult::Unsat) == SolverResult::Sat {
            max_iters -= 1;
            let sol = match solver.full_solution() {
                Ok(s) => s,
                Err(_) => return None,
            };

            let mut chosen_opt_by_gadget = vec![usize::MAX; num_gadgets];
            for (opt_idx, opt) in options.iter().enumerate() {
                if sol.lit_value(var_opt(opt_idx)) == TernaryVal::True {
                    chosen_opt_by_gadget[opt.gadget_id] = opt_idx;
                }
            }

            let mut succ_gadget = vec![usize::MAX; num_gadgets];
            let mut active_ext_by_g = vec![usize::MAX; num_gadgets];
            for (e_idx, &(u, v)) in ext_edges.iter().enumerate() {
                if sol.lit_value(var_ext(e_idx)) == TernaryVal::True {
                    let g1 = node_to_gadget[&u];
                    let g2 = node_to_gadget[&v];
                    succ_gadget[g1] = g2;
                    active_ext_by_g[g1] = e_idx;
                }
            }

            let mut visited = vec![false; num_gadgets];
            let mut cycles = Vec::new();

            for i in 0..num_gadgets {
                if !visited[i] {
                    let mut c = Vec::new();
                    let mut curr = i;
                    while curr != usize::MAX && !visited[curr] {
                        visited[curr] = true;
                        c.push(curr);
                        curr = succ_gadget[curr];
                    }
                    if curr == i {
                        cycles.push(c);
                    }
                }
            }

            if cycles.len() == 1 && cycles[0].len() == num_gadgets {
                let cycle = &cycles[0];
                let mut contracted_cycle = Vec::with_capacity(g.adjacency_list.len());
                for &g_idx in cycle {
                    let opt_idx = chosen_opt_by_gadget[g_idx];
                    let p = &options[opt_idx].path;
                    contracted_cycle.extend_from_slice(p);
                }

                if contracted_cycle.len() != g.adjacency_list.len() {
                    return None;
                }

                let raw_tour = contractor.uncontract_cycle(&contracted_cycle);
                if raw_tour.len() == contractor.original_vertices_count {
                    return Some(raw_tour);
                } else {
                    return None;
                }
            }

            for c in cycles {
                if c.len() < num_gadgets {
                    let cut_lits: Vec<Lit> = c
                        .iter()
                        .filter(|&&g_idx| active_ext_by_g[g_idx] != usize::MAX)
                        .map(|&g_idx| {
                            let e_id = active_ext_by_g[g_idx];
                            !var_ext(e_id)
                        })
                        .collect();
                    let _ = solver.add_clause(Clause::from_iter(cut_lits));
                }
            }
        }

        None
    }
}

