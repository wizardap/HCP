use crate::graph::Graph;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

pub struct TourVerifier;

impl TourVerifier {
    /// Independently verifies that `tour` is a valid Hamiltonian cycle of `raw_g`.
    /// 
    /// Checks:
    /// 1. Tour length matches the number of vertices in `raw_g`.
    /// 2. Every vertex in `tour` is unique and exists in `raw_g`.
    /// 3. For every adjacent pair `(tour[i], tour[(i+1)%N])`, an undirected edge exists in `raw_g`.
    pub fn verify_raw_tour(tour: &[i32], raw_g: &Graph) -> Result<(), String> {
        let n = raw_g.adjacency_list.len();
        if tour.len() != n {
            return Err(format!("Tour length {} != graph vertices {}", tour.len(), n));
        }
        if n == 0 {
            return Ok(());
        }

        let mut seen = HashSet::with_capacity(n);
        for &v in tour {
            if !seen.insert(v) {
                return Err(format!("Duplicate vertex {} detected in tour", v));
            }
            if !raw_g.adjacency_list.contains_key(&v) {
                return Err(format!("Vertex {} does not exist in graph", v));
            }
        }

        for i in 0..n {
            let u = tour[i];
            let v = tour[(i + 1) % n];
            if let Some(nbrs) = raw_g.adjacency_list.get(&u) {
                if !nbrs.contains(&v) {
                    return Err(format!("Edge ({}, {}) does not exist in raw graph", u, v));
                }
            } else {
                return Err(format!("Vertex {} has no adjacency list", u));
            }
        }

        Ok(())
    }

    /// Writes a certified tour in standard TSPLIB format (.tour / .hcp).
    pub fn write_tsplib_hcp(tour: &[i32], graph_name: &str, output_path: &str) -> io::Result<()> {
        if let Some(parent) = Path::new(output_path).parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let mut file = File::create(output_path)?;
        writeln!(file, "NAME : {}", graph_name)?;
        writeln!(file, "TYPE : TOUR")?;
        writeln!(file, "DIMENSION : {}", tour.len())?;
        writeln!(file, "TOUR_SECTION")?;
        for &v in tour {
            writeln!(file, "{}", v)?;
        }
        writeln!(file, "-1")?;
        writeln!(file, "EOF")?;
        Ok(())
    }
}
