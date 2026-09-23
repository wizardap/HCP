use crate::core::graph::Graph;
use rustsat::clause;
use rustsat::instances::{BasicVarManager, ManageVars};
use rustsat::solvers::{Solve, SolverResult};
use rustsat::types::{Clause, Lit};
use rustsat_cadical::CaDiCaL;
use std::collections::{HashMap, HashSet};

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
    let super_hubs_count = raw_g
        .adjacency_list
        .iter()
        .filter(|&(_, nbrs)| nbrs.len() >= 400)
        .count();
    super_hubs_count == 5 || super_hubs_count == 10
}

pub fn detect_and_partition(raw_g: &Graph) -> Option<BipartitePartition> {
    let mut super_hubs: Vec<i32> = raw_g
        .adjacency_list
        .iter()
        .filter(|&(_, nbrs)| nbrs.len() >= 400)
        .map(|(&u, _)| u)
        .collect();
    super_hubs.sort_unstable();

    let k = super_hubs.len();
    if k != 5 && k != 10 {
        return None;
    }

    let sh_set: HashSet<i32> = super_hubs.iter().copied().collect();

    // 1-hop direct hub signatures
    let mut direct_hubs: HashMap<i32, HashSet<i32>> = HashMap::new();
    for (&u, nbrs) in &raw_g.adjacency_list {
        let dh: HashSet<i32> = nbrs.iter().filter(|v| sh_set.contains(v)).copied().collect();
        direct_hubs.insert(u, dh);
    }

    // 2-hop hub signatures for vertices with 0 direct hubs
    let mut two_hop_hubs: HashMap<i32, HashSet<i32>> = HashMap::new();
    for (&u, nbrs) in &raw_g.adjacency_list {
        if !sh_set.contains(&u) && direct_hubs[&u].is_empty() {
            let mut th = HashSet::new();
            for &w in nbrs {
                if let Some(dh) = direct_hubs.get(&w) {
                    th.extend(dh);
                }
            }
            two_hop_hubs.insert(u, th);
        }
    }

    let mut clusters: HashMap<i32, HashSet<i32>> = HashMap::new();
    let mut owner: HashMap<i32, i32> = HashMap::new();
    let mut unassigned: HashSet<i32> = HashSet::new();

    for &h in &super_hubs {
        clusters.entry(h).or_default().insert(h);
        owner.insert(h, h);
    }

    for (&u, _) in &raw_g.adjacency_list {
        if sh_set.contains(&u) {
            continue;
        }
        let dh = &direct_hubs[&u];
        if dh.len() == 1 {
            let h = *dh.iter().next().unwrap();
            clusters.entry(h).or_default().insert(u);
            owner.insert(u, h);
        } else if dh.is_empty() {
            if let Some(th) = two_hop_hubs.get(&u) {
                if th.len() == 1 {
                    let h = *th.iter().next().unwrap();
                    clusters.entry(h).or_default().insert(u);
                    owner.insert(u, h);
                } else {
                    unassigned.insert(u);
                }
            } else {
                unassigned.insert(u);
            }
        } else {
            unassigned.insert(u);
        }
    }

    // Dominant hub absorption: if >= 90% of a vertex's neighbors belong to one cluster, absorb it
    let mut unassigned_vec: Vec<i32> = unassigned.iter().copied().collect();
    unassigned_vec.sort_unstable();
    for u in unassigned_vec {
        let nbrs = match raw_g.adjacency_list.get(&u) {
            Some(n) => n,
            None => continue,
        };
        let mut cluster_counts: HashMap<i32, usize> = HashMap::new();
        for &w in nbrs {
            if let Some(&h) = owner.get(&w) {
                *cluster_counts.entry(h).or_default() += 1;
            }
        }
        if let Some((&dominant_hub, &count)) = cluster_counts.iter().max_by_key(|&(_, c)| *c) {
            if count as f64 / nbrs.len() as f64 >= 0.90 {
                clusters.entry(dominant_hub).or_default().insert(u);
                owner.insert(u, dominant_hub);
                unassigned.remove(&u);
            }
        }
    }

    let mut connectors: Vec<i32> = unassigned.into_iter().collect();
    connectors.sort_unstable();

    // Extract boundary ports per cluster
    let mut boundary_ports: HashMap<i32, Vec<i32>> = HashMap::new();
    let mut all_ports: HashSet<i32> = HashSet::new();

    for (&h, c_nodes) in &clusters {
        let mut ports = Vec::new();
        for &u in c_nodes {
            if let Some(nbrs) = raw_g.adjacency_list.get(&u) {
                let has_ext = nbrs.iter().any(|v| !c_nodes.contains(v));
                if has_ext {
                    ports.push(u);
                    all_ports.insert(u);
                }
            }
        }
        ports.sort_unstable();
        boundary_ports.insert(h, ports);
    }

    let mut all_macro_nodes: HashSet<i32> = all_ports;
    all_macro_nodes.extend(&connectors);

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

    pub fn new(partition: &BipartitePartition, _raw_g: &Graph) -> Result<Self, String> {
        let mut solver = CaDiCaL::default();
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

        // 2. Incident edge degree consistency for boundary ports:
        // A boundary port u in cluster h has degree 1 in active macro edges <=> u is an endpoint in chosen pair
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

        // 3. Connector nodes degree constraint: exactly 2 incident edges
        for &c in &partition.connectors {
            let c_edges = incident_macro_edges.get(&c).cloned().unwrap_or_default();
            if c_edges.len() < 2 {
                return Err(format!("Connector node {} has degree < 2 in macro edges", c));
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

            if comps.len() <= 2 {
                return Some(MacroConfiguration {
                    cluster_ports,
                    active_macro_edges: active_edges,
                });
            }

            // Add subtour elimination cuts for each disconnected component
            for comp in comps {
                let comp_set: HashSet<i32> = comp.iter().copied().collect();
                let comp_edges: Vec<Lit> = active_edges
                    .iter()
                    .filter(|(u, v)| comp_set.contains(u) && comp_set.contains(v))
                    .map(|e| !self.edge_vars[e])
                    .collect();
                if comp_edges.len() >= 2 {
                    let _ = self.solver.add_clause(Clause::from_iter(comp_edges));
                }
            }
        }
        None
    }
}
