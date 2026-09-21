use cegar_fix::graph::Graph;
use cegar_fix::encoder::Encoder;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::quotient_block_cutter::QuotientBlockCutter;

#[test]
fn test_quotient_block_cutter_detects_blocks_and_cuts() {
    let mut g = Graph::new();
    let mut contractor = Degree2Contractor::new();

    // 2 modular blocks B0 = {1, 2, 3, 4} and B1 = {5, 6, 7, 8}
    // Protected chains inside B0: (1, 2), (3, 4)
    // Protected chains inside B1: (5, 6), (7, 8)
    contractor.chain_map.insert((1, 2), vec![101]);
    contractor.chain_map.insert((2, 1), vec![101]);
    contractor.chain_map.insert((3, 4), vec![102]);
    contractor.chain_map.insert((4, 3), vec![102]);

    contractor.chain_map.insert((5, 6), vec![103]);
    contractor.chain_map.insert((6, 5), vec![103]);
    contractor.chain_map.insert((7, 8), vec![104]);
    contractor.chain_map.insert((8, 7), vec![104]);

    // Internal edges of B0
    g.add_edge(1, 2);
    g.add_edge(2, 3);
    g.add_edge(3, 4);
    g.add_edge(4, 1);

    // Internal edges of B1
    g.add_edge(5, 6);
    g.add_edge(6, 7);
    g.add_edge(7, 8);
    g.add_edge(8, 5);

    // Cross-edges between B0 and B1
    g.add_edge(2, 5);
    g.add_edge(4, 7);

    // Subcycles: cycle 0 is B0: [1, 2, 3, 4], cycle 1 is B1: [5, 6, 7, 8]
    let cycles = vec![vec![1, 2, 3, 4], vec![5, 6, 7, 8]];

    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let blocks = QuotientBlockCutter::detect_modular_blocks(&g, &contractor);
    assert!(!blocks.is_empty(), "Should detect modular blocks");

    let quotient_cuts = QuotientBlockCutter::generate_quotient_sec_clauses(&cycles, &blocks, &g, &encoder);
    assert!(!quotient_cuts.is_empty(), "Should generate quotient cut clauses for block partition");

    // The cut clause for B0 must contain cross edges leaving B0: (2, 5) and (4, 7)
    let l_25 = *encoder.graph_lit_map.get(&(2, 5)).unwrap();
    let l_47 = *encoder.graph_lit_map.get(&(4, 7)).unwrap();

    let clauses: Vec<Vec<rustsat::types::Lit>> = quotient_cuts
        .into_iter()
        .map(|c| c.into_iter().collect())
        .collect();

    // At least one clause contains l_25 or l_47 as a positive literal
    let found = clauses.iter().any(|c| c.contains(&l_25) || c.contains(&l_47));
    assert!(found, "Quotient SEC must contain cross-edge literals connecting blocks");
}
