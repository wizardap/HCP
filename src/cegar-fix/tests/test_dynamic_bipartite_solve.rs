use cegar_fix::core::file_operations;
use cegar_fix::core::tour_verifier::TourVerifier;
use cegar_fix::macro_decomp::dynamic_bipartite;
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

#[test]
fn test_solve_graph746_dynamic_de_novo() {
    let graph_path = find_graph("graph746.col");
    let g = file_operations::parse_graph_from_file(&graph_path).expect("Failed to parse graph746");
    assert!(dynamic_bipartite::can_solve_bipartite(&g));

    let tour = dynamic_bipartite::solve_bipartite(&g, 240.0)
        .expect("solve_bipartite should solve graph746 within 240s");

    assert_eq!(tour.len(), 4286);
    let (valid, err) = TourVerifier::verify(&g, &tour);
    assert!(valid, "TourVerifier failed: {}", err);

    let is_ham = TourVerifier::is_hamiltonian(&g, &tour);
    assert!(is_ham, "TourVerifier::is_hamiltonian returned false");

    let upstream_valid = TourVerifier::verify_upstream_python(&graph_path, &tour)
        .expect("Failed to run upstream is_hamiltonian.py");
    assert!(upstream_valid, "Upstream is_hamiltonian.py returned False");
}

#[test]
fn test_solve_all_clusters() {
    let graph_path = find_graph("graph746.col");
    let g = file_operations::parse_graph_from_file(&graph_path).expect("Failed to parse graph746");
    let partition = dynamic_bipartite::detect_and_partition(&g).expect("Failed to partition");
    let mut macro_solver = dynamic_bipartite::MacroSatSolver::new(&partition, &g).expect("MacroSatSolver");
    let config = macro_solver.solve_next_configuration().expect("Config");
    println!("Config has {} clusters", config.cluster_ports.len());

    let active_macro_nodes: std::collections::HashSet<i32> = config
        .active_macro_edges
        .iter()
        .flat_map(|&(u, v)| [u, v])
        .collect();

    let h = 1430;
    let (u_in, u_out) = config.cluster_ports[&h];
    let mut nodes = partition.clusters[&h].clone();
    if active_macro_nodes.contains(&h) && h != u_in && h != u_out {
        nodes.remove(&h);
    }
    println!("Solving cluster {} ({} nodes) from {} to {}...", h, nodes.len(), u_in, u_out);
    let t0 = std::time::Instant::now();
    let res = dynamic_bipartite::solve_cluster_path(
        h,
        u_in,
        u_out,
        &nodes,
        &g.adjacency_list,
        std::time::Instant::now() + std::time::Duration::from_secs(300),
    );
    println!("Cluster {} finished in {:.2}s, res={}", h, t0.elapsed().as_secs_f64(), res.is_some());
    assert!(res.is_some());
}
