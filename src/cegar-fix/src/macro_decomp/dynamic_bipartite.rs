use crate::core::graph::Graph;
use crate::core::tour_verifier::TourVerifier;
use rayon::prelude::*;
use rustsat::clause;
use rustsat::instances::{BasicVarManager, ManageVars};
use rustsat::solvers::{Solve, SolverResult};
use rustsat::types::{Clause, Lit};
use rustsat_cadical::CaDiCaL;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct BipartitePartition {
    pub super_hubs: Vec<i32>,
    pub clusters: HashMap<i32, HashSet<i32>>,
    pub owner: HashMap<i32, i32>,
    pub boundary_ports: HashMap<i32, Vec<i32>>,
    pub connectors: Vec<i32>,
    pub macro_edges: Vec<(i32, i32)>,
}

pub fn can_solve_bipartite(raw_g: &Graph) -> bool {
    detect_and_partition(raw_g).is_some()
}

// === PERFORMANCE KNOBS ===
// These thresholds control the sensitivity of the dense bipartite hub
// recognizer. They do NOT affect correctness — changing them only
// changes which graphs are attempted by this decomposition vs.
// falling through to the monolithic fallback.

/// Minimum maximum-degree for a graph to have "super-hub" structure.
const MIN_MAX_DEGREE: usize = 30;

/// Super-hubs must have degree >= this fraction of the maximum degree.
const HUB_DEGREE_FRACTION: f64 = 0.7;

/// Maximum number of super-hubs as a fraction of N.
const MAX_HUB_FRACTION: f64 = 0.02;

/// Minimum graph size for hub-spoke decomposition to be meaningful.
const MIN_GRAPH_SIZE: usize = 50;

pub fn detect_and_partition(raw_g: &Graph) -> Option<BipartitePartition> {
    let n = raw_g.adjacency_list.len();
    if n < MIN_GRAPH_SIZE {
        return None;
    }

    let max_deg = raw_g.adjacency_list.values().map(|nbrs| nbrs.len()).max().unwrap_or(0);
    // Topological outlier condition: Super-hubs must have degree >= 30 and >= 2% of total vertices N
    if max_deg < MIN_MAX_DEGREE || max_deg < ((n as f64) * MAX_HUB_FRACTION) as usize {
        return None;
    }

    // Identify super-hubs: vertices having degree >= 70% of maximum degree
    let min_hub_deg = ((max_deg as f64) * HUB_DEGREE_FRACTION) as usize;
    let mut super_hubs: Vec<i32> = raw_g
        .adjacency_list
        .iter()
        .filter(|&(_, nbrs)| nbrs.len() >= min_hub_deg)
        .map(|(&u, _)| u)
        .collect();
    super_hubs.sort_unstable();

    let k = super_hubs.len();
    if k < 2 || k > ((n as f64) * MAX_HUB_FRACTION) as usize {
        return None;
    }

    let sh_set: HashSet<i32> = super_hubs.iter().copied().collect();

    // 1. Identify low-degree vertices (degree <= 4, not in super_hubs)
    let low_deg: HashSet<i32> = raw_g
        .adjacency_list
        .iter()
        .filter(|&(&u, nbrs)| nbrs.len() <= 4 && !sh_set.contains(&u))
        .map(|(&u, _)| u)
        .collect();

    // 2. Identify seed connectors: low-degree vertices that connect to >= 2 super-hubs,
    // or 0 super-hubs, or have >= 2 low-degree neighbors
    let mut seeds = HashSet::new();
    for &u in &low_deg {
        if let Some(nbrs) = raw_g.adjacency_list.get(&u) {
            let sh_c = nbrs.iter().filter(|v| sh_set.contains(v)).count();
            let ld_nbrs = nbrs.iter().filter(|v| low_deg.contains(v)).count();
            if sh_c >= 2 || sh_c == 0 || ld_nbrs >= 2 {
                seeds.insert(u);
            }
        }
    }

    // Expand seeds along low-degree edges
    let mut connectors_set = seeds.clone();
    for &u in &seeds {
        if let Some(nbrs) = raw_g.adjacency_list.get(&u) {
            for &v in nbrs {
                if low_deg.contains(&v) {
                    connectors_set.insert(v);
                }
            }
        }
    }

    let mut corridor: HashSet<i32> = sh_set.clone();
    corridor.extend(&connectors_set);

    let mut connectors: Vec<i32> = connectors_set.into_iter().collect();
    connectors.sort_unstable();

    // 3. Bulks (clusters): Partition V \ corridor among super_hubs
    let mut clusters: HashMap<i32, HashSet<i32>> = HashMap::new();
    let mut owner: HashMap<i32, i32> = HashMap::new();
    let mut unassigned: Vec<i32> = Vec::new();

    for (&u, nbrs) in &raw_g.adjacency_list {
        if corridor.contains(&u) {
            continue;
        }
        let sh_nbrs: Vec<i32> = nbrs.iter().filter(|v| sh_set.contains(v)).copied().collect();
        if sh_nbrs.len() == 1 {
            let h = sh_nbrs[0];
            clusters.entry(h).or_default().insert(u);
            owner.insert(u, h);
        } else {
            unassigned.push(u);
        }
    }

    // Assign unassigned vertices to the cluster with the maximum neighbor count
    for u in unassigned {
        if let Some(nbrs) = raw_g.adjacency_list.get(&u) {
            let mut counts: HashMap<i32, usize> = HashMap::new();
            for &v in nbrs {
                if let Some(&h) = owner.get(&v) {
                    *counts.entry(h).or_default() += 1;
                }
            }
            if let Some((&best_h, _)) = counts.iter().max_by_key(|&(_, c)| *c) {
                clusters.entry(best_h).or_default().insert(u);
                owner.insert(u, best_h);
            }
        }
    }

    // 4. Extract boundary ports per cluster
    let mut boundary_ports: HashMap<i32, Vec<i32>> = HashMap::new();
    let mut all_ports: HashSet<i32> = HashSet::new();

    for (&h, c_nodes) in &clusters {
        let mut ports = Vec::new();
        for &u in c_nodes {
            if let Some(nbrs) = raw_g.adjacency_list.get(&u) {
                let has_ext = nbrs.iter().any(|&v| v != h && (corridor.contains(&v) || owner.get(&v) != Some(&h)));
                if has_ext {
                    ports.push(u);
                    all_ports.insert(u);
                }
            }
        }
        ports.sort_unstable();
        boundary_ports.insert(h, ports);
    }

    // 5. Macro edges: all edges within the corridor and between corridor and boundary ports, and between boundary ports
    let mut all_macro_nodes: HashSet<i32> = corridor.clone();
    all_macro_nodes.extend(&all_ports);

    let mut macro_edges = Vec::new();
    for &u in &all_macro_nodes {
        if let Some(nbrs) = raw_g.adjacency_list.get(&u) {
            for &v in nbrs {
                if u < v && all_macro_nodes.contains(&v) {
                    let ou = owner.get(&u);
                    let ov = owner.get(&v);
                    if ou != ov || ou.is_none() || ov.is_none() {
                        macro_edges.push((u, v));
                    }
                }
            }
        }
    }
    macro_edges.sort_unstable();

    // Soundness validation:
    // 1. MATHEMATICAL REQUIREMENT: A Hamiltonian cycle must enter and exit each
    // hub cluster, requiring >= 2 boundary ports per hub.
    for &h in &super_hubs {
        let ports = match boundary_ports.get(&h) {
            Some(p) => p,
            None => return None,
        };
        if ports.len() < 2 {
            return None;
        }
    }

    // 2. MATHEMATICAL REQUIREMENT: Every vertex must belong to exactly one
    // partition element. Missing vertices would create an incomplete tour.
    let total_partitioned: usize = clusters.values().map(|c| c.len()).sum::<usize>() + corridor.len();
    if total_partitioned != n {
        return None;
    }

    Some(BipartitePartition {
        super_hubs,
        clusters,
        owner,
        boundary_ports,
        connectors,
        macro_edges,
    })
}

#[derive(Debug, Clone)]
pub struct MacroConfiguration {
    pub cluster_ports: HashMap<i32, (i32, i32)>,
    pub active_macro_edges: Vec<(i32, i32)>,
}

pub struct MacroSatSolver {
    solver: CaDiCaL<'static, 'static>,
    #[allow(dead_code)]
    var_mgr: BasicVarManager,
    edge_vars: HashMap<(i32, i32), Lit>,
    pair_vars: HashMap<i32, HashMap<(i32, i32), Lit>>,
    pub partition: BipartitePartition,
}

impl MacroSatSolver {
    pub fn partition(&self) -> &BipartitePartition {
        &self.partition
    }

    pub fn new(partition: &BipartitePartition, raw_g: &Graph) -> Result<Self, String> {
        Self::new_with_deadline(partition, raw_g, Instant::now() + std::time::Duration::from_secs(3600))
    }

    pub fn new_with_deadline(partition: &BipartitePartition, raw_g: &Graph, deadline: Instant) -> Result<Self, String> {
        let mut solver = crate::core::solver_utils::create_solver_with_deadline(deadline);
        let mut var_mgr = BasicVarManager::default();
        let mut edge_vars = HashMap::new();

        for &e in &partition.macro_edges {
            let lit = var_mgr.new_var().pos_lit();
            edge_vars.insert(e, lit);
        }

        let mut pair_vars: HashMap<i32, HashMap<(i32, i32), Lit>> = HashMap::new();

        // 1. For each cluster, define pair variables and enforce exactly 1 chosen pair
        for (&h, ports) in &partition.boundary_ports {
            let mut p_map = HashMap::new();
            let mut p_lits = Vec::new();
            for i in 0..ports.len() {
                for j in (i + 1)..ports.len() {
                    let u = ports[i].min(ports[j]);
                    let v = ports[i].max(ports[j]);
                    let lit = var_mgr.new_var().pos_lit();
                    p_map.insert((u, v), lit);
                    p_lits.push(lit);
                }
            }
            if p_lits.is_empty() {
                return Err(format!("Cluster {} has < 2 boundary ports", h));
            }

            // At-least-1 pair
            let _ = solver.add_clause(Clause::from_iter(p_lits.iter().copied()));
            // At-most-1 pair
            for i in 0..p_lits.len() {
                for j in (i + 1)..p_lits.len() {
                    let _ = solver.add_clause(clause![!p_lits[i], !p_lits[j]]);
                }
            }
            pair_vars.insert(h, p_map);
        }

        // Degree-1 mandatory endpoint constraint:
        // Any vertex in cluster h that has internal degree <= 1 MUST be an endpoint
        // of any internal Hamiltonian path. Therefore, any chosen pair MUST include it!
        for (&h, p_map) in &pair_vars {
            let c_nodes = &partition.clusters[&h];
            for &u in c_nodes {
                let internal_deg = raw_g.adjacency_list.get(&u)
                    .map(|nbrs| nbrs.iter().filter(|v| c_nodes.contains(v)).count())
                    .unwrap_or(0);
                if internal_deg <= 1 {
                    for (&(x, y), &p_lit) in p_map {
                        if x != u && y != u {
                            let _ = solver.add_clause(clause![!p_lit]);
                        }
                    }
                }
            }
        }

        // 2. Incident edge degree consistency for boundary ports:
        let sh_set: HashSet<i32> = partition.super_hubs.iter().copied().collect();
        let mut incident_macro_edges: HashMap<i32, Vec<Lit>> = HashMap::new();
        for (&(u, v), &lit) in &edge_vars {
            incident_macro_edges.entry(u).or_default().push(lit);
            incident_macro_edges.entry(v).or_default().push(lit);
        }

        for (&h, ports) in &partition.boundary_ports {
            let p_map = &pair_vars[&h];
            for &u in ports {
                let ext_edges = incident_macro_edges.get(&u).cloned().unwrap_or_default();
                // Whether u is an endpoint in the chosen pair:
                // u_is_endpoint <=> OR_{v in ports \ {u}} P_{min(u,v), max(u,v)}
                let mut endpoint_lits = Vec::new();
                for &v in ports {
                    if u != v {
                        let key = (u.min(v), u.max(v));
                        if let Some(&p_lit) = p_map.get(&key) {
                            endpoint_lits.push(p_lit);
                        }
                    }
                }

                if sh_set.contains(&u) {
                    // Super-hub port: can either be an endpoint (deg 1) or a bridge (deg 2) or unused (deg 0)
                    let b_var = var_mgr.new_var().pos_lit();

                    // If u is a bridge, it cannot be an endpoint in cluster h
                    for &p_lit in &endpoint_lits {
                        let _ = solver.add_clause(clause![!b_var, !p_lit]);
                    }

                    // Degree <= 2 always: at most 2 external edges
                    crate::core::encoder::add_at_most_2(&mut solver, &mut var_mgr, &ext_edges);

                    // Any active edge implies b_var \/ OR(endpoint_lits)
                    for &e_lit in &ext_edges {
                        let mut cl = Vec::with_capacity(endpoint_lits.len() + 2);
                        cl.push(!e_lit);
                        cl.push(b_var);
                        cl.extend(&endpoint_lits);
                        let _ = solver.add_clause(Clause::from_iter(cl));
                    }

                    // If b_var is True, degree >= 2
                    if ext_edges.len() < 2 {
                        let _ = solver.add_clause(clause![!b_var]);
                    } else {
                        for i in 0..ext_edges.len() {
                            let mut cl = Vec::with_capacity(ext_edges.len());
                            cl.push(!b_var);
                            for j in 0..ext_edges.len() {
                                if i != j {
                                    cl.push(ext_edges[j]);
                                }
                            }
                            let _ = solver.add_clause(Clause::from_iter(cl));
                        }
                    }

                    // If u is an endpoint, degree >= 1
                    if !ext_edges.is_empty() {
                        for &p_lit in &endpoint_lits {
                            let mut cl = Vec::with_capacity(ext_edges.len() + 1);
                            cl.push(!p_lit);
                            cl.extend(&ext_edges);
                            let _ = solver.add_clause(Clause::from_iter(cl));
                        }
                    } else if !endpoint_lits.is_empty() {
                        for &p_lit in &endpoint_lits {
                            let _ = solver.add_clause(clause![!p_lit]);
                        }
                    }

                    // If u is an endpoint, degree <= 1: at most 1 external edge
                    for &p_lit in &endpoint_lits {
                        for i in 0..ext_edges.len() {
                            for j in (i + 1)..ext_edges.len() {
                                let _ = solver.add_clause(clause![!p_lit, !ext_edges[i], !ext_edges[j]]);
                            }
                        }
                    }
                } else {
                    // Non-super-hub port: degree 1 if endpoint, degree 0 if not

                    // If u is NOT an endpoint, ext_edges degree must be 0
                    // e_lit implies u_is_endpoint (OR of endpoint_lits)
                    for &e_lit in &ext_edges {
                        let mut cl = Vec::with_capacity(endpoint_lits.len() + 1);
                        cl.push(!e_lit);
                        cl.extend(&endpoint_lits);
                        let _ = solver.add_clause(Clause::from_iter(cl));
                    }

                    // If u is an endpoint, ext_edges degree must be >= 1
                    if !ext_edges.is_empty() {
                        for &p_lit in &endpoint_lits {
                            let mut cl = Vec::with_capacity(ext_edges.len() + 1);
                            cl.push(!p_lit);
                            cl.extend(&ext_edges);
                            let _ = solver.add_clause(Clause::from_iter(cl));
                        }
                    } else if !endpoint_lits.is_empty() {
                        // Port has no external edges, cannot be an endpoint
                        for &p_lit in &endpoint_lits {
                            let _ = solver.add_clause(clause![!p_lit]);
                        }
                    }

                    // At-most-1 external edge for port
                    for i in 0..ext_edges.len() {
                        for j in (i + 1)..ext_edges.len() {
                            let _ = solver.add_clause(clause![!ext_edges[i], !ext_edges[j]]);
                        }
                    }
                }
            }
        }

        // 3. Corridor nodes (connectors + super_hubs) degree constraint: exactly 2 incident edges
        let mut corridor_nodes: HashSet<i32> = partition.connectors.iter().copied().collect();
        corridor_nodes.extend(&partition.super_hubs);

        for &c in &corridor_nodes {
            let c_edges = incident_macro_edges.get(&c).cloned().unwrap_or_default();
            if c_edges.len() < 2 {
                return Err(format!("Corridor node {} has degree < 2 in macro edges", c));
            }
            crate::core::encoder::add_at_most_2(&mut solver, &mut var_mgr, &c_edges);
            // At least 2:
            // For any edge, if it is not selected, the remaining must have at least 1
            for i in 0..c_edges.len() {
                let mut cl = Vec::new();
                for j in 0..c_edges.len() {
                    if i != j {
                        cl.push(c_edges[j]);
                    }
                }
                let _ = solver.add_clause(Clause::from_iter(cl));
            }
        }

        Ok(Self {
            solver,
            var_mgr,
            edge_vars,
            pair_vars,
            partition: partition.clone(),
        })
    }

    pub fn block_pair(&mut self, cluster_hub: i32, u_in: i32, u_out: i32) {
        let key = (u_in.min(u_out), u_in.max(u_out));
        if let Some(p_map) = self.pair_vars.get(&cluster_hub) {
            if let Some(&lit) = p_map.get(&key) {
                let _ = self.solver.add_clause(clause![!lit]);
            }
        }
    }

    pub fn solve_next_configuration(&mut self) -> Option<MacroConfiguration> {
        let max_subtour_iters = 50;
        for _ in 0..max_subtour_iters {
            match self.solver.solve() {
                Ok(SolverResult::Sat) => {}
                _ => return None,
            }

            let sol = self.solver.full_solution().ok()?;
            let mut active_edges = Vec::new();
            for (&e, &lit) in &self.edge_vars {
                if sol.lit_value(lit) == rustsat::types::TernaryVal::True {
                    active_edges.push(e);
                }
            }

            let mut cluster_ports = HashMap::new();
            for (&h, p_map) in &self.pair_vars {
                for (&(u, v), &lit) in p_map {
                    if sol.lit_value(lit) == rustsat::types::TernaryVal::True {
                        cluster_ports.insert(h, (u, v));
                        break;
                    }
                }
            }

            // Check connectivity / subtour elimination
            // Build macro adjacency: active_edges + internal virtual edges (u_in, u_out)
            let mut macro_graph: HashMap<i32, Vec<i32>> = HashMap::new();
            for &(u, v) in &active_edges {
                macro_graph.entry(u).or_default().push(v);
                macro_graph.entry(v).or_default().push(u);
            }
            for &(u, v) in cluster_ports.values() {
                macro_graph.entry(u).or_default().push(v);
                macro_graph.entry(v).or_default().push(u);
            }

            // Find connected components in macro_graph
            let mut visited = HashSet::new();
            let mut comps = Vec::new();
            let all_active_nodes: Vec<i32> = macro_graph.keys().copied().collect();

            for &start in &all_active_nodes {
                if !visited.contains(&start) {
                    let mut comp = Vec::new();
                    let mut q = std::collections::VecDeque::new();
                    visited.insert(start);
                    q.push_back(start);
                    while let Some(curr) = q.pop_front() {
                        comp.push(curr);
                        if let Some(nbrs) = macro_graph.get(&curr) {
                            for &nxt in nbrs {
                                if visited.insert(nxt) {
                                    q.push_back(nxt);
                                }
                            }
                        }
                    }
                    comps.push(comp);
                }
            }

            if comps.len() == 1 {
                return Some(MacroConfiguration {
                    cluster_ports,
                    active_macro_edges: active_edges,
                });
            }

            // Subtour elimination:
            // 1. Sound DFJ Cut-Crossing:
            // The macro cycle must connect all clusters. If a component visits a strict subset
            // of clusters, any valid connected cycle must cross the cut between these clusters and the rest.
            // All boundary ports of clusters in the component form the cut set.
            for comp in &comps {
                let comp_set: HashSet<i32> = comp.iter().copied().collect();
                let mut comp_clusters = HashSet::new();
                for &u in comp {
                    if let Some(&h) = self.partition.owner.get(&u) {
                        comp_clusters.insert(h);
                    }
                }

                if !comp_clusters.is_empty() && comp_clusters.len() < self.partition.super_hubs.len() {
                    let mut cut_nodes: HashSet<i32> = HashSet::new();
                    for &h in &comp_clusters {
                        if let Some(ports) = self.partition.boundary_ports.get(&h) {
                            cut_nodes.extend(ports);
                        }
                    }
                    for &u in comp {
                        if self.partition.connectors.contains(&u) {
                            cut_nodes.insert(u);
                        }
                    }

                    let mut cut_lits = Vec::new();
                    for &(u, v) in &self.partition.macro_edges {
                        if cut_nodes.contains(&u) != cut_nodes.contains(&v) {
                            cut_lits.push(self.edge_vars[&(u, v)]);
                        }
                    }
                    if !cut_lits.is_empty() {
                        let _ = self.solver.add_clause(Clause::from_iter(cut_lits));
                    }
                }

                // 2. Subtour cycle nogood:
                // Ban the exact combination of active edges and cluster internal pairs that formed this isolated cycle
                let mut cycle_nogood: Vec<Lit> = Vec::new();
                for &(u, v) in &active_edges {
                    if comp_set.contains(&u) && comp_set.contains(&v) {
                        cycle_nogood.push(!self.edge_vars[&(u, v)]);
                    }
                }
                for (&h, &(u, v)) in &cluster_ports {
                    if comp_set.contains(&u) && comp_set.contains(&v) {
                        let key = (u.min(v), u.max(v));
                        if let Some(p_map) = self.pair_vars.get(&h) {
                            if let Some(&lit) = p_map.get(&key) {
                                cycle_nogood.push(!lit);
                            }
                        }
                    }
                }
                if cycle_nogood.len() >= 2 {
                    let _ = self.solver.add_clause(Clause::from_iter(cycle_nogood));
                }
            }
        }
        None
    }
}

pub fn solve_cluster_path(
    _cluster_id: i32,
    u_in: i32,
    u_out: i32,
    cluster_nodes: &HashSet<i32>,
    adj: &HashMap<i32, Vec<i32>>,
    deadline: Instant,
) -> Option<Vec<i32>> {
    let m = cluster_nodes.len();
    if m == 1 {
        if u_in == u_out && cluster_nodes.contains(&u_in) {
            return Some(vec![u_in]);
        } else {
            return None;
        }
    }

    if u_in == u_out || !cluster_nodes.contains(&u_in) || !cluster_nodes.contains(&u_out) {
        return None;
    }

    // Dense indexing: 0..m
    let mut nodes_vec: Vec<i32> = cluster_nodes.iter().copied().collect();
    nodes_vec.sort_unstable();
    let mut node_to_idx: HashMap<i32, usize> = HashMap::with_capacity(m);
    for (i, &u) in nodes_vec.iter().enumerate() {
        node_to_idx.insert(u, i);
    }

    let u_in_idx = node_to_idx[&u_in];
    let u_out_idx = node_to_idx[&u_out];

    let mut edges: Vec<(usize, usize)> = Vec::new();
    let mut g_c: Vec<Vec<usize>> = vec![Vec::new(); m];

    for (u_idx, &u) in nodes_vec.iter().enumerate() {
        if let Some(nbrs) = adj.get(&u) {
            for &v in nbrs {
                if let Some(&v_idx) = node_to_idx.get(&v) {
                    g_c[u_idx].push(v_idx);
                    if u_idx < v_idx {
                        edges.push((u_idx, v_idx));
                    }
                }
            }
        }
    }
    edges.sort_unstable();

    let mut solver = crate::core::solver_utils::create_solver_with_deadline(deadline);
    let mut var_mgr = BasicVarManager::default();
    let mut inc_edges: Vec<Vec<(usize, Lit)>> = vec![Vec::new(); m];
    let mut edge_lits: Vec<Lit> = Vec::with_capacity(edges.len());

    for &(u_idx, v_idx) in &edges {
        let lit = var_mgr.new_var().pos_lit();
        edge_lits.push(lit);
        inc_edges[u_idx].push((v_idx, lit));
        inc_edges[v_idx].push((u_idx, lit));
    }

    for u_idx in 0..m {
        let lits: Vec<Lit> = inc_edges[u_idx].iter().map(|&(_, lit)| lit).collect();
        let target = if u_idx == u_in_idx || u_idx == u_out_idx { 1 } else { 2 };
        if lits.len() < target {
            return None;
        }

        if target == 1 {
            let _ = solver.add_clause(Clause::from_iter(lits.iter().copied()));
            for i in 0..lits.len() {
                for j in (i + 1)..lits.len() {
                    let _ = solver.add_clause(clause![!lits[i], !lits[j]]);
                }
            }
        } else {
            if lits.len() == 2 {
                let _ = solver.add_clause(clause![lits[0]]);
                let _ = solver.add_clause(clause![lits[1]]);
            } else {
                let _ = solver.add_clause(Clause::from_iter(lits.iter().copied()));
                for i in 0..lits.len() {
                    let mut cl = Vec::with_capacity(lits.len() - 1);
                    for j in 0..lits.len() {
                        if i != j {
                            cl.push(lits[j]);
                        }
                    }
                    let _ = solver.add_clause(Clause::from_iter(cl));
                }
                crate::core::encoder::add_at_most_2(&mut solver, &mut var_mgr, &lits);
            }
        }
    }

    let mut adj_matrix = vec![false; m * m];
    for (u_idx, nbrs) in g_c.iter().enumerate() {
        for &v_idx in nbrs {
            adj_matrix[u_idx * m + v_idx] = true;
        }
    }

    let max_it = 300;
    let mut in_cyc = vec![false; m];
    let mut vis = vec![false; m];

    for it in 0..max_it {
        if Instant::now() >= deadline {
            eprintln!("[cluster {}] hit deadline at iter {}", _cluster_id, it);
            return None;
        }

        match solver.solve() {
            Ok(SolverResult::Sat) => {}
            _ => return None,
        }

        let sol = solver.full_solution().ok()?;
        let mut active_adj: Vec<Vec<usize>> = vec![Vec::with_capacity(2); m];
        for (e_idx, &(u_idx, v_idx)) in edges.iter().enumerate() {
            let lit = edge_lits[e_idx];
            if sol.lit_value(lit) == rustsat::types::TernaryVal::True {
                active_adj[u_idx].push(v_idx);
                active_adj[v_idx].push(u_idx);
            }
        }

        vis.fill(false);
        let mut path: Vec<usize> = Vec::with_capacity(m);
        path.push(u_in_idx);
        vis[u_in_idx] = true;
        let mut curr = u_in_idx;
        let mut prev: Option<usize> = None;

        while curr != u_out_idx {
            let nxt = active_adj[curr].iter().copied().find(|&w| Some(w) != prev);
            match nxt {
                Some(w) => {
                    path.push(w);
                    vis[w] = true;
                    prev = Some(curr);
                    curr = w;
                }
                None => break,
            }
        }

        let mut cycles: Vec<Vec<usize>> = Vec::new();
        for u in 0..m {
            if !vis[u] {
                let mut cyc = Vec::new();
                let mut curr_c = u;
                let mut prev_c: Option<usize> = None;
                while !vis[curr_c] {
                    vis[curr_c] = true;
                    cyc.push(curr_c);
                    let nxt = active_adj[curr_c].iter().copied().find(|&w| Some(w) != prev_c);
                    match nxt {
                        Some(w) => {
                            prev_c = Some(curr_c);
                            curr_c = w;
                        }
                        None => break,
                    }
                }
                if cyc.len() >= 3 {
                    cycles.push(cyc);
                }
            }
        }

        if it % 20 == 0 || cycles.len() <= 3 {
            println!(
                "[cluster {}] iter {}: path.len={}, curr==u_out={}, cycles.len={}",
                _cluster_id, it, path.len(), curr == u_out_idx, cycles.len()
            );
        }

        if cycles.is_empty() && path.len() == m && curr == u_out_idx {
            let orig_path: Vec<i32> = path.into_iter().map(|idx| nodes_vec[idx]).collect();
            return Some(orig_path);
        }

        // Fast 2-opt cycle merge until fixpoint when <= 16 cycles remain
        if cycles.len() <= 16 && curr == u_out_idx {
            let mut merged_path = path.clone();
            let mut unmerged_cycles = cycles.clone();
            let mut progress = true;

            while progress && !unmerged_cycles.is_empty() {
                progress = false;
                let mut remaining = Vec::with_capacity(unmerged_cycles.len());

                for cyc in unmerged_cycles {
                    let n_p = merged_path.len();
                    let k = cyc.len();
                    let mut merged = false;

                    for i in 0..(n_p - 1) {
                        let pu = merged_path[i];
                        let pv = merged_path[i + 1];

                        for j in 0..k {
                            let cu = cyc[j];
                            let cv = cyc[(j + 1) % k];

                            if adj_matrix[pu * m + cu] && adj_matrix[cv * m + pv] {
                                let mut new_p = Vec::with_capacity(n_p + k);
                                new_p.extend_from_slice(&merged_path[..=i]);
                                for step in 0..k {
                                    let idx = (j + k - (step % k)) % k;
                                    new_p.push(cyc[idx]);
                                }
                                new_p.extend_from_slice(&merged_path[(i + 1)..]);
                                merged_path = new_p;
                                merged = true;
                                progress = true;
                                break;
                            }
                            if adj_matrix[pu * m + cv] && adj_matrix[cu * m + pv] {
                                let mut new_p = Vec::with_capacity(n_p + k);
                                new_p.extend_from_slice(&merged_path[..=i]);
                                for step in 0..k {
                                    let idx = (j + 1 + step) % k;
                                    new_p.push(cyc[idx]);
                                }
                                new_p.extend_from_slice(&merged_path[(i + 1)..]);
                                merged_path = new_p;
                                merged = true;
                                progress = true;
                                break;
                            }
                        }

                        if merged {
                            break;
                        }
                    }

                    if !merged {
                        remaining.push(cyc);
                    }
                }

                unmerged_cycles = remaining;
            }

            if unmerged_cycles.is_empty() && merged_path.len() == m {
                let orig_path: Vec<i32> = merged_path.into_iter().map(|idx| nodes_vec[idx]).collect();
                return Some(orig_path);
            }
        }

        for cyc in &cycles {
            for &u in cyc {
                in_cyc[u] = true;
            }

            let mut cut_lits = Vec::new();
            for &u in cyc {
                for &(v, lit) in &inc_edges[u] {
                    if !in_cyc[v] {
                        cut_lits.push(lit);
                    }
                }
            }

            for &u in cyc {
                in_cyc[u] = false;
            }

            if !cut_lits.is_empty() {
                let _ = solver.add_clause(Clause::from_iter(cut_lits.iter().copied()));
                if cut_lits.len() <= 10 {
                    for idx_e in 0..cut_lits.len() {
                        let mut cl = Vec::with_capacity(cut_lits.len());
                        cl.push(!cut_lits[idx_e]);
                        for j in 0..cut_lits.len() {
                            if j != idx_e {
                                cl.push(cut_lits[j]);
                            }
                        }
                        let _ = solver.add_clause(Clause::from_iter(cl));
                    }
                }
            }

            let mut neg_clause = Vec::with_capacity(cyc.len());
            for k in 0..cyc.len() {
                let u = cyc[k];
                let v = cyc[(k + 1) % cyc.len()];
                if let Some(&(_, lit)) = inc_edges[u].iter().find(|&&(w, _)| w == v) {
                    neg_clause.push(!lit);
                }
            }
            let _ = solver.add_clause(Clause::from_iter(neg_clause));
        }
    }

    None
}

pub fn solve_bipartite(raw_g: &Graph, timeout_secs: f64) -> Option<Vec<i32>> {
    let t_start = Instant::now();
    let deadline = t_start + std::time::Duration::from_secs_f64(timeout_secs);

    let partition = detect_and_partition(raw_g)?;
    println!(
        "[dynamic_bipartite] Partitioned into {} clusters and {} connectors",
        partition.clusters.len(),
        partition.connectors.len()
    );

    let mut macro_solver = MacroSatSolver::new_with_deadline(&partition, raw_g, deadline).ok()?;

    while Instant::now() < deadline {
        let config = match macro_solver.solve_next_configuration() {
            Some(cfg) => cfg,
            None => {
                eprintln!("[dynamic_bipartite] MacroSatSolver exhausted all configurations");
                return None;
            }
        };

        println!(
            "[dynamic_bipartite] Testing macro configuration with {} active macro edges...",
            config.active_macro_edges.len()
        );

        let active_macro_nodes: HashSet<i32> = config
            .active_macro_edges
            .iter()
            .flat_map(|&(u, v)| [u, v])
            .collect();

        let cluster_tasks: Vec<(i32, i32, i32, HashSet<i32>)> = partition
            .super_hubs
            .iter()
            .map(|&h| {
                let (u_in, u_out) = config.cluster_ports[&h];
                let mut nodes = partition.clusters[&h].clone();
                // If super-hub h is acting as an external bridge in this configuration,
                // it is traversed externally, so it must not be included in the internal cluster path.
                if active_macro_nodes.contains(&h) && h != u_in && h != u_out {
                    nodes.remove(&h);
                }
                (h, u_in, u_out, nodes)
            })
            .collect();

        let cluster_results: Vec<(i32, i32, i32, Option<Vec<i32>>)> = cluster_tasks
            .par_iter()
            .map(|(h, u_in, u_out, nodes)| {
                let path = solve_cluster_path(*h, *u_in, *u_out, nodes, &raw_g.adjacency_list, deadline);
                (*h, *u_in, *u_out, path)
            })
            .collect();

        let mut all_sat = true;
        let mut solved_paths: HashMap<i32, Vec<i32>> = HashMap::new();

        for (h, u_in, u_out, path_opt) in cluster_results {
            match path_opt {
                Some(p) => {
                    solved_paths.insert(h, p);
                }
                None => {
                    all_sat = false;
                    if Instant::now() < deadline {
                        println!(
                            "[dynamic_bipartite] Cluster {} UNSAT for port pair ({}, {}). Learning conflict...",
                            h, u_in, u_out
                        );
                        macro_solver.block_pair(h, u_in, u_out);
                    } else {
                        return None;
                    }
                }
            }
        }

        if all_sat {
            println!(
                "[dynamic_bipartite] All {} clusters solved! Splicing tour...",
                partition.clusters.len()
            );

            let mut ext_adj: HashMap<i32, Vec<i32>> = HashMap::new();
            for &(u, v) in &config.active_macro_edges {
                ext_adj.entry(u).or_default().push(v);
                ext_adj.entry(v).or_default().push(u);
            }

            let start_node = if !partition.connectors.is_empty() {
                partition.connectors[0]
            } else {
                config.active_macro_edges[0].0
            };

            let nxts = match ext_adj.get(&start_node) {
                Some(n) if !n.is_empty() => n.clone(),
                _ => return None,
            };

            let mut tour = Vec::with_capacity(raw_g.adjacency_list.len());
            let mut visited_clusters = HashSet::new();

            tour.push(start_node);
            let mut prev = Some(start_node);
            let mut curr = nxts[0];

            while curr != start_node {
                let mut is_cluster_entry = false;
                if let Some(&h) = partition.owner.get(&curr) {
                    if !visited_clusters.contains(&h) {
                        let (u_in, u_out) = config.cluster_ports[&h];
                        if curr == u_in {
                            is_cluster_entry = true;
                            visited_clusters.insert(h);
                            let p = &solved_paths[&h];
                            tour.extend_from_slice(p);
                            let exit_port = u_out;
                            let nxt = ext_adj
                                .get(&exit_port)
                                .and_then(|nbrs| nbrs.iter().copied().find(|&w| Some(w) != prev).or_else(|| nbrs.first().copied()));
                            match nxt {
                                Some(nxt_node) => {
                                    prev = Some(exit_port);
                                    curr = nxt_node;
                                }
                                None => break,
                            }
                        } else if curr == u_out {
                            is_cluster_entry = true;
                            visited_clusters.insert(h);
                            let p = &solved_paths[&h];
                            let mut rev_p = p.clone();
                            rev_p.reverse();
                            tour.extend_from_slice(&rev_p);
                            let exit_port = u_in;
                            let nxt = ext_adj
                                .get(&exit_port)
                                .and_then(|nbrs| nbrs.iter().copied().find(|&w| Some(w) != prev).or_else(|| nbrs.first().copied()));
                            match nxt {
                                Some(nxt_node) => {
                                    prev = Some(exit_port);
                                    curr = nxt_node;
                                }
                                None => break,
                            }
                        }
                    }
                }

                if !is_cluster_entry {
                    tour.push(curr);
                    let nxts = ext_adj.get(&curr).cloned().unwrap_or_default();
                    let valid_nxt = nxts.into_iter().find(|&w| Some(w) != prev);
                    match valid_nxt {
                        Some(nxt_node) => {
                            prev = Some(curr);
                            curr = nxt_node;
                        }
                        None => break,
                    }
                }
            }

            let (valid, err) = TourVerifier::verify(raw_g, &tour);
            if valid {
                println!(
                    "[dynamic_bipartite] Tour certified in {:.2}s!",
                    t_start.elapsed().as_secs_f64()
                );
                return Some(tour);
            } else {
                eprintln!("[dynamic_bipartite] Tour verification error: {}", err);
            }
        }
    }

    None
}
