use std::collections::HashSet;
use cegar_fix::graph::Graph;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::macro_crossover_splicer::MacroCrossoverSplicer;

#[test]
fn test_macro_crossover_splicer_4opt_parity_merge() {
    let mut g = Graph::new();
    let mut contractor = Degree2Contractor::new();

    // Cycle 1: 1..=8
    let c1 = vec![1, 2, 3, 4, 5, 6, 7, 8];
    for i in 0..8 {
        let u = c1[i];
        let v = c1[(i + 1) % 8];
        g.add_edge(u, v);
    }
    // Protected edges in C1: (1, 2), (3, 4), (5, 6), (7, 8)
    contractor.chain_map.insert((1, 2), vec![101]);
    contractor.chain_map.insert((2, 1), vec![101]);
    contractor.chain_map.insert((3, 4), vec![102]);
    contractor.chain_map.insert((4, 3), vec![102]);
    contractor.chain_map.insert((5, 6), vec![103]);
    contractor.chain_map.insert((6, 5), vec![103]);
    contractor.chain_map.insert((7, 8), vec![104]);
    contractor.chain_map.insert((8, 7), vec![104]);

    // Cycle 2: 9..=16
    let c2 = vec![9, 10, 11, 12, 13, 14, 15, 16];
    for i in 0..8 {
        let u = c2[i];
        let v = c2[(i + 1) % 8];
        g.add_edge(u, v);
    }
    // Protected edges in C2: (9, 10), (11, 12), (13, 14), (15, 16)
    contractor.chain_map.insert((9, 10), vec![105]);
    contractor.chain_map.insert((10, 9), vec![105]);
    contractor.chain_map.insert((11, 12), vec![106]);
    contractor.chain_map.insert((12, 11), vec![106]);
    contractor.chain_map.insert((13, 14), vec![107]);
    contractor.chain_map.insert((14, 13), vec![107]);
    contractor.chain_map.insert((15, 16), vec![108]);
    contractor.chain_map.insert((16, 15), vec![108]);

    // Add 4-opt crossover edges (unprotected swaps):
    // In C1, unprotected edges are (2, 3) and (6, 7).
    // In C2, unprotected edges are (10, 11) and (14, 15).
    // Replacement cross-edges: (2, 10), (3, 15), (6, 11), (7, 14)
    g.add_edge(2, 10);
    g.add_edge(3, 15);
    g.add_edge(6, 11);
    g.add_edge(7, 14);

    let cycles = vec![c1, c2];

    let result = MacroCrossoverSplicer::try_crossover_splice(&cycles, &g, &contractor);
    assert!(result.is_some(), "MacroCrossoverSplicer should find the 4-opt crossover tour");
    let tour = result.unwrap();
    assert_eq!(tour.len(), 16, "Tour must visit all 16 vertices");

    // Verify all protected edges are preserved
    let mut tour_edges = HashSet::new();
    for i in 0..tour.len() {
        let u = tour[i];
        let v = tour[(i + 1) % tour.len()];
        let e = if u < v { (u, v) } else { (v, u) };
        tour_edges.insert(e);
    }

    for (&(u, w), _) in &contractor.chain_map {
        let e = if u < w { (u, w) } else { (w, u) };
        assert!(tour_edges.contains(&e), "Tour must preserve protected edge {:?}", e);
    }
}
