use cegar_fix::core::graph::Graph;
use cegar_fix::decomp::fast_filters::check_fast_invariants;

#[test]
fn test_filter_rejects_disconnected() {
    let mut g = Graph::new();
    g.add_edge(1, 2);
    g.add_edge(3, 4);
    assert_eq!(check_fast_invariants(&g), Err("UNSAT: Graph is disconnected"));
}

#[test]
fn test_filter_rejects_degree_one() {
    let mut g = Graph::new();
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 1);
    g.add_edge(3, 4); // node 4 has degree 1
    assert_eq!(check_fast_invariants(&g), Err("UNSAT: Graph contains vertex with degree < 2"));
}

#[test]
fn test_filter_rejects_cut_vertex() {
    let mut g = Graph::new();
    // Two triangles sharing vertex 3 (articulation point)
    g.add_edge(1, 2); g.add_edge(2, 3); g.add_edge(3, 1);
    g.add_edge(3, 4); g.add_edge(4, 5); g.add_edge(5, 3);
    assert_eq!(check_fast_invariants(&g), Err("UNSAT: Graph contains cut-vertex"));
}

#[test]
fn test_filter_rejects_unbalanced_bipartite() {
    let mut g = Graph::new();
    // K_{2,3} complete bipartite graph (unbalanced: 2 vs 3)
    let left = [1, 2];
    let right = [3, 4, 5];
    for &u in &left {
        for &v in &right {
            g.add_edge(u, v);
        }
    }
    assert_eq!(check_fast_invariants(&g), Err("UNSAT: Bipartite graph has unequal partition sizes"));
}

#[test]
fn test_filter_accepts_valid_cycle() {
    let mut g = Graph::new();
    for i in 1..=6 {
        let nxt = if i == 6 { 1 } else { i + 1 };
        g.add_edge(i, nxt);
    }
    assert_eq!(check_fast_invariants(&g), Ok(()));
}
