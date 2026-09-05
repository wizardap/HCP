use cegar_fix::file_operations;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::hub_registry::HubRegistry;
use cegar_fix::hybrid_orchestrator::{HybridOptions, HybridOrchestrator};
use std::time::Instant;

fn main() {
    let g = file_operations::input_to_graph("FHCPCS-col/graph678.col");
    let opts = HybridOptions {
        auto_mode: true,
        timeout_secs: 400.0,
        output_tour: None,
    };
    println!("Starting debug run on graph678...");
    let res = HybridOrchestrator::solve(&g, &opts);
    println!("Result: {:?}", res.is_some());
}
