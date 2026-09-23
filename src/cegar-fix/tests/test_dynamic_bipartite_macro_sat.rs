use cegar_fix::core::file_operations;
use cegar_fix::macro_decomp::dynamic_bipartite::{detect_and_partition, MacroSatSolver};
use std::path::Path;

fn find_graph(name: &str) -> String {
    let candidates = [
        format!("../../FHCPCS-col/{}", name),
        format!("../FHCPCS-col/{}", name),
        format!("FHCPCS-col/{}", name),
        format!("/home/ubuntu/HCP/FHCPCS-col/{}", name),
    ];
    for p in &candidates {
        if Path::new(p).exists() {
            return p.clone();
        }
    }
    format!("../../FHCPCS-col/{}", name)
}

fn assert_macro_connectivity(config: &cegar_fix::macro_decomp::dynamic_bipartite::MacroConfiguration) {
    let mut macro_adj: std::collections::HashMap<i32, Vec<i32>> = std::collections::HashMap::new();
    for &(u, v) in &config.active_macro_edges {
        macro_adj.entry(u).or_default().push(v);
        macro_adj.entry(v).or_default().push(u);
    }
    for &(u, v) in config.cluster_ports.values() {
        macro_adj.entry(u).or_default().push(v);
        macro_adj.entry(v).or_default().push(u);
    }

    assert!(!config.active_macro_edges.is_empty(), "Active macro edges cannot be empty");
    let start = config.active_macro_edges[0].0;
    let mut visited = std::collections::HashSet::new();
    let mut queue = std::collections::VecDeque::new();
    visited.insert(start);
    queue.push_back(start);
    while let Some(curr) = queue.pop_front() {
        if let Some(nbrs) = macro_adj.get(&curr) {
            for &nxt in nbrs {
                if visited.insert(nxt) {
                    queue.push_back(nxt);
                }
            }
        }
    }
    assert_eq!(
        visited.len(),
        macro_adj.len(),
        "Macro graph must have exactly 1 connected component (visited {} / {})",
        visited.len(),
        macro_adj.len()
    );
}

#[test]
fn test_macro_sat_graph746() {
    let graph_path = find_graph("graph746.col");
    let g = file_operations::parse_graph_from_file(&graph_path).expect("Failed to parse graph746");
    let partition = detect_and_partition(&g).expect("Failed to partition");

    let mut solver = MacroSatSolver::new(&partition, &g).expect("Failed to build MacroSatSolver");
    let config = solver.solve_next_configuration().expect("Expected a valid macro configuration for graph746");

    assert_eq!(config.cluster_ports.len(), 5, "All 5 clusters must have a chosen port pair");
    for &h in &partition.super_hubs {
        let (u_in, u_out) = config.cluster_ports[&h];
        assert_ne!(u_in, u_out, "Port u_in and u_out must be distinct for hub {}", h);
        let ports = &partition.boundary_ports[&h];
        assert!(ports.contains(&u_in), "u_in {} must be in boundary ports {:?}", u_in, ports);
        assert!(ports.contains(&u_out), "u_out {} must be in boundary ports {:?}", u_out, ports);
    }

    assert!(config.active_macro_edges.len() >= 7, "Macro cycle must have active external edges");
    assert_macro_connectivity(&config);
}

#[test]
fn test_macro_sat_graph950() {
    let graph_path = find_graph("graph950.col");
    let g = file_operations::parse_graph_from_file(&graph_path).expect("Failed to parse graph950");
    let partition = detect_and_partition(&g).expect("Failed to partition");

    let mut solver = MacroSatSolver::new(&partition, &g).expect("Failed to build MacroSatSolver");
    let config = solver.solve_next_configuration().expect("Expected a valid macro configuration for graph950");

    assert_eq!(config.cluster_ports.len(), 10, "All 10 clusters must have a chosen port pair");
    for &h in &partition.super_hubs {
        let (u_in, u_out) = config.cluster_ports[&h];
        assert_ne!(u_in, u_out, "Port u_in and u_out must be distinct for hub {}", h);
        let ports = &partition.boundary_ports[&h];
        assert!(ports.contains(&u_in), "u_in {} must be in boundary ports {:?}", u_in, ports);
        assert!(ports.contains(&u_out), "u_out {} must be in boundary ports {:?}", u_out, ports);
    }
    assert_macro_connectivity(&config);
}

#[test]
fn test_macro_sat_block_pair() {
    let graph_path = find_graph("graph746.col");
    let g = file_operations::parse_graph_from_file(&graph_path).expect("Failed to parse graph746");
    let partition = detect_and_partition(&g).expect("Failed to partition");

    let mut solver = MacroSatSolver::new(&partition, &g).expect("Failed to build MacroSatSolver");
    let config1 = solver.solve_next_configuration().expect("Expected first macro configuration");

    let h = partition.super_hubs[0];
    let (u_in, u_out) = config1.cluster_ports[&h];

    // Block this port pair
    solver.block_pair(h, u_in, u_out);

    if let Some(config2) = solver.solve_next_configuration() {
        let (new_u_in, new_u_out) = config2.cluster_ports[&h];
        let p1 = (u_in.min(u_out), u_in.max(u_out));
        let p2 = (new_u_in.min(new_u_out), new_u_in.max(new_u_out));
        assert_ne!(p1, p2, "Blocked pair should not be selected in subsequent configuration");
    }
}
