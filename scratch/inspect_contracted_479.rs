use cegar_fix::graph::Graph;
use cegar_fix::contraction::Degree2Contractor;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::collections::{HashMap, HashSet};

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
    let pruned = g.prune_degree2_triangles();
    println!("Pruned: {}", pruned);
    let (cg, contractor) = Degree2Contractor::contract(&g);
    println!("Contracted graph: N = {}, M = {}", cg.adjacency_list.len(), cg.arcs.len() / 2);
    println!("Protected chains: {}", contractor.chain_map.len());
}
