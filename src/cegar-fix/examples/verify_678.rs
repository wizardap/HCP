use std::fs::File;
use std::io::{BufRead, BufReader};

#[path = "../src/file_operations.rs"]
mod file_operations;
#[path = "../src/graph.rs"]
mod graph;
#[path = "../src/tour_verifier.rs"]
mod tour_verifier;

use tour_verifier::TourVerifier;

fn main() {
    let col_path = "/home/ubuntu/HCP/FHCPCS-col/graph678.col";
    let tour_path = "/home/ubuntu/HCP/scratch/graph678_certified.tour";

    println!("Loading raw graph from {}...", col_path);
    let raw_g = file_operations::input_to_graph(col_path);
    println!("Loaded graph: {} vertices", raw_g.adjacency_list.len());

    println!("Loading tour from {}...", tour_path);
    let file = File::open(tour_path).expect("Failed to open tour file");
    let reader = BufReader::new(file);

    let mut tour = Vec::new();
    let mut in_section = false;
    for line in reader.lines() {
        let l = line.unwrap();
        let trimmed = l.trim();
        if trimmed.is_empty() || trimmed == "EOF" || trimmed == "-1" {
            continue;
        }
        if trimmed == "TOUR_SECTION" {
            in_section = true;
            continue;
        }
        if in_section {
            if let Ok(v) = trimmed.parse::<i32>() {
                tour.push(v);
            }
        }
    }

    println!("Loaded tour: {} vertices", tour.len());

    match TourVerifier::verify_raw_tour(&tour, &raw_g) {
        Ok(()) => {
            println!("==========================================================");
            println!("*** RUST TourVerifier: 100% SOUND & VALID HAMILTONIAN TOUR! ***");
            println!("==========================================================");
        }
        Err(err) => {
            eprintln!("Verification failed: {}", err);
            std::process::exit(1);
        }
    }
}
