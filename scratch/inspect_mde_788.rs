use std::time::Instant;
use cegar_fix::bipartite_module_detector::BipartiteModuleDetector;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::file_operations;
use cegar_fix::module_dual_path_extractor::ModuleDualPathExtractor;

fn main() {
    let raw_g = file_operations::input_to_graph("FHCPCS-col/graph788.col");
    let (g, contractor) = Degree2Contractor::contract(&raw_g);

    println!("Graph 788: raw N={}, contracted N={}, virtual edges={}", raw_g.adjacency_list.len(), g.adjacency_list.len(), contractor.chain_map.len() / 2);

    let t0 = Instant::now();
    let modules = BipartiteModuleDetector::detect_44_modules(&g, &contractor);
    println!("Detected {} modules in {:?}", modules.len(), t0.elapsed());

    let mut extracted_count = 0;
    for (idx, m) in modules.iter().enumerate() {
        if let Some((t_path, f_path)) = ModuleDualPathExtractor::extract_dual_paths(&m.vertices, &g, &contractor) {
            extracted_count += 1;
            if idx < 3 {
                println!("  Module {}: size={}, T_path len={}, F_path len={}", idx, m.vertices.len(), t_path.len(), f_path.len());
            }
        }
    }
    println!("Successfully extracted dual paths for {}/{} modules!", extracted_count, modules.len());
}
