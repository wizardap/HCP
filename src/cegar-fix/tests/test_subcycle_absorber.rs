// tests/test_subcycle_absorber.rs
use cegar_fix::graph::Graph;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::hub_registry::HubRegistry;
use cegar_fix::subcycle_absorber::SubcycleAbsorber;
use std::collections::HashMap;

fn empty_contractor() -> Degree2Contractor {
    Degree2Contractor {
        chain_map: HashMap::new(),
        original_vertices_count: 0,
        contracted_vertices_count: 0,
        is_direct_cycle: None,
        is_infeasible: false,
    }
}

fn empty_hub_registry() -> HubRegistry {
    HubRegistry {
        is_hub: Vec::new(),
        hub_vertices: Vec::new(),
        hub_neighbors: HashMap::new(),
        min_hub_degree: 3,
    }
}

#[test]
fn test_absorb_small_cycle_into_giant_cycle() {
    let mut g = Graph::new();
    // Giant cycle: 1 - 2 - 3 - 4 - 5 - 6 - 1
    // Small cycle: 7 - 8 - 9 - 7
    // Bridge edges: (2, 7) and (9, 3)
    let edges = vec![
        (1, 2), (2, 3), (3, 4), (4, 5), (5, 6), (6, 1),
        (7, 8), (8, 9), (9, 7),
        (2, 7), (9, 3),
    ];
    for &(u, v) in &edges {
        g.add_edge(u, v);
    }

    let contractor = empty_contractor();
    let hub_reg = empty_hub_registry();

    let cycles = vec![
        vec![1, 2, 3, 4, 5, 6],
        vec![7, 8, 9],
    ];

    let absorbed = SubcycleAbsorber::absorb_subcycles(&cycles, &g, &contractor, &hub_reg);
    assert_eq!(absorbed.len(), 1);
    assert_eq!(absorbed[0].len(), 9);
}
