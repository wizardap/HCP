use std::collections::{BTreeMap, HashSet};
use std::thread;
use std::time::Duration;
use crate::graph::Graph;
use crate::encoder::Encoder;
use rustsat::solvers::*;
use rustsat_cadical::CaDiCaL;

#[derive(Debug)]
pub enum SubHcpResult {
    Solved(Vec<i32>),
    Unsolved,
}

/// Partition active cycles into clusters of cycles based on cross-edge weights.
pub fn cluster_subcycles(
    cycles: &Vec<Vec<i32>>,
    g: &Graph,
    max_cluster_size: usize,
) -> Vec<Vec<Vec<i32>>> {
    let n = cycles.len();
    if n <= 1 {
        return vec![cycles.clone()];
    }

    // Step 1: Map each vertex to its cycle index
    let mut vertex_to_cycle: BTreeMap<i32, usize> = BTreeMap::new();
    for (c_idx, cycle) in cycles.iter().enumerate() {
        for &v in cycle {
            vertex_to_cycle.insert(v, c_idx);
        }
    }

    // Step 2: Build cross-edge weights between cycles
    let mut cross_weights: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    for (c_idx, cycle) in cycles.iter().enumerate() {
        for &u in cycle {
            if let Some(adjs) = g.adjacency_list.get(&u) {
                for &v in adjs {
                    if let Some(&other_idx) = vertex_to_cycle.get(&v) {
                        if c_idx < other_idx {
                            *cross_weights.entry((c_idx, other_idx)).or_insert(0) += 1;
                        }
                    }
                }
            }
        }
    }

    // Sort edges by weight descending
    let mut edges: Vec<((usize, usize), usize)> = cross_weights.into_iter().collect();
    edges.sort_by(|a, b| b.1.cmp(&a.1));

    // Step 3: Greedy Union-Find
    let mut parent: Vec<usize> = (0..n).collect();
    let mut cluster_sizes: Vec<usize> = cycles.iter().map(|c| c.len()).collect();

    fn find(parent: &mut Vec<usize>, i: usize) -> usize {
        if parent[i] == i {
            i
        } else {
            let p = parent[i];
            parent[i] = find(parent, p);
            parent[i]
        }
    }

    for ((u, v), _w) in edges {
        let root_u = find(&mut parent, u);
        let root_v = find(&mut parent, v);
        if root_u != root_v {
            if cluster_sizes[root_u] + cluster_sizes[root_v] <= max_cluster_size {
                parent[root_v] = root_u;
                cluster_sizes[root_u] += cluster_sizes[root_v];
            }
        }
    }

    // Group cycle indices by root
    let mut clusters_map: BTreeMap<usize, Vec<Vec<i32>>> = BTreeMap::new();
    for i in 0..n {
        let root = find(&mut parent, i);
        clusters_map.entry(root).or_insert_with(Vec::new).push(cycles[i].clone());
    }

    clusters_map.into_values().collect()
}

/// Solve a single cluster sub-HCP on an induced subgraph using a fresh Encoder and CaDiCaL instance.
pub fn solve_cluster_sub_hcp(
    cluster_cycles: Vec<Vec<i32>>,
    full_graph: &Graph,
    timeout_secs: u64,
) -> SubHcpResult {
    let mut vertices = HashSet::new();
    for cycle in &cluster_cycles {
        for &v in cycle {
            vertices.insert(v);
        }
    }

    let sub_g = full_graph.induced_subgraph(&vertices);
    let mut encoder = Encoder::new();
    let cnf = encoder.encode(&sub_g, 1, 0, 0, 0, 0, 0);

    let mut solver = CaDiCaL::default();
    let _ = solver.add_cnf(cnf);

    let start = std::time::Instant::now();
    let max_duration = Duration::from_secs(timeout_secs);

    // Run mini-CEGAR loop on the induced subgraph
    let mut iteration = 0;
    while start.elapsed() < max_duration {
        iteration += 1;
        match solver.solve() {
            Ok(SolverResult::Sat) => {
                let sol = solver.full_solution().unwrap();
                let mut sol_arcs = Vec::new();
                for (&(u, v), &lit) in &encoder.graph_lit_map {
                    if sol.lit_value(lit) == rustsat::types::TernaryVal::True {
                        sol_arcs.push((u, v));
                    }
                }
                
                // Reconstruct cycles from solution arcs
                let mut adj_map: BTreeMap<i32, i32> = BTreeMap::new();
                for (u, v) in sol_arcs {
                    adj_map.insert(u, v);
                }
                
                let mut visited = HashSet::new();
                let mut sub_cycles = Vec::new();
                for &start_v in vertices.iter() {
                    if visited.contains(&start_v) { continue; }
                    let mut cycle = Vec::new();
                    let mut curr = start_v;
                    while !visited.contains(&curr) {
                        visited.insert(curr);
                        cycle.push(curr);
                        if let Some(&next_v) = adj_map.get(&curr) {
                            curr = next_v;
                        } else {
                            break;
                        }
                    }
                    if !cycle.is_empty() {
                        sub_cycles.push(cycle);
                    }
                }

                if sub_cycles.len() == 1 {
                    return SubHcpResult::Solved(sub_cycles[0].clone());
                }

                // If multiple subcycles, add blocking clause for smallest subcycle
                if let Some(smallest) = sub_cycles.iter().min_by_key(|c| c.len()) {
                    let mut clause = rustsat::types::Clause::new();
                    let len = smallest.len();
                    for i in 0..len {
                        let u = smallest[i];
                        let v = smallest[(i + 1) % len];
                        if let Some(&lit) = encoder.graph_lit_map.get(&(u, v)) {
                            clause.add(!lit);
                        }
                    }
                    if clause.len() > 0 {
                        let _ = solver.add_clause(clause);
                    } else {
                        break;
                    }
                }
            }
            _ => break,
        }
        if iteration > 100 {
            break;
        }
    }

    SubHcpResult::Unsolved
}

/// Solve multiple cluster sub-HCPs concurrently using multi-threading.
pub fn solve_parallel_clusters(
    active_cycles: &Vec<Vec<i32>>,
    g: &Graph,
    max_cluster_size: usize,
    sub_hcp_timeout: u64,
) -> (bool, Vec<Vec<i32>>) {
    let clusters = cluster_subcycles(active_cycles, g, max_cluster_size);
    let mut handles = Vec::new();

    for cluster in clusters {
        if cluster.len() <= 1 {
            // Single subcycle in cluster: skip sub-HCP solve
            continue;
        }
        let g_clone = g.clone();
        let handle = thread::spawn(move || {
            let res = solve_cluster_sub_hcp(cluster.clone(), &g_clone, sub_hcp_timeout);
            (cluster, res)
        });
        handles.push(handle);
    }

    if handles.is_empty() {
        return (false, active_cycles.clone());
    }

    let mut any_merged = false;
    let mut merged_active_cycles = Vec::new();
    let mut handled_clusters: HashSet<Vec<Vec<i32>>> = HashSet::new();

    for handle in handles {
        if let Ok((cluster, res)) = handle.join() {
            handled_clusters.insert(cluster.clone());
            match res {
                SubHcpResult::Solved(new_cycle) => {
                    any_merged = true;
                    println!("Cluster merged {} subcycles into 1 cycle of length {}", cluster.len(), new_cycle.len());
                    merged_active_cycles.push(new_cycle);
                }
                SubHcpResult::Unsolved => {
                    // Keep original subcycles in cluster
                    merged_active_cycles.extend(cluster);
                }
            }
        }
    }

    // Re-add any active cycles that were not in multi-subcycle clusters
    let all_clustered_cycles: HashSet<Vec<i32>> = handled_clusters.iter().flatten().cloned().collect();
    for cycle in active_cycles {
        if !all_clustered_cycles.contains(cycle) {
            merged_active_cycles.push(cycle.clone());
        }
    }

    (any_merged, merged_active_cycles)
}
