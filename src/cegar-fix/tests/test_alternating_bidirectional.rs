use std::collections::HashSet;
use cegar_fix::graph::Graph;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::alternating_port_engine::{AlternatingPortEngine, min_max};

#[test]
fn test_repair_tier2_bidirectional_bfs_interface() {
    let mut g = Graph::new();
    let n_blocks = 4;
    let mut contractor = Degree2Contractor::new();

    for b in 0..n_blocks {
        let u = (b * 2) as i32;
        let w = (b * 2 + 1) as i32;
        contractor.chain_map.insert((u, w), vec![]);
        contractor.chain_map.insert((w, u), vec![]);
        g.add_edge(u, w);
    }

    let mut edges = HashSet::new();
    edges.insert(min_max(1, 2));
    edges.insert(min_max(3, 0));
    edges.insert(min_max(5, 6));
    edges.insert(min_max(7, 4));

    for &e in &edges {
        g.add_edge(e.0, e.1);
    }

    // 2-opt cross edges
    g.add_edge(1, 6);
    g.add_edge(2, 5);

    let (blocks_count, node_to_port, port_to_node) = AlternatingPortEngine::setup_ports(&contractor);

    let improved = AlternatingPortEngine::repair_tier2_bidirectional_bfs(
        &mut edges,
        &g,
        &node_to_port,
        &port_to_node,
        blocks_count,
        3,
        1000,
        std::time::Instant::now(),
    );
    assert!(improved, "repair_tier2_bidirectional_bfs must return true on valid repair");
    let (cycs, _) = AlternatingPortEngine::get_cycles(&edges, &node_to_port, &port_to_node, blocks_count);
    assert_eq!(cycs.len(), 1, "Edges must now form a single Hamiltonian cycle");
}

#[test]
fn test_bidirectional_bfs_finds_10hop_alternating_cycle() {
    // Build synthetic graph with 6 blocks (12 ports)
    // Connecting 2 separate cycles of 6 ports each via a 6-block corridor
    let mut g = Graph::new();
    let n_blocks = 6;
    let mut contractor = Degree2Contractor::new();

    for b in 0..n_blocks {
        let u = (b * 2) as i32;
        let w = (b * 2 + 1) as i32;
        contractor.chain_map.insert((u, w), vec![]);
        contractor.chain_map.insert((w, u), vec![]);
        g.add_edge(u, w);
    }

    // Active edges form 2 disjoint cycles:
    // Cycle 1: block 0, 1, 2 -> ports (0,1)-(2,0), (2,1)-(4,0), (4,1)-(0,0)
    // Cycle 2: block 3, 4, 5 -> ports (6,1)-(8,0), (8,1)-(10,0), (10,1)-(6,0)
    let mut initial_edges = HashSet::new();
    initial_edges.insert(min_max(1, 2));
    initial_edges.insert(min_max(3, 4));
    initial_edges.insert(min_max(5, 0));

    initial_edges.insert(min_max(7, 8));
    initial_edges.insert(min_max(9, 10));
    initial_edges.insert(min_max(11, 6));

    for &e in &initial_edges {
        g.add_edge(e.0, e.1);
    }

    // Add cross inactive edges forming a 10-hop (5 edges added, 5 removed) alternating path:
    // 1 -> 8 -[act]-> 7 -> 9 -[act]-> 10 -> 3 -[act]-> 4 -> 0 -[act]-> 5 -> 2 -[act]-> 1
    g.add_edge(1, 8);
    g.add_edge(7, 9);
    g.add_edge(10, 3);
    g.add_edge(4, 0);
    g.add_edge(5, 2);

    let (blocks_count, node_to_port, port_to_node) = AlternatingPortEngine::setup_ports(&contractor);
    assert_eq!(blocks_count, 6);

    let (cycs, _) = AlternatingPortEngine::get_cycles(&initial_edges, &node_to_port, &port_to_node, blocks_count);
    assert_eq!(cycs.len(), 2, "Initially must have 2 cycles");

    let mut input_cycles = Vec::new();
    for pc in &cycs {
        let mut c = Vec::new();
        for p in pc {
            c.push(port_to_node[p]);
        }
        input_cycles.push(c);
    }

    // Repair with depth 5 (max_depth_per_dir)
    let repaired = AlternatingPortEngine::repair(&input_cycles, &g, &contractor, 5, 1000);
    assert_eq!(repaired.len(), 1, "Bidirectional BFS must merge the 2 cycles into 1 Hamiltonian cycle");
}

#[test]
fn test_smallest_first_ordering() {
    let mut contractor = Degree2Contractor::new();
    let mut g = Graph::new();

    // 4 blocks: 2 for a small cycle, 2 for another
    for b in 0..4 {
        let u = (b * 2) as i32;
        let w = (b * 2 + 1) as i32;
        contractor.chain_map.insert((u, w), vec![]);
        contractor.chain_map.insert((w, u), vec![]);
        g.add_edge(u, w);
    }

    let mut edges = HashSet::new();
    edges.insert(min_max(1, 2));
    edges.insert(min_max(3, 0));
    edges.insert(min_max(5, 6));
    edges.insert(min_max(7, 4));

    for &e in &edges {
        g.add_edge(e.0, e.1);
    }

    // Add 2-opt cross edges (1, 6) and (2, 5)
    g.add_edge(1, 6);
    g.add_edge(2, 5);

    let (blocks_count, node_to_port, port_to_node) = AlternatingPortEngine::setup_ports(&contractor);
    let (cycs, _) = AlternatingPortEngine::get_cycles(&edges, &node_to_port, &port_to_node, blocks_count);

    let mut input_cycles = Vec::new();
    for c in &cycs {
        input_cycles.push(c.iter().map(|p| port_to_node[p]).collect());
    }

    let repaired = AlternatingPortEngine::repair(&input_cycles, &g, &contractor, 4, 500);
    assert_eq!(repaired.len(), 1);
}

#[test]
fn test_bidirectional_depth6_corridor() {
    // 8 blocks: 4 in Cycle 1, 4 in Cycle 2
    // Cycle 1: (0, 1), (2, 3), (4, 5), (6, 7)
    // External: (1, 2), (3, 4), (5, 6), (7, 0)
    // Cycle 2: (8, 9), (10, 11), (12, 13), (14, 15)
    // External: (9, 10), (11, 12), (13, 14), (15, 8)
    let mut g = Graph::new();
    let n_blocks = 8;
    let mut contractor = Degree2Contractor::new();

    for b in 0..n_blocks {
        let u = (b * 2) as i32;
        let w = (b * 2 + 1) as i32;
        contractor.chain_map.insert((u, w), vec![]);
        contractor.chain_map.insert((w, u), vec![]);
        g.add_edge(u, w);
    }

    let mut edges = HashSet::new();
    edges.insert(min_max(1, 2));
    edges.insert(min_max(3, 4));
    edges.insert(min_max(5, 6));
    edges.insert(min_max(7, 0));

    edges.insert(min_max(9, 10));
    edges.insert(min_max(11, 12));
    edges.insert(min_max(13, 14));
    edges.insert(min_max(15, 8));

    for &e in &edges {
        g.add_edge(e.0, e.1);
    }

    // 6-hop alternating corridor (6 edges added, 6 edges removed)
    // Hop 1: (1, 10) inact, (10, 9) act
    // Hop 2: (9, 12) inact, (12, 11) act
    // Hop 3: (11, 14) inact, (14, 13) act
    // Hop 4: (13, 4) inact, (4, 3) act
    // Hop 5: (3, 6) inact, (6, 5) act
    // Hop 6: (5, 2) inact, (2, 1) act
    g.add_edge(1, 10);
    g.add_edge(9, 12);
    g.add_edge(11, 14);
    g.add_edge(13, 4);
    g.add_edge(3, 6);
    g.add_edge(5, 2);

    let (blocks_count, node_to_port, port_to_node) = AlternatingPortEngine::setup_ports(&contractor);
    let (cycs, _) = AlternatingPortEngine::get_cycles(&edges, &node_to_port, &port_to_node, blocks_count);
    assert_eq!(cycs.len(), 2);

    let mut input_cycles = Vec::new();
    for c in &cycs {
        input_cycles.push(c.iter().map(|p| port_to_node[p]).collect());
    }

    // Bidirectional BFS with max_depth_per_dir = 3 reaches 3+3=6 hops and merges cycles
    let repaired = AlternatingPortEngine::repair(&input_cycles, &g, &contractor, 3, 1000);
    assert_eq!(repaired.len(), 1, "Bidirectional BFS with depth 3 per dir must merge the 6-hop corridor");
}
