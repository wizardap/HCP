use std::collections::HashSet;
use cegar_fix::graph::Graph;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::encoder::Encoder;
use cegar_fix::alternating_port_engine::{AlternatingPortEngine, min_max};
use cegar_fix::port_corridor_lns::PortCorridorLns;

#[test]
fn test_port_corridor_cluster_mtz_merges_satellite_cycle() {
    let mut g = Graph::new();
    let mut contractor = Degree2Contractor::new();

    // 8 blocks total
    for b in 0..8 {
        let u = (b * 2) as i32;
        let w = (b * 2 + 1) as i32;
        contractor.chain_map.insert((u, w), vec![]);
        contractor.chain_map.insert((w, u), vec![]);
        g.add_edge(u, w);
    }

    let mut edges = HashSet::new();
    // Giant cycle: blocks 0, 1, 2, 3, 4, 5
    edges.insert(min_max(1, 2));
    edges.insert(min_max(3, 4));
    edges.insert(min_max(5, 6));
    edges.insert(min_max(7, 8));
    edges.insert(min_max(9, 10));
    edges.insert(min_max(11, 0));

    // Satellite cycle: blocks 6, 7
    edges.insert(min_max(13, 14));
    edges.insert(min_max(15, 12));

    for &e in &edges {
        g.add_edge(e.0, e.1);
    }

    // Clustered docking cross edges connecting blocks 6, 7 into blocks 1, 2:
    // (13, 2), (14, 3), (15, 4), (12, 5) plus complementary alternating edges (13, 4), (12, 3)
    g.add_edge(13, 2);
    g.add_edge(14, 3);
    g.add_edge(15, 4);
    g.add_edge(12, 5);
    g.add_edge(13, 4);
    g.add_edge(12, 3);

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
    assert_eq!(repaired.len(), 1, "Must absorb satellite cycle into 1 Hamiltonian cycle");
    assert_eq!(repaired[0].len(), 16, "Reconstructed cycle must visit all 16 nodes");
}
