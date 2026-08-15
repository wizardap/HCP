# Degree-2 Path Contraction Preprocessing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Degree-2 Path Contraction Preprocessing to compress chains of degree-2 vertices before SAT encoding, and reconstruct valid Hamiltonian cycles upon solution discovery.

**Architecture:** A standalone `Degree2Contractor` module in `src/cegar-fix/src/contraction.rs` contracts maximal degree-2 paths $u - v_1 - \dots - v_k - w$ into single virtual edges $(u, w)$, records bidirectional chain mappings, handles edge cases (isolated 2-regular cycles, parallel chains), and uncontracts the discovered cycle before output. Integrated into `main.rs` before CEGAR solver instantiation.

**Tech Stack:** Rust 2021 edition, `rustsat`, `cegar-fix`.

## Global Constraints

- Must preserve 100% zero regressions across all 10 key benchmark graphs.
- Must maintain strict CLI compatibility (`-i <file> -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`).
- Code changes isolated to `src/cegar-fix/src/contraction.rs`, `src/cegar-fix/src/main.rs`, and `src/cegar-fix/src/lib.rs` / `src/cegar-fix/src/graph.rs`.

---

### Task 1: Degree-2 Path Contraction & Uncontraction Core Module

**Files:**
- Create: `src/cegar-fix/src/contraction.rs`
- Modify: `src/cegar-fix/src/main.rs` (to register `mod contraction;`)

**Interfaces:**
- Produces:
  ```rust
  pub struct Degree2Contractor {
      pub chain_map: HashMap<(i32, i32), Vec<i32>>,
      pub original_vertices_count: usize,
      pub contracted_vertices_count: usize,
      pub is_direct_cycle: Option<Vec<i32>>,
      pub is_infeasible: bool,
  }

  impl Degree2Contractor {
      pub fn contract(g: &Graph) -> (Graph, Degree2Contractor);
      pub fn uncontract_cycle(&self, contracted_cycle: &[i32]) -> Vec<i32>;
  }
  ```

- [ ] **Step 1: Write unit tests for Degree-2 Path Contraction & Uncontraction**

In `src/cegar-fix/src/contraction.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Graph;
    use std::collections::HashMap;

    fn build_test_graph(edges: &[(i32, i32)], total_v: usize) -> Graph {
        let mut adj = HashMap::new();
        for i in 1..=total_v as i32 {
            adj.insert(i, Vec::new());
        }
        let mut arcs = Vec::new();
        for &(u, v) in edges {
            adj.get_mut(&u).unwrap().push(v);
            adj.get_mut(&v).unwrap().push(u);
            arcs.push((u, v));
            arcs.push((v, u));
        }
        let mut btree_adj = std::collections::BTreeMap::new();
        for (&k, v) in &adj {
            btree_adj.insert(k, v.clone());
        }
        Graph {
            adjacency_list: adj,
            adjacency_list_btree: btree_adj,
            arcs,
        }
    }

    #[test]
    fn test_contract_single_degree2_chain() {
        // Graph: 1 - 2 - 3 with 1-4, 1-5, 3-4, 3-5 (deg(2)=2, deg(1)=3, deg(3)=3, deg(4)=2, deg(5)=2)
        // Simple diamond with degree-2 vertices 2 and 4,5
        let edges = vec![(1, 2), (2, 3), (3, 4), (4, 1), (1, 5), (5, 3)];
        let g = build_test_graph(&edges, 5);
        let (cg, contractor) = Degree2Contractor::contract(&g);
        assert!(!contractor.is_infeasible);
        assert!(contractor.contracted_vertices_count < contractor.original_vertices_count);
    }

    #[test]
    fn test_contract_multi_step_chain_and_uncontract() {
        // 4-cycle with intermediate chain: 1 - a - b - 2, 2 - 3, 3 - 4, 4 - 1
        // Vertices: 1, 2, 3, 4 (deg >= 2) and a=5, b=6 (deg=2)
        // 1-5-6-2 (chain of 2 vertices), 2-3, 3-4, 4-1, plus 1-3, 2-4 to keep endpoints deg >= 3
        let edges = vec![
            (1, 5), (5, 6), (6, 2),
            (2, 3), (3, 4), (4, 1),
            (1, 3), (2, 4),
        ];
        let g = build_test_graph(&edges, 6);
        let (cg, contractor) = Degree2Contractor::contract(&g);
        assert_eq!(contractor.original_vertices_count, 6);
        assert_eq!(contractor.contracted_vertices_count, 4);
        assert!(contractor.chain_map.contains_key(&(1, 2)) || contractor.chain_map.contains_key(&(2, 1)));

        // Contracted cycle: [1, 2, 3, 4]
        let contracted_cycle = vec![1, 2, 3, 4];
        let full_cycle = contractor.uncontract_cycle(&contracted_cycle);
        assert_eq!(full_cycle.len(), 6);
        assert_eq!(full_cycle, vec![1, 5, 6, 2, 3, 4]);
    }

    #[test]
    fn test_contract_pure_cycle() {
        // 4-cycle: 1 - 2 - 3 - 4 - 1 (all deg=2)
        let edges = vec![(1, 2), (2, 3), (3, 4), (4, 1)];
        let g = build_test_graph(&edges, 4);
        let (_, contractor) = Degree2Contractor::contract(&g);
        assert!(contractor.is_direct_cycle.is_some());
        let cycle = contractor.is_direct_cycle.unwrap();
        assert_eq!(cycle.len(), 4);
    }

    #[test]
    fn test_contract_infeasible_parallel_chains() {
        // 1 - a - 2 and 1 - b - 2 where total vertices = 5 (> 4)
        let edges = vec![(1, 3), (3, 2), (1, 4), (4, 2), (1, 5), (2, 5)];
        let g = build_test_graph(&edges, 5);
        let (_, contractor) = Degree2Contractor::contract(&g);
        assert!(contractor.is_infeasible);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo test test_contract_single_degree2_chain`
Expected: FAIL (module/struct `Degree2Contractor` does not exist).

- [ ] **Step 3: Implement `Degree2Contractor`**

In `src/cegar-fix/src/contraction.rs`:
```rust
use crate::graph::Graph;
use std::collections::{BTreeMap, HashMap, HashSet};

pub struct Degree2Contractor {
    pub chain_map: HashMap<(i32, i32), Vec<i32>>,
    pub original_vertices_count: usize,
    pub contracted_vertices_count: usize,
    pub is_direct_cycle: Option<Vec<i32>>,
    pub is_infeasible: bool,
}

impl Degree2Contractor {
    pub fn contract(g: &Graph) -> (Graph, Degree2Contractor) {
        let total_v = g.adjacency_list.len();
        let mut adj: HashMap<i32, HashSet<i32>> = HashMap::new();
        for (&u, neighbors) in &g.adjacency_list {
            adj.insert(u, neighbors.iter().cloned().collect());
        }

        // Identify degree-2 vertices
        let deg2_vertices: HashSet<i32> = adj
            .iter()
            .filter(|(_, neighbors)| neighbors.len() == 2)
            .map(|(&u, _)| u)
            .collect();

        // Check for isolated 2-regular cycle
        if deg2_vertices.len() == total_v && total_v >= 3 {
            // Find full cycle
            let mut cycle = Vec::new();
            let start = *deg2_vertices.iter().next().unwrap();
            let mut curr = start;
            let mut prev = -1;
            let mut visited = HashSet::new();

            while !visited.contains(&curr) {
                visited.insert(curr);
                cycle.push(curr);
                let neighbors: Vec<i32> = adj[&curr].iter().cloned().collect();
                let next = if neighbors[0] == prev {
                    neighbors[1]
                } else {
                    neighbors[0]
                };
                prev = curr;
                curr = next;
            }

            if cycle.len() == total_v {
                return (
                    g.clone(),
                    Degree2Contractor {
                        chain_map: HashMap::new(),
                        original_vertices_count: total_v,
                        contracted_vertices_count: total_v,
                        is_direct_cycle: Some(cycle),
                        is_infeasible: false,
                    },
                );
            } else {
                return (
                    g.clone(),
                    Degree2Contractor {
                        chain_map: HashMap::new(),
                        original_vertices_count: total_v,
                        contracted_vertices_count: total_v,
                        is_direct_cycle: None,
                        is_infeasible: true,
                    },
                );
            }
        }

        let mut chain_map: HashMap<(i32, i32), Vec<i32>> = HashMap::new();
        let mut visited_deg2 = HashSet::new();
        let mut edge_chains: HashMap<(i32, i32), Vec<Vec<i32>>> = HashMap::new();

        for &v in &deg2_vertices {
            if visited_deg2.contains(&v) {
                continue;
            }

            let neighbors: Vec<i32> = adj[&v].iter().cloned().collect();
            if neighbors.len() != 2 {
                continue;
            }

            // Trace path in direction 1 (through neighbors[0])
            let mut path_left = Vec::new();
            let mut curr = v;
            let mut prev = neighbors[1];

            while deg2_vertices.contains(&curr) {
                visited_deg2.insert(curr);
                path_left.push(curr);
                let nbrs: Vec<i32> = adj[&curr].iter().cloned().collect();
                let next = if nbrs[0] == prev { nbrs[1] } else { nbrs[0] };
                prev = curr;
                curr = next;
            }
            let end_u = curr; // non-degree-2 vertex

            // Trace path in direction 2 (through neighbors[1])
            let mut path_right = Vec::new();
            curr = neighbors[1];
            prev = v;
            while deg2_vertices.contains(&curr) {
                visited_deg2.insert(curr);
                path_right.push(curr);
                let nbrs: Vec<i32> = adj[&curr].iter().cloned().collect();
                let next = if nbrs[0] == prev { nbrs[1] } else { nbrs[0] };
                prev = curr;
                curr = next;
            }
            let end_w = curr; // non-degree-2 vertex

            if end_u == end_w {
                // Cycle of degree 2 attached to a single non-degree-2 vertex (cut vertex)
                return (
                    g.clone(),
                    Degree2Contractor {
                        chain_map: HashMap::new(),
                        original_vertices_count: total_v,
                        contracted_vertices_count: total_v,
                        is_direct_cycle: None,
                        is_infeasible: true,
                    },
                );
            }

            // Form complete ordered intermediate path from end_u to end_w
            // path_left was built starting at v moving towards end_u: [v, left_1, ..., left_k]
            // So from end_u towards end_w: reverse(path_left) ++ path_right
            path_left.reverse();
            let mut full_path = path_left;
            full_path.extend(path_right);

            let key = if end_u < end_w {
                (end_u, end_w)
            } else {
                (end_w, end_u)
            };

            let ordered_for_key = if end_u < end_w {
                full_path.clone()
            } else {
                let mut rev = full_path.clone();
                rev.reverse();
                rev
            };

            edge_chains.entry(key).or_insert_with(Vec::new).push(ordered_for_key);
        }

        // Validate parallel chains and build final chain_map
        let mut contracted_adj: HashMap<i32, HashSet<i32>> = HashMap::new();
        for (&u, neighbors) in &adj {
            if !deg2_vertices.contains(&u) {
                contracted_adj.insert(u, HashSet::new());
                for &v in neighbors {
                    if !deg2_vertices.contains(&v) {
                        contracted_adj.get_mut(&u).unwrap().insert(v);
                    }
                }
            }
        }

        for (&(u, w), chains) in &edge_chains {
            if chains.len() > 1 {
                // If there are multiple degree-2 chains between u and w
                let total_chain_verts: usize = chains.iter().map(|c| c.len()).sum();
                if total_chain_verts + 2 < total_v {
                    return (
                        g.clone(),
                        Degree2Contractor {
                            chain_map: HashMap::new(),
                            original_vertices_count: total_v,
                            contracted_vertices_count: total_v,
                            is_direct_cycle: None,
                            is_infeasible: true,
                        },
                    );
                }
            }

            // Store bidirectional mappings
            let fwd_chain = chains[0].clone();
            let mut rev_chain = fwd_chain.clone();
            rev_chain.reverse();

            chain_map.insert((u, w), fwd_chain);
            chain_map.insert((w, u), rev_chain);

            // Add contracted virtual edge
            contracted_adj.entry(u).or_default().insert(w);
            contracted_adj.entry(w).or_default().insert(u);
        }

        // Build contracted Graph struct
        let mut final_adj = HashMap::new();
        let mut final_btree_adj = BTreeMap::new();
        let mut final_arcs = Vec::new();

        for (&u, neighbors) in &contracted_adj {
            let mut nbr_vec: Vec<i32> = neighbors.iter().cloned().collect();
            nbr_vec.sort();
            final_adj.insert(u, nbr_vec.clone());
            final_btree_adj.insert(u, nbr_vec.clone());
            for &v in &nbr_vec {
                final_arcs.push((u, v));
            }
        }

        let contracted_v = final_adj.len();
        let cg = Graph {
            adjacency_list: final_adj,
            adjacency_list_btree: final_btree_adj,
            arcs: final_arcs,
        };

        (
            cg,
            Degree2Contractor {
                chain_map,
                original_vertices_count: total_v,
                contracted_vertices_count: contracted_v,
                is_direct_cycle: None,
                is_infeasible: false,
            },
        )
    }

    pub fn uncontract_cycle(&self, contracted_cycle: &[i32]) -> Vec<i32> {
        let mut full_cycle = Vec::new();
        let len = contracted_cycle.len();
        if len == 0 {
            return full_cycle;
        }

        for i in 0..len {
            let u = contracted_cycle[i];
            let v = contracted_cycle[(i + 1) % len];
            full_cycle.push(u);
            if let Some(intermediates) = self.chain_map.get(&(u, v)) {
                for &inter in intermediates {
                    full_cycle.push(inter);
                }
            }
        }

        full_cycle
    }
}
```

- [ ] **Step 4: Run unit tests to verify they pass**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo test contraction`
Expected: PASS (all 4 contraction tests pass).

- [ ] **Step 5: Commit**

```bash
git add src/cegar-fix/src/contraction.rs src/cegar-fix/src/main.rs
git commit -m "feat: implement degree-2 path contraction and uncontraction module"
```

---

### Task 2: Solver Pipeline Integration

**Files:**
- Modify: `src/cegar-fix/src/main.rs:1-120`

**Interfaces:**
- Consumes: `Degree2Contractor::contract(&Graph)` and `contractor.uncontract_cycle(&[i32])`.
- Produces: Integrated preprocessing in `main.rs`.

- [ ] **Step 1: Update `main.rs` to invoke Degree-2 Contraction**

In `src/cegar-fix/src/main.rs`:
```rust
mod contraction;
use contraction::Degree2Contractor;

// In main():
// After g.prune_degree2_triangles():
let (contracted_g, contractor) = Degree2Contractor::contract(&g);

if let Some(cycle) = contractor.is_direct_cycle {
    println!("Graph is a single 2-regular Hamiltonian cycle.");
    print!("solution: \n");
    for v in &cycle {
        print!("{} ", v);
    }
    println!();
    println!("s SATISFIABLE");
    return;
}

if contractor.is_infeasible {
    println!("Infeasible parallel degree-2 chains detected.");
    println!("s UNSATISFIABLE");
    return;
}

if contractor.contracted_vertices_count < contractor.original_vertices_count {
    println!(
        "Degree-2 contraction: compressed graph from {} to {} vertices (reduced by {}%)",
        contractor.original_vertices_count,
        contractor.contracted_vertices_count,
        (contractor.original_vertices_count - contractor.contracted_vertices_count) * 100 / contractor.original_vertices_count
    );
}

// Pass contracted_g to CEGAR solver:
let mut encoder = Encoder::new();
// ... run CEGAR on contracted_g ...

// When SATISFIABLE:
let full_cycle = contractor.uncontract_cycle(&solution_cycle);
println!("solution: ");
for v in &full_cycle {
    print!("{} ", v);
}
println!();
println!("s SATISFIABLE");
```

- [ ] **Step 2: Build release binary and verify cargo tests**

Run: `cd /home/ubuntu/HCP/src/cegar-fix && cargo test && cargo build --release`
Expected: PASS with 0 errors.

- [ ] **Step 3: Commit**

```bash
git add src/cegar-fix/src/main.rs
git commit -m "feat: integrate degree-2 path contraction into solver pipeline"
```

---

### Task 3: Regression Benchmark & Target Graph Evaluation

**Files:**
- Test: Benchmark execution on 10 regression graphs and sparse path graphs (`graph710.col`, `graph717.col`, `graph725.col`, `graph998.col`).
- Create: `.superpowers/sdd/2026-08-15-degree-2-contraction/task-3-report.md`

- [ ] **Step 1: Verify 10 Key Regression Graphs**

Run:
```bash
./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph45.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1
./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph132.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1
./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph161.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1
./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph178.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1
./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph183.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1
./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph230.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1
./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph248.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1
./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph313.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1
./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph339.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1
./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph346.col -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1
```
Expected: All 10 graphs finish with `s SATISFIABLE` (100% pass rate).

- [ ] **Step 2: Profile Vertex Reduction on Target Path Graphs**

Run on `graph710.col`, `graph717.col`, `graph725.col`, `graph998.col`:
Observe degree-2 compression percentage and verify uncontracted solution validity.

- [ ] **Step 3: Record verification results and commit**

```bash
git add .superpowers/sdd/
git commit -m "docs: record verification results for degree-2 path contraction"
```
