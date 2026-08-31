use std::collections::HashSet;
use cegar_fix::graph::Graph;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::module_dual_path_extractor::ModuleDualPathExtractor;

#[test]
fn test_module_dual_path_extractor_synthetic() {
    let mut g = Graph::new();
    let mut contractor = Degree2Contractor::new();

    // Small module of 6 vertices: 1, 2, 3, 4, 5, 6
    // Virtual edges: (1, 2), (3, 4), (5, 6)
    contractor.chain_map.insert((1, 2), vec![101]);
    contractor.chain_map.insert((2, 1), vec![101]);
    contractor.chain_map.insert((3, 4), vec![102]);
    contractor.chain_map.insert((4, 3), vec![102]);
    contractor.chain_map.insert((5, 6), vec![103]);
    contractor.chain_map.insert((6, 5), vec![103]);

    // Real internal edges:
    // Path 1 (True): 1 - 2 - 3 - 4 - 5 - 6
    // Edges: (2, 3), (4, 5)
    g.add_edge(2, 3);
    g.add_edge(4, 5);

    // Path 2 (False): 1 - 4 - 3 - 6 - 5 - 2 (or alternative crossing 1-6-5-4-3-2)
    // Add real edges: (1, 4), (3, 6)
    g.add_edge(1, 4);
    g.add_edge(3, 6);

    let mod_vertices = vec![1, 2, 3, 4, 5, 6];
    let dual_paths = ModuleDualPathExtractor::extract_dual_paths(&mod_vertices, &g, &contractor);

    assert!(dual_paths.is_some(), "Must extract dual paths for module");
    let (t_path, f_path) = dual_paths.unwrap();

    assert_eq!(t_path.len(), 6, "True path must visit all 6 vertices");
    assert_eq!(f_path.len(), 6, "False path must visit all 6 vertices");

    // Both paths must visit all vertices uniquely
    let set_t: HashSet<i32> = t_path.iter().copied().collect();
    let set_f: HashSet<i32> = f_path.iter().copied().collect();
    assert_eq!(set_t.len(), 6);
    assert_eq!(set_f.len(), 6);
}
