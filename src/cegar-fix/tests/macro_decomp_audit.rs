use cegar_fix::core::graph::Graph;
use cegar_fix::core::tour_verifier::TourVerifier;
use cegar_fix::macro_corridor;
use cegar_fix::macro_788;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::Path;
use std::time::Instant;

fn find_benchmark(name: &str) -> String {
    let candidates = [
        format!("FHCPCS-col/{}", name),
        format!("../../FHCPCS-col/{}", name),
        format!("../FHCPCS-col/{}", name),
        format!("/root/HCP/FHCPCS-col/{}", name),
    ];
    for p in &candidates {
        if Path::new(p).exists() {
            return p.clone();
        }
    }
    format!("FHCPCS-col/{}", name)
}

fn build_graph_from_edges(n: usize, edges: &[(i32, i32)]) -> Graph {
    let mut adj_hash: HashMap<i32, Vec<i32>> = HashMap::new();
    let mut adj_btree: BTreeMap<i32, Vec<i32>> = BTreeMap::new();
    let mut arcs = Vec::new();

    for i in 0..n as i32 {
        adj_hash.insert(i, Vec::new());
        adj_btree.insert(i, Vec::new());
    }

    for &(u, v) in edges {
        adj_hash.get_mut(&u).unwrap().push(v);
        adj_hash.get_mut(&v).unwrap().push(u);
        adj_btree.get_mut(&u).unwrap().push(v);
        adj_btree.get_mut(&v).unwrap().push(u);
        arcs.push((u, v));
        arcs.push((v, u));
    }

    for (_, nbrs) in adj_hash.iter_mut() {
        nbrs.sort_unstable();
        nbrs.dedup();
    }
    for (_, nbrs) in adj_btree.iter_mut() {
        nbrs.sort_unstable();
        nbrs.dedup();
    }

    Graph {
        adjacency_list: adj_hash,
        adjacency_list_btree: adj_btree,
        arcs,
    }
}

fn verify_2cut(g: &Graph, u: i32, v: i32) -> Vec<usize> {
    let mut rem_nodes: HashSet<i32> = g.adjacency_list.keys().copied().collect();
    rem_nodes.remove(&u);
    rem_nodes.remove(&v);

    let mut visited = HashSet::new();
    let mut comp_sizes = Vec::new();

    for &node in &rem_nodes {
        if !visited.contains(&node) {
            let mut sz = 0;
            let mut q = VecDeque::new();
            visited.insert(node);
            q.push_back(node);

            while let Some(curr) = q.pop_front() {
                sz += 1;
                if let Some(nbrs) = g.adjacency_list.get(&curr) {
                    for &nbr in nbrs {
                        if rem_nodes.contains(&nbr) && visited.insert(nbr) {
                            q.push_back(nbr);
                        }
                    }
                }
            }
            comp_sizes.push(sz);
        }
    }
    comp_sizes
}

#[test]
fn test_a_min_comp_sz_fix() {
    // Graph with u=0, v=30.
    // Component A: vertices 1..=29 (29 vertices)
    // Component B: vertices 31..=230 (200 vertices)
    // Total vertices: N = 231 (0..=230)
    let n = 231;
    let mut edges = Vec::new();

    edges.push((0, 1));
    for i in 1..29 {
        edges.push((i, i + 1));
    }
    edges.push((29, 30));

    edges.push((30, 31));
    for i in 31..230 {
        edges.push((i, i + 1));
    }
    edges.push((230, 0));

    for i in 1..=29 {
        if i + 2 <= 29 {
            edges.push((i, i + 2));
        }
        if i + 3 <= 29 {
            edges.push((i, i + 3));
        }
    }
    edges.push((0, 2));
    edges.push((30, 28));

    for i in 31..=230 {
        if i + 2 <= 230 {
            edges.push((i, i + 2));
        }
        if i + 3 <= 230 {
            edges.push((i, i + 3));
        }
    }
    edges.push((30, 32));
    edges.push((0, 229));

    let g = build_graph_from_edges(n, &edges);
    assert!(!g.has_articulation_points(), "Graph must be 2-connected");

    let comps = verify_2cut(&g, 0, 30);
    assert_eq!(comps.len(), 2);
    assert!(comps.contains(&29) && comps.contains(&200));

    let tour: Vec<i32> = (0..=230).collect();
    let (h_valid, _) = TourVerifier::verify(&g, &tour);
    assert!(h_valid, "Graph must be Hamiltonian");

    // After fix: find_2cut_ports must NOT return None
    let res_ports = macro_corridor::find_2cut_ports(&g);
    assert!(
        res_ports.is_some(),
        "find_2cut_ports must detect asymmetric 2-cut after min_comp_sz fix"
    );
}

#[test]
fn test_b_degree_filter_fix() {
    // Graph with u=0, v=1.
    // Component A: vertices 2..=49 (48 vertices)
    // Component B: vertices 50..=99 (50 vertices)
    // Total N = 100.
    // Deg(0) = 12 > 10, Deg(1) = 12 > 10.
    let n = 100;
    let mut edges = Vec::new();

    edges.push((0, 2));
    for i in 2..49 {
        edges.push((i, i + 1));
    }
    edges.push((49, 1));

    edges.push((1, 50));
    for i in 50..99 {
        edges.push((i, i + 1));
    }
    edges.push((99, 0));

    for i in 2..=49 {
        if i + 2 <= 49 {
            edges.push((i, i + 2));
        }
    }
    for i in 50..=99 {
        if i + 2 <= 99 {
            edges.push((i, i + 2));
        }
    }

    for &w in &[4, 6, 8, 10, 12] {
        edges.push((0, w));
    }
    for &w in &[90, 92, 94, 96, 98] {
        edges.push((0, w));
    }
    for &w in &[39, 41, 43, 45, 47] {
        edges.push((1, w));
    }
    for &w in &[52, 54, 56, 58, 60] {
        edges.push((1, w));
    }

    let g = build_graph_from_edges(n, &edges);
    assert_eq!(g.adjacency_list[&0].len(), 12);
    assert_eq!(g.adjacency_list[&1].len(), 12);
    assert!(!g.has_articulation_points(), "Graph must be 2-connected");

    let comps = verify_2cut(&g, 0, 1);
    assert_eq!(comps.len(), 2);

    // After fix: find_2cut_ports must NOT return None even though both separator vertices have deg > 10
    let res_ports = macro_corridor::find_2cut_ports(&g);
    assert!(
        res_ports.is_some(),
        "find_2cut_ports must detect 2-cut with high-degree ports after filter removal"
    );
}

#[test]
fn test_c_contract_mismatch_fix() {
    // Graph with u=0, v=1, and 3 components of size 35 each:
    // Comp 1: 2..=36 (35v)
    // Comp 2: 37..=71 (35v)
    // Comp 3: 72..=106 (35v)
    // Total N = 107.
    let n = 107;
    let mut edges = Vec::new();

    let comp_ranges = [2..=36, 37..=71, 72..=106];
    for r in &comp_ranges {
        let nodes: Vec<i32> = r.clone().collect();
        for i in 0..nodes.len() {
            let u = nodes[i];
            let v = nodes[(i + 1) % nodes.len()];
            edges.push((u, v));
            let w = nodes[(i + 2) % nodes.len()];
            edges.push((u, w));
        }
        edges.push((0, nodes[0]));
        edges.push((0, nodes[1]));
        edges.push((1, nodes[2]));
        edges.push((1, nodes[3]));
    }

    let g = build_graph_from_edges(n, &edges);
    let comps = verify_2cut(&g, 0, 1);
    assert_eq!(comps.len(), 3);
    assert!(!g.has_articulation_points(), "Graph is 2-connected");

    // After fix: 3 components must be rejected by check_2cut_split due to toughness theorem
    let res_ports = macro_corridor::find_2cut_ports(&g);
    assert!(
        res_ports.is_none(),
        "find_2cut_ports must reject 3-component separator (toughness violation)"
    );
}

#[test]
fn test_e_portfolio_788_non_bipartite_fix() {
    let mut edges = Vec::new();
    let num_pairs = 11;
    for i in 0..num_pairs {
        let u = 1000 + 2 * i;
        let w = 1000 + 2 * i + 1;
        let m = 2000 + i;
        edges.push((u, m));
        edges.push((m, w));
    }

    // Odd cycle: w0 - u1 - (pair 1) - w1 - u2 - (pair 2) - w2 - w0
    edges.push((1000 + 1, 1000 + 2));
    edges.push((1000 + 3, 1000 + 4));
    edges.push((1000 + 5, 1000 + 1));

    edges.push((1000 + 0, 1000 + 6));
    for i in 3..num_pairs - 1 {
        let curr_w = 1000 + 2 * i + 1;
        let next_u = 1000 + 2 * (i + 1);
        edges.push((curr_w, next_u));
    }
    edges.push((1000 + 2 * (num_pairs - 1) + 1, 1000 + 4));

    for i in 0..num_pairs {
        let u = 1000 + 2 * i;
        let w = 1000 + 2 * i + 1;
        let next_u = 1000 + 2 * ((i + 1) % num_pairs);
        let next_w = 1000 + 2 * ((i + 2) % num_pairs) + 1;
        edges.push((u, next_u));
        edges.push((w, next_w));
    }

    let mut all_v: Vec<i32> = edges.iter().flat_map(|&(a, b)| vec![a, b]).collect();
    all_v.sort();
    all_v.dedup();
    let n = all_v.len();
    let v_map: HashMap<i32, i32> = all_v.iter().enumerate().map(|(idx, &v)| (v, idx as i32)).collect();
    let remapped_edges: Vec<(i32, i32)> = edges.into_iter().map(|(a, b)| (v_map[&a], v_map[&b])).collect();

    let g = build_graph_from_edges(n, &remapped_edges);

    // After fix: can_solve_alternating_pairs must return false due to odd-cycle conflict detection
    let can_solve = macro_788::can_solve_alternating_pairs(&g);
    assert!(
        !can_solve,
        "can_solve_alternating_pairs must reject non-bipartite pair graph"
    );
}

#[test]
fn test_f_timeout_preemption() {
    let p = find_benchmark("graph710.col");
    if !Path::new(&p).exists() {
        eprintln!("Skipping test_f: benchmark file not found at {}", p);
        return;
    }
    let g710 = cegar_fix::core::file_operations::parse_graph_from_file(&p).unwrap();
    let start = Instant::now();
    let _result = macro_corridor::solve_2cut_corridor(&g710, 2.0);
    let elapsed = start.elapsed().as_secs_f64();
    assert!(
        elapsed < 4.0,
        "solve_2cut_corridor must respect 2s deadline (took {:.1}s)",
        elapsed
    );
}

#[test]
fn test_h_portfolio_positive() {
    let p = find_benchmark("graph788.col");
    if !Path::new(&p).exists() {
        eprintln!("Skipping test_h: benchmark file not found at {}", p);
        return;
    }
    let g788 = cegar_fix::core::file_operations::parse_graph_from_file(&p).unwrap();
    let can_solve = macro_788::can_solve_alternating_pairs(&g788);
    assert!(
        can_solve,
        "can_solve_alternating_pairs must still accept graph788 (bipartite pair structure)"
    );
}
