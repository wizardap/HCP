use cegar_fix::graph::Graph;
use cegar_fix::hub_hierarchical_decomposer::{HubHierarchicalDecomposer, HubModule};
use cegar_fix::tour_verifier::TourVerifier;
use std::collections::HashSet;

/// Builds a synthetic graph with 4 high-degree hubs and attached ladder modules.
/// Hubs: 1, 2, 3, 4 (each with degree >= 10).
/// Module i has hub i and ladder rungs (a_{i, j}, b_{i, j}) for j = 1..5.
/// External connections link Module 1 -> Module 2 -> Module 3 -> Module 4 -> Module 1.
fn build_synthetic_multi_hub_graph() -> Graph {
    let mut g = Graph::new();
    let k = 5; // number of rungs per ladder

    for i in 1..=4 {
        let hub = i;
        let base_a = i * 100 + 10; // 110, 210, 310, 410
        let base_b = i * 100 + 20; // 120, 220, 320, 420

        // Ladder rails and rungs
        for j in 1..=k {
            let a_j = base_a + j;
            let b_j = base_b + j;

            // Rung edge
            g.add_edge(a_j, b_j);

            // Rail edges
            if j < k {
                g.add_edge(a_j, base_a + j + 1);
                g.add_edge(b_j, base_b + j + 1);
            }

            // Chords to hub to ensure degree(hub) >= 10
            g.add_edge(hub, a_j);
            g.add_edge(hub, b_j);
        }
    }

    // Connect modules in a ring via their boundary interface ports (a_{i, k}, b_{i, k})
    // Module 1 exit (125) -> Module 2 entry (215)
    g.add_edge(125, 215);
    // Module 2 exit (225) -> Module 3 entry (315)
    g.add_edge(225, 315);
    // Module 3 exit (325) -> Module 4 entry (415)
    g.add_edge(325, 415);
    // Module 4 exit (425) -> Module 1 entry (115)
    g.add_edge(425, 115);

    g
}

#[test]
fn test_extract_hub_modules_synthetic() {
    let g = build_synthetic_multi_hub_graph();
    let min_hub_degree = 10;

    let modules: Vec<HubModule> = HubHierarchicalDecomposer::extract_hub_modules(&g, min_hub_degree);
    assert_eq!(modules.len(), 4, "Expected exactly 4 hub modules");

    let mut hub_ids: Vec<i32> = modules.iter().map(|m| m.hub_id).collect();
    hub_ids.sort_unstable();
    assert_eq!(hub_ids, vec![1, 2, 3, 4], "Hub IDs must match 1, 2, 3, 4");

    let mut all_module_vertices = HashSet::new();
    for m in &modules {
        // Each module has 1 hub + 2 * 5 ladder vertices = 11 vertices
        assert_eq!(m.vertices.len(), 11, "Module {} should have 11 vertices", m.hub_id);
        assert!(m.vertices.contains(&m.hub_id), "Module {} must contain hub {}", m.hub_id, m.hub_id);

        for &v in &m.vertices {
            assert!(all_module_vertices.insert(v), "Duplicate vertex {} across modules", v);
        }

        // Interface ports should be exactly the boundary ports connected externally
        assert_eq!(m.interface_ports.len(), 2, "Module {} should have 2 interface ports", m.hub_id);
        let base_a = m.hub_id * 100 + 10 + 5;
        let base_b = m.hub_id * 100 + 20 + 5;
        assert!(m.interface_ports.contains(&base_a), "Interface ports must contain {}", base_a);
        assert!(m.interface_ports.contains(&base_b), "Interface ports must contain {}", base_b);

        // Internal paths must exist between interface ports
        assert!(!m.internal_paths.is_empty(), "Module {} must have internal paths", m.hub_id);
        for (entry, exit, path) in &m.internal_paths {
            assert_eq!(path.len(), m.vertices.len(), "Path length must match module vertices");
            assert_eq!(path[0], *entry, "Path start must match entry port");
            assert_eq!(path[path.len() - 1], *exit, "Path end must match exit port");

            let path_set: HashSet<i32> = path.iter().copied().collect();
            let vert_set: HashSet<i32> = m.vertices.iter().copied().collect();
            assert_eq!(path_set, vert_set, "Path must span all module vertices");
        }
    }

    assert_eq!(all_module_vertices.len(), g.adjacency_list.len(), "Modules must partition all vertices");
}

#[test]
fn test_try_solve_hierarchical_synthetic() {
    let g = build_synthetic_multi_hub_graph();

    let tour_opt = HubHierarchicalDecomposer::try_solve_hierarchical(&g);
    assert!(tour_opt.is_some(), "Expected hierarchical decomposer to find Hamiltonian tour");

    let tour = tour_opt.unwrap();
    assert_eq!(tour.len(), g.adjacency_list.len(), "Tour length must equal total vertices in G");

    let verify_res = TourVerifier::verify_raw_tour(&tour, &g);
    assert!(verify_res.is_ok(), "Tour must be verified by TourVerifier: {:?}", verify_res.err());
}

#[test]
fn test_hierarchical_decomposer_infeasible() {
    let mut g = Graph::new();
    // Disconnected graph: two separate 6-cycles
    for i in 1..=6 {
        g.add_edge(i, if i == 6 { 1 } else { i + 1 });
    }
    for i in 11..=16 {
        g.add_edge(i, if i == 16 { 11 } else { i + 1 });
    }

    let tour_opt = HubHierarchicalDecomposer::try_solve_hierarchical(&g);
    assert!(tour_opt.is_none(), "Disconnected graph must return None");
}
