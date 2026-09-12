use std::collections::HashSet;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::file_operations;
use cegar_fix::modular_ring_dp_solver::ModularRingDecomposer;

#[test]
fn test_42_module_decomposition_graph868() {
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

    assert_eq!(g.adjacency_list.len(), 3696);
    assert_eq!(contractor.chain_map.len() / 2, 1848);

    let modules = ModularRingDecomposer::decompose(&g, &contractor)
        .expect("Decomposition must succeed for graph868");

    assert_eq!(modules.len(), 42, "Must decompose into exactly 42 modules");

    let mut seen_vertices = HashSet::new();
    let mut seen_ve = HashSet::new();

    for (i, m) in modules.iter().enumerate() {
        assert_eq!(m.id, i);
        assert_eq!(m.vertices.len(), 88, "Module {} must have 88 vertices", i);
        assert_eq!(m.virtual_edges.len(), 44, "Module {} must have 44 virtual edges", i);
        assert!(!m.ports_in.is_empty(), "Module {} must have ports_in", i);
        assert!(!m.ports_out.is_empty(), "Module {} must have ports_out", i);

        for &v in &m.vertices {
            assert!(seen_vertices.insert(v), "Duplicate vertex {} in module {}", v, i);
        }
        for &(u, w) in &m.virtual_edges {
            let ve = (u.min(w), u.max(w));
            assert!(seen_ve.insert(ve), "Duplicate VE {:?} in module {}", ve, i);
        }
    }

    assert_eq!(seen_vertices.len(), 3696);
    assert_eq!(seen_ve.len(), 1848);

    // Verify circular connectivity: ports_out of M_i connect to ports_in of M_{(i+1)%42}
    for i in 0..42 {
        let next_i = (i + 1) % 42;
        let next_in: HashSet<i32> = modules[next_i].ports_in.iter().copied().collect();
        let mut has_link = false;
        for &p_out in &modules[i].ports_out {
            if let Some(nbrs) = g.adjacency_list.get(&p_out) {
                for &nxt in nbrs {
                    if next_in.contains(&nxt) {
                        has_link = true;
                        break;
                    }
                }
            }
            if has_link { break; }
        }
        assert!(has_link, "Module {} must connect to module {} along the ring", i, next_i);
    }
}
