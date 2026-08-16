# Dense Hub Optimization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Dense Hub Optimization featuring Hub-Aware local search cycle merging and Hub-Component Star Cut generation to accelerate Hamiltonian Cycle solving on dense hub instances.

**Architecture:** A standalone `HubRegistry` module in `src/cegar-fix/src/hub_registry.rs` identifies super hubs and stores fast-lookup adjacency sets. `two_opt` and `merge_three_cycles` in `hcp_solver.rs` use the registry to prioritize hub-connected subcycles and accelerate candidate edge swaps. `get_blocking_clauses` generates Hub Bridge Star Cuts for satellite subcycles to suppress hub oscillation in CEGAR.

**Tech Stack:** Rust 2021 edition, `rustsat`, `cegar-fix`.

## Global Constraints

- Must preserve 100% zero regressions across all 10 key benchmark graphs (`graph45`, `graph132`, `graph161`, `graph178`, `graph183`, `graph230`, `graph248`, `graph313`, `graph339`, `graph346`).
- Must strictly maintain standard command-line compatibility (`-i <file> -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`).
- Must preserve degree-2 mandatory edge constraints and prevent cutting virtual edges in local search.
- All code changes isolated to `src/cegar-fix/src/hub_registry.rs`, `src/cegar-fix/src/hcp_solver.rs`, and `src/cegar-fix/src/main.rs`.

---

### Task 1: HubRegistry Data Structure & Hub Detection Module

**Files:**
- Create: `src/cegar-fix/src/hub_registry.rs`
- Modify: `src/cegar-fix/src/main.rs` (to register `mod hub_registry;`)

**Interfaces:**
- Produces:
  ```rust
  pub struct HubRegistry {
      pub is_hub: Vec<bool>,
      pub hub_vertices: Vec<i32>,
      pub hub_neighbors: HashMap<i32, HashSet<i32>>,
      pub min_hub_degree: usize,
  }
  impl HubRegistry {
      pub fn new(g: &Graph) -> Self;
      pub fn is_hub_vertex(&self, v: i32) -> bool;
  }
  ```

- [ ] **Step 1: Create `src/cegar-fix/src/hub_registry.rs`**

Implement `HubRegistry` with `new` and `is_hub_vertex`. Compute `avg_deg` and `max_deg`. Mark vertices with $deg(v) \ge \min(50, \max(20, \max\_deg / 2))$ and $deg(v) \ge 3 \times \bar{d}$ as hubs. Store `hub_neighbors` as `HashSet<i32>` for $O(1)$ adjacency checks.

- [ ] **Step 2: Add unit tests in `src/cegar-fix/src/hub_registry.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Graph;
    use std::collections::BTreeMap;

    fn build_test_graph(edges: &[(i32, i32)], num_v: usize) -> Graph {
        let mut adj = BTreeMap::new();
        let mut adj_btree = BTreeMap::new();
        let mut arcs = BTreeMap::new();
        for v in 1..=num_v as i32 {
            adj.insert(v, Vec::new());
            adj_btree.insert(v, std::collections::BTreeSet::new());
            arcs.insert(v, Vec::new());
        }
        for &(u, v) in edges {
            adj.get_mut(&u).unwrap().push(v);
            adj.get_mut(&v).unwrap().push(u);
            adj_btree.get_mut(&u).unwrap().insert(v);
            adj_btree.get_mut(&v).unwrap().insert(u);
        }
        Graph {
            filename: "test".to_string(),
            adjacency_list: adj,
            adjacency_list_btree: adj_btree,
            arcs,
        }
    }

    #[test]
    fn test_hub_detection_star_graph() {
        // Hub 1 connected to 30 nodes (deg=30), nodes 2..31 have deg 2 or 3
        let mut edges = Vec::new();
        for v in 2..=31 {
            edges.push((1, v));
            let next_v = if v == 31 { 2 } else { v + 1 };
            edges.push((v, next_v));
        }
        let g = build_test_graph(&edges, 31);
        let registry = HubRegistry::new(&g);
        assert!(registry.is_hub_vertex(1));
        assert_eq!(registry.hub_vertices, vec![1]);
        assert!(!registry.is_hub_vertex(2));
    }
}
```

- [ ] **Step 3: Register `mod hub_registry;` in `src/cegar-fix/src/main.rs`**

- [ ] **Step 4: Run unit tests**

```bash
cd /home/ubuntu/HCP/src/cegar-fix && cargo test hub_registry
```

- [ ] **Step 5: Commit changes**

```bash
git add src/cegar-fix/src/hub_registry.rs src/cegar-fix/src/main.rs
git commit -m "feat: implement HubRegistry for dense hub detection"
```

---

### Task 2: Pillar 1 - Hub-Aware 2-Opt & 3-Opt Local Search

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Modify: `src/cegar-fix/src/main.rs`

**Interfaces:**
- Consumes: `HubRegistry` from Task 1.
- Produces: Hub-accelerated `two_opt` and `merge_three_cycles` in `hcp_solver.rs`.

- [ ] **Step 1: Update `main.rs` to construct `HubRegistry` and pass to `solve_hamilton`**

```rust
let hub_registry = hub_registry::HubRegistry::new(&contracted_g);
if !hub_registry.hub_vertices.is_empty() {
    println!("Dense Hub optimization: detected {} hub vertices (sample: {:?})", 
        hub_registry.hub_vertices.len(), 
        &hub_registry.hub_vertices[..hub_registry.hub_vertices.len().min(5)]);
}
hcp_solver::solve_hamilton(contracted_g, &contractor, &hub_registry, ...);
```

- [ ] **Step 2: Update `two_opt` to sort active cycles by hub affinity**

In `two_opt` in `src/cegar-fix/src/hcp_solver.rs`:
Sort or partition `active_cycles_number` so that subcycles containing a hub vertex or adjacent to a hub vertex appear first in the merge loop.

- [ ] **Step 3: Accelerate `swap_node` with Hub shortcuts**

In `swap_node`:
When checking candidate edges between `cycle1` and `cycle2`:
If `cycle1` contains a hub $H \in \text{hub\_vertices}$, fast-path check the intersection of `cycle2` vertices with `hub_registry.hub_neighbors[&H]` to find cut candidates in $O(|C_2|)$.
Ensure `!contractor.chain_map.contains_key(&(u, v))` check is strictly enforced.

- [ ] **Step 4: Build and test**

```bash
cd /home/ubuntu/HCP/src/cegar-fix && cargo test && cargo build --release
```

- [ ] **Step 5: Commit changes**

```bash
git add src/cegar-fix/src/hcp_solver.rs src/cegar-fix/src/main.rs
git commit -m "feat: implement hub-aware 2-opt and 3-opt heuristic search"
```

---

### Task 3: Pillar 2 - Hub-Component Star Cuts

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs`

**Interfaces:**
- Consumes: `HubRegistry` from Task 1, `get_blocking_clauses` in `hcp_solver.rs`.
- Produces: `get_hub_star_cut_clauses(cycle: &[i32], encoder: &Encoder, g: &Graph, hub_registry: &HubRegistry) -> Vec<Clause>`.

- [ ] **Step 1: Implement `get_hub_star_cut_clauses` in `src/cegar-fix/src/hcp_solver.rs`**

```rust
pub fn get_hub_star_cut_clauses(
    cycle: &[i32],
    encoder: &Encoder,
    g: &Graph,
    hub_registry: &HubRegistry,
) -> Vec<Clause> {
    let mut clauses = Vec::new();
    if hub_registry.hub_vertices.is_empty() || cycle.len() >= g.adjacency_list.len() / 2 {
        return clauses;
    }
    
    let cycle_set: HashSet<i32> = cycle.iter().cloned().collect();
    let mut incident_hubs = Vec::new();
    for &h in &hub_registry.hub_vertices {
        if !cycle_set.contains(&h) {
            if let Some(neighbors) = hub_registry.hub_neighbors.get(&h) {
                if neighbors.iter().any(|v| cycle_set.contains(v)) {
                    incident_hubs.push(h);
                }
            }
        }
    }
    
    // If subcycle connects primarily to 1 or 2 hubs, force active egress/ingress
    if !incident_hubs.is_empty() && incident_hubs.len() <= 3 {
        let mut lits = Vec::new();
        for &u in cycle {
            if let Some(adjs) = g.adjacency_list.get(&u) {
                for &w in adjs {
                    if !cycle_set.contains(&w) && incident_hubs.contains(&w) {
                        if let Some(&lit) = encoder.graph_lit_map.get(&(u, w)) {
                            lits.push(lit);
                        }
                        if let Some(&lit) = encoder.graph_lit_map.get(&(w, u)) {
                            lits.push(lit);
                        }
                    }
                }
            }
        }
        if !lits.is_empty() {
            lits.sort_unstable();
            lits.dedup();
            clauses.push(Clause::from_vec(lits));
        }
    }
    clauses
}
```

- [ ] **Step 2: Hook `get_hub_star_cut_clauses` into `get_blocking_clauses` under `block_method == 3`**

- [ ] **Step 3: Add unit test in `src/cegar-fix/src/hcp_solver.rs`**

- [ ] **Step 4: Build and test**

```bash
cd /home/ubuntu/HCP/src/cegar-fix && cargo test && cargo build --release
```

- [ ] **Step 5: Commit changes**

```bash
git add src/cegar-fix/src/hcp_solver.rs
git commit -m "feat: implement hub-component star cuts in CEGAR solver"
```

---

### Task 4: Full Benchmark, Regression Testing & Dense Hub Profiling

**Files:**
- Create: `.superpowers/sdd/2026-08-16-dense-hub-optimization/task-4-report.md`

- [ ] **Step 1: Verify 10 Key Regression Graphs (100% Pass Required)**

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

- [ ] **Step 2: Profile Dense Hub instances**

Run on `graph560.col`, `graph562.col`, `graph584.col`, `graph647.col` and measure CEGAR iterations and solving time.

- [ ] **Step 3: Record verification results and commit**

```bash
git add .superpowers/sdd/
git commit -m "docs: record verification results for dense hub optimization"
```
