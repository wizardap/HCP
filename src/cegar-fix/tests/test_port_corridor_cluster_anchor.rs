use std::collections::HashSet;
use cegar_fix::graph::Graph;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::alternating_port_engine::{AlternatingPortEngine, min_max};
use cegar_fix::port_corridor_lns::PortCorridorLns;

#[test]
fn test_find_anchor_candidates_selects_closest_docking_pair() {
    let mut g = Graph::new();
    let mut contractor = Degree2Contractor::new();

    // 8 blocks: blocks 0..5 on giant, blocks 6, 7 on satellite
    for b in 0..8 {
        let u = (b * 2) as i32;
        let w = (b * 2 + 1) as i32;
        contractor.chain_map.insert((u, w), vec![]);
        contractor.chain_map.insert((w, u), vec![]);
        g.add_edge(u, w);
    }

    let mut edges = HashSet::new();
    // Giant cycle: blocks 0, 1, 2, 3, 4, 5 (12 ports)
    edges.insert(min_max(1, 2));
    edges.insert(min_max(3, 4));
    edges.insert(min_max(5, 6));
    edges.insert(min_max(7, 8));
    edges.insert(min_max(9, 10));
    edges.insert(min_max(11, 0));

    // Satellite cycle: blocks 6, 7 (4 ports)
    edges.insert(min_max(13, 14));
    edges.insert(min_max(15, 12));

    for &e in &edges {
        g.add_edge(e.0, e.1);
    }

    // Docking cross edges:
    // Sat connects to block 1 (node 3) and block 2 (node 4) -> span = 2 ports (adjacent!)
    // Sat also connects to block 5 (node 11) -> span to block 1 is 8 ports (far!)
    g.add_edge(13, 3);
    g.add_edge(15, 4);
    g.add_edge(12, 11);

    let (n_blocks, node_to_port, port_to_node) = AlternatingPortEngine::setup_ports(&contractor);
    let (cycs, _) = AlternatingPortEngine::get_cycles(&edges, &node_to_port, &port_to_node, n_blocks);
    assert_eq!(cycs.len(), 2);

    let giant = &cycs[0];
    let sat = &cycs[1];
    let giant_pos = PortCorridorLns::map_giant_coordinates(giant, n_blocks);

    let anchors = PortCorridorLns::find_anchor_candidates(
        sat,
        giant,
        &giant_pos,
        &g,
        &node_to_port,
        &port_to_node,
        30,
    );

    assert!(!anchors.is_empty(), "Must find candidate anchors");
    // Shortest anchor must be between block 1 and block 2 (span <= 4 ports)
    assert!(anchors[0].span <= 4, "First anchor candidate must have minimal span <= 4, found {}", anchors[0].span);

    let corridor = PortCorridorLns::build_corridor_from_anchor(&anchors[0], sat, giant, 1);
    assert!(corridor.is_some(), "Corridor building must succeed");
    let c = corridor.unwrap();
    assert_eq!(c.sat_blocks.len(), 2);
    assert!(c.unfrozen_blocks.contains(&6) && c.unfrozen_blocks.contains(&7));
    assert!(c.unfrozen_blocks.contains(&1) && c.unfrozen_blocks.contains(&2));
}
