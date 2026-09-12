use std::collections::HashSet;
use cegar_fix::graph::Graph;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::encoder::Encoder;
use cegar_fix::alternating_port_engine::{AlternatingPortEngine, min_max};
use cegar_fix::port_corridor_lns::PortCorridorLns;

#[test]
fn test_port_corridor_absorbs_satellite_cycle() {
    let mut g = Graph::new();
    let mut contractor = Degree2Contractor::new();

    // 6 blocks
    for b in 0..6 {
        let u = (b * 2) as i32;
        let w = (b * 2 + 1) as i32;
        contractor.chain_map.insert((u, w), vec![]);
        contractor.chain_map.insert((w, u), vec![]);
        g.add_edge(u, w);
    }

    let mut edges = HashSet::new();
    // Giant cycle: blocks 0, 1, 2, 3
    edges.insert(min_max(1, 2));
    edges.insert(min_max(3, 4));
    edges.insert(min_max(5, 6));
    edges.insert(min_max(7, 0));

    // Satellite cycle: blocks 4, 5
    edges.insert(min_max(9, 10));
    edges.insert(min_max(11, 8));

    for &e in &edges {
        g.add_edge(e.0, e.1);
    }

    // Docking cross edges: (9, 2), (11, 4) plus complementary alternating edges (3, 10), (4, 9)
    g.add_edge(9, 2);
    g.add_edge(11, 4);
    g.add_edge(3, 10);
    g.add_edge(4, 9);

    let (n_blocks, node_to_port, port_to_node) = AlternatingPortEngine::setup_ports(&contractor);
    let (cycs, _) = AlternatingPortEngine::get_cycles(&edges, &node_to_port, &port_to_node, n_blocks);
    assert_eq!(cycs.len(), 2);

    let mut input_cycles = Vec::new();
    for pc in &cycs {
        input_cycles.push(pc.iter().map(|p| port_to_node[p]).collect());
    }

    let mut encoder = Encoder::new();
    let base_cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);

    let repaired = PortCorridorLns::repair(&input_cycles, &g, &contractor, &encoder, &base_cnf);
    assert_eq!(repaired.len(), 1, "PortCorridorLns must absorb satellite cycle into 1 Hamiltonian cycle");
    assert_eq!(repaired[0].len(), 12, "Reconstructed cycle must visit all 12 nodes");
}
