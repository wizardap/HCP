use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::File;
use std::io::{BufRead, BufReader};

fn main() {
    let file = File::open("FHCPCS-col/graph668.col").expect("open graph668.col failed");
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

    let hubs: Vec<i32> = adj.iter().filter(|(_, nbrs)| nbrs.len() == 14).map(|(&v, _)| v).collect();
    let hub_set: HashSet<i32> = hubs.iter().copied().collect();

    // Partition non-hub vertices by closest hub (Voronoi partition on graph)
    let mut closest_hub: HashMap<i32, usize> = HashMap::new();
    let mut q = VecDeque::new();

    for (idx, &h) in hubs.iter().enumerate() {
        closest_hub.insert(h, idx);
        q.push_back(h);
    }

    while let Some(curr) = q.pop_front() {
        let h_idx = closest_hub[&curr];
        if let Some(nbrs) = adj.get(&curr) {
            for &nxt in nbrs {
                if !closest_hub.contains_key(&nxt) {
                    closest_hub.insert(nxt, h_idx);
                    q.push_back(nxt);
                }
            }
        }
    }

    let mut cluster_sizes: HashMap<usize, usize> = HashMap::new();
    for &h_idx in closest_hub.values() {
        *cluster_sizes.entry(h_idx).or_default() += 1;
    }

    let sizes: Vec<usize> = cluster_sizes.values().copied().collect();
    println!("60 Hub Voronoi Clusters: count = {}", sizes.len());
    println!("Cluster sizes: min={}, max={}, avg={}", sizes.iter().min().unwrap(), sizes.iter().max().unwrap(), sizes.iter().sum::<usize>() / sizes.len());
    println!("Sizes distribution: {:?}", sizes);
}
