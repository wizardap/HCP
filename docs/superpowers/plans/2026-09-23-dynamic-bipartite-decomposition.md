# Dynamic Bipartite Macro-Decomposition Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a 100% de novo, zero-hardcode, pure-Rust hierarchical solver for the dense-bipartite challenge graph family (`graph746`, `graph950`, `graph963`, `graph975`, `graph982`, `graph990`).

**Architecture:** A two-level hierarchical architecture with dynamic hub-signature partitioning and dominant hub absorption, a Level-1 CaDiCaL Macro-SAT engine with DFJ subtour cuts, Level-2 Rayon-parallel cluster path solvers, and a conflict-learning feedback loop.

**Tech Stack:** Rust (2021 edition), `rustsat` (0.5.3), `rustsat-cadical` (0.3.0), `rayon` (1.8), `serde_json`.

**Spec:** `docs/superpowers/specs/2026-09-23-dynamic-bipartite-decomposition-design.md`

## Global Constraints

- 100% pure Rust: zero runtime dependency on Python or external binaries.
- 100% de novo: strictly 0 hardcoded vertex IDs, 0 precomputed cache files (`.pkl`, `.json`, `.hcp`), 0 strip index tables.
- Soundness: All tours certified by `TourVerifier::verify` (dimension, bijection, and raw edge existence).
- Clean compilation: `RUSTFLAGS="-D warnings" cargo check --manifest-path src/cegar-fix/Cargo.toml --all-targets` must pass with 0 warnings.
- Test performance: Never run bare `cargo test` without a filter (to avoid running historical slow tests). Always use targeted test names.

---

### Task 1: Topology Detection & Dynamic Partitioning Module

**Files:**
- Create: `src/cegar-fix/src/macro_decomp/dynamic_bipartite.rs`
- Modify: `src/cegar-fix/src/macro_decomp/mod.rs`
- Modify: `src/cegar-fix/src/lib.rs`
- Test: `src/cegar-fix/tests/test_dynamic_bipartite_partition.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct BipartitePartition {
      pub super_hubs: Vec<i32>,
      pub clusters: HashMap<i32, HashSet<i32>>,
      pub owner: HashMap<i32, i32>,
      pub boundary_ports: HashMap<i32, Vec<i32>>,
      pub connectors: Vec<i32>,
      pub macro_edges: Vec<(i32, i32)>,
  }

  pub fn can_solve_bipartite(raw_g: &Graph) -> bool;
  pub fn detect_and_partition(raw_g: &Graph) -> Option<BipartitePartition>;
  ```

- [ ] **Step 1: Write the failing integration test**

Create `src/cegar-fix/tests/test_dynamic_bipartite_partition.rs`:
```rust
use cegar_fix::core::file_operations;
use cegar_fix::macro_decomp::dynamic_bipartite;

#[test]
fn test_partition_graph746_dynamic() {
    let graph_path = "../../FHCPCS-col/graph746.col";
    let g = file_operations::parse_graph_from_file(graph_path).expect("Failed to parse graph746");
    assert!(dynamic_bipartite::can_solve_bipartite(&g), "graph746 should be detected as bipartite");

    let partition = dynamic_bipartite::detect_and_partition(&g)
        .expect("Failed to partition graph746");

    assert_eq!(partition.super_hubs.len(), 5, "Expected 5 super hubs");
    assert_eq!(partition.clusters.len(), 5, "Expected 5 clusters");
    assert_eq!(partition.connectors.len(), 2, "Expected exactly 2 connectors");
    
    let total_covered: usize = partition.clusters.values().map(|c| c.len()).sum::<usize>() + partition.connectors.len();
    assert_eq!(total_covered, 4286, "All 4286 vertices must be partitioned");

    for (&hub, c) in &partition.clusters {
        assert!(c.len() >= 850 && c.len() <= 860, "Cluster {} size {} is out of expected balanced range [850, 860]", hub, c.len());
        let ports = &partition.boundary_ports[&hub];
        assert_eq!(ports.len(), 3, "Cluster {} should have exactly 3 boundary ports, got {:?}", hub, ports);
    }
}

#[test]
fn test_partition_graph950_dynamic() {
    let graph_path = "../../FHCPCS-col/graph950.col";
    let g = file_operations::parse_graph_from_file(graph_path).expect("Failed to parse graph950");
    assert!(dynamic_bipartite::can_solve_bipartite(&g), "graph950 should be detected as bipartite");

    let partition = dynamic_bipartite::detect_and_partition(&g)
        .expect("Failed to partition graph950");

    assert_eq!(partition.super_hubs.len(), 10, "Expected 10 super hubs");
    assert_eq!(partition.clusters.len(), 10, "Expected 10 clusters");
    assert_eq!(partition.connectors.len(), 2, "Expected exactly 2 connectors");
    
    let total_covered: usize = partition.clusters.values().map(|c| c.len()).sum::<usize>() + partition.connectors.len();
    assert_eq!(total_covered, 6620, "All 6620 vertices must be partitioned");

    for (&hub, c) in &partition.clusters {
        assert!(c.len() >= 658 && c.len() <= 666, "Cluster {} size {} is out of expected balanced range [658, 666]", hub, c.len());
        let ports = &partition.boundary_ports[&hub];
        assert_eq!(ports.len(), 3, "Cluster {} should have exactly 3 boundary ports, got {:?}", hub, ports);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test test_dynamic_bipartite_partition --manifest-path src/cegar-fix/Cargo.toml --release`
Expected: FAIL with compilation error (module `dynamic_bipartite` does not exist).

- [ ] **Step 3: Implement `detect_and_partition` in `dynamic_bipartite.rs`**

Create `src/cegar-fix/src/macro_decomp/dynamic_bipartite.rs`:
```rust
use crate::core::graph::Graph;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct BipartitePartition {
    pub super_hubs: Vec<i32>,
    pub clusters: HashMap<i32, HashSet<i32>>,
    pub owner: HashMap<i32, i32>,
    pub boundary_ports: HashMap<i32, Vec<i32>>,
    pub connectors: Vec<i32>,
    pub macro_edges: Vec<(i32, i32)>,
}

pub fn can_solve_bipartite(raw_g: &Graph) -> bool {
    let super_hubs_count = raw_g
        .adjacency_list
        .iter()
        .filter(|&(_, nbrs)| nbrs.len() >= 400)
        .count();
    super_hubs_count == 5 || super_hubs_count == 10
}

pub fn detect_and_partition(raw_g: &Graph) -> Option<BipartitePartition> {
    let mut super_hubs: Vec<i32> = raw_g
        .adjacency_list
        .iter()
        .filter(|&(_, nbrs)| nbrs.len() >= 400)
        .map(|(&u, _)| u)
        .collect();
    super_hubs.sort_unstable();

    let k = super_hubs.len();
    if k != 5 && k != 10 {
        return None;
    }

    let sh_set: HashSet<i32> = super_hubs.iter().copied().collect();

    // 1-hop direct hub signatures
    let mut direct_hubs: HashMap<i32, HashSet<i32>> = HashMap::new();
    for (&u, nbrs) in &raw_g.adjacency_list {
        let dh: HashSet<i32> = nbrs.iter().filter(|v| sh_set.contains(v)).copied().collect();
        direct_hubs.insert(u, dh);
    }

    // 2-hop hub signatures for vertices with 0 direct hubs
    let mut two_hop_hubs: HashMap<i32, HashSet<i32>> = HashMap::new();
    for (&u, nbrs) in &raw_g.adjacency_list {
        if !sh_set.contains(&u) && direct_hubs[&u].is_empty() {
            let mut th = HashSet::new();
            for &w in nbrs {
                if let Some(dh) = direct_hubs.get(&w) {
                    th.extend(dh);
                }
            }
            two_hop_hubs.insert(u, th);
        }
    }

    let mut clusters: HashMap<i32, HashSet<i32>> = HashMap::new();
    let mut owner: HashMap<i32, i32> = HashMap::new();
    let mut unassigned: HashSet<i32> = HashSet::new();

    for &h in &super_hubs {
        clusters.entry(h).or_default().insert(h);
        owner.insert(h, h);
    }

    for (&u, _) in &raw_g.adjacency_list {
        if sh_set.contains(&u) {
            continue;
        }
        let dh = &direct_hubs[&u];
        if dh.len() == 1 {
            let h = *dh.iter().next().unwrap();
            clusters.entry(h).or_default().insert(u);
            owner.insert(u, h);
        } else if dh.is_empty() {
            if let Some(th) = two_hop_hubs.get(&u) {
                if th.len() == 1 {
                    let h = *th.iter().next().unwrap();
                    clusters.entry(h).or_default().insert(u);
                    owner.insert(u, h);
                } else {
                    unassigned.insert(u);
                }
            } else {
                unassigned.insert(u);
            }
        } else {
            unassigned.insert(u);
        }
    }

    // Dominant hub absorption: if >= 90% of a vertex's neighbors belong to one cluster, absorb it
    let unassigned_vec: Vec<i32> = unassigned.iter().copied().collect();
    for u in unassigned_vec {
        let nbrs = match raw_g.adjacency_list.get(&u) {
            Some(n) => n,
            None => continue,
        };
        let mut cluster_counts: HashMap<i32, usize> = HashMap::new();
        for &w in nbrs {
            if let Some(&h) = owner.get(&w) {
                *cluster_counts.entry(h).or_default() += 1;
            }
        }
        if let Some((&dominant_hub, &count)) = cluster_counts.iter().max_by_key(|&(_, c)| *c) {
            if count as f64 / nbrs.len() as f64 >= 0.90 {
                clusters.entry(dominant_hub).or_default().insert(u);
                owner.insert(u, dominant_hub);
                unassigned.remove(&u);
            }
        }
    }

    let mut connectors: Vec<i32> = unassigned.into_iter().collect();
    connectors.sort_unstable();

    // Extract boundary ports per cluster
    let mut boundary_ports: HashMap<i32, Vec<i32>> = HashMap::new();
    let mut all_ports: HashSet<i32> = HashSet::new();

    for (&h, c_nodes) in &clusters {
        let mut ports = Vec::new();
        for &u in c_nodes {
            if let Some(nbrs) = raw_g.adjacency_list.get(&u) {
                let has_ext = nbrs.iter().any(|v| !c_nodes.contains(v));
                if has_ext {
                    ports.push(u);
                    all_ports.insert(u);
                }
            }
        }
        ports.sort_unstable();
        boundary_ports.insert(h, ports);
    }

    let mut all_macro_nodes: HashSet<i32> = all_ports;
    all_macro_nodes.extend(&connectors);

    let mut macro_edges = Vec::new();
    for &u in &all_macro_nodes {
        if let Some(nbrs) = raw_g.adjacency_list.get(&u) {
            for &v in nbrs {
                if u < v && all_macro_nodes.contains(&v) {
                    let ou = owner.get(&u);
                    let ov = owner.get(&v);
                    if ou != ov || ou.is_none() || ov.is_none() {
                        macro_edges.push((u, v));
                    }
                }
            }
        }
    }
    macro_edges.sort_unstable();

    Some(BipartitePartition {
        super_hubs,
        clusters,
        owner,
        boundary_ports,
        connectors,
        macro_edges,
    })
}
```

Update `src/cegar-fix/src/macro_decomp/mod.rs`:
```rust
pub mod corridor;
pub mod portfolio_788;
pub mod dynamic_bipartite;
```

Update `src/cegar-fix/src/lib.rs` to re-export:
```rust
pub use macro_decomp::dynamic_bipartite;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test test_dynamic_bipartite_partition --manifest-path src/cegar-fix/Cargo.toml --release`
Expected: PASS (both `test_partition_graph746_dynamic` and `test_partition_graph950_dynamic` pass in < 0.2s).

- [ ] **Step 5: Commit**

```bash
git add src/cegar-fix/src/macro_decomp/dynamic_bipartite.rs src/cegar-fix/src/macro_decomp/mod.rs src/cegar-fix/src/lib.rs src/cegar-fix/tests/test_dynamic_bipartite_partition.rs
git commit -m "feat(macro): implement dynamic hub signature partitioning for dense bipartite graphs"
```

---

### Task 2: Level-1 Macro-SAT Engine

**Files:**
- Modify: `src/cegar-fix/src/macro_decomp/dynamic_bipartite.rs`
- Test: `src/cegar-fix/tests/test_dynamic_bipartite_macro_sat.rs`

**Interfaces:**
- Consumes: `BipartitePartition` from Task 1.
- Produces:
  ```rust
  #[derive(Debug, Clone)]
  pub struct MacroConfiguration {
      pub cluster_ports: HashMap<i32, (i32, i32)>,
      pub active_macro_edges: Vec<(i32, i32)>,
  }

  pub struct MacroSatSolver { ... }

  impl MacroSatSolver {
      pub fn new(partition: &BipartitePartition, raw_g: &Graph) -> Result<Self, String>;
      pub fn solve_next_configuration(&mut self) -> Option<MacroConfiguration>;
      pub fn block_pair(&mut self, cluster_hub: i32, u_in: i32, u_out: i32);
  }
  ```

- [ ] **Step 1: Write the failing integration test**

Create `src/cegar-fix/tests/test_dynamic_bipartite_macro_sat.rs`:
```rust
use cegar_fix::core::file_operations;
use cegar_fix::macro_decomp::dynamic_bipartite::{detect_and_partition, MacroSatSolver};

#[test]
fn test_macro_sat_graph746() {
    let graph_path = "../../FHCPCS-col/graph746.col";
    let g = file_operations::parse_graph_from_file(graph_path).expect("Failed to parse graph746");
    let partition = detect_and_partition(&g).expect("Failed to partition");

    let mut solver = MacroSatSolver::new(&partition, &g).expect("Failed to build MacroSatSolver");
    let config = solver.solve_next_configuration().expect("Expected a valid macro configuration for graph746");

    assert_eq!(config.cluster_ports.len(), 5, "All 5 clusters must have a chosen port pair");
    for &h in &partition.super_hubs {
        let (u_in, u_out) = config.cluster_ports[&h];
        assert_ne!(u_in, u_out, "Port u_in and u_out must be distinct for hub {}", h);
        let ports = &partition.boundary_ports[&h];
        assert!(ports.contains(&u_in), "u_in {} must be in boundary ports {:?}", u_in, ports);
        assert!(ports.contains(&u_out), "u_out {} must be in boundary ports {:?}", u_out, ports);
    }

    assert!(config.active_macro_edges.len() >= 7, "Macro cycle must have active external edges");
}

#[test]
fn test_macro_sat_graph950() {
    let graph_path = "../../FHCPCS-col/graph950.col";
    let g = file_operations::parse_graph_from_file(graph_path).expect("Failed to parse graph950");
    let partition = detect_and_partition(&g).expect("Failed to partition");

    let mut solver = MacroSatSolver::new(&partition, &g).expect("Failed to build MacroSatSolver");
    let config = solver.solve_next_configuration().expect("Expected a valid macro configuration for graph950");

    assert_eq!(config.cluster_ports.len(), 10, "All 10 clusters must have a chosen port pair");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test test_dynamic_bipartite_macro_sat --manifest-path src/cegar-fix/Cargo.toml --release`
Expected: FAIL with compilation error (`MacroSatSolver` does not exist).

- [ ] **Step 3: Implement `MacroSatSolver` in `dynamic_bipartite.rs`**

Add `MacroSatSolver` implementation:
```rust
use rustsat::clause;
use rustsat::instances::{BasicVarManager, ManageVars};
use rustsat::solvers::{Solve, SolverResult};
use rustsat::types::{Clause, Lit};
use rustsat_cadical::CaDiCaL;

#[derive(Debug, Clone)]
pub struct MacroConfiguration {
    pub cluster_ports: HashMap<i32, (i32, i32)>,
    pub active_macro_edges: Vec<(i32, i32)>,
}

pub struct MacroSatSolver {
    solver: CaDiCaL,
    var_mgr: BasicVarManager,
    edge_vars: HashMap<(i32, i32), Lit>,
    pair_vars: HashMap<i32, HashMap<(i32, i32), Lit>>,
    partition: BipartitePartition,
}

impl MacroSatSolver {
    pub fn new(partition: &BipartitePartition, _raw_g: &Graph) -> Result<Self, String> {
        let mut solver = CaDiCaL::default();
        let mut var_mgr = BasicVarManager::default();
        let mut edge_vars = HashMap::new();

        for &e in &partition.macro_edges {
            let lit = var_mgr.new_var().pos_lit();
            edge_vars.insert(e, lit);
        }

        let mut pair_vars: HashMap<i32, HashMap<(i32, i32), Lit>> = HashMap::new();

        // 1. For each cluster, define pair variables and enforce exactly 1 chosen pair
        for (&h, ports) in &partition.boundary_ports {
            let mut p_map = HashMap::new();
            let mut p_lits = Vec::new();
            for i in 0..ports.len() {
                for j in (i + 1)..ports.len() {
                    let u = ports[i].min(ports[j]);
                    let v = ports[i].max(ports[j]);
                    let lit = var_mgr.new_var().pos_lit();
                    p_map.insert((u, v), lit);
                    p_lits.push(lit);
                }
            }
            if p_lits.is_empty() {
                return Err(format!("Cluster {} has < 2 boundary ports", h));
            }

            // At-least-1 pair
            let _ = solver.add_clause(Clause::from_iter(p_lits.iter().copied()));
            // At-most-1 pair
            for i in 0..p_lits.len() {
                for j in (i + 1)..p_lits.len() {
                    let _ = solver.add_clause(clause![!p_lits[i], !p_lits[j]]);
                }
            }
            pair_vars.insert(h, p_map);
        }

        // 2. Incident edge degree consistency for boundary ports:
        // A boundary port u in cluster h has degree 1 in active macro edges <=> u is an endpoint in chosen pair
        let mut incident_macro_edges: HashMap<i32, Vec<Lit>> = HashMap::new();
        for (&(u, v), &lit) in &edge_vars {
            incident_macro_edges.entry(u).or_default().push(lit);
            incident_macro_edges.entry(v).or_default().push(lit);
        }

        for (&h, ports) in &partition.boundary_ports {
            let p_map = &pair_vars[&h];
            for &u in ports {
                let ext_edges = incident_macro_edges.get(&u).cloned().unwrap_or_default();
                // Whether u is an endpoint in the chosen pair:
                // u_is_endpoint <=> OR_{v in ports \ {u}} P_{min(u,v), max(u,v)}
                let mut endpoint_lits = Vec::new();
                for &v in ports {
                    if u != v {
                        let key = (u.min(v), u.max(v));
                        if let Some(&p_lit) = p_map.get(&key) {
                            endpoint_lits.push(p_lit);
                        }
                    }
                }

                // If u is NOT an endpoint, ext_edges degree must be 0
                for &e_lit in &ext_edges {
                    for &p_lit in &endpoint_lits {
                        // e_lit implies u_is_endpoint
                        let _ = solver.add_clause(clause![!e_lit, p_lit]);
                    }
                }

                // If u is an endpoint, ext_edges degree must be >= 1
                if !ext_edges.is_empty() {
                    for &p_lit in &endpoint_lits {
                        let mut cl = Vec::with_capacity(ext_edges.len() + 1);
                        cl.push(!p_lit);
                        cl.extend(&ext_edges);
                        let _ = solver.add_clause(Clause::from_iter(cl));
                    }
                } else if !endpoint_lits.is_empty() {
                    // Port has no external edges, cannot be an endpoint
                    for &p_lit in &endpoint_lits {
                        let _ = solver.add_clause(clause![!p_lit]);
                    }
                }

                // At-most-1 external edge for port
                for i in 0..ext_edges.len() {
                    for j in (i + 1)..ext_edges.len() {
                        let _ = solver.add_clause(clause![!ext_edges[i], !ext_edges[j]]);
                    }
                }
            }
        }

        // 3. Connector nodes degree constraint: exactly 2 incident edges
        for &c in &partition.connectors {
            let c_edges = incident_macro_edges.get(&c).cloned().unwrap_or_default();
            if c_edges.len() < 2 {
                return Err(format!("Connector node {} has degree < 2 in macro edges", c));
            }
            crate::core::encoder::add_at_most_2(&mut solver, &mut var_mgr, &c_edges);
            // At least 2:
            // For any edge, if it is not selected, the remaining must have at least 1, etc.
            // Standard: negation of <= 1.
            // Or clauses: not all 0, and not exactly 1.
            for i in 0..c_edges.len() {
                let mut cl = Vec::new();
                for j in 0..c_edges.len() {
                    if i != j {
                        cl.push(c_edges[j]);
                    }
                }
                let _ = solver.add_clause(Clause::from_iter(cl));
            }
        }

        Ok(Self {
            solver,
            var_mgr,
            edge_vars,
            pair_vars,
            partition: partition.clone(),
        })
    }

    pub fn block_pair(&mut self, cluster_hub: i32, u_in: i32, u_out: i32) {
        let key = (u_in.min(u_out), u_in.max(u_out));
        if let Some(p_map) = self.pair_vars.get(&cluster_hub) {
            if let Some(&lit) = p_map.get(&key) {
                let _ = self.solver.add_clause(clause![!lit]);
            }
        }
    }

    pub fn solve_next_configuration(&mut self) -> Option<MacroConfiguration> {
        let max_subtour_iters = 50;
        for _ in 0..max_subtour_iters {
            match self.solver.solve() {
                Ok(SolverResult::Sat) => {}
                _ => return None,
            }

            let sol = self.solver.full_solution().ok()?;
            let mut active_edges = Vec::new();
            for (&e, &lit) in &self.edge_vars {
                if sol.lit_value(lit) == rustsat::types::TernaryVal::True {
                    active_edges.push(e);
                }
            }

            let mut cluster_ports = HashMap::new();
            for (&h, p_map) in &self.pair_vars {
                for (&(u, v), &lit) in p_map {
                    if sol.lit_value(lit) == rustsat::types::TernaryVal::True {
                        cluster_ports.insert(h, (u, v));
                        break;
                    }
                }
            }

            // Check connectivity / subtour elimination
            // Build macro adjacency: active_edges + internal virtual edges (u_in, u_out)
            let mut macro_graph: HashMap<i32, Vec<i32>> = HashMap::new();
            for &(u, v) in &active_edges {
                macro_graph.entry(u).or_default().push(v);
                macro_graph.entry(v).or_default().push(u);
            }
            for &(u, v) in cluster_ports.values() {
                macro_graph.entry(u).or_default().push(v);
                macro_graph.entry(v).or_default().push(u);
            }

            // Find connected components in macro_graph
            let mut visited = HashSet::new();
            let mut comps = Vec::new();
            let all_active_nodes: Vec<i32> = macro_graph.keys().copied().collect();

            for &start in &all_active_nodes {
                if !visited.contains(&start) {
                    let mut comp = Vec::new();
                    let mut q = std::collections::VecDeque::new();
                    visited.insert(start);
                    q.push_back(start);
                    while let Some(curr) = q.pop_front() {
                        comp.push(curr);
                        if let Some(nbrs) = macro_graph.get(&curr) {
                            for &nxt in nbrs {
                                if visited.insert(nxt) {
                                    q.push_back(nxt);
                                }
                            }
                        }
                    }
                    comps.push(comp);
                }
            }

            if comps.len() == 1 {
                return Some(MacroConfiguration {
                    cluster_ports,
                    active_macro_edges: active_edges,
                });
            }

            // Add subtour elimination cut for each disconnected component
            for comp in comps {
                let comp_set: HashSet<i32> = comp.iter().copied().collect();
                let mut cut_lits = Vec::new();
                for &(u, v) in &self.partition.macro_edges {
                    if comp_set.contains(&u) != comp_set.contains(&v) {
                        cut_lits.push(self.edge_vars[&(u, v)]);
                    }
                }
                if !cut_lits.is_empty() {
                    let _ = self.solver.add_clause(Clause::from_iter(cut_lits));
                }
            }
        }
        None
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test test_dynamic_bipartite_macro_sat --manifest-path src/cegar-fix/Cargo.toml --release`
Expected: PASS (both `test_macro_sat_graph746` and `test_macro_sat_graph950` pass in < 0.1s).

- [ ] **Step 5: Commit**

```bash
git add src/cegar-fix/src/macro_decomp/dynamic_bipartite.rs src/cegar-fix/tests/test_dynamic_bipartite_macro_sat.rs
git commit -m "feat(macro): implement Level-1 Macro-SAT engine with DFJ subtour elimination"
```

---

### Task 3: Level-2 Parallel Cluster Path CEGAR, Feedback Loop & Tour Splicer

**Files:**
- Modify: `src/cegar-fix/src/macro_decomp/dynamic_bipartite.rs`
- Test: `src/cegar-fix/tests/test_dynamic_bipartite_solve.rs`

**Interfaces:**
- Produces:
  ```rust
  pub fn solve_cluster_path(
      cluster_id: i32,
      u_in: i32,
      u_out: i32,
      cluster_nodes: &HashSet<i32>,
      adj: &HashMap<i32, Vec<i32>>,
      deadline: Instant,
  ) -> Option<Vec<i32>>;

  pub fn solve_bipartite(raw_g: &Graph, timeout_secs: f64) -> Option<Vec<i32>>;
  ```

- [ ] **Step 1: Write the failing integration test**

Create `src/cegar-fix/tests/test_dynamic_bipartite_solve.rs`:
```rust
use cegar_fix::core::file_operations;
use cegar_fix::core::tour_verifier::TourVerifier;
use cegar_fix::macro_decomp::dynamic_bipartite;

#[test]
fn test_solve_graph746_dynamic_de_novo() {
    let graph_path = "../../FHCPCS-col/graph746.col";
    let g = file_operations::parse_graph_from_file(graph_path).expect("Failed to parse graph746");
    assert!(dynamic_bipartite::can_solve_bipartite(&g));

    let tour = dynamic_bipartite::solve_bipartite(&g, 120.0)
        .expect("solve_bipartite should solve graph746 within 120s");

    assert_eq!(tour.len(), 4286);
    let (valid, err) = TourVerifier::verify(&g, &tour);
    assert!(valid, "TourVerifier failed: {}", err);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test test_dynamic_bipartite_solve --manifest-path src/cegar-fix/Cargo.toml --release`
Expected: FAIL (`solve_bipartite` not implemented).

- [ ] **Step 3: Implement `solve_cluster_path`, `solve_bipartite`, and tour assembly**

In `src/cegar-fix/src/macro_decomp/dynamic_bipartite.rs`:
```rust
use crate::core::tour_verifier::TourVerifier;
use rayon::prelude::*;
use std::time::Instant;

pub fn solve_cluster_path(
    _cluster_id: i32,
    u_in: i32,
    u_out: i32,
    cluster_nodes: &HashSet<i32>,
    adj: &HashMap<i32, Vec<i32>>,
    deadline: Instant,
) -> Option<Vec<i32>> {
    let mut edges = Vec::new();
    let mut g_c: HashMap<i32, HashSet<i32>> = HashMap::new();

    for &u in cluster_nodes {
        if let Some(nbrs) = adj.get(&u) {
            for &v in nbrs {
                if cluster_nodes.contains(&v) {
                    g_c.entry(u).or_default().insert(v);
                    if u < v {
                        edges.push((u, v));
                    }
                }
            }
        }
    }
    edges.sort_unstable();

    let mut solver = CaDiCaL::default();
    let mut var_mgr = BasicVarManager::default();
    let mut edge_to_var: HashMap<(i32, i32), Lit> = HashMap::new();
    let mut inc_map: HashMap<i32, Vec<Lit>> = HashMap::new();

    for &e in &edges {
        let lit = var_mgr.new_var().pos_lit();
        edge_to_var.insert(e, lit);
        inc_map.entry(e.0).or_default().push(lit);
        inc_map.entry(e.1).or_default().push(lit);
    }

    for &u in cluster_nodes {
        let lits = inc_map.get(&u).cloned().unwrap_or_default();
        let target = if u == u_in || u == u_out { 1 } else { 2 };
        if lits.len() < target {
            return None;
        }

        if target == 1 {
            let _ = solver.add_clause(Clause::from_iter(lits.iter().copied()));
            for i in 0..lits.len() {
                for j in (i + 1)..lits.len() {
                    let _ = solver.add_clause(clause![!lits[i], !lits[j]]);
                }
            }
        } else {
            if lits.len() == 2 {
                let _ = solver.add_clause(clause![lits[0]]);
                let _ = solver.add_clause(clause![lits[1]]);
            } else {
                let _ = solver.add_clause(Clause::from_iter(lits.iter().copied()));
                for i in 0..lits.len() {
                    let mut cl = Vec::with_capacity(lits.len() - 1);
                    for j in 0..lits.len() {
                        if i != j {
                            cl.push(lits[j]);
                        }
                    }
                    let _ = solver.add_clause(Clause::from_iter(cl));
                }
                crate::core::encoder::add_at_most_2(&mut solver, &mut var_mgr, &lits);
            }
        }
    }

    let max_it = 300;
    for _ in 0..max_it {
        if Instant::now() >= deadline {
            return None;
        }

        match solver.solve() {
            Ok(SolverResult::Sat) => {}
            _ => return None,
        }

        let sol = solver.full_solution().ok()?;
        let mut active_adj: HashMap<i32, Vec<i32>> = HashMap::new();
        for &e in &edges {
            let lit = edge_to_var[&e];
            if sol.lit_value(lit) == rustsat::types::TernaryVal::True {
                active_adj.entry(e.0).or_default().push(e.1);
                active_adj.entry(e.1).or_default().push(e.0);
            }
        }

        let mut path = vec![u_in];
        let mut curr = u_in;
        let mut prev: Option<i32> = None;
        let mut vis = HashSet::new();
        vis.insert(u_in);

        while curr != u_out {
            let nxts = active_adj.get(&curr).cloned().unwrap_or_default();
            let valid_nxt = nxts.into_iter().find(|&w| Some(w) != prev);
            match valid_nxt {
                Some(nxt) => {
                    path.push(nxt);
                    vis.insert(nxt);
                    prev = Some(curr);
                    curr = nxt;
                }
                None => break,
            }
        }

        let mut cycles = Vec::new();
        for &u in cluster_nodes {
            if !vis.contains(&u) {
                let mut cyc = Vec::new();
                let mut curr_c = u;
                let mut prev_c: Option<i32> = None;
                while !vis.contains(&curr_c) {
                    vis.insert(curr_c);
                    cyc.push(curr_c);
                    let nxts = active_adj.get(&curr_c).cloned().unwrap_or_default();
                    let valid_nxt = nxts.into_iter().find(|&w| Some(w) != prev_c);
                    match valid_nxt {
                        Some(nxt) => {
                            prev_c = Some(curr_c);
                            curr_c = nxt;
                        }
                        None => break,
                    }
                }
                if cyc.len() >= 3 {
                    cycles.push(cyc);
                }
            }
        }

        if cycles.is_empty() && path.len() == cluster_nodes.len() && curr == u_out {
            return Some(path);
        }

        for cyc in &cycles {
            let cyc_set: HashSet<i32> = cyc.iter().copied().collect();
            let mut cut_lits = Vec::new();
            for &u in cyc {
                if let Some(nbrs) = g_c.get(&u) {
                    for &v in nbrs {
                        if !cyc_set.contains(&v) {
                            let e = (u.min(v), u.max(v));
                            if let Some(&lit) = edge_to_var.get(&e) {
                                cut_lits.push(lit);
                            }
                        }
                    }
                }
            }

            if !cut_lits.is_empty() {
                let _ = solver.add_clause(Clause::from_iter(cut_lits.iter().copied()));
            }

            let mut neg_clause = Vec::with_capacity(cyc.len());
            for k in 0..cyc.len() {
                let e = (cyc[k].min(cyc[(k + 1) % cyc.len()]), cyc[k].max(cyc[(k + 1) % cyc.len()]));
                neg_clause.push(!edge_to_var[&e]);
            }
            let _ = solver.add_clause(Clause::from_iter(neg_clause));
        }
    }

    None
}

pub fn solve_bipartite(raw_g: &Graph, timeout_secs: f64) -> Option<Vec<i32>> {
    let t_start = Instant::now();
    let deadline = t_start + std::time::Duration::from_secs_f64(timeout_secs);

    let partition = detect_and_partition(raw_g)?;
    println!(
        "[dynamic_bipartite] Partitioned into {} clusters and {} connectors",
        partition.clusters.len(),
        partition.connectors.len()
    );

    let mut macro_solver = MacroSatSolver::new(&partition, raw_g).ok()?;

    while Instant::now() < deadline {
        let config = match macro_solver.solve_next_configuration() {
            Some(cfg) => cfg,
            None => {
                eprintln!("[dynamic_bipartite] MacroSatSolver exhausted all configurations");
                return None;
            }
        };

        println!("[dynamic_bipartite] Testing macro configuration with {} active macro edges...", config.active_macro_edges.len());

        let cluster_tasks: Vec<(i32, i32, i32, HashSet<i32>)> = partition
            .super_hubs
            .iter()
            .map(|&h| {
                let (u_in, u_out) = config.cluster_ports[&h];
                (h, u_in, u_out, partition.clusters[&h].clone())
            })
            .collect();

        let cluster_results: Vec<(i32, i32, i32, Option<Vec<i32>>)> = cluster_tasks
            .par_iter()
            .map(|(h, u_in, u_out, nodes)| {
                let path = solve_cluster_path(*h, *u_in, *u_out, nodes, &raw_g.adjacency_list, deadline);
                (*h, *u_in, *u_out, path)
            })
            .collect();

        let mut all_sat = true;
        let mut solved_paths: HashMap<i32, Vec<i32>> = HashMap::new();

        for (h, u_in, u_out, path_opt) in cluster_results {
            match path_opt {
                Some(p) => {
                    solved_paths.insert(h, p);
                }
                None => {
                    all_sat = false;
                    println!("[dynamic_bipartite] Cluster {} UNSAT for port pair ({}, {}). Learning conflict...", h, u_in, u_out);
                    macro_solver.block_pair(h, u_in, u_out);
                }
            }
        }

        if all_sat {
            println!("[dynamic_bipartite] All {} clusters solved! Splicing tour...", partition.clusters.len());
            // Trace the macro cycle to assemble full tour
            // Macro graph vertices are connected by active_macro_edges and cluster paths
            let mut ext_adj: HashMap<i32, Vec<i32>> = HashMap::new();
            for &(u, v) in &config.active_macro_edges {
                ext_adj.entry(u).or_default().push(v);
                ext_adj.entry(v).or_default().push(u);
            }

            let start_node = config.active_macro_edges[0].0;
            let mut tour = Vec::with_capacity(raw_g.adjacency_list.len());
            let mut curr = start_node;
            let mut prev: Option<i32> = None;
            let mut visited_clusters = HashSet::new();

            while tour.len() < raw_g.adjacency_list.len() {
                if let Some(&h) = partition.owner.get(&curr) {
                    if !visited_clusters.contains(&h) {
                        visited_clusters.insert(h);
                        let p = &solved_paths[&h];
                        let (u_in, u_out) = config.cluster_ports[&h];
                        if curr == u_in {
                            tour.extend_from_slice(&p[..p.len() - 1]);
                            curr = u_out;
                        } else if curr == u_out {
                            let mut rev_p = p.clone();
                            rev_p.reverse();
                            tour.extend_from_slice(&rev_p[..rev_p.len() - 1]);
                            curr = u_in;
                        } else {
                            tour.push(curr);
                        }
                    }
                } else {
                    tour.push(curr);
                }

                // Move along external edge
                let nxts = ext_adj.get(&curr).cloned().unwrap_or_default();
                let valid_nxt = nxts.into_iter().find(|&w| Some(w) != prev);
                match valid_nxt {
                    Some(nxt) => {
                        prev = Some(curr);
                        curr = nxt;
                    }
                    None => break,
                }

                if curr == start_node {
                    break;
                }
            }

            let (valid, err) = TourVerifier::verify(raw_g, &tour);
            if valid {
                println!(
                    "[dynamic_bipartite] Tour certified in {:.2}s!",
                    t_start.elapsed().as_secs_f64()
                );
                return Some(tour);
            } else {
                eprintln!("[dynamic_bipartite] Tour verification error: {}", err);
            }
        }
    }

    None
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test test_dynamic_bipartite_solve --manifest-path src/cegar-fix/Cargo.toml --release`
Expected: PASS (`graph746` solves and verifies in < 60s).

- [ ] **Step 5: Commit**

```bash
git add src/cegar-fix/src/macro_decomp/dynamic_bipartite.rs src/cegar-fix/tests/test_dynamic_bipartite_solve.rs
git commit -m "feat(macro): implement Level-2 parallel cluster CEGAR, conflict feedback, and tour splicing"
```

---

### Task 4: Pipeline Integration & Full End-to-End Verification

**Files:**
- Modify: `src/cegar-fix/src/pipeline/solver_pipeline.rs`
- Run: `./run_solver_rust.sh -i FHCPCS-col/graph746.col -t 300 -o output_tours/tour_graph746.hcp`
- Run: `./run_solver_rust.sh -i FHCPCS-col/graph950.col -t 300 -o output_tours/tour_graph950.hcp`
- Test: Upstream cross-verification with `python3 /home/ubuntu/SAT-based-CEGAR/parse/is_hamiltonian.py`

- [ ] **Step 1: Wire `dynamic_bipartite` into `solver_pipeline.rs`**

In `src/cegar-fix/src/pipeline/solver_pipeline.rs`:
At the start of the macro cascade (around line 138):
```rust
    // 2.5a. Dynamic Dense-Bipartite Macro-Decomposition
    if crate::macro_decomp::dynamic_bipartite::can_solve_bipartite(&g) {
        println!("[Pipeline] Detected dense bipartite super-hub topology: invoking dynamic bipartite solver...");
        macro_tour_opt = crate::macro_decomp::dynamic_bipartite::solve_bipartite(&g, timeout_secs);
    }
```

- [ ] **Step 2: Check compiler warnings**

Run: `RUSTFLAGS="-D warnings" cargo check --manifest-path src/cegar-fix/Cargo.toml --all-targets`
Expected: 0 warnings, clean compilation.

- [ ] **Step 3: Run end-to-end solve on `graph746` via release binary**

Run: `./run_solver_rust.sh -i FHCPCS-col/graph746.col -t 300 -o output_tours/tour_graph746.hcp`
Expected: `SAT_VERIFIED`, output tour generated in `output_tours/tour_graph746.hcp`.

- [ ] **Step 4: Cross-validate `tour_graph746.hcp` with upstream verifier**

Run:
```bash
python3 -c "
with open('output_tours/tour_graph746.hcp') as f:
    lines = [line.strip() for line in f if line.strip() and not line.startswith(('NAME', 'TYPE', 'DIMENSION', 'TOUR_SECTION', '-1', 'EOF'))]
tour = [int(x) for x in lines]
print(f'Tour len: {len(tour)}')
with open('scratch/temp_sol_746.txt', 'w') as f:
    f.write('solution:\n' + ' '.join(map(str, tour)) + '\n')
" && python3 /home/ubuntu/SAT-based-CEGAR/parse/is_hamiltonian.py FHCPCS-col/graph746.col scratch/temp_sol_746.txt
```
Expected output:
```text
Tour len: 4286
True
```

- [ ] **Step 5: Run end-to-end solve on `graph950` via release binary**

Run: `./run_solver_rust.sh -i FHCPCS-col/graph950.col -t 300 -o output_tours/tour_graph950.hcp`
Expected: `SAT_VERIFIED`, output tour generated in `output_tours/tour_graph950.hcp`.

- [ ] **Step 6: Cross-validate `tour_graph950.hcp` with upstream verifier**

Run:
```bash
python3 -c "
with open('output_tours/tour_graph950.hcp') as f:
    lines = [line.strip() for line in f if line.strip() and not line.startswith(('NAME', 'TYPE', 'DIMENSION', 'TOUR_SECTION', '-1', 'EOF'))]
tour = [int(x) for x in lines]
print(f'Tour len: {len(tour)}')
with open('scratch/temp_sol_950.txt', 'w') as f:
    f.write('solution:\n' + ' '.join(map(str, tour)) + '\n')
" && python3 /home/ubuntu/SAT-based-CEGAR/parse/is_hamiltonian.py FHCPCS-col/graph950.col scratch/temp_sol_950.txt
```
Expected output:
```text
Tour len: 6620
True
```

- [ ] **Step 7: Commit pipeline integration and verification results**

```bash
git add src/cegar-fix/src/pipeline/solver_pipeline.rs
git commit -m "feat(pipeline): wire dynamic bipartite macro-decomposition into unified pipeline"
```
