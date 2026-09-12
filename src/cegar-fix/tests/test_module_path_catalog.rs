use std::collections::{HashMap, HashSet};
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::file_operations;
use cegar_fix::modular_ring_dp_solver::{ModularRingDecomposer, ModulePathCatalogExtractor};

#[test]
fn test_module_catalog_extraction() {
    let graph_path = "FHCPCS-col/graph868.col";
    let alt_graph_path = "../../FHCPCS-col/graph868.col";
    let path = if std::path::Path::new(graph_path).exists() {
        graph_path
    } else if std::path::Path::new(alt_graph_path).exists() {
        alt_graph_path
    } else {
        "/home/ubuntu/HCP/FHCPCS-col/graph868.col"
    };

    let raw_g = file_operations::input_to_graph(path);
    let (g, contractor) = Degree2Contractor::contract(&raw_g);
    let modules = ModularRingDecomposer::decompose(&g, &contractor).unwrap();

    let m0 = &modules[0];
    let catalog = ModulePathCatalogExtractor::extract_catalog(m0, &g, &contractor);

    assert!(!catalog.is_empty(), "Catalog for module 0 must contain at least 1 valid configuration");

    let mod_set: HashSet<i32> = m0.vertices.iter().copied().collect();

    for cfg in &catalog {
        assert!(m0.ports_in.contains(&cfg.port_in), "port_in must belong to ports_in");
        assert!(m0.ports_out.contains(&cfg.port_out), "port_out must belong to ports_out");

        // Verify that internal_real_edges + virtual_edges forms a single path covering all 88 vertices
        let mut adj: HashMap<i32, Vec<i32>> = HashMap::new();
        for &(u, w) in &m0.virtual_edges {
            adj.entry(u).or_default().push(w);
            adj.entry(w).or_default().push(u);
        }
        for &(u, v) in &cfg.internal_real_edges {
            assert!(mod_set.contains(&u) && mod_set.contains(&v), "Internal edge must stay inside module");
            adj.entry(u).or_default().push(v);
            adj.entry(v).or_default().push(u);
        }

        // Check degrees: port_in and port_out have deg 1, all other 86 vertices have deg 2
        for &v in &m0.vertices {
            let d = adj.get(&v).map_or(0, |nbrs| nbrs.len());
            if v == cfg.port_in || v == cfg.port_out {
                assert_eq!(d, 1, "Port endpoints must have internal degree 1");
            } else {
                assert_eq!(d, 2, "Internal vertices must have internal degree 2");
            }
        }

        // Check path continuity from port_in to port_out
        let mut visited = HashSet::new();
        let mut curr = cfg.port_in;
        visited.insert(curr);
        while curr != cfg.port_out {
            let nxts = adj.get(&curr).unwrap();
            let unvisited: Vec<i32> = nxts.iter().copied().filter(|x| !visited.contains(x)).collect();
            assert_eq!(unvisited.len(), 1, "Must have exactly 1 unvisited neighbor along the path");
            curr = unvisited[0];
            visited.insert(curr);
        }
        assert_eq!(visited.len(), 88, "Path must visit all 88 vertices in module 0");
    }
}
