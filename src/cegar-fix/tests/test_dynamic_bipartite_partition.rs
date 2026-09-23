use cegar_fix::core::file_operations;
use cegar_fix::macro_decomp::dynamic_bipartite;
use std::path::Path;

fn find_graph(name: &str) -> String {
    let candidates = [
        format!("../../FHCPCS-col/{}", name),
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

#[test]
fn test_partition_graph746_dynamic() {
    let graph_path = find_graph("graph746.col");
    let g = file_operations::parse_graph_from_file(&graph_path).expect("Failed to parse graph746");
    assert!(dynamic_bipartite::can_solve_bipartite(&g), "graph746 should be detected as bipartite");

    let partition = dynamic_bipartite::detect_and_partition(&g)
        .expect("Failed to partition graph746");

    assert_eq!(partition.super_hubs.len(), 5, "Expected 5 super hubs");
    assert_eq!(partition.clusters.len(), 5, "Expected 5 clusters");
    assert_eq!(partition.connectors.len(), 2, "Expected exactly 2 connectors");
    
    let total_covered: usize = partition.clusters.values().map(|c| c.len()).sum::<usize>() + partition.connectors.len();
    assert_eq!(total_covered, 4286, "All 4286 vertices must be partitioned");

    for (&hub, c) in &partition.clusters {
        assert!(c.len() >= 850 && c.len() <= 860, "Cluster {} size {} is out of expected balanced range [850, 860]", hub, c.len());
        let ports = &partition.boundary_ports[&hub];
        assert_eq!(ports.len(), 3, "Cluster {} should have exactly 3 boundary ports, got {:?}", hub, ports);
    }
}

#[test]
fn test_partition_graph950_dynamic() {
    let graph_path = find_graph("graph950.col");
    let g = file_operations::parse_graph_from_file(&graph_path).expect("Failed to parse graph950");
    assert!(dynamic_bipartite::can_solve_bipartite(&g), "graph950 should be detected as bipartite");

    let partition = dynamic_bipartite::detect_and_partition(&g)
        .expect("Failed to partition graph950");

    assert_eq!(partition.super_hubs.len(), 10, "Expected 10 super hubs");
    assert_eq!(partition.clusters.len(), 10, "Expected 10 clusters");
    assert_eq!(partition.connectors.len(), 2, "Expected exactly 2 connectors");
    
    let total_covered: usize = partition.clusters.values().map(|c| c.len()).sum::<usize>() + partition.connectors.len();
    assert_eq!(total_covered, 6620, "All 6620 vertices must be partitioned");

    for (&hub, c) in &partition.clusters {
        assert!(c.len() >= 658 && c.len() <= 666, "Cluster {} size {} is out of expected balanced range [658, 666]", hub, c.len());
        let ports = &partition.boundary_ports[&hub];
        assert_eq!(ports.len(), 3, "Cluster {} should have exactly 3 boundary ports, got {:?}", hub, ports);
    }
}
