use std::time::Instant;
use cegar_fix::file_operations;
use cegar_fix::two_tier_decomposer::decompose_graph;
use cegar_fix::hub_registry::HubRegistry;
use cegar_fix::contraction::Degree2Contractor;

fn main() {
    let raw_g = file_operations::input_to_graph("FHCPCS-col/graph678.col");
    let (g, _contractor) = Degree2Contractor::contract(&raw_g);
    let hub_registry = HubRegistry::new(&g);

    println!("Graph 678: raw N={}, contracted N={}, hubs={}", raw_g.adjacency_list.len(), g.adjacency_list.len(), hub_registry.hub_vertices.len());

    let t0 = Instant::now();
    let decomp = decompose_graph(&g);
    println!("Decomposed in {:?}: all_hubs={}, strips={}", t0.elapsed(), decomp.all_hubs.len(), decomp.strips.len());
}
