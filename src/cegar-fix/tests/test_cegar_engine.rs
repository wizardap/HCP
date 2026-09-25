use cegar_fix::core::graph::Graph;
use cegar_fix::core::tour_verifier::TourVerifier;
use cegar_fix::solver::cegar_engine::{solve_cycle, solve_path};
use std::collections::HashMap;

fn build_cycle_graph(n: usize) -> Graph {
    let mut adj = HashMap::new();
    let mut arcs = Vec::new();
    for i in 1..=n as i32 {
        adj.insert(i, Vec::new());
    }
    for i in 1..=n as i32 {
        let nxt = if i == n as i32 { 1 } else { i + 1 };
        adj.get_mut(&i).unwrap().push(nxt);
        adj.get_mut(&nxt).unwrap().push(i);
        arcs.push((i, nxt));
        arcs.push((nxt, i));
    }
    let mut btree_adj = std::collections::BTreeMap::new();
    for (&k, v) in &adj {
        btree_adj.insert(k, v.clone());
    }
    Graph {
        adjacency_list: adj,
        adjacency_list_btree: btree_adj,
        arcs,
    }
}

#[test]
fn test_solve_cycle_simple_hamiltonian() {
    let g = build_cycle_graph(6);
    let res = solve_cycle(&g, 5.0).expect("Should find Hamiltonian cycle on C6");
    assert_eq!(res.len(), 6);
    let (ok, msg) = TourVerifier::verify(&g, &res);
    assert!(ok, "Tour verification failed: {}", msg);
}

#[test]
fn test_solve_path_simple() {
    let g = build_cycle_graph(6);
    // Path on C6 between 1 and 6 should cover all 6 vertices: 1, 2, 3, 4, 5, 6
    let path = solve_path(&g, 1, 6, 5.0).expect("Should find Hamiltonian path between 1 and 6");
    assert_eq!(path.len(), 6);
    assert_eq!(path[0], 1);
    assert_eq!(*path.last().unwrap(), 6);
}

#[test]
fn test_solve_cycle_unsat() {
    // K2 is UNSAT for Hamiltonian cycle
    let mut adj = HashMap::new();
    adj.insert(1, vec![2]);
    adj.insert(2, vec![1]);
    let mut btree_adj = std::collections::BTreeMap::new();
    btree_adj.insert(1, vec![2]);
    btree_adj.insert(2, vec![1]);
    let g = Graph {
        adjacency_list: adj,
        adjacency_list_btree: btree_adj,
        arcs: vec![(1, 2), (2, 1)],
    };
    let res = solve_cycle(&g, 1.0);
    assert!(res.is_err());
}
