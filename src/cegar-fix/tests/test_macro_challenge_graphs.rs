use cegar_fix::solver_pipeline::solve_single_graph;
use std::path::Path;

fn find_graph(gid: usize) -> String {
    let candidates = [
        format!("FHCPCS-col/graph{}.col", gid),
        format!("../../FHCPCS-col/graph{}.col", gid),
        format!("/home/ubuntu/HCP/FHCPCS-col/graph{}.col", gid),
    ];
    for p in &candidates {
        if Path::new(p).exists() {
            return p.clone();
        }
    }
    panic!("graph{}.col not found", gid);
}

#[test]
fn test_macro_corridor_graph710() {
    let path = find_graph(710);
    let res = solve_single_graph(&path, 600.0, None);
    assert!(res.is_ok(), "solve_single_graph failed on graph710: {:?}", res.err());
    let (tour, elapsed, v_count) = res.unwrap();
    assert_eq!(v_count, 4064);
    assert_eq!(tour.len(), 4064);
    println!("Pipeline certified graph710 in {:.2}s", elapsed);
}
