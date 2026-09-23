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
    assert_eq!(partition.connectors.len(), 6, "Expected exactly 6 connectors");
    
    let total_covered: usize = partition.clusters.values().map(|c| c.len()).sum::<usize>()
        + partition.connectors.len()
        + partition.super_hubs.len();
    assert_eq!(total_covered, 4286, "All 4286 vertices must be partitioned");

    for (&hub, c) in &partition.clusters {
        assert_eq!(c.len(), 855, "Cluster {} size must be exactly 855", hub);
        let ports = &partition.boundary_ports[&hub];
        assert_eq!(ports.len(), 2, "Cluster {} should have exactly 2 boundary ports, got {:?}", hub, ports);
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
    assert_eq!(partition.connectors.len(), 10, "Expected exactly 10 connectors");
    
    let total_covered: usize = partition.clusters.values().map(|c| c.len()).sum::<usize>()
        + partition.connectors.len()
        + partition.super_hubs.len();
    assert_eq!(total_covered, 6620, "All 6620 vertices must be partitioned");

    for (&hub, c) in &partition.clusters {
        assert_eq!(c.len(), 660, "Cluster {} size must be exactly 660", hub);
        let ports = &partition.boundary_ports[&hub];
        assert_eq!(ports.len(), 2, "Cluster {} should have exactly 2 boundary ports, got {:?}", hub, ports);
    }
}

#[test]
fn test_partition_rejects_non_bipartite() {
    let g1_path = find_graph("graph1.col");
    let g1 = file_operations::parse_graph_from_file(&g1_path).expect("Failed to parse graph1");
    assert!(!dynamic_bipartite::can_solve_bipartite(&g1), "graph1 must not be detected as bipartite");
    assert!(dynamic_bipartite::detect_and_partition(&g1).is_none());

    let g710_path = find_graph("graph710.col");
    let g710 = file_operations::parse_graph_from_file(&g710_path).expect("Failed to parse graph710");
    assert!(!dynamic_bipartite::can_solve_bipartite(&g710), "graph710 must not be detected as bipartite");
    assert!(dynamic_bipartite::detect_and_partition(&g710).is_none());
}

#[test]
fn test_partition_graph963_to_990_dynamic() {
    for &(gid, expected_n, expected_cluster_size) in &[
        (963, 7020, 700),
        (975, 7420, 740),
        (982, 7620, 760),
        (990, 8020, 800),
    ] {
        let path = find_graph(&format!("graph{}.col", gid));
        let g = file_operations::parse_graph_from_file(&path).expect("Failed to parse graph");
        assert!(dynamic_bipartite::can_solve_bipartite(&g), "graph{} should be detected dynamically", gid);
        let p = dynamic_bipartite::detect_and_partition(&g).expect("Failed to partition");
        assert_eq!(p.super_hubs.len(), 10);
        assert_eq!(p.clusters.len(), 10);
        assert_eq!(p.connectors.len(), 10);
        let total_covered: usize = p.clusters.values().map(|c| c.len()).sum::<usize>()
            + p.connectors.len()
            + p.super_hubs.len();
        assert_eq!(total_covered, expected_n);
        for (&hub, c) in &p.clusters {
            assert_eq!(c.len(), expected_cluster_size, "Cluster {} size mismatch on graph{}", hub, gid);
            assert_eq!(p.boundary_ports[&hub].len(), 2);
        }
    }
}

fn permute_graph(raw: &cegar_fix::core::graph::Graph) -> cegar_fix::core::graph::Graph {
    let mut nodes: Vec<i32> = raw.adjacency_list.keys().copied().collect();
    nodes.sort_unstable();
    let n = nodes.len();
    let mut perm_nodes = nodes.clone();
    let mut seed: u64 = 42;
    for i in (1..n).rev() {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let j = (seed % (i as u64 + 1)) as usize;
        perm_nodes.swap(i, j);
    }
    let map: std::collections::HashMap<i32, i32> = nodes.into_iter().zip(perm_nodes).collect();
    let mut perm_g = cegar_fix::core::graph::Graph::new();
    for (&u, nbrs) in &raw.adjacency_list {
        let pu = map[&u];
        for &v in nbrs {
            if u < v {
                let pv = map[&v];
                perm_g.add_edge(pu, pv);
            }
        }
    }
    perm_g
}

#[test]
fn test_partition_permuted_graph746() {
    let path = find_graph("graph746.col");
    let orig_g = file_operations::parse_graph_from_file(&path).expect("Failed to parse graph746");
    let g = permute_graph(&orig_g);
    assert!(dynamic_bipartite::can_solve_bipartite(&g), "Permuted graph must be detected as bipartite");
    let p = dynamic_bipartite::detect_and_partition(&g).expect("Failed to partition permuted graph");
    
    assert_eq!(p.super_hubs.len(), 5);
    assert_eq!(p.clusters.len(), 5);
    assert_eq!(p.connectors.len(), 6);
    let total_covered: usize = p.clusters.values().map(|c| c.len()).sum::<usize>()
        + p.connectors.len()
        + p.super_hubs.len();
    assert_eq!(total_covered, 4286);
    for (&hub, c) in &p.clusters {
        assert_eq!(c.len(), 855);
        assert_eq!(p.boundary_ports[&hub].len(), 2);
    }
}



