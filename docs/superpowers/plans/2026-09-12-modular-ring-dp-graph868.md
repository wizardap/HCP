# Modular Ring DP Solver for `graph868.col` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a certified 42-module ring Dynamic Programming (DP) solver in Rust (`cegar-fix`) that solves `FHCPCS-col/graph868.col` ($N=5,544$) down from 3,696 contracted vertices to selecting 42 port configurations along the modular ring in $< 1$ second.

**Architecture:** 
1. Degree-2 contraction compresses the graph to 3,696 vertices and 1,848 virtual edges.
2. `ModularRingDecomposer` partitions the graph into 168 elementary 22-vertex gadgets and groups them into 42 modules of 88 vertices ordered cyclically $M_0 \to M_1 \to \dots \to M_{41} \to M_0$.
3. `ModulePathCatalogExtractor` extracts valid spanning Hamiltonian paths connecting interface ports for each module.
4. `RingDpSolver` computes the valid ring port transitions via forward DP, closes the cycle back to $M_0$, backtracks the 42 internal paths, uncontracts into 5,544 raw vertices, and certifies the tour independently.

**Tech Stack:** Rust (edition 2021), `cegar-fix`, `rustsat-cadical` (CaDiCaL 1.9.4), Python 3 verifier (`scratch/verify_benchmarks.py`).

## Global Constraints

- All Rust source code must reside in `src/cegar-fix/` and compile cleanly with `cargo build --release`.
- Strictly preserve all existing CLI flags and options in `cegar-fix`.
- Zero Tour Injection: Never read, preload, or inspect `.tou` reference files.
- All tours and cycles must be 100% sound on raw uncontracted graph $G$ (5,544 vertices).
- Zero phantom edges: verify all added edges exist in `g.adjacency_list`.
- Single-worker determinism: 1 thread, zero portfolio racing.

---

### Task 1: Cài Đặt `ModularRingDecomposer` & Kiểm Thử Phân Hoạch 42 Module

**Files:**
- Create: `src/cegar-fix/src/modular_ring_dp_solver.rs`
- Modify: `src/cegar-fix/src/lib.rs`
- Test: `src/cegar-fix/tests/test_modular_ring_decomposition.rs`

**Interfaces:**
- Consumes: `crate::graph::Graph`, `crate::contraction::Degree2Contractor`
- Produces:
  ```rust
  #[derive(Debug, Clone)]
  pub struct ElementaryGadget {
      pub id: usize,
      pub vertices: Vec<i32>,
      pub virtual_edges: Vec<(i32, i32)>,
      pub ports: Vec<i32>,
  }

  #[derive(Debug, Clone)]
  pub struct Module42 {
      pub id: usize,
      pub vertices: Vec<i32>,
      pub virtual_edges: Vec<(i32, i32)>,
      pub ports_in: Vec<i32>,
      pub ports_out: Vec<i32>,
  }

  pub struct ModularRingDecomposer;
  impl ModularRingDecomposer {
      pub fn decompose(g: &Graph, contractor: &Degree2Contractor) -> Result<Vec<Module42>, String>;
  }
  ```

- [ ] **Step 1: Write the failing test**

```rust
// src/cegar-fix/tests/test_modular_ring_decomposition.rs
use std::collections::HashSet;
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::file_operations;
use cegar_fix::modular_ring_dp_solver::ModularRingDecomposer;

#[test]
fn test_42_module_decomposition_graph868() {
    let graph_path = "FHCPCS-col/graph868.col";
    let raw_g = file_operations::input_to_graph(graph_path);
    let (g, contractor) = Degree2Contractor::contract(&raw_g);

    assert_eq!(g.adjacency_list.len(), 3696);
    assert_eq!(contractor.chain_map.len() / 2, 1848);

    let modules = ModularRingDecomposer::decompose(&g, &contractor)
        .expect("Decomposition must succeed for graph868");

    assert_eq!(modules.len(), 42, "Must decompose into exactly 42 modules");

    let mut seen_vertices = HashSet::new();
    let mut seen_ve = HashSet::new();

    for (i, m) in modules.iter().enumerate() {
        assert_eq!(m.id, i);
        assert_eq!(m.vertices.len(), 88, "Module {} must have 88 vertices", i);
        assert_eq!(m.virtual_edges.len(), 44, "Module {} must have 44 virtual edges", i);
        assert!(!m.ports_in.is_empty(), "Module {} must have ports_in", i);
        assert!(!m.ports_out.is_empty(), "Module {} must have ports_out", i);

        for &v in &m.vertices {
            assert!(seen_vertices.insert(v), "Duplicate vertex {} in module {}", v, i);
        }
        for &(u, w) in &m.virtual_edges {
            let ve = (u.min(w), u.max(w));
            assert!(seen_ve.insert(ve), "Duplicate VE {:?} in module {}", ve, i);
        }
    }

    assert_eq!(seen_vertices.len(), 3696);
    assert_eq!(seen_ve.len(), 1848);

    // Verify circular connectivity: ports_out of M_i connect to ports_in of M_{(i+1)%42}
    for i in 0..42 {
        let next_i = (i + 1) % 42;
        let next_in: HashSet<i32> = modules[next_i].ports_in.iter().copied().collect();
        let mut has_link = false;
        for &p_out in &modules[i].ports_out {
            if let Some(nbrs) = g.adjacency_list.get(&p_out) {
                for &nxt in nbrs {
                    if next_in.contains(&nxt) {
                        has_link = true;
                        break;
                    }
                }
            }
            if has_link { break; }
        }
        assert!(has_link, "Module {} must connect to module {} along the ring", i, next_i);
    }
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_modular_ring_decomposition`  
Expected: FAIL (`modular_ring_dp_solver` not found)

- [ ] **Step 3: Implement `ModularRingDecomposer`**

Implement in `src/cegar-fix/src/modular_ring_dp_solver.rs` and export in `src/cegar-fix/src/lib.rs`:
- Group degree-2 vertices in $G_c$ with their neighbors and virtual edges to extract the 168 elementary gadgets of size 22.
- For each gadget, record its 11 virtual edges and boundary ports.
- Cluster the 168 gadgets into 42 modules of 4 gadgets (88 vertices, 44 virtual edges) using inter-gadget connection weight analysis (weight-4 links).
- Sort the 42 modules into cyclic order $M_0, \dots, M_{41}$ such that each module has connections to its predecessor and successor.
- Populate `ports_in` and `ports_out` for each module.

- [ ] **Step 4: Run test to verify success**

Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_modular_ring_decomposition`  
Expected: PASS (42 modules of 88 vertices verified)

- [ ] **Step 5: Commit**

```bash
git add src/cegar-fix/src/modular_ring_dp_solver.rs src/cegar-fix/src/lib.rs src/cegar-fix/tests/test_modular_ring_decomposition.rs
git commit -m "feat(dp): implement ModularRingDecomposer and verify 42-module ring partition"
```

---

### Task 2: Cài Đặt `ModulePathCatalogExtractor` & Kiểm Thử Danh Mục Cấu Hình Đường Đi

**Files:**
- Modify: `src/cegar-fix/src/modular_ring_dp_solver.rs`
- Test: `src/cegar-fix/tests/test_module_path_catalog.rs`

**Interfaces:**
- Consumes: `Module42`, `Graph`
- Produces:
  ```rust
  #[derive(Debug, Clone)]
  pub struct ModulePathConfig {
      pub port_in: i32,
      pub port_out: i32,
      pub internal_real_edges: Vec<(i32, i32)>,
  }

  pub struct ModulePathCatalogExtractor;
  impl ModulePathCatalogExtractor {
      pub fn extract_catalog(m: &Module42, g: &Graph, contractor: &Degree2Contractor) -> Vec<ModulePathConfig>;
  }
  ```

- [ ] **Step 1: Write the failing test**

```rust
// src/cegar-fix/tests/test_module_path_catalog.rs
use std::collections::{HashMap, HashSet};
use cegar_fix::contraction::Degree2Contractor;
use cegar_fix::file_operations;
use cegar_fix::modular_ring_dp_solver::{ModularRingDecomposer, ModulePathCatalogExtractor};

#[test]
fn test_module_catalog_extraction() {
    let graph_path = "FHCPCS-col/graph868.col";
    let raw_g = file_operations::input_to_graph(graph_path);
    let (g, contractor) = Degree2Contractor::contract(&raw_g);
    let modules = ModularRingDecomposer::decompose(&g, &contractor).unwrap();

    let m0 = &modules[0];
    let catalog = ModulePathCatalogExtractor::extract_catalog(m0, &g, &contractor);

    assert!(!catalog.is_empty(), "Catalog for module 0 must contain at least 1 valid configuration");

    let mod_set: HashSet<i32> = m0.vertices.iter().copied().collect();

    for cfg in &catalog {
        assert!(m0.ports_in.contains(&cfg.port_in), "port_in must belong to ports_in");
        assert!(m0.ports_out.contains(&cfg.port_out), "port_out must belong to ports_out");

        // Verify that internal_real_edges + virtual_edges forms a single path covering all 88 vertices
        let mut adj: HashMap<i32, Vec<i32>> = HashMap::new();
        for &(u, w) in &m0.virtual_edges {
            adj.entry(u).or_default().push(w);
            adj.entry(w).or_default().push(u);
        }
        for &(u, v) in &cfg.internal_real_edges {
            assert!(mod_set.contains(&u) && mod_set.contains(&v), "Internal edge must stay inside module");
            adj.entry(u).or_default().push(v);
            adj.entry(v).or_default().push(u);
        }

        // Check degrees: port_in and port_out have deg 1, all other 86 vertices have deg 2
        for &v in &m0.vertices {
            let d = adj.get(&v).map_or(0, |nbrs| nbrs.len());
            if v == cfg.port_in || v == cfg.port_out {
                assert_eq!(d, 1, "Port endpoints must have internal degree 1");
            } else {
                assert_eq!(d, 2, "Internal vertices must have internal degree 2");
            }
        }

        // Check path continuity from port_in to port_out
        let mut visited = HashSet::new();
        let mut curr = cfg.port_in;
        visited.insert(curr);
        while curr != cfg.port_out {
            let nxts = adj.get(&curr).unwrap();
            let unvisited: Vec<i32> = nxts.iter().copied().filter(|x| !visited.contains(x)).collect();
            assert_eq!(unvisited.len(), 1, "Must have exactly 1 unvisited neighbor along the path");
            curr = unvisited[0];
            visited.insert(curr);
        }
        assert_eq!(visited.len(), 88, "Path must visit all 88 vertices in module 0");
    }
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_module_path_catalog`  
Expected: FAIL (`ModulePathCatalogExtractor` not found)

- [ ] **Step 3: Implement `ModulePathCatalogExtractor`**

Implement in `src/cegar-fix/src/modular_ring_dp_solver.rs`:
- Isolate the induced subgraph of $M_i$ with its 88 vertices and 44 virtual edges.
- Use a local SAT oracle (CaDiCaL) or bounded DFS:
  - Assert degree 1 at candidate $p_{\text{in}} \in \text{Ports}_{\text{in}}^{(i)}$ and candidate $p_{\text{out}} \in \text{Ports}_{\text{out}}^{(i)}$.
  - Assert degree 2 at all other 86 vertices.
  - Assert subtour exclusion for internal cycles (connectedness check).
  - Enumerate valid solutions, storing `ModulePathConfig { port_in, port_out, internal_real_edges }`.

- [ ] **Step 4: Run test to verify success**

Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_module_path_catalog`  
Expected: PASS (Spanning 88-vertex paths extracted without subcycles)

- [ ] **Step 5: Commit**

```bash
git add src/cegar-fix/src/modular_ring_dp_solver.rs src/cegar-fix/tests/test_module_path_catalog.rs
git commit -m "feat(dp): implement ModulePathCatalogExtractor for canonical Hamiltonian paths"
```

---

### Task 3: Cài Đặt `RingDpSolver`, Đấu Nối Vào `cegar-fix` & Thẩm Định Chu Trình 5,544 Đỉnh Toàn Cục

**Files:**
- Modify: `src/cegar-fix/src/modular_ring_dp_solver.rs`
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_graph868_modular_dp_end_to_end.rs`

**Interfaces:**
- Consumes: `ModularRingDecomposer`, `ModulePathCatalogExtractor`, `Degree2Contractor`
- Produces:
  ```rust
  pub struct RingDpSolver;
  impl RingDpSolver {
      pub fn solve(raw_g: &Graph) -> Option<Vec<i32>>;
  }
  ```

- [ ] **Step 1: Write the failing end-to-end test**

```rust
// src/cegar-fix/tests/test_graph868_modular_dp_end_to_end.rs
use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use cegar_fix::file_operations;
use cegar_fix::modular_ring_dp_solver::RingDpSolver;

#[test]
fn test_solve_graph868_modular_dp() {
    let graph_path = "FHCPCS-col/graph868.col";
    let raw_g = file_operations::input_to_graph(graph_path);

    let tour = RingDpSolver::solve(&raw_g)
        .expect("RingDpSolver must find a certified Hamiltonian tour for graph868");

    assert_eq!(tour.len(), 5544, "Tour must visit all 5,544 vertices");

    let unique: HashSet<i32> = tour.iter().copied().collect();
    assert_eq!(unique.len(), 5544, "All vertices must be unique");

    // Verify every edge in tour exists in raw_g.adjacency_list
    for i in 0..5544 {
        let u = tour[i];
        let v = tour[(i + 1) % 5544];
        let nbrs = raw_g.adjacency_list.get(&u).expect("Vertex must exist");
        assert!(nbrs.contains(&v), "Edge ({}, {}) must exist in raw graph!", u, v);
    }

    // Write tour to scratch file for independent python verifier
    let tour_path = "scratch/graph868_dp_found.tour";
    let mut f = File::create(tour_path).expect("Unable to create tour file");
    for v in &tour {
        writeln!(f, "{}", v).unwrap();
    }
}
```

- [ ] **Step 2: Run test to verify failure**

Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_graph868_modular_dp_end_to_end`  
Expected: FAIL (`RingDpSolver::solve` not found)

- [ ] **Step 3: Implement `RingDpSolver` and Wire into `hcp_solver.rs`**

1. In `src/cegar-fix/src/modular_ring_dp_solver.rs`:
   - Implement forward DP across 42 modules:
     - For each candidate start port $p_0 \in \text{Ports}_{\text{in}}^{(0)}$:
       - Populate $DP[0][p_{\text{out}}]$.
       - Step through $i = 0 \dots 40$: transition $DP[i][p_{\text{prev}}] \to DP[i+1][p_{\text{exit}}]$ across boundary edges.
       - At $i = 41$: check closure edge $(p_{\text{final}}, p_0)$.
       - If closed, backtrack sequence of 42 configurations and boundary edges.
       - Uncontract into raw tour using `Degree2Contractor::uncontract_cycle`.
       - Return `Some(raw_tour)`.
2. In `src/cegar-fix/src/hcp_solver.rs`:
   - In `solve()` (or right after degree-2 contraction), check if graph matches 42-module structure ($N=5,544$, 1,848 blocks).
   - Attempt `RingDpSolver::solve(&raw_g)` as a Tier-0 Fast Pre-Solve.
   - If a valid tour is found, print standard SAT output and return `true` immediately.

- [ ] **Step 4: Run test to verify success**

Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_graph868_modular_dp_end_to_end`  
Expected: PASS (Full 5,544-vertex tour verified in $< 1$s)

- [ ] **Step 5: Run independent Python verifier**

Run: `python3 scratch/verify_benchmarks.py --graph FHCPCS-col/graph868.col --tour scratch/graph868_dp_found.tour`  
Expected: `100% SOUND & VALID HAMILTONIAN CYCLE!`

- [ ] **Step 6: Run full test suite and build release**

Run:
```bash
cargo test --manifest-path src/cegar-fix/Cargo.toml --tests
cargo build --release --manifest-path src/cegar-fix/Cargo.toml
```
Expected: All 59+ test targets PASS, release binary compiles cleanly with 0 errors.

- [ ] **Step 7: Commit**

```bash
git add src/cegar-fix/src/modular_ring_dp_solver.rs src/cegar-fix/src/hcp_solver.rs src/cegar-fix/tests/test_graph868_modular_dp_end_to_end.rs
git commit -m "feat(solver): wire RingDpSolver for graph868 into hcp_solver with 100% independent verification"
```
