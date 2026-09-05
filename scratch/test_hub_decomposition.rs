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

    println!("Total nodes: {}, Hubs: {}", adj.len(), hubs.len());

    // For each hub, find vertices that only connect to this hub and other local vertices
    let mut hub_modules: Vec<(i32, Vec<i32>)> = Vec::new();
    let mut claimed: HashSet<i32> = HashSet::new();

    for &h in &hubs {
        let mut local_nodes = Vec::new();
        if let Some(nbrs) = adj.get(&h) {
            for &n in nbrs {
                if !hub_set.contains(&n) && !claimed.contains(&n) {
                    local_nodes.push(n);
                }
            }
        }
        // Expand 1 hop to get the full 16-vertex module around hub h
        let mut full_module = local_nodes.clone();
        for &u in &local_nodes {
            if let Some(nbrs_u) = adj.get(&u) {
                for &v in nbrs_u {
                    if !hub_set.contains(&v) && !claimed.contains(&v) && !full_module.contains(&v) {
                        full_module.push(v);
                    }
                }
            }
        }
        if full_module.len() >= 8 && full_module.len() <= 32 {
            for &v in &full_module {
                claimed.insert(v);
            }
            hub_modules.push((h, full_module));
        }
    }

    println!("Extracted {} clean hub modules! Total module vertices: {}", hub_modules.len(), claimed.len());
    for (i, (h, mod_nodes)) in hub_modules.iter().enumerate().take(10) {
        println!("Module {}: Hub={}, size={}", i, h, mod_nodes.len());
    }
}
