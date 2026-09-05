use cegar_fix::graph::Graph;
use cegar_fix::contraction::Degree2Contractor;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::collections::HashMap;

fn main() {
    let file = File::open("FHCPCS-col/graph479.col").expect("open graph479.col failed");
    let reader = BufReader::new(file);

    let mut g = Graph::new();
    for line in reader.lines() {
        let l = line.unwrap();
        if l.starts_with('e') {
            let parts: Vec<&str> = l.split_whitespace().collect();
            let u: i32 = parts[1].parse().unwrap();
            let v: i32 = parts[2].parse().unwrap();
            g.add_edge(u, v);
        }
    }

    let contractor = Degree2Contractor::contract(&g);
    let cg = &contractor.contracted_graph;
    println!("Contracted graph: N = {}", cg.adjacency_list.len());

    let mut deg_dist: HashMap<usize, usize> = HashMap::new();
    for (_v, nbrs) in &cg.adjacency_list {
        *deg_dist.entry(nbrs.len()).or_default() += 1;
    }
    println!("Contracted degree distribution: {:?}", deg_dist);
    println!("Protected chains (edges): {}", contractor.chain_map.len());
}
