use std::collections::HashSet;
use cegar_fix::graph::Graph;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::alternating_port_engine::{AlternatingPortEngine, min_max};
use cegar_fix::port_corridor_lns::PortCorridorLns;

#[test]
fn test_port_coordinate_mapping_and_corridor_extraction() {
    let mut g = Graph::new();
    let mut contractor = Degree2Contractor::new();

    // 6 blocks (12 ports)
    // Giant cycle: blocks 0, 1, 2, 3 (8 ports)
    // Satellite cycle: blocks 4, 5 (4 ports)
    for b in 0..6 {
        let u = (b * 2) as i32;
        let w = (b * 2 + 1) as i32;
        contractor.chain_map.insert((u, w), vec![]);
        contractor.chain_map.insert((w, u), vec![]);
        g.add_edge(u, w);
    }

    let mut edges = HashSet::new();
    // Giant: (1, 2), (3, 4), (5, 6), (7, 0)
    edges.insert(min_max(1, 2));
    edges.insert(min_max(3, 4));
    edges.insert(min_max(5, 6));
    edges.insert(min_max(7, 0));

    // Satellite: (9, 10), (11, 8)
    edges.insert(min_max(9, 10));
    edges.insert(min_max(11, 8));

    for &e in &edges {
        g.add_edge(e.0, e.1);
    }

    // Inactive docking edges from satellite to giant:
    // (9, 2) and (11, 4)
    g.add_edge(9, 2);
    g.add_edge(11, 4);

    let (n_blocks, node_to_port, port_to_node) = AlternatingPortEngine::setup_ports(&contractor);
    let (cycs, _) = AlternatingPortEngine::get_cycles(&edges, &node_to_port, &port_to_node, n_blocks);

    assert_eq!(cycs.len(), 2);
    let giant = &cycs[0];
    let sat = &cycs[1];

    let giant_pos = PortCorridorLns::map_giant_coordinates(giant, n_blocks);
    let giant_blocks: HashSet<usize> = giant.iter().map(|p| p.block).collect();

    let docking = PortCorridorLns::find_docking_ports(sat, &giant_blocks, &g, &node_to_port, &port_to_node);
    assert_eq!(docking.len(), 2, "Must identify 2 docking ports in giant cycle");

    let corridor = PortCorridorLns::build_subpath_corridor(sat, giant, &giant_pos, &g, &node_to_port, &port_to_node, 10, 1)
        .expect("Must successfully construct corridor");

    assert!(corridor.unfrozen_blocks.contains(&4));
    assert!(corridor.unfrozen_blocks.contains(&5));
    assert!(corridor.unfrozen_blocks.len() >= 4, "Must unfreeze satellite blocks + giant subpath blocks");
}
