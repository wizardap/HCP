use std::fs::File;
use std::io::{BufRead, BufReader};
use std::collections::{HashMap, HashSet};

fn main() {
    let file = File::open("FHCPCS-col/graph479.col").expect("open graph479.col failed");
    let reader = BufReader::new(file);

    let mut adj: HashMap<i32, Vec<i32>> = HashMap::new();
    for line in reader.lines() {
        let l = line.unwrap();
        if l.starts_with('e') {
            let parts: Vec<&str> = l.split_whitespace().collect();
            let u: i32 = parts[1].parse().unwrap();
            let v: i32 = parts[2].parse().unwrap();
            adj.entry(u).or_default().push(v);
            adj.entry(v).or_default().push(u);
        }
    }

    println!("Graph479 total vertices: {}", adj.len());
    let mut deg_dist: HashMap<usize, usize> = HashMap::new();
    for (_v, nbrs) in &adj {
        *deg_dist.entry(nbrs.len()).or_default() += 1;
    }
    println!("Degree distribution: {:?}", deg_dist);
}
