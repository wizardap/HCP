use std::collections::HashSet;
use cegar_fix::graph::Graph;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::bipartite_module_detector::BipartiteModuleDetector;

#[test]
fn test_detect_44_modules_synthetic() {
    let mut g = Graph::new();
    let mut contractor = Degree2Contractor::new();

    // Construct 2 synthetic modules M0 (nodes 1..=44) and M1 (nodes 45..=88)
    // In M0: 22 virtual edges (1, 2), (3, 4), ..., (43, 44)
    for i in (1..=43).step_by(2) {
        let u = i;
        let v = i + 1;
        contractor.chain_map.insert((u, v), vec![1000 + i]);
        contractor.chain_map.insert((v, u), vec![1000 + i]);
        // Dense internal connectivity inside M0
        g.add_edge(v, (v % 44) + 1);
        g.add_edge(u, ((u + 2) % 44) + 1);
        g.add_edge(v, ((v + 6) % 44) + 1);
        g.add_edge(u, ((u + 10) % 44) + 1);
    }

    // In M1: 22 virtual edges (45, 46), ..., (87, 88)
    for i in (45..=87).step_by(2) {
        let u = i;
        let v = i + 1;
        contractor.chain_map.insert((u, v), vec![2000 + i]);
        contractor.chain_map.insert((v, u), vec![2000 + i]);
        // Dense internal connectivity inside M1
        let next_v = if v == 88 { 45 } else { v + 1 };
        g.add_edge(v, next_v);
        let next_u = 45 + ((u - 45 + 2) % 44);
        g.add_edge(u, next_u);
        let chord_v = 45 + ((v - 45 + 6) % 44);
        g.add_edge(v, chord_v);
        let chord_u = 45 + ((u - 45 + 10) % 44);
        g.add_edge(u, chord_u);
    }

    // Cross edges between M0 and M1 (only 2 boundary edges)
    g.add_edge(44, 45);
    g.add_edge(88, 1);

    let modules = BipartiteModuleDetector::detect_44_modules(&g, &contractor);
    assert_eq!(modules.len(), 2, "Must detect exactly 2 modules of 44 vertices");
    assert_eq!(modules[0].vertices.len(), 44);
    assert_eq!(modules[1].vertices.len(), 44);

    let mod0_nodes: HashSet<i32> = modules[0].vertices.iter().copied().collect();
    let mod1_nodes: HashSet<i32> = modules[1].vertices.iter().copied().collect();
    let has_m0 = mod0_nodes.contains(&1) || mod1_nodes.contains(&1);
    let has_m1 = mod0_nodes.contains(&45) || mod1_nodes.contains(&45);
    assert!(has_m0 && has_m1, "Both modules must be detected");
    if mod0_nodes.contains(&1) {
        assert!(mod0_nodes.contains(&44));
        assert!(mod1_nodes.contains(&45) && mod1_nodes.contains(&88));
    } else {
        assert!(mod1_nodes.contains(&44));
        assert!(mod0_nodes.contains(&45) && mod0_nodes.contains(&88));
    }
}
