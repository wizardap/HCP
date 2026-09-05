use std::collections::{HashMap, HashSet};
use cegar_fix::graph::Graph;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::bipartite_module_detector::BipartiteModuleDetector;
use cegar_fix::module_dual_path_extractor::ModuleDualPathExtractor;

fn main() {
    let raw_g = Graph::from_col_file("FHCPCS-col/graph479.col").expect("Failed to read graph479");
    let (contracted_g, contractor) = Degree2Contractor::contract(&raw_g);

    println!("Contracted graph: {} vertices", contracted_g.adjacency_list.len());
    let modules = BipartiteModuleDetector::detect_44_modules(&contracted_g, &contractor);
    println!("Detected {} modules", modules.len());

    let mut successful_dual = 0;
    for (i, m) in modules.iter().enumerate() {
        if let Some((t_path, f_path)) = ModuleDualPathExtractor::extract_dual_paths(&m.vertices, &contracted_g, &contractor) {
            successful_dual += 1;
            if i < 3 {
                println!("Module {}: T len={}, F len={}", i, t_path.len(), f_path.len());
            }
        } else {
            println!("Module {} failed to extract dual paths (size {})", i, m.vertices.len());
        }
    }
    println!("Total successful dual-path modules: {} / {}", successful_dual, modules.len());
}
