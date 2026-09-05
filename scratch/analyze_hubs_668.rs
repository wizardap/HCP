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
    println!("Found {} hubs of degree 14: {:?}", hubs.len(), hubs);

    // Check distances between hubs
    let hub_set: HashSet<i32> = hubs.iter().copied().collect();
    let mut hub_to_hub_edges = 0;
    for &h in &hubs {
        if let Some(nbrs) = adj.get(&h) {
            for &n in nbrs {
                if hub_set.contains(&n) && h < n {
                    hub_to_hub_edges += 1;
                }
            }
        }
    }
    println!("Direct edges between hubs: {}", hub_to_hub_edges);

    // BFS distance between hubs
    for &h in &hubs[..5] {
        let mut dist: HashMap<i32, usize> = HashMap::new();
        let mut q = VecDeque::new();
        dist.insert(h, 0);
        q.push_back(h);

        let mut other_hub_dist: Vec<usize> = Vec::new();
        while let Some(curr) = q.pop_front() {
            let d = dist[&curr];
            if hub_set.contains(&curr) && curr != h {
                other_hub_dist.push(d);
            }
            if let Some(nbrs) = adj.get(&curr) {
                for &nxt in nbrs {
                    if !dist.contains_key(&nxt) {
                        dist.insert(nxt, d + 1);
                        q.push_back(nxt);
                    }
                }
            }
        }
        println!("Hub {} distance to other hubs (min/max/avg): min={}, max={}", h, other_hub_dist.iter().min().unwrap(), other_hub_dist.iter().max().unwrap());
    }
}
