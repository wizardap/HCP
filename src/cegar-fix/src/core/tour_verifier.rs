use crate::core::graph::Graph;
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
    /// Returns (true, "") if valid, or (false, err_msg) if invalid.
    pub fn verify(raw_g: &Graph, tour: &[i32]) -> (bool, String) {
        match Self::verify_raw_tour(tour, raw_g) {
            Ok(()) => (true, String::new()),
            Err(e) => (false, e),
        }
    }

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

    /// Verifies if a solution is a Hamiltonian cycle, following the exact algorithm from Takehide Soh's `is_hamiltonian.py`.
    pub fn is_hamiltonian(raw_g: &Graph, solution: &[i32]) -> bool {
        let n = raw_g.adjacency_list.len();
        if solution.len() != n {
            return false;
        }
        for i in 0..n {
            let u = solution[i];
            let v = solution[(i + 1) % n];
            match raw_g.adjacency_list.get(&u) {
                Some(nbrs) if nbrs.contains(&v) => continue,
                _ => return false,
            }
        }
        true
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

    /// Verifies the tour using Takehide Soh's upstream `is_hamiltonian.py` script.
    pub fn verify_upstream_python(graph_path: &str, tour: &[i32]) -> Result<bool, String> {
        let tmp_path = format!(
            "/tmp/verify_{}_{}.txt",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        {
            let mut file = File::create(&tmp_path).map_err(|e| e.to_string())?;
            writeln!(file, "solution:").map_err(|e| e.to_string())?;
            let tour_str = tour
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(" ");
            writeln!(file, "{}", tour_str).map_err(|e| e.to_string())?;
            writeln!(file, "s SATISFIABLE").map_err(|e| e.to_string())?;
        }
        let output = std::process::Command::new("python3")
            .arg("/home/ubuntu/SAT-based-CEGAR/parse/is_hamiltonian.py")
            .arg(graph_path)
            .arg(&tmp_path)
            .output()
            .map_err(|e| e.to_string())?;
        let _ = std::fs::remove_file(&tmp_path);
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.trim().ends_with("True"))
    }
}
