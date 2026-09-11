use std::collections::HashSet;
use cegar_fix::file_operations;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::alternating_port_engine::{AlternatingPortEngine, min_max};

#[test]
fn test_graph868_snapshot_reduction() {
    let graph_path = "FHCPCS-col/graph868.col";
    let alt_graph_path = "../../FHCPCS-col/graph868.col";
    let path = if std::path::Path::new(graph_path).exists() {
        graph_path
    } else if std::path::Path::new(alt_graph_path).exists() {
        alt_graph_path
    } else {
        eprintln!("Skipping snapshot test: graph868.col not found");
        return;
    };

    let raw_g = file_operations::input_to_graph(path);
    let (g, contractor) = Degree2Contractor::contract(&raw_g);

    let snapshot_path = "scratch/graph868_giant_1686_full_edges.txt";
    let alt_snapshot_path = "../../scratch/graph868_giant_1686_full_edges.txt";
    let snap = if std::path::Path::new(snapshot_path).exists() {
        snapshot_path
    } else if std::path::Path::new(alt_snapshot_path).exists() {
        alt_snapshot_path
    } else {
        eprintln!("Skipping snapshot test: 1686 giant checkpoint not found");
        return;
    };

    let content = std::fs::read_to_string(snap).unwrap();
    let mut initial_edges = HashSet::new();
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() == 2 {
            if let (Ok(u), Ok(v)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>()) {
                initial_edges.insert(min_max(u, v));
            }
        }
    }

    let (n_blocks, node_to_port, port_to_node) = AlternatingPortEngine::setup_ports(&contractor);
    let (cycs, _) = AlternatingPortEngine::get_cycles(&initial_edges, &node_to_port, &port_to_node, n_blocks);
    assert_eq!(cycs.len(), 31, "Snapshot should initially have 31 cycles");

    // Convert to Vec<Vec<i32>> representation (contracted nodes)
    let mut input_cycles = Vec::new();
    for pc in &cycs {
        let mut c = Vec::new();
        for p in pc {
            c.push(port_to_node[p]);
        }
        input_cycles.push(c);
    }

    println!("initial_edges.len = {}", initial_edges.len());
    println!("input_cycles.len = {}", input_cycles.len());

    let t0 = std::time::Instant::now();
    let repaired = AlternatingPortEngine::repair(&input_cycles, &g, &contractor, 4, 1000);
    let elapsed = t0.elapsed();
    println!("repaired.len = {}, elapsed = {:?}", repaired.len(), elapsed);

    assert!(repaired.len() <= 31, "Alternating engine must not increase cycle count");
    assert!(elapsed.as_millis() < 1000, "Tier 1 + Tier 2 repair must finish within 1000ms, took {:?}", elapsed);
}
