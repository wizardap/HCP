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
pub struct HubCluster {
    pub hub_id: i32,
    pub cluster_idx: usize,
    pub vertices: HashSet<i32>,
    pub entry_candidates: Vec<i32>,
    pub exit_candidates: Vec<i32>,
}

pub struct HubPartitionedSolver;

impl HubPartitionedSolver {
    pub fn solve_via_hub_partition(
        g: &Graph,
        contractor: &Degree2Contractor,
        hub_registry: &HubRegistry,
    ) -> Option<Vec<i32>> {
        solve_via_hub_partition(g, contractor, hub_registry)
    }

    pub fn partition_clusters(g: &Graph, hub_registry: &HubRegistry) -> Vec<HubCluster> {
        partition_clusters(g, hub_registry)
    }

    pub fn solve_cluster_hamiltonian_path(
        cluster: &HubCluster,
        g: &Graph,
        in_vertex: i32,
        out_vertex: i32,
    ) -> Option<Vec<i32>> {
        solve_cluster_hamiltonian_path(cluster, g, in_vertex, out_vertex)
    }
}

/// Finds a directed Hamiltonian cycle in the K-node super-hub transition graph.
fn find_hub_cycle(k: usize, hub_adj: &[Vec<usize>]) -> Option<Vec<usize>> {
    let mut visited = vec![false; k];
    let mut path = Vec::with_capacity(k);

    fn dfs(
        curr: usize,
        start: usize,
        k: usize,
        hub_adj: &[Vec<usize>],
        visited: &mut [bool],
        path: &mut Vec<usize>,
    ) -> bool {
        path.push(curr);
        visited[curr] = true;
        if path.len() == k {
            if hub_adj[curr].contains(&start) {
                return true;
            }
            visited[curr] = false;
            path.pop();
            return false;
        }
        for &next in &hub_adj[curr] {
            if !visited[next] {
                if dfs(next, start, k, hub_adj, visited, path) {
                    return true;
                }
            }
        }
        visited[curr] = false;
        path.pop();
        false
    }

    for start in 0..k {
        visited.fill(false);
        path.clear();
        if dfs(start, start, k, hub_adj, &mut visited, &mut path) {
            return Some(path);
        }
    }

    None
}

/// Partitions non-hub vertices into K disjoint subsets assigned to the closest super-hub,
/// orders the super-hubs into a valid cyclic sequence, and identifies candidate entry/exit boundary vertices.
pub fn partition_clusters(g: &Graph, hub_registry: &HubRegistry) -> Vec<HubCluster> {
    if hub_registry.hub_vertices.len() < 3 {
        return Vec::new();
    }

    // Select super-hubs: top hubs by degree (3 to 10 hubs)
    let super_hubs: Vec<i32> = if hub_registry.hub_vertices.len() <= 10 {
        hub_registry.hub_vertices.clone()
    } else {
        let high_deg: Vec<i32> = hub_registry
            .hub_vertices
            .iter()
            .filter(|&&h| g.adjacency_list.get(&h).map_or(0, |adj| adj.len()) >= 50)
            .copied()
            .collect();
        if high_deg.len() >= 3 && high_deg.len() <= 10 {
            high_deg
        } else if high_deg.len() > 10 {
            high_deg[..10].to_vec()
        } else {
            hub_registry.hub_vertices[..10.min(hub_registry.hub_vertices.len())].to_vec()
        }
    };

    let k = super_hubs.len();
    if k < 3 {
        return Vec::new();
    }

    let super_hub_set: HashSet<i32> = super_hubs.iter().copied().collect();
    let mut non_hub_vertices: Vec<i32> = g
        .adjacency_list
        .keys()
        .filter(|v| !super_hub_set.contains(v))
        .copied()
        .collect();
    non_hub_vertices.sort_unstable();

    if non_hub_vertices.is_empty() {
        return Vec::new();
    }

    // Multi-source BFS to calculate shortest hop distance from each super-hub to every vertex
    let mut dist_from_hub: Vec<HashMap<i32, usize>> = vec![HashMap::new(); k];
    for (hub_idx, &hub) in super_hubs.iter().enumerate() {
        let mut q = VecDeque::new();
        if let Some(neighbors) = g.adjacency_list.get(&hub) {
            for &nbr in neighbors {
                if !super_hub_set.contains(&nbr) {
                    dist_from_hub[hub_idx].insert(nbr, 1);
                    q.push_back((nbr, 1));
                }
            }
        }
        while let Some((curr, d)) = q.pop_front() {
            if let Some(neighbors) = g.adjacency_list.get(&curr) {
                for &nbr in neighbors {
                    if !super_hub_set.contains(&nbr) && !dist_from_hub[hub_idx].contains_key(&nbr) {
                        dist_from_hub[hub_idx].insert(nbr, d + 1);
                        q.push_back((nbr, d + 1));
                    }
                }
            }
        }
    }

    // Assign each non-hub vertex to the best super-hub based on affinity score
    let mut vertex_assignment: HashMap<i32, usize> = HashMap::new(); // vertex -> raw_hub_idx (0..k-1)
    let mut cluster_sizes = vec![0usize; k];

    for &v in &non_hub_vertices {
        let mut best_hub = 0;
        let mut best_score = -1.0;

        for hub_idx in 0..k {
            let d = dist_from_hub[hub_idx].get(&v).copied().unwrap_or(usize::MAX);
            if d == usize::MAX {
                continue;
            }

            let mut score = 100.0 / (d as f64);
            if d == 1 {
                score += 50.0;
            }

            // Neighbor affinity: count neighbors connected to this hub
            if let Some(neighbors) = g.adjacency_list.get(&v) {
                for &nbr in neighbors {
                    if dist_from_hub[hub_idx].get(&nbr).copied() == Some(1) {
                        score += 5.0;
                    }
                }
            }

            score -= (cluster_sizes[hub_idx] as f64) * 0.001;

            if score > best_score {
                best_score = score;
                best_hub = hub_idx;
            }
        }

        vertex_assignment.insert(v, best_hub);
        cluster_sizes[best_hub] += 1;
    }

    // Group vertices by raw cluster index
    let mut raw_cluster_vertex_sets: Vec<HashSet<i32>> = vec![HashSet::new(); k];
    for &v in &non_hub_vertices {
        let c_idx = vertex_assignment[&v];
        raw_cluster_vertex_sets[c_idx].insert(v);
    }

    // Build super-hub transition graph:
    // Directed edge i -> j exists if super-hub i connects to cluster j (which is attached to super-hub j)
    let mut hub_adj: Vec<Vec<usize>> = vec![Vec::new(); k];
    for i in 0..k {
        let hub_i = super_hubs[i];
        for j in 0..k {
            if i == j {
                continue;
            }
            let connects_to_j = raw_cluster_vertex_sets[j].iter().any(|&v| {
                g.adjacency_list.get(&v).map_or(false, |adj| adj.contains(&hub_i))
            });
            if connects_to_j {
                hub_adj[i].push(j);
            }
        }
    }

    // Find cyclic order of super-hubs
    let hub_order = if let Some(cycle) = find_hub_cycle(k, &hub_adj) {
        cycle
    } else {
        (0..k).collect() // Fallback to identity order
    };

    // Build ordered HubCluster structs
    let mut clusters = Vec::with_capacity(k);
    for m in 0..k {
        let raw_idx = hub_order[m];
        let prev_raw_idx = hub_order[(m + k - 1) % k];

        let hub_id = super_hubs[raw_idx];
        let prev_hub = super_hubs[prev_raw_idx];
        let verts = raw_cluster_vertex_sets[raw_idx].clone();

        let mut entry_candidates = Vec::new();
        let mut exit_candidates = Vec::new();

        for &v in &verts {
            if let Some(nbrs) = g.adjacency_list.get(&v) {
                if nbrs.contains(&prev_hub) {
                    entry_candidates.push(v);
                }
                if nbrs.contains(&hub_id) {
                    exit_candidates.push(v);
                }
            }
        }

        // Sort candidates: prioritize low internal degree nodes (natural path endpoints)
        entry_candidates.sort_unstable_by(|&a, &b| {
            let deg_a = g.adjacency_list.get(&a).map_or(0, |adj| adj.iter().filter(|x| verts.contains(x)).count());
            let deg_b = g.adjacency_list.get(&b).map_or(0, |adj| adj.iter().filter(|x| verts.contains(x)).count());
            deg_a.cmp(&deg_b)
        });

        exit_candidates.sort_unstable_by(|&a, &b| {
            let deg_a = g.adjacency_list.get(&a).map_or(0, |adj| adj.iter().filter(|x| verts.contains(x)).count());
            let deg_b = g.adjacency_list.get(&b).map_or(0, |adj| adj.iter().filter(|x| verts.contains(x)).count());
            deg_a.cmp(&deg_b)
        });

        clusters.push(HubCluster {
            hub_id,
            cluster_idx: m,
            vertices: verts,
            entry_candidates,
            exit_candidates,
        });
    }

    clusters
}

/// Encodes Hamiltonian Path on the cluster induced subgraph from in_vertex to out_vertex
/// using CaDiCaL SAT solver in RAM with degree-2 (degree-1 at endpoints) constraints and CEGAR subtour cuts.
pub fn solve_cluster_hamiltonian_path(
    cluster: &HubCluster,
    g: &Graph,
    in_vertex: i32,
    out_vertex: i32,
) -> Option<Vec<i32>> {
    solve_cluster_hamiltonian_path_internal(cluster, g, in_vertex, out_vertex, None)
}

fn solve_cluster_hamiltonian_path_internal(
    cluster: &HubCluster,
    g: &Graph,
    in_vertex: i32,
    out_vertex: i32,
    contractor_opt: Option<&Degree2Contractor>,
) -> Option<Vec<i32>> {
    if !cluster.vertices.contains(&in_vertex) || !cluster.vertices.contains(&out_vertex) {
        return None;
    }

    let n = cluster.vertices.len();
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
        return None; // Simple path of length > 1 must have distinct endpoints
    }
    if n == 2 {
        if g.adjacency_list.get(&in_vertex).map_or(false, |adj| adj.contains(&out_vertex)) {
            return Some(vec![in_vertex, out_vertex]);
        } else {
            return None;
        }
    }

    let verts: Vec<i32> = cluster.vertices.iter().copied().collect();
    let mut arc_lit_map: HashMap<(i32, i32), Lit> = HashMap::new();
    let mut var_manager = BasicVarManager::default();

    for &u in &verts {
        if let Some(nbrs) = g.adjacency_list.get(&u) {
            for &v in nbrs {
                if cluster.vertices.contains(&v) && u != v {
                    // No arc can enter in_vertex or leave out_vertex in an s-t path
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
            return None; // Isolated or dead end
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
            return None;
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
            if let (Some(&lit_uv), Some(&lit_vu)) = (arc_lit_map.get(&(u, v)), arc_lit_map.get(&(v, u))) {
                cnf.add_clause(clause!(!lit_uv, !lit_vu));
            }
        }
    }

    // 4. Mandatory edge constraints for degree-2 contracted chains within cluster
    if let Some(contractor) = contractor_opt {
        for (&(u, w), _) in &contractor.chain_map {
            if u < w && cluster.vertices.contains(&u) && cluster.vertices.contains(&w) {
                let lit_uw = arc_lit_map.get(&(u, w));
                let lit_wu = arc_lit_map.get(&(w, u));
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

    let start_time = Instant::now();
    let timeout = Duration::from_millis(1000); // 1.0s max timeout per cluster
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
                        // Add subtour blocking clause
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

                        // Add cut constraint: at least one edge leaving cycle to cluster vertices
                        let c_set: HashSet<i32> = cycle.iter().copied().collect();
                        let mut cut_lits = Vec::new();
                        for &u in &cycle {
                            if let Some(nbrs) = g.adjacency_list.get(&u) {
                                for &v in nbrs {
                                    if cluster.vertices.contains(&v) && !c_set.contains(&v) && v != in_vertex {
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

/// Solves Hamiltonian Cycle via Hub-Partitioned Divide-and-Conquer.
/// Decomposes graph into K clusters, solves localized Hamiltonian paths for each cluster,
/// and stitches them through super-hubs into a single full Hamiltonian cycle.
pub fn solve_via_hub_partition(
    g: &Graph,
    contractor: &Degree2Contractor,
    hub_registry: &HubRegistry,
) -> Option<Vec<i32>> {
    if hub_registry.hub_vertices.is_empty() || hub_registry.hub_vertices.len() < 3 {
        return None;
    }

    let clusters = partition_clusters(g, hub_registry);
    let k = clusters.len();
    if k < 3 {
        return None;
    }

    // Verify all clusters are non-empty and have candidates
    for cluster in &clusters {
        if cluster.vertices.is_empty() {
            return None;
        }
        if cluster.entry_candidates.is_empty() || cluster.exit_candidates.is_empty() {
            return None;
        }
    }

    let mut cluster_paths: Vec<Option<Vec<i32>>> = vec![None; k];
    let mut path_cache: HashMap<(usize, i32, i32), Option<Vec<i32>>> = HashMap::new();

    fn solve_paths_recursive(
        c_idx: usize,
        k: usize,
        clusters: &[HubCluster],
        g: &Graph,
        contractor: &Degree2Contractor,
        path_cache: &mut HashMap<(usize, i32, i32), Option<Vec<i32>>>,
        cluster_paths: &mut [Option<Vec<i32>>],
    ) -> bool {
        if c_idx == k {
            return true;
        }

        let cluster = &clusters[c_idx];
        let n_c = cluster.vertices.len();

        if n_c == 1 {
            let v = *cluster.vertices.iter().next().unwrap();
            if cluster.entry_candidates.contains(&v) && cluster.exit_candidates.contains(&v) {
                cluster_paths[c_idx] = Some(vec![v]);
                if solve_paths_recursive(c_idx + 1, k, clusters, g, contractor, path_cache, cluster_paths) {
                    return true;
                }
                cluster_paths[c_idx] = None;
            }
            return false;
        }

        let entries = &cluster.entry_candidates;
        let exits = &cluster.exit_candidates;

        for &in_v in entries {
            for &out_v in exits {
                if in_v == out_v {
                    continue;
                }

                let key = (c_idx, in_v, out_v);
                let path = if let Some(cached) = path_cache.get(&key) {
                    cached.clone()
                } else {
                    let res = solve_cluster_hamiltonian_path_internal(cluster, g, in_v, out_v, Some(contractor));
                    path_cache.insert(key, res.clone());
                    res
                };

                if let Some(p) = path {
                    cluster_paths[c_idx] = Some(p);
                    if solve_paths_recursive(c_idx + 1, k, clusters, g, contractor, path_cache, cluster_paths) {
                        return true;
                    }
                    cluster_paths[c_idx] = None;
                }
            }
        }

        false
    }

    if !solve_paths_recursive(0, k, &clusters, g, contractor, &mut path_cache, &mut cluster_paths) {
        return None;
    }

    // Stitch all cluster paths through super-hubs:
    // P_0 -> H_0 -> P_1 -> H_1 -> ... -> P_{k-1} -> H_{k-1} -> P_0
    let mut full_tour = Vec::with_capacity(g.adjacency_list.len());
    for i in 0..k {
        if let Some(p) = &cluster_paths[i] {
            full_tour.extend(p);
            full_tour.push(clusters[i].hub_id);
        } else {
            return None;
        }
    }

    if full_tour.len() == g.adjacency_list.len() && is_valid_cycle(&full_tour, g) {
        Some(full_tour)
    } else {
        None
    }
}

pub fn is_valid_cycle(cycle: &[i32], g: &Graph) -> bool {
    let len = cycle.len();
    if len < 3 {
        return false;
    }
    let mut visited = HashSet::with_capacity(len);
    for i in 0..len {
        let u = cycle[i];
        let v = cycle[(i + 1) % len];
        if !visited.insert(u) {
            return false;
        }
        if !g.adjacency_list.get(&u).map_or(false, |adj| adj.contains(&v)) {
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

    fn build_test_graph(edges: &[(i32, i32)]) -> Graph {
        let mut g = Graph::new();
        for &(u, v) in edges {
            g.add_edge(u, v);
        }
        g
    }

    /// Helper to construct a synthetic 3-hub graph where each hub has degree >= 25,
    /// and 3 clusters of 25 nodes form Hamiltonian paths stitched through the hubs.
    fn create_synthetic_3hub_graph() -> (Graph, Degree2Contractor, HubRegistry) {
        let mut edges = Vec::new();
        let n_per_cluster = 25;

        // Hubs: 1, 2, 3
        // Cluster 0: 101..=125
        // Cluster 1: 201..=225
        // Cluster 2: 301..=325
        for i in 0..n_per_cluster {
            // Cluster internal line graphs
            if i + 1 < n_per_cluster {
                edges.push((101 + i, 102 + i));
                edges.push((201 + i, 202 + i));
                edges.push((301 + i, 302 + i));
            }
            // Add internal chord edges so all cluster nodes have degree >= 3
            if i + 2 < n_per_cluster {
                edges.push((101 + i, 103 + i));
                edges.push((201 + i, 203 + i));
                edges.push((301 + i, 303 + i));
            }
            // Hub connections
            edges.push((1, 101 + i)); // Hub 1 connected to all nodes in Cluster 0
            edges.push((2, 201 + i)); // Hub 2 connected to all nodes in Cluster 1
            edges.push((3, 301 + i)); // Hub 3 connected to all nodes in Cluster 2
        }

        // Inter-cluster port connections through super-hubs:
        // Cluster 0 exit (125) -> Hub 1 -> Cluster 1 entry (201)
        edges.push((1, 201));
        // Cluster 1 exit (225) -> Hub 2 -> Cluster 2 entry (301)
        edges.push((2, 301));
        // Cluster 2 exit (325) -> Hub 3 -> Cluster 0 entry (101)
        edges.push((3, 101));

        let g = build_test_graph(&edges);
        let registry = HubRegistry::new(&g);
        let (_, contractor) = Degree2Contractor::contract(&g);

        (g, contractor, registry)
    }

    #[test]
    fn test_hub_partition_clustering() {
        let (g, _contractor, registry) = create_synthetic_3hub_graph();
        assert_eq!(registry.hub_vertices.len(), 3);
        assert!(registry.is_hub_vertex(1));
        assert!(registry.is_hub_vertex(2));
        assert!(registry.is_hub_vertex(3));

        let clusters = partition_clusters(&g, &registry);
        assert_eq!(clusters.len(), 3);

        let mut all_cluster_verts = HashSet::new();
        for (i, cluster) in clusters.iter().enumerate() {
            assert_eq!(cluster.cluster_idx, i);
            assert_eq!(cluster.vertices.len(), 25);
            assert!(!cluster.entry_candidates.is_empty());
            assert!(!cluster.exit_candidates.is_empty());

            // Check disjointness
            for &v in &cluster.vertices {
                assert!(all_cluster_verts.insert(v), "Duplicate vertex across clusters: {}", v);
            }
        }

        // Check completeness: union of cluster vertices + super-hubs == all vertices in g
        let total_v = g.adjacency_list.len();
        assert_eq!(all_cluster_verts.len() + 3, total_v);
    }

    #[test]
    fn test_hub_partition_synthetic_star_graph() {
        let (g, contractor, registry) = create_synthetic_3hub_graph();

        let start = Instant::now();
        let tour_opt = HubPartitionedSolver::solve_via_hub_partition(&g, &contractor, &registry);
        let elapsed = start.elapsed();

        assert!(tour_opt.is_some(), "HubPartitionedSolver should solve synthetic 3-hub graph");
        let tour = tour_opt.unwrap();

        assert_eq!(tour.len(), g.adjacency_list.len());
        assert!(is_valid_cycle(&tour, &g));
        assert!(elapsed < Duration::from_millis(100), "Solving took {:?}, expected < 100ms", elapsed);
    }

    #[test]
    fn test_hub_partition_degree2_safety() {
        // Build graph with degree-2 chains that get contracted
        let mut edges = Vec::new();
        let n_per_cluster = 25;

        for i in 0..n_per_cluster {
            if i + 1 < n_per_cluster {
                // In cluster 0, insert chain 501-502 between 105 and 106
                if 101 + i == 105 {
                    edges.push((105, 501));
                    edges.push((501, 502));
                    edges.push((502, 106));
                } else {
                    edges.push((101 + i, 102 + i));
                }

                // In cluster 1, insert node 601 between 205 and 206
                if 201 + i == 205 {
                    edges.push((205, 601));
                    edges.push((601, 206));
                } else {
                    edges.push((201 + i, 202 + i));
                }

                edges.push((301 + i, 302 + i));
            }
            // Chord edges
            if i + 2 < n_per_cluster {
                edges.push((101 + i, 103 + i));
                edges.push((201 + i, 203 + i));
                edges.push((301 + i, 303 + i));
            }
            edges.push((1, 101 + i));
            edges.push((2, 201 + i));
            edges.push((3, 301 + i));
        }

        edges.push((1, 201));
        edges.push((2, 301));
        edges.push((3, 101));

        let original_g = build_test_graph(&edges);
        let (contracted_g, contractor) = Degree2Contractor::contract(&original_g);

        assert!(contractor.contracted_vertices_count < contractor.original_vertices_count);
        assert_eq!(contractor.chain_map.len(), 4); // 2 chains * 2 directions

        let registry = HubRegistry::new(&contracted_g);
        let tour_opt = HubPartitionedSolver::solve_via_hub_partition(&contracted_g, &contractor, &registry);

        assert!(tour_opt.is_some(), "Should solve on contracted graph");
        let contracted_tour = tour_opt.unwrap();
        assert_eq!(contracted_tour.len(), contracted_g.adjacency_list.len());

        let full_tour = contractor.uncontract_cycle(&contracted_tour);
        assert_eq!(full_tour.len(), original_g.adjacency_list.len());
        assert!(is_valid_cycle(&full_tour, &original_g));
        assert!(full_tour.contains(&501));
        assert!(full_tour.contains(&502));
        assert!(full_tour.contains(&601));
    }
}
