use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};
use crate::contraction::Degree2Contractor;
use crate::graph::Graph;
use crate::hub_registry::HubRegistry;
use rustsat::clause;
use rustsat::instances::{BasicVarManager, Cnf, ManageVars};
use rustsat::solvers::{Solve, SolverResult};
use rustsat::types::{Clause, Lit, TernaryVal};
use rustsat_cadical::CaDiCaL;

#[derive(Clone, Debug)]
pub struct SatelliteModule {
    pub module_id: usize,
    pub vertices: HashSet<i32>,
    pub internal_adj: HashMap<i32, Vec<i32>>,
    pub hub_connections: HashMap<i32, Vec<i32>>, // hub_id -> connected boundary nodes
}

pub struct ModularSolver;

impl ModularSolver {
    /// Extracts connected components of G[V \ Hubs] with >= 5 vertices,
    /// mapping boundary nodes to adjacent hubs.
    pub fn extract_satellite_modules(
        g: &Graph,
        hub_registry: &HubRegistry,
    ) -> Vec<SatelliteModule> {
        if hub_registry.hub_vertices.is_empty() {
            return Vec::new();
        }

        let mut hub_set: HashSet<i32> = hub_registry.hub_vertices.iter().copied().collect();
        // For dense hub graphs, include all high-degree connectors (degree >= 20) in hub_set
        for (&u, nbrs) in &g.adjacency_list {
            if nbrs.len() >= 20 {
                hub_set.insert(u);
            }
        }
        let mut non_hub_vertices: Vec<i32> = g
            .adjacency_list
            .keys()
            .filter(|v| !hub_set.contains(v))
            .copied()
            .collect();

        non_hub_vertices.sort_unstable();

        let mut visited = HashSet::new();
        let mut raw_components: Vec<HashSet<i32>> = Vec::new();

        for &u in &non_hub_vertices {
            if visited.contains(&u) {
                continue;
            }
            let mut comp = HashSet::new();
            let mut q = VecDeque::new();
            visited.insert(u);
            q.push_back(u);

            while let Some(curr) = q.pop_front() {
                comp.insert(curr);
                if let Some(neighbors) = g.adjacency_list.get(&curr) {
                    for &nbr in neighbors {
                        if !hub_set.contains(&nbr) && !visited.contains(&nbr) {
                            visited.insert(nbr);
                            q.push_back(nbr);
                        }
                    }
                }
            }

            if comp.len() >= 5 {
                raw_components.push(comp);
            }
        }

        // Build SatelliteModule structs
        let mut modules = Vec::with_capacity(raw_components.len());
        for (mod_idx, comp_verts) in raw_components.into_iter().enumerate() {
            let mut internal_adj = HashMap::new();
            let mut hub_connections: HashMap<i32, Vec<i32>> = HashMap::new();

            for &u in &comp_verts {
                if let Some(neighbors) = g.adjacency_list.get(&u) {
                    let mut int_nbrs = Vec::new();
                    for &nbr in neighbors {
                        if comp_verts.contains(&nbr) {
                            int_nbrs.push(nbr);
                        } else if hub_registry.is_hub_vertex(nbr) {
                            hub_connections.entry(nbr).or_default().push(u);
                        }
                    }
                    int_nbrs.sort_unstable();
                    internal_adj.insert(u, int_nbrs);
                }
            }

            for boundary_nodes in hub_connections.values_mut() {
                boundary_nodes.sort_unstable();
                boundary_nodes.dedup();
            }

            modules.push(SatelliteModule {
                module_id: mod_idx,
                vertices: comp_verts,
                internal_adj,
                hub_connections,
            });
        }

        modules
    }

    /// Solves directed Hamiltonian path on the module's induced subgraph from in_vertex to out_vertex
    /// using CaDiCaL SAT solver in RAM with degree-2 (endpoints degree-1) constraints and subtour elimination cuts.
    pub fn solve_module_hamiltonian_path(
        module: &SatelliteModule,
        _g: &Graph,
        in_vertex: i32,
        out_vertex: i32,
    ) -> Option<Vec<i32>> {
        if !module.vertices.contains(&in_vertex) || !module.vertices.contains(&out_vertex) {
            return None;
        }

        let n = module.vertices.len();
        if n == 0 {
            return None;
        }
        if n == 1 {
            if in_vertex == out_vertex {
                return Some(vec![in_vertex]);
            }
            return None;
        }
        if in_vertex == out_vertex {
            return None; // Simple path of length >= 2 must have distinct endpoints
        }
        if n == 2 {
            if module
                .internal_adj
                .get(&in_vertex)
                .map_or(false, |adjs| adjs.contains(&out_vertex))
            {
                return Some(vec![in_vertex, out_vertex]);
            } else {
                return None;
            }
        }

        let mut verts: Vec<i32> = module.vertices.iter().copied().collect();
        verts.sort_unstable();

        let mut arc_lit_map: HashMap<(i32, i32), Lit> = HashMap::new();
        let mut var_manager = BasicVarManager::default();

        for &u in &verts {
            if let Some(nbrs) = module.internal_adj.get(&u) {
                for &v in nbrs {
                    if module.vertices.contains(&v) && u != v {
                        // In an s-t path: no arc enters in_vertex, no arc leaves out_vertex
                        if v == in_vertex || u == out_vertex {
                            continue;
                        }
                        let lit = var_manager.new_lit();
                        arc_lit_map.insert((u, v), lit);
                    }
                }
            }
        }

        let mut cnf = Cnf::new();

        // 1. Out-degree constraints:
        // - For in_vertex and intermediate vertices: exactly 1 outgoing edge
        for &u in &verts {
            if u == out_vertex {
                continue;
            }
            let out_lits: Vec<Lit> = verts
                .iter()
                .filter_map(|&v| arc_lit_map.get(&(u, v)).copied())
                .collect();
            if out_lits.is_empty() {
                return None; // Dead end
            }

            // At-least-one
            let mut cl = Clause::new();
            cl.extend(out_lits.clone());
            cnf.add_clause(cl);

            // At-most-one (pairwise)
            for i in 0..out_lits.len() {
                for j in i + 1..out_lits.len() {
                    cnf.add_clause(clause!(!out_lits[i], !out_lits[j]));
                }
            }
        }

        // 2. In-degree constraints:
        // - For out_vertex and intermediate vertices: exactly 1 incoming edge
        for &v in &verts {
            if v == in_vertex {
                continue;
            }
            let in_lits: Vec<Lit> = verts
                .iter()
                .filter_map(|&u| arc_lit_map.get(&(u, v)).copied())
                .collect();
            if in_lits.is_empty() {
                return None; // Unreachable
            }

            // At-least-one
            let mut cl = Clause::new();
            cl.extend(in_lits.clone());
            cnf.add_clause(cl);

            // At-most-one (pairwise)
            for i in 0..in_lits.len() {
                for j in i + 1..in_lits.len() {
                    cnf.add_clause(clause!(!in_lits[i], !in_lits[j]));
                }
            }
        }

        // 3. 2-cycle prohibition for internal pairs
        for (i, &u) in verts.iter().enumerate() {
            for &v in &verts[i + 1..] {
                if let (Some(&lit_uv), Some(&lit_vu)) =
                    (arc_lit_map.get(&(u, v)), arc_lit_map.get(&(v, u)))
                {
                    cnf.add_clause(clause!(!lit_uv, !lit_vu));
                }
            }
        }

        let mut solver = CaDiCaL::default();
        let _ = solver.add_cnf(cnf);

        let start_time = Instant::now();
        let timeout = Duration::from_millis(1500); // 1.5s max timeout per module
        let max_iterations = 200;

        for _ in 0..max_iterations {
            if start_time.elapsed() >= timeout {
                return None;
            }

            match solver.solve() {
                Ok(SolverResult::Sat) => {
                    let sol = solver.full_solution().unwrap();
                    let mut succ_map: HashMap<i32, i32> = HashMap::new();
                    for (&(u, v), &lit) in &arc_lit_map {
                        if sol.lit_value(lit) == TernaryVal::True {
                            succ_map.insert(u, v);
                        }
                    }

                    // Trace path from in_vertex
                    let mut path = Vec::new();
                    let mut curr = in_vertex;
                    let mut visited_in_path = HashSet::new();

                    while !visited_in_path.contains(&curr) {
                        visited_in_path.insert(curr);
                        path.push(curr);
                        if curr == out_vertex {
                            break;
                        }
                        if let Some(&next) = succ_map.get(&curr) {
                            curr = next;
                        } else {
                            break;
                        }
                    }

                    // Check if we found a valid full Hamiltonian path covering all n vertices
                    if path.len() == n && path.last() == Some(&out_vertex) {
                        return Some(path);
                    }

                    // Subtour elimination: find cycles among remaining vertices
                    let mut visited = visited_in_path.clone();
                    let mut found_subcycle = false;

                    for &start_node in &verts {
                        if visited.contains(&start_node) {
                            continue;
                        }
                        let mut cycle = Vec::new();
                        let mut c_curr = start_node;
                        while !visited.contains(&c_curr) {
                            visited.insert(c_curr);
                            cycle.push(c_curr);
                            if let Some(&c_next) = succ_map.get(&c_curr) {
                                c_curr = c_next;
                            } else {
                                break;
                            }
                        }

                        if cycle.len() >= 2 {
                            found_subcycle = true;
                            let mut block_cl = Clause::new();
                            let clen = cycle.len();
                            for k in 0..clen {
                                let u = cycle[k];
                                let v = cycle[(k + 1) % clen];
                                if let Some(&lit) = arc_lit_map.get(&(u, v)) {
                                    block_cl.add(!lit);
                                }
                            }
                            if block_cl.len() > 0 {
                                let _ = solver.add_clause(block_cl);
                            }

                            // Add cut constraint: at least one edge leaving cycle
                            let c_set: HashSet<i32> = cycle.iter().copied().collect();
                            let mut cut_lits = Vec::new();
                            for &u in &cycle {
                                if let Some(nbrs) = module.internal_adj.get(&u) {
                                    for &v in nbrs {
                                        if module.vertices.contains(&v)
                                            && !c_set.contains(&v)
                                            && v != in_vertex
                                        {
                                            if let Some(&lit) = arc_lit_map.get(&(u, v)) {
                                                cut_lits.push(lit);
                                            }
                                        }
                                    }
                                }
                            }
                            if !cut_lits.is_empty() {
                                let mut cut_cl = Clause::new();
                                cut_cl.extend(cut_lits);
                                let _ = solver.add_clause(cut_cl);
                            }
                        }
                    }

                    // If path reached out_vertex prematurely, block this incomplete path
                    if !found_subcycle && path.len() < n {
                        let mut path_block = Clause::new();
                        for k in 0..path.len().saturating_sub(1) {
                            let u = path[k];
                            let v = path[k + 1];
                            if let Some(&lit) = arc_lit_map.get(&(u, v)) {
                                path_block.add(!lit);
                            }
                        }
                        if path_block.len() > 0 {
                            let _ = solver.add_clause(path_block);
                        } else {
                            return None;
                        }
                    }
                }
                _ => return None,
            }
        }

        None
    }

    /// Solves Hamiltonian Cycle via Modular Macro-Decomposition.
    /// Decomposes graph into satellite modules, solves localized Hamiltonian paths,
    /// contracts them into a macro-graph, solves the macro-tour, and reconstructs the full cycle.
    pub fn solve_via_modular_decomposition(
        g: &Graph,
        contractor: &Degree2Contractor,
        hub_registry: &HubRegistry,
    ) -> Option<Vec<i32>> {
        if hub_registry.hub_vertices.is_empty() {
            return None;
        }

        let modules = Self::extract_satellite_modules(g, hub_registry);
        if modules.is_empty() {
            return None;
        }

        let hub_set: HashSet<i32> = hub_registry.hub_vertices.iter().copied().collect();
        let mut module_vertex_set: HashSet<i32> = HashSet::new();
        for m in &modules {
            module_vertex_set.extend(&m.vertices);
        }

        // Remainder vertices not in hubs and not in any satellite module
        let remainder_vertices: Vec<i32> = g
            .adjacency_list
            .keys()
            .filter(|v| !hub_set.contains(v) && !module_vertex_set.contains(v))
            .copied()
            .collect();

        // For each module, find an entry and exit pair connected to hubs and solve internal Hamiltonian path
        let mut solved_module_paths: Vec<(i32, i32, Vec<i32>)> = Vec::with_capacity(modules.len());

        for module in &modules {
            let mut connected_hubs: Vec<i32> = module.hub_connections.keys().copied().collect();
            connected_hubs.sort_unstable_by(|&a, &b| {
                module.hub_connections[&b]
                    .len()
                    .cmp(&module.hub_connections[&a].len())
            });

            if connected_hubs.is_empty() {
                return None; // Module has no connection to hubs
            }

            let mut candidate_pairs: Vec<(i32, i32)> = Vec::new();

            if connected_hubs.len() >= 2 {
                for i in 0..connected_hubs.len() {
                    for j in (i + 1)..connected_hubs.len() {
                        let ha = connected_hubs[i];
                        let hb = connected_hubs[j];
                        let mut in_cands = module.hub_connections[&ha].clone();
                        let mut out_cands = module.hub_connections[&hb].clone();

                        // Sort candidates by internal degree ascending (prefer low-degree endpoints)
                        in_cands.sort_unstable_by_key(|&u| {
                            module.internal_adj.get(&u).map_or(0, |a| a.len())
                        });
                        out_cands.sort_unstable_by_key(|&u| {
                            module.internal_adj.get(&u).map_or(0, |a| a.len())
                        });

                        for &in_v in in_cands.iter().take(10) {
                            for &out_v in out_cands.iter().take(10) {
                                if in_v != out_v {
                                    candidate_pairs.push((in_v, out_v));
                                    candidate_pairs.push((out_v, in_v));
                                }
                            }
                        }
                    }
                }
            } else {
                let h0 = connected_hubs[0];
                let mut cands = module.hub_connections[&h0].clone();
                cands.sort_unstable_by_key(|&u| {
                    module.internal_adj.get(&u).map_or(0, |a| a.len())
                });
                for i in 0..cands.len() {
                    for j in (i + 1)..cands.len() {
                        let in_v = cands[i];
                        let out_v = cands[j];
                        candidate_pairs.push((in_v, out_v));
                        candidate_pairs.push((out_v, in_v));
                    }
                }
            }

            let mut found_path: Option<(i32, i32, Vec<i32>)> = None;
            for (in_v, out_v) in candidate_pairs {
                if let Some(path) = Self::solve_module_hamiltonian_path(module, g, in_v, out_v) {
                    found_path = Some((in_v, out_v, path));
                    break;
                }
            }

            match found_path {
                Some(res) => solved_module_paths.push(res),
                None => return None, // Could not solve Hamiltonian path for this module
            }
        }



        // Build Reduced Macro-Graph
        // Macro-nodes: Hubs + Remainder vertices + (in_v, out_v) for each module
        let mut macro_vertices: Vec<i32> = Vec::new();
        let mut macro_vertex_set: HashSet<i32> = HashSet::new();

        for &h in &hub_registry.hub_vertices {
            if macro_vertex_set.insert(h) {
                macro_vertices.push(h);
            }
        }
        for &r in &remainder_vertices {
            if macro_vertex_set.insert(r) {
                macro_vertices.push(r);
            }
        }
        for &(in_v, out_v, _) in &solved_module_paths {
            if macro_vertex_set.insert(in_v) {
                macro_vertices.push(in_v);
            }
            if macro_vertex_set.insert(out_v) {
                macro_vertices.push(out_v);
            }
        }

        let num_macro_nodes = macro_vertices.len();
        if num_macro_nodes < 3 {
            return None;
        }

        let mut v_to_idx: HashMap<i32, usize> = HashMap::new();
        for (idx, &v) in macro_vertices.iter().enumerate() {
            v_to_idx.insert(v, idx);
        }

        let mut module_pairs: Vec<(usize, usize)> = Vec::new();
        let mut module_node_set: HashSet<usize> = HashSet::new();

        for &(in_v, out_v, _) in &solved_module_paths {
            let in_idx = v_to_idx[&in_v];
            let out_idx = v_to_idx[&out_v];
            module_pairs.push((in_idx, out_idx));
            module_node_set.insert(in_idx);
            module_node_set.insert(out_idx);
        }

        // Build macro adjacency list
        let mut macro_adj: Vec<Vec<usize>> = vec![Vec::new(); num_macro_nodes];

        // 1. Add bidirectional internal macro-edges for each module
        for &(in_idx, out_idx) in &module_pairs {
            macro_adj[in_idx].push(out_idx);
            macro_adj[out_idx].push(in_idx);
        }

        // 2. Add external connections to and from graph vertices
        for (u_idx, &u) in macro_vertices.iter().enumerate() {
            let is_u_module = module_node_set.contains(&u_idx);
            if let Some(nbrs) = g.adjacency_list.get(&u) {
                for &v in nbrs {
                    if let Some(&v_idx) = v_to_idx.get(&v) {
                        if u_idx != v_idx {
                            let is_v_module = module_node_set.contains(&v_idx);
                            // Avoid adding external connections between module endpoints of the same module (already added)
                            if is_u_module && is_v_module {
                                continue;
                            }
                            macro_adj[u_idx].push(v_idx);
                        }
                    }
                }
            }
            macro_adj[u_idx].sort_unstable();
            macro_adj[u_idx].dedup();
        }

        // Solve Macro-Tour via SAT
        let mut var_manager = BasicVarManager::default();
        let mut arc_lit_map: HashMap<(usize, usize), Lit> = HashMap::new();

        for u_idx in 0..num_macro_nodes {
            for &v_idx in &macro_adj[u_idx] {
                let lit = var_manager.new_lit();
                arc_lit_map.insert((u_idx, v_idx), lit);
            }
        }

        let mut cnf = Cnf::new();

        // 1. Exactly-One traversal direction constraint for each module
        for &(in_idx, out_idx) in &module_pairs {
            if let (Some(&lit_fwd), Some(&lit_rev)) = (
                arc_lit_map.get(&(in_idx, out_idx)),
                arc_lit_map.get(&(out_idx, in_idx)),
            ) {
                // At-least-one direction
                cnf.add_clause(clause!(lit_fwd, lit_rev));
                // At-most-one direction
                cnf.add_clause(clause!(!lit_fwd, !lit_rev));
            } else {
                return None;
            }
        }

        // 2. Degree-1 outgoing constraints
        for u_idx in 0..num_macro_nodes {
            let out_lits: Vec<Lit> = macro_adj[u_idx]
                .iter()
                .map(|&v_idx| arc_lit_map[&(u_idx, v_idx)])
                .collect();
            if out_lits.is_empty() {
                return None;
            }

            let mut cl = Clause::new();
            cl.extend(out_lits.clone());
            cnf.add_clause(cl);

            for i in 0..out_lits.len() {
                for j in i + 1..out_lits.len() {
                    cnf.add_clause(clause!(!out_lits[i], !out_lits[j]));
                }
            }
        }

        // 3. Degree-1 incoming constraints
        for v_idx in 0..num_macro_nodes {
            let mut in_lits = Vec::new();
            for u_idx in 0..num_macro_nodes {
                if let Some(&lit) = arc_lit_map.get(&(u_idx, v_idx)) {
                    in_lits.push(lit);
                }
            }
            if in_lits.is_empty() {
                return None;
            }

            let mut cl = Clause::new();
            cl.extend(in_lits.clone());
            cnf.add_clause(cl);

            for i in 0..in_lits.len() {
                for j in i + 1..in_lits.len() {
                    cnf.add_clause(clause!(!in_lits[i], !in_lits[j]));
                }
            }
        }

        // 4. 2-cycle prohibition for non-module edges
        if num_macro_nodes > 2 {
            let module_pair_set: HashSet<(usize, usize)> = module_pairs
                .iter()
                .copied()
                .flat_map(|(a, b)| vec![(a, b), (b, a)])
                .collect();

            for u_idx in 0..num_macro_nodes {
                for &v_idx in &macro_adj[u_idx] {
                    if u_idx < v_idx && !module_pair_set.contains(&(u_idx, v_idx)) {
                        if let Some(&lit_vu) = arc_lit_map.get(&(v_idx, u_idx)) {
                            let lit_uv = arc_lit_map[&(u_idx, v_idx)];
                            cnf.add_clause(clause!(!lit_uv, !lit_vu));
                        }
                    }
                }
            }
        }

        // 5. Degree-2 contractor mandatory edges
        for (&(u, w), _) in &contractor.chain_map {
            if u < w {
                if let (Some(&u_idx), Some(&w_idx)) = (v_to_idx.get(&u), v_to_idx.get(&w)) {
                    let lit_uw = arc_lit_map.get(&(u_idx, w_idx));
                    let lit_wu = arc_lit_map.get(&(w_idx, u_idx));
                    match (lit_uw, lit_wu) {
                        (Some(&l1), Some(&l2)) => cnf.add_clause(clause!(l1, l2)),
                        (Some(&l1), None) => cnf.add_clause(clause!(l1)),
                        (None, Some(&l2)) => cnf.add_clause(clause!(l2)),
                        _ => {}
                    }
                }
            }
        }

        let mut solver = CaDiCaL::default();
        let _ = solver.add_cnf(cnf);

        let max_macro_iterations = 200;
        let mut macro_tour: Option<Vec<usize>> = None;

        for _ in 0..max_macro_iterations {
            match solver.solve() {
                Ok(SolverResult::Sat) => {
                    let sol = solver.full_solution().unwrap();
                    let mut succ_map: HashMap<usize, usize> = HashMap::new();
                    for (&(u, v), &lit) in &arc_lit_map {
                        if sol.lit_value(lit) == TernaryVal::True {
                            succ_map.insert(u, v);
                        }
                    }

                    let mut visited = vec![false; num_macro_nodes];
                    let mut macro_cycles = Vec::new();

                    for start in 0..num_macro_nodes {
                        if visited[start] {
                            continue;
                        }
                        let mut cycle = Vec::new();
                        let mut curr = start;
                        while !visited[curr] {
                            visited[curr] = true;
                            cycle.push(curr);
                            if let Some(&next) = succ_map.get(&curr) {
                                curr = next;
                            } else {
                                break;
                            }
                        }
                        if !cycle.is_empty() {
                            macro_cycles.push(cycle);
                        }
                    }

                    if macro_cycles.len() == 1 && macro_cycles[0].len() == num_macro_nodes {
                        macro_tour = Some(macro_cycles[0].clone());
                        break;
                    } else {
                        // Add subtour elimination cuts
                        for subtour in &macro_cycles {
                            if subtour.len() < num_macro_nodes {
                                let mut subtour_block = Clause::new();
                                let m = subtour.len();
                                for t in 0..m {
                                    let u = subtour[t];
                                    let v = subtour[(t + 1) % m];
                                    if let Some(&lit) = arc_lit_map.get(&(u, v)) {
                                        subtour_block.add(!lit);
                                    }
                                }
                                if subtour_block.len() > 0 {
                                    let _ = solver.add_clause(subtour_block);
                                }

                                let subtour_set: HashSet<usize> =
                                    subtour.iter().copied().collect();
                                let mut cut_lits = Vec::new();
                                for &u in subtour {
                                    for &v in &macro_adj[u] {
                                        if !subtour_set.contains(&v) {
                                            if let Some(&lit) = arc_lit_map.get(&(u, v)) {
                                                cut_lits.push(lit);
                                            }
                                        }
                                    }
                                }
                                if !cut_lits.is_empty() {
                                    let mut cut_clause = Clause::new();
                                    cut_clause.extend(cut_lits);
                                    let _ = solver.add_clause(cut_clause);
                                }
                            }
                        }
                    }
                }
                _ => return None,
            }
        }

        let m_tour = macro_tour?;

        // Map (in_v, out_v) and (out_v, in_v) to intermediate path
        let mut module_path_map: HashMap<(i32, i32), Vec<i32>> = HashMap::new();
        for (in_v, out_v, path) in solved_module_paths {
            let mut rev_path = path.clone();
            rev_path.reverse();
            module_path_map.insert((in_v, out_v), path);
            module_path_map.insert((out_v, in_v), rev_path);
        }

        // Expand macro-tour into full graph cycle
        let mut full_tour = Vec::with_capacity(g.adjacency_list.len());
        let k = m_tour.len();

        for i in 0..k {
            let u_idx = m_tour[i];
            let v_idx = m_tour[(i + 1) % k];
            let u = macro_vertices[u_idx];
            let v = macro_vertices[v_idx];

            if let Some(path) = module_path_map.get(&(u, v)) {
                // Macro-arc: insert path vertices [p0, p1, ..., p_last-1]
                // The last vertex will be pushed as u in the next iteration
                for &p_node in &path[..path.len() - 1] {
                    full_tour.push(p_node);
                }
            } else {
                full_tour.push(u);
            }
        }

        if is_valid_cycle(&full_tour, g) && full_tour.len() == g.adjacency_list.len() {
            Some(full_tour)
        } else {
            None
        }
    }
}

pub fn is_valid_cycle(cycle: &[i32], g: &Graph) -> bool {
    let len = cycle.len();
    if len < 3 {
        return false;
    }
    for i in 0..len {
        let u = cycle[i];
        let v = cycle[(i + 1) % len];
        if !g.adjacency_list.get(&u).map_or(false, |adj| adj.contains(&v)) {
            return false;
        }
    }
    let mut seen = HashSet::with_capacity(len);
    for &v in cycle {
        if !seen.insert(v) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contraction::Degree2Contractor;
    use crate::graph::Graph;
    use crate::hub_registry::HubRegistry;
    use std::collections::HashMap;

    fn build_test_graph(edges: &[(i32, i32)]) -> Graph {
        let mut g = Graph::new();
        for &(u, v) in edges {
            g.add_edge(u, v);
        }
        g
    }

    fn empty_contractor() -> Degree2Contractor {
        Degree2Contractor {
            chain_map: HashMap::new(),
            original_vertices_count: 0,
            contracted_vertices_count: 0,
            is_direct_cycle: None,
            is_infeasible: false,
        }
    }

    #[test]
    fn test_satellite_module_extraction() {
        // Build graph with 2 hubs (nodes 1 and 2) and 2 satellite modules:
        // Module A: {10, 11, 12, 13, 14, 15}
        // Module B: {20, 21, 22, 23, 24, 25}
        let mut edges = Vec::new();

        // Hub 1 and Hub 2 connections to create high degree (simulating dense hubs)
        for v in 100..130 {
            edges.push((1, v));
            edges.push((2, v));
        }

        // Module A internal path
        edges.push((10, 11));
        edges.push((11, 12));
        edges.push((12, 13));
        edges.push((13, 14));
        edges.push((14, 15));
        // Hub connections for Module A
        edges.push((1, 10));
        edges.push((2, 15));

        // Module B internal path
        edges.push((20, 21));
        edges.push((21, 22));
        edges.push((22, 23));
        edges.push((23, 24));
        edges.push((24, 25));
        // Hub connections for Module B
        edges.push((1, 20));
        edges.push((2, 25));

        let g = build_test_graph(&edges);
        let hub_registry = HubRegistry::new(&g);

        assert!(hub_registry.is_hub_vertex(1));
        assert!(hub_registry.is_hub_vertex(2));

        let modules = ModularSolver::extract_satellite_modules(&g, &hub_registry);
        assert_eq!(modules.len(), 2);

        let mod_a = modules.iter().find(|m| m.vertices.contains(&10)).unwrap();
        assert_eq!(mod_a.vertices.len(), 6);
        assert!(mod_a.hub_connections.contains_key(&1));
        assert!(mod_a.hub_connections.contains_key(&2));
        assert_eq!(mod_a.hub_connections[&1], vec![10]);
        assert_eq!(mod_a.hub_connections[&2], vec![15]);

        let mod_b = modules.iter().find(|m| m.vertices.contains(&20)).unwrap();
        assert_eq!(mod_b.vertices.len(), 6);
        assert!(mod_b.hub_connections.contains_key(&1));
        assert!(mod_b.hub_connections.contains_key(&2));
        assert_eq!(mod_b.hub_connections[&1], vec![20]);
        assert_eq!(mod_b.hub_connections[&2], vec![25]);
    }

    #[test]
    fn test_module_hamiltonian_path_solving() {
        // Satellite module of 6 vertices: 10 - 11 - 12 - 13 - 14 - 15 with some cross edges
        let mut edges = Vec::new();
        edges.push((10, 11));
        edges.push((11, 12));
        edges.push((12, 13));
        edges.push((13, 14));
        edges.push((14, 15));
        edges.push((10, 13));
        edges.push((11, 14));
        edges.push((12, 15));

        let g = build_test_graph(&edges);

        let mut internal_adj = HashMap::new();
        for &u in &[10, 11, 12, 13, 14, 15] {
            let nbrs = g.adjacency_list.get(&u).unwrap().clone();
            internal_adj.insert(u, nbrs);
        }

        let module = SatelliteModule {
            module_id: 0,
            vertices: [10, 11, 12, 13, 14, 15].iter().copied().collect(),
            internal_adj,
            hub_connections: HashMap::new(),
        };

        let path_opt = ModularSolver::solve_module_hamiltonian_path(&module, &g, 10, 15);
        assert!(path_opt.is_some(), "Should find Hamiltonian path from 10 to 15");
        let path = path_opt.unwrap();
        assert_eq!(path.len(), 6);
        assert_eq!(path[0], 10);
        assert_eq!(path[5], 15);

        // Verify valid simple path
        for i in 0..5 {
            let u = path[i];
            let v = path[i + 1];
            assert!(g.adjacency_list[&u].contains(&v));
        }

        // Test disconnected/impossible endpoint
        let path_impossible = ModularSolver::solve_module_hamiltonian_path(&module, &g, 10, 999);
        assert!(path_impossible.is_none());
    }

    #[test]
    fn test_modular_solver_end_to_end() {
        // 3 Hubs (1, 2, 3) connected to 3 satellite modules (each 5 vertices)
        // Hubs are explicitly registered in HubRegistry
        let mut edges = Vec::new();

        // Hub cycle: 1 - 2 - 3 - 1
        edges.push((1, 2));
        edges.push((2, 3));
        edges.push((3, 1));

        // Module 1: 10..14 (5 vertices)
        // Path: 10 - 11 - 12 - 13 - 14
        edges.push((10, 11));
        edges.push((11, 12));
        edges.push((12, 13));
        edges.push((13, 14));
        edges.push((10, 14)); // chords
        edges.push((1, 10));  // Hub 1 -> Module 1 entry (10)
        edges.push((2, 14));  // Module 1 exit (14) -> Hub 2

        // Module 2: 20..24 (5 vertices)
        // Path: 20 - 21 - 22 - 23 - 24
        edges.push((20, 21));
        edges.push((21, 22));
        edges.push((22, 23));
        edges.push((23, 24));
        edges.push((20, 24));
        edges.push((2, 20));  // Hub 2 -> Module 2 entry (20)
        edges.push((3, 24));  // Module 2 exit (24) -> Hub 3

        // Module 3: 30..34 (5 vertices)
        // Path: 30 - 31 - 32 - 33 - 34
        edges.push((30, 31));
        edges.push((31, 32));
        edges.push((32, 33));
        edges.push((33, 34));
        edges.push((30, 34));
        edges.push((3, 30));  // Hub 3 -> Module 3 entry (30)
        edges.push((1, 34));  // Module 3 exit (34) -> Hub 1

        let g = build_test_graph(&edges);
        let contractor = empty_contractor();

        // Configure HubRegistry with hubs 1, 2, 3
        let max_v = g.adjacency_list.keys().copied().max().unwrap_or(0) as usize;
        let mut is_hub = vec![false; max_v + 1];
        is_hub[1] = true;
        is_hub[2] = true;
        is_hub[3] = true;
        let mut hub_neighbors = HashMap::new();
        for &h in &[1, 2, 3] {
            hub_neighbors.insert(h, g.adjacency_list[&h].iter().copied().collect());
        }
        let hub_registry = HubRegistry {
            is_hub,
            hub_vertices: vec![1, 2, 3],
            hub_neighbors,
            min_hub_degree: 2,
        };

        let tour_opt = ModularSolver::solve_via_modular_decomposition(&g, &contractor, &hub_registry);
        assert!(tour_opt.is_some(), "ModularSolver should find Hamiltonian cycle");
        let tour = tour_opt.unwrap();
        assert_eq!(tour.len(), g.adjacency_list.len());
        assert!(is_valid_cycle(&tour, &g));
    }

    #[test]
    fn test_modular_solver_degree2_contraction_safety() {
        // 3 Hubs and 3 Modules, with degree-2 chains inside and outside modules
        let mut edges = Vec::new();

        // Hub cycle: 1 - 2 - 3 - 1
        edges.push((1, 2));
        edges.push((2, 3));
        edges.push((3, 1));

        // Module 1: 10..14 (5 vertices)
        edges.push((10, 11));
        edges.push((11, 12));
        edges.push((12, 13));
        edges.push((13, 14));
        edges.push((10, 14));
        edges.push((1, 10));
        edges.push((2, 14));

        // Module 2: 20..24 (5 vertices)
        edges.push((20, 21));
        edges.push((21, 22));
        edges.push((22, 23));
        edges.push((23, 24));
        edges.push((20, 24));
        edges.push((2, 20));
        edges.push((3, 24));

        // Module 3: 30..34 (5 vertices)
        edges.push((30, 31));
        edges.push((31, 32));
        edges.push((32, 33));
        edges.push((33, 34));
        edges.push((30, 34));
        edges.push((3, 30));
        edges.push((1, 34));

        let g = build_test_graph(&edges);
        let mut contractor = empty_contractor();
        // Insert a degree-2 chain between (11, 12)
        contractor.chain_map.insert((11, 12), vec![115]);
        contractor.chain_map.insert((12, 11), vec![115]);

        let max_v = 115;
        let mut is_hub = vec![false; max_v + 1];
        is_hub[1] = true;
        is_hub[2] = true;
        is_hub[3] = true;
        let mut hub_neighbors = HashMap::new();
        for &h in &[1, 2, 3] {
            hub_neighbors.insert(h, g.adjacency_list[&h].iter().copied().collect());
        }
        let hub_registry = HubRegistry {
            is_hub,
            hub_vertices: vec![1, 2, 3],
            hub_neighbors,
            min_hub_degree: 2,
        };

        let tour_opt = ModularSolver::solve_via_modular_decomposition(&g, &contractor, &hub_registry);
        assert!(tour_opt.is_some());
        let tour = tour_opt.unwrap();
        assert_eq!(tour.len(), g.adjacency_list.len());
        assert!(is_valid_cycle(&tour, &g));

        // Uncontracting via contractor should yield the full 19-vertex cycle including node 115
        let full_uncontracted = contractor.uncontract_cycle(&tour);
        assert!(full_uncontracted.contains(&115));
        assert_eq!(full_uncontracted.len(), g.adjacency_list.len() + 1);
    }

    #[test]
    fn test_modular_solver_empty_hub_fallback() {
        let edges = vec![(1, 2), (2, 3), (3, 1)];
        let g = build_test_graph(&edges);
        let contractor = empty_contractor();
        let hub_registry = HubRegistry::new(&g);
        assert!(hub_registry.hub_vertices.is_empty());
        let result = ModularSolver::solve_via_modular_decomposition(&g, &contractor, &hub_registry);
        assert!(result.is_none());
    }

    #[test]
    #[ignore]
    fn test_modular_dense_graph560() {
        let g = crate::file_operations::input_to_graph("../../FHCPCS-col/graph560.col");
        let (contracted_g, contractor) = Degree2Contractor::contract(&g);
        let hub_registry = HubRegistry::new(&contracted_g);
        println!("Hubs detected: {}", hub_registry.hub_vertices.len());
        for h in &hub_registry.hub_vertices {
            println!("  Hub {}: degree {}", h, contracted_g.adjacency_list[h].len());
        }
        let modules = ModularSolver::extract_satellite_modules(&contracted_g, &hub_registry);
        println!("Modules extracted: {}", modules.len());
        for (i, m) in modules.iter().enumerate() {
            println!("  Module {}: size={}, connected to {} hubs", i, m.vertices.len(), m.hub_connections.len());
        }
        let t0 = std::time::Instant::now();
        let res = ModularSolver::solve_via_modular_decomposition(&contracted_g, &contractor, &hub_registry);
        println!("ModularSolver result: {:?}, elapsed: {:?}", res.is_some(), t0.elapsed());
    }
}

