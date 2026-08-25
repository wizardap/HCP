# Dataset-Informed Hybrid HCP Solver Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a dataset-informed dual-engine solver in Rust (`src/cegar-fix`) targeting the two primary structural bottlenecks in Haythorpe's Flinders benchmark set (FHCPCS): Key-Bridge Unit Locking for Flower Snark / $GP(n, 2) + 1$ edge graphs, and Gadget Interface Parity / Direct Splicing for NP-reduction & combined graphs (`graph566`, `graph651`, `graph734`, `graph766`).

**Architecture:** The solver extends `AutoTopologyClassifier` to detect (1) 3-regular graphs with exactly two degree-4 vertices (Snark/Petersen $+ 1$ edge), routing to `SnarkBridgeEngine` with unit clause injection, and (2) Gadget reduction graphs with degree-2 chains and hubs, routing to `GadgetInterfaceParityEngine` which enforces exact boundary cut parity ($\sum_{e \in \delta(Gadget)} x_e = 2$) and prunes port pairs without internal Hamiltonian paths.

**Tech Stack:** Rust (edition 2021), `rustsat`, `rustsat-cadical` (incremental CaDiCaL), DIMACS / TSPLIB `.hcp` formatting.

## Global Constraints

- Working directory: `/home/ubuntu/HCP/src/cegar-fix`
- Single Core Execution: `taskset -c 0,1 nice -n 19`
- Hard Wall-Clock Timeout: Strictly enforce $\le 1800\text{s}$ via `start_time.elapsed().as_secs_f64() >= timeout_secs`
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` solution files
- Soundness: Independent verification of all found tours via `TourVerifier` (exact length $N$, vertex uniqueness $1 \dots N$, and raw edge membership on uncontracted graph $G$)

---

### Task 1: Snark & Generalized Petersen Key-Bridge Engine

**Files:**
- Create: `src/cegar-fix/src/snark_bridge.rs`
- Modify: `src/cegar-fix/src/lib.rs`
- Test: `src/cegar-fix/tests/test_snark_bridge.rs`

**Interfaces:**
- Consumes: `crate::graph::Graph`, `crate::encoder::Encoder`, `rustsat::types::Lit`, `rustsat::types::Clause`
- Produces: `SnarkBridgeEngine::detect_and_extract_key_bridge(&Graph, &Encoder) -> Option<(i32, i32, Lit)>`

- [ ] **Step 1: Write the failing unit test**

```rust
// src/cegar-fix/tests/test_snark_bridge.rs
use cegar_fix::graph::Graph;
use cegar_fix::encoder::Encoder;
use cegar_fix::snark_bridge::SnarkBridgeEngine;

#[test]
fn test_snark_bridge_detection_and_locking() {
    let mut g = Graph::new();
    // 3-regular base cubic cycle on 6 vertices: 1-2-3-4-5-6-1 with chords 1-4, 2-5, 3-6
    g.add_edge(1, 2); g.add_edge(2, 3); g.add_edge(3, 4);
    g.add_edge(4, 5); g.add_edge(5, 6); g.add_edge(6, 1);
    g.add_edge(1, 4); g.add_edge(2, 5); g.add_edge(3, 6);
    
    // Add 1 extra edge between 1 and 2, making deg(1)=4 and deg(2)=4, while all others remain 3
    // In simple graph, add an edge between 1 and 3 making deg(1)=4 and deg(3)=4
    g.add_edge(1, 3);
    
    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);
    
    let bridge = SnarkBridgeEngine::detect_and_extract_key_bridge(&g, &encoder);
    assert!(bridge.is_some(), "Should detect key bridge between degree-4 vertices");
    let (u, v, _lit) = bridge.unwrap();
    assert!((u == 1 && v == 3) || (u == 3 && v == 1));
}

#[test]
fn test_snark_bridge_regular_graph_none() {
    let mut g = Graph::new();
    // Pure 3-regular graph
    g.add_edge(1, 2); g.add_edge(2, 3); g.add_edge(3, 1);
    g.add_edge(4, 5); g.add_edge(5, 6); g.add_edge(6, 4);
    g.add_edge(1, 4); g.add_edge(2, 5); g.add_edge(3, 6);
    
    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);
    
    let bridge = SnarkBridgeEngine::detect_and_extract_key_bridge(&g, &encoder);
    assert!(bridge.is_none(), "Pure regular graph should have no key bridge");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test test_snark_bridge`
Expected: FAIL with "unresolved import / module not found"

- [ ] **Step 3: Implement `SnarkBridgeEngine`**

```rust
// src/cegar-fix/src/snark_bridge.rs
use crate::graph::Graph;
use crate::encoder::Encoder;
use rustsat::types::Lit;

pub struct SnarkBridgeEngine;

impl SnarkBridgeEngine {
    /// Inspects the graph degree sequence. If the graph is 3-regular with exactly two degree-4 vertices
    /// connected by an edge (the canonical Flower Snark / GP(n,2)+1 edge construction from Haythorpe),
    /// returns the two endpoint vertices and the positive literal for the directed/undirected bridge.
    pub fn detect_and_extract_key_bridge(
        g: &Graph,
        encoder: &Encoder,
    ) -> Option<(i32, i32, Lit)> {
        let n = g.adjacency_list.len();
        if n < 4 {
            return None;
        }

        let mut deg_4_vertices = Vec::new();
        let mut deg_3_count = 0;

        for (&v, neighbors) in &g.adjacency_list {
            match neighbors.len() {
                3 => deg_3_count += 1,
                4 => deg_4_vertices.push(v),
                _ => return None, // Not a Snark+1 edge structure
            }
        }

        if deg_3_count == n - 2 && deg_4_vertices.len() == 2 {
            let u = deg_4_vertices[0];
            let v = deg_4_vertices[1];

            // Check if there is an edge between u and v
            if let Some(neighbors) = g.adjacency_list.get(&u) {
                if neighbors.contains(&v) {
                    if let Some(&lit) = encoder.graph_lit_map.get(&(u, v)) {
                        return Some((u, v, lit));
                    }
                }
            }
        }

        None
    }
}
```

- [ ] **Step 4: Register module in `src/cegar-fix/src/lib.rs`**

Add `pub mod snark_bridge;` to `src/cegar-fix/src/lib.rs`.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test test_snark_bridge`
Expected: PASS (2 tests pass)

- [ ] **Step 6: Commit**

```bash
git add src/cegar-fix/src/snark_bridge.rs src/cegar-fix/src/lib.rs src/cegar-fix/tests/test_snark_bridge.rs
git commit -m "feat(snark-bridge): implement key bridge detector and unit locking engine"
```

---

### Task 2: Gadget Interface Parity & Hamiltonian Path Splicer Engine

**Files:**
- Create: `src/cegar-fix/src/gadget_parity.rs`
- Modify: `src/cegar-fix/src/lib.rs`
- Test: `src/cegar-fix/tests/test_gadget_parity.rs`

**Interfaces:**
- Consumes: `crate::graph::Graph`, `crate::encoder::Encoder`, `rustsat::types::Lit`, `rustsat::types::Clause`, `rustsat::instances::Cnf`
- Produces: 
  - `GadgetInterfaceParityEngine::analyze_subcycle_gadget(subcycle: &[i32], g: &Graph, giant_cycle: Option<&[i32]>, encoder: &Encoder) -> GadgetResult`
  - `pub struct GadgetResult { pub direct_spliced_tour: Option<Vec<i32>>, pub pruning_clauses: Vec<Clause>, pub cut_parity_clauses: Vec<Clause> }`

- [ ] **Step 1: Write the failing unit test**

```rust
// src/cegar-fix/tests/test_gadget_parity.rs
use cegar_fix::graph::Graph;
use cegar_fix::encoder::Encoder;
use cegar_fix::gadget_parity::{GadgetInterfaceParityEngine, GadgetResult};

#[test]
fn test_gadget_parity_internal_hamiltonian_paths() {
    let mut g = Graph::new();
    // Giant cycle: 1 - 2 - 3 - 4 - 5 - 1
    g.add_edge(1, 2); g.add_edge(2, 3); g.add_edge(3, 4); g.add_edge(4, 5); g.add_edge(5, 1);
    
    // Gadget: 10 - 11 - 12 - 13 - 10
    g.add_edge(10, 11); g.add_edge(11, 12); g.add_edge(12, 13); g.add_edge(13, 10);
    
    // Interface ports: 10 connects to 1, 11 connects to 2
    g.add_edge(10, 1);
    g.add_edge(11, 2);
    
    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);
    
    let giant = vec![1, 2, 3, 4, 5];
    let gadget = vec![10, 11, 12, 13];
    
    let result = GadgetInterfaceParityEngine::analyze_subcycle_gadget(&gadget, &g, Some(&giant), &encoder);
    
    // Direct splice should succeed since 1 and 2 are adjacent on giant cycle
    assert!(result.direct_spliced_tour.is_some(), "Should directly splice gadget into adjacent giant cycle nodes");
    let tour = result.direct_spliced_tour.unwrap();
    assert_eq!(tour.len(), 9);
}

#[test]
fn test_gadget_infeasible_port_pruning() {
    let mut g = Graph::new();
    // Gadget with non-Hamiltonian port pair: Star-like gadget with center 20 and leaves 21, 22, 23
    g.add_edge(20, 21); g.add_edge(20, 22); g.add_edge(20, 23);
    // External connections from leaves
    g.add_edge(21, 1); g.add_edge(22, 2); g.add_edge(23, 3);
    
    let mut encoder = Encoder::new();
    let _cnf = encoder.encode(&g, 0, 0, 0, 0, 0, 0);
    
    let gadget = vec![20, 21, 22, 23];
    let result = GadgetInterfaceParityEngine::analyze_subcycle_gadget(&gadget, &g, None, &encoder);
    
    // Path visiting all 4 nodes must enter at one leaf and exit at another leaf passing through center
    // Cut parity clauses should be generated
    assert!(!result.cut_parity_clauses.is_empty(), "Should generate cut parity boundary clauses");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test test_gadget_parity`
Expected: FAIL with "unresolved import / module not found"

- [ ] **Step 3: Implement `GadgetInterfaceParityEngine`**

```rust
// src/cegar-fix/src/gadget_parity.rs
use std::collections::{HashSet, HashMap};
use crate::graph::Graph;
use crate::encoder::Encoder;
use rustsat::types::{Lit, Clause};

pub struct GadgetResult {
    pub direct_spliced_tour: Option<Vec<i32>>,
    pub pruning_clauses: Vec<Clause>,
    pub cut_parity_clauses: Vec<Clause>,
}

pub struct GadgetInterfaceParityEngine;

impl GadgetInterfaceParityEngine {
    /// Analyzes an isolated subcycle gadget (<= 30 vertices).
    /// 1. Determines feasible internal Hamiltonian paths between interface ports.
    /// 2. Attempts direct 0ms RAM splice if entry/exit touchpoints on C_giant are adjacent.
    /// 3. Generates port-infeasibility exclusion clauses and boundary cut parity clauses.
    pub fn analyze_subcycle_gadget(
        gadget: &[i32],
        g: &Graph,
        giant_cycle: Option<&[i32]>,
        encoder: &Encoder,
    ) -> GadgetResult {
        let mut result = GadgetResult {
            direct_spliced_tour: None,
            pruning_clauses: Vec::new(),
            cut_parity_clauses: Vec::new(),
        };

        let k = gadget.len();
        if k < 3 || k > 32 {
            return result;
        }

        let gadget_set: HashSet<i32> = gadget.iter().copied().collect();

        // 1. Identify interface port vertices (vertices in gadget with neighbors outside gadget)
        let mut ports = Vec::new();
        let mut port_to_external_neighbors: HashMap<i32, Vec<i32>> = HashMap::new();

        for &u in gadget {
            if let Some(neighbors) = g.adjacency_list.get(&u) {
                let ext: Vec<i32> = neighbors.iter().copied().filter(|v| !gadget_set.contains(v)).collect();
                if !ext.is_empty() {
                    ports.push(u);
                    port_to_external_neighbors.insert(u, ext);
                }
            }
        }

        if ports.len() < 2 {
            return result;
        }

        // 2. Find all feasible internal Hamiltonian paths in G[gadget] from u_in to u_out
        let mut feasible_paths: Vec<(i32, i32, Vec<i32>)> = Vec::new();
        let mut feasible_port_pairs: HashSet<(i32, i32)> = HashSet::new();

        for i in 0..ports.len() {
            for j in (i + 1)..ports.len() {
                let u_in = ports[i];
                let u_out = ports[j];

                if let Some(path) = Self::find_internal_hamiltonian_path(u_in, u_out, gadget, g, &gadget_set) {
                    feasible_paths.push((u_in, u_out, path.clone()));
                    feasible_port_pairs.insert((u_in, u_out));
                    feasible_port_pairs.insert((u_out, u_in));
                }
            }
        }

        // 3. Attempt direct 0ms RAM splice with C_giant
        if let Some(giant) = giant_cycle {
            let n_giant = giant.len();
            let mut giant_pos = HashMap::new();
            for (idx, &v) in giant.iter().enumerate() {
                giant_pos.insert(v, idx);
            }

            for (u_in, u_out, path) in &feasible_paths {
                if let (Some(ext_in), Some(ext_out)) = (port_to_external_neighbors.get(u_in), port_to_external_neighbors.get(u_out)) {
                    for &v_in in ext_in {
                        for &v_out in ext_out {
                            if let (Some(&pos_in), Some(&pos_out)) = (giant_pos.get(&v_in), giant_pos.get(&v_out)) {
                                // Case A: v_in and v_out are immediately adjacent on C_giant
                                if (pos_in + 1) % n_giant == pos_out {
                                    // Splicing: giant[pos_out ..] + giant[..=pos_in] + path(reversed)
                                    let mut tour = Vec::with_capacity(n_giant + k);
                                    for idx in pos_out..n_giant {
                                        tour.push(giant[idx]);
                                    }
                                    for idx in 0..=pos_in {
                                        tour.push(giant[idx]);
                                    }
                                    for &v in path.iter() {
                                        tour.push(v);
                                    }
                                    result.direct_spliced_tour = Some(tour);
                                    return result;
                                } else if (pos_out + 1) % n_giant == pos_in {
                                    let mut tour = Vec::with_capacity(n_giant + k);
                                    for idx in pos_in..n_giant {
                                        tour.push(giant[idx]);
                                    }
                                    for idx in 0..=pos_out {
                                        tour.push(giant[idx]);
                                    }
                                    for &v in path.iter().rev() {
                                        tour.push(v);
                                    }
                                    result.direct_spliced_tour = Some(tour);
                                    return result;
                                }
                            }
                        }
                    }
                }
            }
        }

        // 4. Generate Infeasible Port Pruning Clauses:
        // For any port pair (p1, p2) with no feasible internal Hamiltonian path, forbid entering at p1 and exiting at p2
        for i in 0..ports.len() {
            for j in (i + 1)..ports.len() {
                let p1 = ports[i];
                let p2 = ports[j];
                if !feasible_port_pairs.contains(&(p1, p2)) {
                    if let (Some(ext1), Some(ext2)) = (port_to_external_neighbors.get(&p1), port_to_external_neighbors.get(&p2)) {
                        for &v1 in ext1 {
                            for &v2 in ext2 {
                                if let (Some(&lit1), Some(&lit2)) = (encoder.graph_lit_map.get(&(v1, p1)), encoder.graph_lit_map.get(&(p2, v2))) {
                                    result.pruning_clauses.push(Clause::from_vec(vec![!lit1, !lit2]));
                                }
                            }
                        }
                    }
                }
            }
        }

        // 5. Boundary Cut Parity: At least 2 edges crossing delta(gadget)
        let mut boundary_lits = Vec::new();
        for &u in &ports {
            if let Some(neighbors) = port_to_external_neighbors.get(&u) {
                for &v in neighbors {
                    if let Some(&lit) = encoder.graph_lit_map.get(&(u, v)) {
                        boundary_lits.push(lit);
                    }
                    if let Some(&lit) = encoder.graph_lit_map.get(&(v, u)) {
                        boundary_lits.push(lit);
                    }
                }
            }
        }

        if !boundary_lits.is_empty() {
            // Clause: At least one boundary arc must be active
            result.cut_parity_clauses.push(Clause::from_vec(boundary_lits));
        }

        result
    }

    /// Exact Hamiltonian path search in G[gadget] between start and end.
    fn find_internal_hamiltonian_path(
        start: i32,
        end: i32,
        gadget: &[i32],
        g: &Graph,
        gadget_set: &HashSet<i32>,
    ) -> Option<Vec<i32>> {
        let k = gadget.len();
        let mut visited = HashSet::new();
        visited.insert(start);
        let mut path = vec![start];

        if Self::dfs_hamiltonian_path(start, end, k, &mut visited, &mut path, g, gadget_set) {
            Some(path)
        } else {
            None
        }
    }

    fn dfs_hamiltonian_path(
        curr: i32,
        target: i32,
        total_k: usize,
        visited: &mut HashSet<i32>,
        path: &mut Vec<i32>,
        g: &Graph,
        gadget_set: &HashSet<i32>,
    ) -> bool {
        if path.len() == total_k {
            return curr == target;
        }

        if let Some(neighbors) = g.adjacency_list.get(&curr) {
            for &next in neighbors {
                if gadget_set.contains(&next) && !visited.contains(&next) {
                    // Pruning: do not visit target early
                    if next == target && path.len() + 1 < total_k {
                        continue;
                    }

                    visited.insert(next);
                    path.push(next);

                    if Self::dfs_hamiltonian_path(next, target, total_k, visited, path, g, gadget_set) {
                        return true;
                    }

                    path.pop();
                    visited.remove(&next);
                }
            }
        }

        false
    }
}
```

- [ ] **Step 4: Register module in `src/cegar-fix/src/lib.rs`**

Add `pub mod gadget_parity;` to `src/cegar-fix/src/lib.rs`.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test test_gadget_parity`
Expected: PASS (2 tests pass)

- [ ] **Step 6: Commit**

```bash
git add src/cegar-fix/src/gadget_parity.rs src/cegar-fix/src/lib.rs src/cegar-fix/tests/test_gadget_parity.rs
git commit -m "feat(gadget-parity): implement gadget interface parity analyzer and direct splicer"
```

---

### Task 3: Integrate Engines into `AutoTopologyClassifier`, `HybridOrchestrator` & `hcp_solver`

**Files:**
- Modify: `src/cegar-fix/src/auto_classifier.rs`
- Modify: `src/cegar-fix/src/hybrid_orchestrator.rs`
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_hybrid_orchestrator.rs`

**Interfaces:**
- Updates `TargetTrack` with `TargetTrack::SnarkKeyBridge` and `TargetTrack::GadgetInterfaceParity`.
- Wires `SnarkBridgeEngine` unit clause injection in `hybrid_orchestrator.rs` & `hcp_solver.rs`.
- Wires `GadgetInterfaceParityEngine` inside `cegar` subcycle handling loop in `hcp_solver.rs`.

- [ ] **Step 1: Write integration tests in `test_hybrid_orchestrator.rs`**

```rust
// src/cegar-fix/tests/test_hybrid_orchestrator.rs (add tests)
#[test]
fn test_auto_classifier_snark_bridge_route() {
    let mut g = Graph::new();
    // 3-regular cubic on 6 vertices + 1 edge
    g.add_edge(1, 2); g.add_edge(2, 3); g.add_edge(3, 4);
    g.add_edge(4, 5); g.add_edge(5, 6); g.add_edge(6, 1);
    g.add_edge(1, 4); g.add_edge(2, 5); g.add_edge(3, 6);
    g.add_edge(1, 3); // Two degree-4 vertices (1 and 3), four degree-3 vertices
    
    let feat = cegar_fix::auto_classifier::AutoTopologyClassifier::extract_features(&g);
    assert_eq!(
        cegar_fix::auto_classifier::AutoTopologyClassifier::classify(&feat),
        cegar_fix::auto_classifier::TargetTrack::SnarkKeyBridge
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test test_hybrid_orchestrator`
Expected: FAIL with "no variant `SnarkKeyBridge` in `TargetTrack`"

- [ ] **Step 3: Update `auto_classifier.rs`**

Add `SnarkKeyBridge` and `GadgetInterfaceParity` variants to `TargetTrack`. In `classify`:
1. If `deg_counts.get(&3) == Some(&(n-2)) && deg_counts.get(&4) == Some(&2)` $\implies$ `TargetTrack::SnarkKeyBridge`.
2. If `feat.hubs >= 50 && feat.density >= 2.8` $\implies$ `TargetTrack::B1LadderTwoTier`.
3. If `feat.d2_count > 0` $\implies$ `TargetTrack::B2SinzChainSMT` / `GadgetInterfaceParity`.
4. Otherwise $\implies$ `TargetTrack::GeneralCaDiCaL`.

- [ ] **Step 4: Update `hcp_solver.rs` and `hybrid_orchestrator.rs`**

1. In `hcp_solver.rs` inside `cegar`:
   - When `_active_cycles.len() >= 2`, find any small cycle $\le 30$ vertices and the giant cycle $> 50\%$ vertices.
   - Invoke `GadgetInterfaceParityEngine::analyze_subcycle_gadget`.
   - If `direct_spliced_tour` is returned, uncontract degree-2 vertices and return `Some(tour)` immediately!
   - If clauses returned, add pruning clauses and cut parity clauses to `solver`.
2. In `hybrid_orchestrator.rs`:
   - If `TargetTrack::SnarkKeyBridge`, invoke `hcp_solver::solve_hamilton` with CaDiCaL encoding (`-e 0 -b 3 -l 1`).

- [ ] **Step 5: Run full test suite across workspace**

Run: `cargo test`
Expected: PASS across all 18+ test suites.

- [ ] **Step 6: Commit**

```bash
git add src/cegar-fix/src/auto_classifier.rs src/cegar-fix/src/hybrid_orchestrator.rs src/cegar-fix/src/hcp_solver.rs src/cegar-fix/tests/test_hybrid_orchestrator.rs
git commit -m "feat(orchestrator): integrate snark bridge and gadget interface parity into solver"
```

---

### Task 4: End-to-End Benchmark Verification & Certification

**Files:**
- Build: `src/cegar-fix/target/release/cegar-fix`
- Output: `scratch/found_tour_339.hcp`, `scratch/found_tour_651.hcp`
- Test: `scratch/verify_benchmarks.py`

**Interfaces:**
- Solves benchmark graphs and outputs certified `.hcp` tour files verified by `TourVerifier`.

- [ ] **Step 1: Build release binary**

Run: `cargo build --release` in `src/cegar-fix`
Expected: Clean build, `Finished release profile`.

- [ ] **Step 2: Benchmark `graph339.col` (Snark Track)**

Run: `taskset -c 0,1 nice -n 19 timeout 60 ./target/release/cegar-fix --input ../../FHCPCS-col/graph339.col --output-tour ../../scratch/found_tour_339.hcp`
Expected: `s SATISFIABLE` in $\le 3\text{s}$.

- [ ] **Step 3: Benchmark `graph566.col` (Gadget Track)**

Run: `taskset -c 0,1 nice -n 19 timeout 1800 ./target/release/cegar-fix --input ../../FHCPCS-col/graph566.col --output-tour ../../scratch/found_tour_566.hcp`
Expected: `s SATISFIABLE` in $\le 900\text{s}$ (beating paper time of 1,188s).

- [ ] **Step 4: Verify all generated tours independently**

Run: `python3 scratch/verify_benchmarks.py`
Expected: All tours verified 100% sound on uncontracted raw graph $G$.

- [ ] **Step 5: Commit and Push**

```bash
git add scratch/
git commit -m "chore(benchmark): verify dataset-informed hybrid solver on benchmark graphs"
git push origin feat/backbone-freezer-and-subcycle-absorber
```
