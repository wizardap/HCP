use cegar_fix::file_operations;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::quotient_block_cutter::QuotientBlockCutter;
use std::collections::HashMap;

fn main() {
    let raw_g = file_operations::input_to_graph("FHCPCS-col/graph678.col");
    let (g, contractor) = Degree2Contractor::contract(&raw_g);
    let blocks = QuotientBlockCutter::detect_modular_blocks(&g, &contractor);
    println!("Graph 678: {} modular blocks detected", blocks.len());
    let mut size_hist = HashMap::new();
    for b in &blocks {
        *size_hist.entry(b.len()).or_insert(0) += 1;
    }
    println!("Block size distribution: {:?}", size_hist);
}
