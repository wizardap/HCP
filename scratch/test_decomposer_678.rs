use std::time::Instant;
use cegar_fix::file_operations;
use cegar_fix::hub_hierarchical_decomposer::HubHierarchicalDecomposer;
use cegar_fix::two_tier_orchestrator::TwoTierOrchestrator;
use cegar_fix::hub_registry::HubRegistry;
use cegar_fix::contraction::Degree2Contractor;

fn main() {
    let raw_g = file_operations::input_to_graph("FHCPCS-col/graph678.col");
    let (g, contractor) = Degree2Contractor::contract(&raw_g);
    let hub_registry = HubRegistry::from_graph(&g);

    println!("Graph 678: raw N={}, contracted N={}, hubs={}", raw_g.adjacency_list.len(), g.adjacency_list.len(), hub_registry.hub_vertices.len());

    let t0 = Instant::now();
    let decomp = TwoTierOrchestrator::decompose_hub_ladder(&g, &contractor, &hub_registry);
    println!("Decomposed in {:?}: is_valid={}", t0.elapsed(), decomp.is_some());
    if let Some(d) = decomp {
        println!("  All hubs count: {}", d.all_hubs.len());
        println!("  Strips count: {}", d.strips.len());
    }
}
