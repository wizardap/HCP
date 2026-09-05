use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::File;
use std::io::{BufRead, BufReader};

fn main() {
    let file = File::open("FHCPCS-col/graph668.col").expect("open graph668.col failed");
    let reader = BufReader::new(file);

    let mut adj: HashMap<i32, Vec<i32>> = HashMap::new();
    let mut num_nodes = 0;
    let mut num_edges = 0;

    for line in reader.lines() {
        let l = line.unwrap();
        if l.starts_with("p edge") {
            let parts: Vec<&str> = l.split_whitespace().collect();
            num_nodes = parts[2].parse().unwrap();
            num_edges = parts[3].parse().unwrap();
        } else if l.starts_with('e') {
            let parts: Vec<&str> = l.split_whitespace().collect();
            let u: i32 = parts[1].parse().unwrap();
            let v: i32 = parts[2].parse().unwrap();
            adj.entry(u).or_default().push(v);
            adj.entry(v).or_default().push(u);
        }
    }

    println!("Graph668: N = {}, M = {}", num_nodes, num_edges);

    // Degree distribution
    let mut deg_dist: HashMap<usize, usize> = HashMap::new();
    for (_v, nbrs) in &adj {
        *deg_dist.entry(nbrs.len()).or_default() += 1;
    }
    println!("Degree distribution: {:?}", deg_dist);

    // Triangles and 4-cycles
    let mut triangle_count = 0;
    let mut quad_count = 0;
    for (&u, nbrs_u) in &adj {
        let set_u: HashSet<i32> = nbrs_u.iter().copied().collect();
        for &v in nbrs_u {
            if u < v {
                if let Some(nbrs_v) = adj.get(&v) {
                    let common = nbrs_v.iter().filter(|w| set_u.contains(w)).count();
                    triangle_count += common;
                }
            }
        }
    }
    println!("Triangles (x3): {}", triangle_count);
}
