use std::collections::{HashMap, HashSet, VecDeque};
use crate::graph::Graph;
use crate::tour_verifier::TourVerifier;
use rustsat::clause;
use rustsat::instances::{BasicVarManager, ManageVars};
use rustsat::solvers::{Solve, SolverResult};
use rustsat::types::{Clause, Lit, TernaryVal};
use rustsat_cadical::CaDiCaL;

#[derive(Debug, Clone)]
pub struct HubModule {
    pub hub_id: i32,
    pub vertices: Vec<i32>,
    pub interface_ports: Vec<i32>,
    pub internal_paths: Vec<(i32, i32, Vec<i32>)>, // (entry_port, exit_port, path)
}

pub struct HubHierarchicalDecomposer;

impl HubHierarchicalDecomposer {
    /// Extracts localized hub modules around high-degree hub nodes (deg(v) >= min_hub_degree),
    /// partitions vertices to their nearest hub, identifies external interface ports,
    /// and enumerates valid internal Hamiltonian paths spanning each module.
    pub fn extract_hub_modules(g: &Graph, min_hub_degree: usize) -> Vec<HubModule> {
        let mut hubs: Vec<i32> = g
            .adjacency_list
            .iter()
            .filter(|(_, neighbors)| neighbors.len() >= min_hub_degree)
            .map(|(&v, _)| v)
            .collect();
        hubs.sort_unstable();

        if hubs.is_empty() {
            return Vec::new();
        }

        let hub_set: HashSet<i32> = hubs.iter().copied().collect();

        // Multi-source BFS to assign non-hub vertices to their closest hub
        // Distance and closest hub mapping: vertex -> (distance, closest_hub)
        let mut assignment: HashMap<i32, (usize, i32)> = HashMap::new();
        let mut queue = VecDeque::new();

        for &h in &hubs {
            assignment.insert(h, (0, h));
            queue.push_back(h);
        }

        while let Some(curr) = queue.pop_front() {
            let (curr_dist, curr_hub) = assignment[&curr];
            if let Some(neighbors) = g.adjacency_list.get(&curr) {
                for &next_v in neighbors {
                    if !hub_set.contains(&next_v) {
                        let next_dist = curr_dist + 1;
                        match assignment.get(&next_v) {
                            None => {
                                assignment.insert(next_v, (next_dist, curr_hub));
                                queue.push_back(next_v);
                            }
                            Some(&(existing_dist, existing_hub)) => {
                                if next_dist < existing_dist
                                    || (next_dist == existing_dist && curr_hub < existing_hub)
                                {
                                    assignment.insert(next_v, (next_dist, curr_hub));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Group vertices into modules by hub
        let mut hub_to_vertices: HashMap<i32, Vec<i32>> = HashMap::new();
        for &h in &hubs {
            hub_to_vertices.insert(h, Vec::new());
        }

        for (&v, &(_, h)) in &assignment {
            hub_to_vertices.entry(h).or_default().push(v);
        }

        let mut modules = Vec::with_capacity(hubs.len());

        for &h in &hubs {
            let mut vertices = hub_to_vertices.remove(&h).unwrap_or_default();
            vertices.sort_unstable();

            let vert_set: HashSet<i32> = vertices.iter().copied().collect();

            // Identify interface ports: vertices in module that have at least one neighbor outside the module
            let mut interface_ports = Vec::new();
            for &u in &vertices {
                if let Some(neighbors) = g.adjacency_list.get(&u) {
                    if neighbors.iter().any(|w| !vert_set.contains(w)) {
                        interface_ports.push(u);
                    }
                }
            }
            interface_ports.sort_unstable();

            // Enumerate internal Hamiltonian paths between interface port pairs
            let mut internal_paths = Vec::new();

            if vertices.len() == 1 {
                let v = vertices[0];
                let ports = if interface_ports.is_empty() {
                    vec![v]
                } else {
                    interface_ports.clone()
                };
                for &p in &ports {
                    internal_paths.push((p, p, vec![v]));
                }
            } else {
                for i in 0..interface_ports.len() {
                    for j in 0..interface_ports.len() {
                        if i == j {
                            continue;
                        }
                        let u = interface_ports[i];
                        let v = interface_ports[j];
                        if let Some(path) = Self::solve_module_hamiltonian_path(g, &vertices, u, v) {
                            internal_paths.push((u, v, path));
                        }
                    }
                }
            }

            internal_paths.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));

            modules.push(HubModule {
                hub_id: h,
                vertices,
                interface_ports,
                internal_paths,
            });
        }

        modules
    }

    /// Solves for an exact Hamiltonian path spanning all vertices in `module_verts` from `start_u` to `end_v`.
    pub fn solve_module_hamiltonian_path(
        g: &Graph,
        module_verts: &[i32],
        start_u: i32,
        end_v: i32,
    ) -> Option<Vec<i32>> {
        let n = module_verts.len();
        if n == 0 {
            return None;
        }
        if n == 1 {
            if start_u == end_v && module_verts[0] == start_u {
                return Some(vec![start_u]);
            }
            return None;
        }
        if start_u == end_v {
            return None;
        }

        let vert_set: HashSet<i32> = module_verts.iter().copied().collect();
        if !vert_set.contains(&start_u) || !vert_set.contains(&end_v) {
            return None;
        }

        if n == 2 {
            if let Some(neighbors) = g.adjacency_list.get(&start_u) {
                if neighbors.contains(&end_v) {
                    return Some(vec![start_u, end_v]);
                }
            }
            return None;
        }

        // Exact SAT formulation with CaDiCaL and MTZ unary ordering
        let mut solver = CaDiCaL::default();
        let mut var_mgr = BasicVarManager::default();

        // Directed edge variables: (u, v) -> Lit
        let mut edge_vars: HashMap<(i32, i32), Lit> = HashMap::new();
        let mut out_edges: HashMap<i32, Vec<Lit>> = HashMap::new();
        let mut in_edges: HashMap<i32, Vec<Lit>> = HashMap::new();

        for &v in module_verts {
            out_edges.insert(v, Vec::new());
            in_edges.insert(v, Vec::new());
        }

        for &u in module_verts {
            if let Some(neighbors) = g.adjacency_list.get(&u) {
                for &v in neighbors {
                    if vert_set.contains(&v) && u != v {
                        let lit = var_mgr.new_var().pos_lit();
                        edge_vars.insert((u, v), lit);
                        out_edges.get_mut(&u).unwrap().push(lit);
                        in_edges.get_mut(&v).unwrap().push(lit);
                    }
                }
            }
        }

        // 1. Out-degree constraints:
        // For vertices except end_v: exactly 1 outgoing edge
        // For end_v: exactly 0 outgoing edges
        for &u in module_verts {
            let outs = &out_edges[&u];
            if u == end_v {
                for &lit in outs {
                    let _ = solver.add_clause(clause![!lit]);
                }
            } else {
                if outs.is_empty() {
                    return None;
                }
                let mut cl = Clause::new();
                cl.extend(outs.clone());
                let _ = solver.add_clause(cl);
                for i in 0..outs.len() {
                    for j in i + 1..outs.len() {
                        let _ = solver.add_clause(clause![!outs[i], !outs[j]]);
                    }
                }
            }
        }

        // 2. In-degree constraints:
        // For vertices except start_u: exactly 1 incoming edge
        // For start_u: exactly 0 incoming edges
        for &v in module_verts {
            let ins = &in_edges[&v];
            if v == start_u {
                for &lit in ins {
                    let _ = solver.add_clause(clause![!lit]);
                }
            } else {
                if ins.is_empty() {
                    return None;
                }
                let mut cl = Clause::new();
                cl.extend(ins.clone());
                let _ = solver.add_clause(cl);
                for i in 0..ins.len() {
                    for j in i + 1..ins.len() {
                        let _ = solver.add_clause(clause![!ins[i], !ins[j]]);
                    }
                }
            }
        }

        // 3. MTZ Unary Order Constraints for vertices in V \ {start_u}
        // Ladder order variables O_{v, 1} .. O_{v, n-1}
        let mut order_vars: HashMap<i32, Vec<Lit>> = HashMap::new();
        for &w in module_verts {
            if w != start_u {
                let mut o_w = Vec::with_capacity(n - 1);
                for _ in 0..(n - 1) {
                    let lit = var_mgr.new_var().pos_lit();
                    o_w.push(lit);
                }
                // Monotonicity: !O_{w, k+1} \/ O_{w, k}
                for s in 0..(n.saturating_sub(2)) {
                    let _ = solver.add_clause(clause![!o_w[s + 1], o_w[s]]);
                }
                order_vars.insert(w, o_w);
            }
        }

        // Transition implication clauses:
        for (&(a, b), &lit_ab) in &edge_vars {
            if a == start_u {
                if let Some(o_b) = order_vars.get(&b) {
                    let _ = solver.add_clause(clause![!lit_ab, o_b[0]]);
                }
            } else if let (Some(o_a), Some(o_b)) = (order_vars.get(&a), order_vars.get(&b)) {
                let _ = solver.add_clause(clause![!lit_ab, o_b[0]]);
                for s in 0..(n.saturating_sub(2)) {
                    let _ = solver.add_clause(clause![!lit_ab, !o_a[s], o_b[s + 1]]);
                }
                let max_s = n - 2;
                let _ = solver.add_clause(clause![!lit_ab, !o_a[max_s]]);
            }
        }

        // Enforce end_v at the final position n-1
        if let Some(o_end) = order_vars.get(&end_v) {
            let max_s = n - 2;
            let _ = solver.add_clause(clause![o_end[max_s]]);
        }

        match solver.solve() {
            Ok(SolverResult::Sat) => {
                let sol = solver.full_solution().unwrap();
                let mut succ_map: HashMap<i32, i32> = HashMap::new();
                for (&(u, v), &lit) in &edge_vars {
                    if sol.lit_value(lit) == TernaryVal::True {
                        succ_map.insert(u, v);
                    }
                }

                let mut path = Vec::with_capacity(n);
                let mut curr = start_u;
                path.push(curr);

                let mut visited = HashSet::new();
                visited.insert(curr);

                while curr != end_v {
                    if let Some(&nxt) = succ_map.get(&curr) {
                        if visited.insert(nxt) {
                            path.push(nxt);
                            curr = nxt;
                        } else {
                            break; // Cycle detected
                        }
                    } else {
                        break;
                    }
                }

                if path.len() == n && path.last() == Some(&end_v) {
                    Some(path)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Solves small graphs directly using CaDiCaL SAT with unary MTZ subcycle elimination.
    pub fn solve_direct_mtz(g: &Graph) -> Option<Vec<i32>> {
        let n = g.adjacency_list.len();
        if n < 3 {
            return None;
        }

        let mut vertices: Vec<i32> = g.adjacency_list.keys().copied().collect();
        vertices.sort_unstable();

        let root = vertices[0];
        let non_roots = &vertices[1..];
        let n_non_roots = non_roots.len();

        let mut solver = CaDiCaL::default();
        let mut var_mgr = BasicVarManager::default();

        let mut edge_vars: HashMap<(i32, i32), Lit> = HashMap::new();
        let mut out_edges: HashMap<i32, Vec<Lit>> = HashMap::new();
        let mut in_edges: HashMap<i32, Vec<Lit>> = HashMap::new();

        for &v in &vertices {
            out_edges.insert(v, Vec::new());
            in_edges.insert(v, Vec::new());
        }

        for &u in &vertices {
            if let Some(neighbors) = g.adjacency_list.get(&u) {
                for &v in neighbors {
                    if u != v {
                        let lit = var_mgr.new_var().pos_lit();
                        edge_vars.insert((u, v), lit);
                        out_edges.get_mut(&u).unwrap().push(lit);
                        in_edges.get_mut(&v).unwrap().push(lit);
                    }
                }
            }
        }

        // Degree-1 constraints
        for &u in &vertices {
            let outs = &out_edges[&u];
            let ins = &in_edges[&u];
            if outs.is_empty() || ins.is_empty() {
                return None;
            }

            let mut cl_out = Clause::new();
            cl_out.extend(outs.clone());
            let _ = solver.add_clause(cl_out);
            for i in 0..outs.len() {
                for j in i + 1..outs.len() {
                    let _ = solver.add_clause(clause![!outs[i], !outs[j]]);
                }
            }

            let mut cl_in = Clause::new();
            cl_in.extend(ins.clone());
            let _ = solver.add_clause(cl_in);
            for i in 0..ins.len() {
                for j in i + 1..ins.len() {
                    let _ = solver.add_clause(clause![!ins[i], !ins[j]]);
                }
            }
        }

        // 2-cycle prohibition
        for &u in &vertices {
            if let Some(neighbors) = g.adjacency_list.get(&u) {
                for &v in neighbors {
                    if u < v {
                        if let (Some(&lit_uv), Some(&lit_vu)) = (edge_vars.get(&(u, v)), edge_vars.get(&(v, u))) {
                            let _ = solver.add_clause(clause![!lit_uv, !lit_vu]);
                        }
                    }
                }
            }
        }

        // MTZ Unary order variables for non-roots
        let mut order_vars: HashMap<i32, Vec<Lit>> = HashMap::new();
        for &w in non_roots {
            let mut o_w = Vec::with_capacity(n_non_roots);
            for _ in 0..n_non_roots {
                let lit = var_mgr.new_var().pos_lit();
                o_w.push(lit);
            }
            for s in 0..(n_non_roots.saturating_sub(1)) {
                let _ = solver.add_clause(clause![!o_w[s + 1], o_w[s]]);
            }
            order_vars.insert(w, o_w);
        }

        for (&(a, b), &lit_ab) in &edge_vars {
            if a == root && b != root {
                if let Some(o_b) = order_vars.get(&b) {
                    let _ = solver.add_clause(clause![!lit_ab, o_b[0]]);
                }
            } else if a != root && b != root {
                if let (Some(o_a), Some(o_b)) = (order_vars.get(&a), order_vars.get(&b)) {
                    let _ = solver.add_clause(clause![!lit_ab, o_b[0]]);
                    for s in 0..(n_non_roots.saturating_sub(1)) {
                        let _ = solver.add_clause(clause![!lit_ab, !o_a[s], o_b[s + 1]]);
                    }
                    let max_s = n_non_roots - 1;
                    let _ = solver.add_clause(clause![!lit_ab, !o_a[max_s]]);
                }
            }
        }

        match solver.solve() {
            Ok(SolverResult::Sat) => {
                let sol = solver.full_solution().unwrap();
                let mut succ_map: HashMap<i32, i32> = HashMap::new();
                for (&(u, v), &lit) in &edge_vars {
                    if sol.lit_value(lit) == TernaryVal::True {
                        succ_map.insert(u, v);
                    }
                }

                let mut tour = Vec::with_capacity(n);
                let mut curr = root;
                let mut visited = HashSet::new();

                while visited.insert(curr) {
                    tour.push(curr);
                    if let Some(&nxt) = succ_map.get(&curr) {
                        curr = nxt;
                    } else {
                        break;
                    }
                }

                if tour.len() == n && TourVerifier::verify_raw_tour(&tour, g).is_ok() {
                    Some(tour)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Hierarchically solves the graph by extracting hub modules, contracting into a macro-graph,
    /// solving the macro-tour via CaDiCaL SAT with MTZ ordering, expanding the macro tour into
    /// verified internal paths, and certifying the complete Hamiltonian tour.
    pub fn try_solve_hierarchical(g: &Graph) -> Option<Vec<i32>> {
        let n = g.adjacency_list.len();
        if n < 3 {
            return None;
        }

        // Try extracting modules with standard min_hub_degree = 10, or fall back to lower degrees
        let candidate_min_degrees = [10, 8, 6, 5];
        for &min_deg in &candidate_min_degrees {
            let modules = Self::extract_hub_modules(g, min_deg);
            if modules.len() >= 2 && modules.len() <= 100 && modules.len() < n {
                // Check if modules partition all vertices and each module is non-trivial
                let total_mod_verts: usize = modules.iter().map(|m| m.vertices.len()).sum();
                if total_mod_verts == n && modules.iter().all(|m| m.vertices.len() >= 2) {
                    if let Some(tour) = Self::solve_macro_and_expand(g, &modules) {
                        if TourVerifier::verify_raw_tour(&tour, g).is_ok() {
                            return Some(tour);
                        }
                    }
                }
            }
        }

        None
    }

    /// Contracts modules into a macro-graph, solves the macro-tour with CaDiCaL SAT + MTZ ordering,
    /// and expands each macro-edge into its verified internal path.
    fn solve_macro_and_expand(g: &Graph, modules: &[HubModule]) -> Option<Vec<i32>> {
        let k = modules.len();
        if k < 2 {
            return None;
        }

        // Map vertex -> module index
        let mut vert_to_mod: HashMap<i32, usize> = HashMap::new();
        for (m_idx, m) in modules.iter().enumerate() {
            // Every module must have at least one internal path
            if m.internal_paths.is_empty() {
                return None;
            }
            for &v in &m.vertices {
                vert_to_mod.insert(v, m_idx);
            }
        }

        let mut solver = CaDiCaL::default();
        let mut var_mgr = BasicVarManager::default();

        // 1. Path Choice Variables: path_vars[m_idx][p_idx]
        let mut path_vars: Vec<Vec<Lit>> = Vec::with_capacity(k);
        for m in modules {
            let mut p_lits = Vec::with_capacity(m.internal_paths.len());
            for _ in 0..m.internal_paths.len() {
                p_lits.push(var_mgr.new_var().pos_lit());
            }
            // Exactly 1 path chosen per module
            let mut cl = Clause::new();
            cl.extend(p_lits.clone());
            let _ = solver.add_clause(cl);
            for i in 0..p_lits.len() {
                for j in i + 1..p_lits.len() {
                    let _ = solver.add_clause(clause![!p_lits[i], !p_lits[j]]);
                }
            }
            path_vars.push(p_lits);
        }

        // 2. Cross-Edge Variables: cross_vars[(u, v)] for u in M_a, v in M_b (a != b)
        let mut cross_vars: HashMap<(i32, i32), Lit> = HashMap::new();
        let mut mod_out_cross: Vec<Vec<Lit>> = vec![Vec::new(); k];
        let mut mod_in_cross: Vec<Vec<Lit>> = vec![Vec::new(); k];
        let mut vert_out_cross: HashMap<i32, Vec<Lit>> = HashMap::new();
        let mut vert_in_cross: HashMap<i32, Vec<Lit>> = HashMap::new();
        let mut mod_pair_cross: HashMap<(usize, usize), Vec<Lit>> = HashMap::new();

        for m_idx in 0..k {
            for &u in &modules[m_idx].interface_ports {
                vert_out_cross.entry(u).or_default();
                vert_in_cross.entry(u).or_default();

                if let Some(neighbors) = g.adjacency_list.get(&u) {
                    for &v in neighbors {
                        if let Some(&other_m_idx) = vert_to_mod.get(&v) {
                            if m_idx != other_m_idx {
                                let lit = var_mgr.new_var().pos_lit();
                                cross_vars.insert((u, v), lit);
                                mod_out_cross[m_idx].push(lit);
                                mod_in_cross[other_m_idx].push(lit);
                                vert_out_cross.entry(u).or_default().push(lit);
                                vert_in_cross.entry(v).or_default().push(lit);
                                mod_pair_cross.entry((m_idx, other_m_idx)).or_default().push(lit);
                            }
                        }
                    }
                }
            }
        }

        // Exactly 1 outgoing cross-edge and 1 incoming cross-edge per module
        for m_idx in 0..k {
            let outs = &mod_out_cross[m_idx];
            let ins = &mod_in_cross[m_idx];
            if outs.is_empty() || ins.is_empty() {
                return None;
            }

            let mut cl_out = Clause::new();
            cl_out.extend(outs.clone());
            let _ = solver.add_clause(cl_out);
            for i in 0..outs.len() {
                for j in i + 1..outs.len() {
                    let _ = solver.add_clause(clause![!outs[i], !outs[j]]);
                }
            }

            let mut cl_in = Clause::new();
            cl_in.extend(ins.clone());
            let _ = solver.add_clause(cl_in);
            for i in 0..ins.len() {
                for j in i + 1..ins.len() {
                    let _ = solver.add_clause(clause![!ins[i], !ins[j]]);
                }
            }
        }

        // 3. Link Path Choice to Cross-Edges
        for m_idx in 0..k {
            for (p_idx, &(entry, exit, _)) in modules[m_idx].internal_paths.iter().enumerate() {
                let p_lit = path_vars[m_idx][p_idx];

                // If path chosen, an outgoing cross edge from exit must be active
                let out_lits = vert_out_cross.get(&exit).cloned().unwrap_or_default();
                if out_lits.is_empty() {
                    let _ = solver.add_clause(clause![!p_lit]);
                } else {
                    let mut cl = Clause::new();
                    cl.extend(std::iter::once(!p_lit).chain(out_lits));
                    let _ = solver.add_clause(cl);
                }

                // If path chosen, an incoming cross edge to entry must be active
                let in_lits = vert_in_cross.get(&entry).cloned().unwrap_or_default();
                if in_lits.is_empty() {
                    let _ = solver.add_clause(clause![!p_lit]);
                } else {
                    let mut cl = Clause::new();
                    cl.extend(std::iter::once(!p_lit).chain(in_lits));
                    let _ = solver.add_clause(cl);
                }
            }

            // Conversely, if cross edge (u, v) is active, module of u must choose a path with exit == u
            for &(u, v) in cross_vars.keys() {
                if vert_to_mod[&u] == m_idx {
                    let lit_uv = cross_vars[&(u, v)];
                    let mut matching_paths = Vec::new();
                    for (p_idx, &(_, exit, _)) in modules[m_idx].internal_paths.iter().enumerate() {
                        if exit == u {
                            matching_paths.push(path_vars[m_idx][p_idx]);
                        }
                    }
                    if matching_paths.is_empty() {
                        let _ = solver.add_clause(clause![!lit_uv]);
                    } else {
                        let mut cl = Clause::new();
                        cl.extend(std::iter::once(!lit_uv).chain(matching_paths));
                        let _ = solver.add_clause(cl);
                    }
                }
                if vert_to_mod[&v] == m_idx {
                    let lit_uv = cross_vars[&(u, v)];
                    let mut matching_paths = Vec::new();
                    for (p_idx, &(entry, _, _)) in modules[m_idx].internal_paths.iter().enumerate() {
                        if entry == v {
                            matching_paths.push(path_vars[m_idx][p_idx]);
                        }
                    }
                    if matching_paths.is_empty() {
                        let _ = solver.add_clause(clause![!lit_uv]);
                    } else {
                        let mut cl = Clause::new();
                        cl.extend(std::iter::once(!lit_uv).chain(matching_paths));
                        let _ = solver.add_clause(cl);
                    }
                }
            }
        }

        // 4. Macro Transition Variables and MTZ ordering on modules
        let mut macro_vars: HashMap<(usize, usize), Lit> = HashMap::new();
        for (&(a, b), lits) in &mod_pair_cross {
            let m_lit = var_mgr.new_var().pos_lit();
            macro_vars.insert((a, b), m_lit);

            for &lit in lits {
                let _ = solver.add_clause(clause![!lit, m_lit]);
            }
            let mut cl = Clause::new();
            cl.extend(std::iter::once(!m_lit).chain(lits.clone()));
            let _ = solver.add_clause(cl);
        }

        // Unary MTZ ladder variables on non-root modules (1..k)
        let root = 0;
        let num_non_roots = k - 1;
        let mut order_vars: HashMap<usize, Vec<Lit>> = HashMap::new();

        for m_idx in 1..k {
            let mut o_m = Vec::with_capacity(num_non_roots);
            for _ in 0..num_non_roots {
                let lit = var_mgr.new_var().pos_lit();
                o_m.push(lit);
            }
            for s in 0..(num_non_roots.saturating_sub(1)) {
                let _ = solver.add_clause(clause![!o_m[s + 1], o_m[s]]);
            }
            order_vars.insert(m_idx, o_m);
        }

        for (&(a, b), &m_lit) in &macro_vars {
            if a == root && b != root {
                if let Some(o_b) = order_vars.get(&b) {
                    let _ = solver.add_clause(clause![!m_lit, o_b[0]]);
                }
            } else if a != root && b != root {
                if let (Some(o_a), Some(o_b)) = (order_vars.get(&a), order_vars.get(&b)) {
                    let _ = solver.add_clause(clause![!m_lit, o_b[0]]);
                    for s in 0..(num_non_roots.saturating_sub(1)) {
                        let _ = solver.add_clause(clause![!m_lit, !o_a[s], o_b[s + 1]]);
                    }
                    let max_s = num_non_roots - 1;
                    let _ = solver.add_clause(clause![!m_lit, !o_a[max_s]]);
                }
            }
        }

        if k > 2 {
            for a in 0..k {
                for b in (a + 1)..k {
                    if let (Some(&m_ab), Some(&m_ba)) = (macro_vars.get(&(a, b)), macro_vars.get(&(b, a))) {
                        let _ = solver.add_clause(clause![!m_ab, !m_ba]);
                    }
                }
            }
        }

        match solver.solve() {
            Ok(SolverResult::Sat) => {
                let sol = solver.full_solution().unwrap();

                // Extract selected internal path for each module
                let mut selected_paths: Vec<Vec<i32>> = vec![Vec::new(); k];
                for m_idx in 0..k {
                    for (p_idx, path_lit) in path_vars[m_idx].iter().enumerate() {
                        if sol.lit_value(*path_lit) == TernaryVal::True {
                            selected_paths[m_idx] = modules[m_idx].internal_paths[p_idx].2.clone();
                            break;
                        }
                    }
                }

                // Extract macro succession
                let mut macro_succ: HashMap<usize, usize> = HashMap::new();
                for (&(a, b), &m_lit) in &macro_vars {
                    if sol.lit_value(m_lit) == TernaryVal::True {
                        macro_succ.insert(a, b);
                    }
                }

                // Assemble the full tour starting from module 0
                let mut tour = Vec::new();
                let mut curr_mod = root;
                let mut visited_mods = HashSet::new();

                while visited_mods.insert(curr_mod) {
                    let path = &selected_paths[curr_mod];
                    tour.extend_from_slice(path);
                    if let Some(&nxt_mod) = macro_succ.get(&curr_mod) {
                        curr_mod = nxt_mod;
                    } else {
                        break;
                    }
                }

                if visited_mods.len() == k && tour.len() == g.adjacency_list.len() {
                    Some(tour)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
