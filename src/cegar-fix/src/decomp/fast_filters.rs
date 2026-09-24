use crate::core::graph::Graph;
use std::collections::{HashMap, HashSet, VecDeque};

pub fn check_fast_invariants(g: &Graph) -> Result<(), &'static str> {
    let nv = g.adjacency_list.len();
    if nv < 3 {
        return Err("UNSAT: Graph has fewer than 3 vertices");
    }

    // 1. Connectivity check (BFS)
    let start_node = *g.adjacency_list.keys().next().unwrap();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    visited.insert(start_node);
    queue.push_back(start_node);

    while let Some(u) = queue.pop_front() {
        if let Some(neighbors) = g.adjacency_list.get(&u) {
            for &v in neighbors {
                if visited.insert(v) {
                    queue.push_back(v);
                }
            }
        }
    }
    if visited.len() != nv {
        return Err("UNSAT: Graph is disconnected");
    }

    // 2. Minimum degree check
    for (&_node, neighbors) in &g.adjacency_list {
        if neighbors.len() < 2 {
            return Err("UNSAT: Graph contains vertex with degree < 2");
        }
    }

    // 3. Cut-vertex check (Tarjan DFS)
    if g.has_articulation_points() {
        return Err("UNSAT: Graph contains cut-vertex");
    }

    // 4. Bipartite parity check
    let mut color: HashMap<i32, u8> = HashMap::new();
    let mut is_bipartite = true;
    let mut count_color = [0usize, 0usize];

    color.insert(start_node, 0);
    count_color[0] += 1;
    let mut q = VecDeque::new();
    q.push_back(start_node);

    while let Some(u) = q.pop_front() {
        let c_u = color[&u];
        let c_v = 1 - c_u;
        if let Some(neighbors) = g.adjacency_list.get(&u) {
            for &v in neighbors {
                if let Some(&exist_c) = color.get(&v) {
                    if exist_c == c_u {
                        is_bipartite = false;
                        break;
                    }
                } else {
                    color.insert(v, c_v);
                    count_color[c_v as usize] += 1;
                    q.push_back(v);
                }
            }
        }
        if !is_bipartite {
            break;
        }
    }

    if is_bipartite && count_color[0] != count_color[1] {
        return Err("UNSAT: Bipartite graph has unequal partition sizes");
    }

    Ok(())
}
