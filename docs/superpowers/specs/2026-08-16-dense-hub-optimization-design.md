# Design Spec: Dense Hub Optimization (Hub-Aware Local Search & Hub Star Cuts)

**Date:** 2026-08-16  
**Topic:** Dense Hub Optimization for Hamiltonian Cycle CEGAR Solver  
**Status:** Approved by User  

---

## 1. Overview & Motivation

In the FHCP Challenge Set (FHCPCS), a distinct class of benchmark instances (specifically `graph560.col` through `graph684.col`, comprising 20 timeout instances in the official baseline) features a **Dense Hub / Hub-and-Spoke** topology:
- Graph size: ~3,300 to 3,700 vertices, ~14,000 to 15,000 edges.
- Skewed Degree Distribution: ~85% of vertices have degree 5–6, but there exist **5 Super Hubs** with degree 662–683 (each connecting to ~20% of the entire graph), plus 25–50 intermediate hubs with degree 26–137.
- **Root Cause of Baseline Timeouts:** 
  1. In standard CEGAR, SAT solvers generate hundreds of overlapping subcycles that repeatedly oscillate around and connect through the same 5 super hubs.
  2. Standard 2-opt and 3-opt heuristic merging iterates through subcycle pairs in arbitrary order, wasting millions of edge-pair checks on low-degree vertices rather than leveraging the massive connectivity of hub vertices.
  3. Intermediate subcycles forming satellite components around hubs generate weak boundary cuts that do not force active cross-hub traversal.

**Goal:** Implement a dual-pillar Dense Hub optimization:
1. **Pillar 1: Hub-Aware 2-Opt & 3-Opt Merging:** Prioritize subcycles containing or adjacent to super hubs, using hub neighbor index lookups for fast $O(|C_2|)$ candidate edge swaps.
2. **Pillar 2: Hub-Component Star Cuts:** Generate reinforced Hub Bridge clauses for subcycles whose exit boundary connects primarily to identified hubs, eliminating localized satellite subcycle oscillations.

---

## 2. Architecture & Data Structures

### 2.1 Hub Registry (`hub_registry.rs`)

A lightweight registry constructed once at graph ingestion:

```rust
use std::collections::{HashMap, HashSet};
use crate::graph::Graph;

#[derive(Clone, Debug)]
pub struct HubRegistry {
    pub is_hub: Vec<bool>,
    pub hub_vertices: Vec<i32>,
    pub hub_neighbors: HashMap<i32, HashSet<i32>>,
    pub min_hub_degree: usize,
}

impl HubRegistry {
    pub fn new(g: &Graph) -> Self {
        let total_v = g.adjacency_list.len();
        let total_deg: usize = g.adjacency_list.values().map(|v| v.len()).sum();
        let avg_deg = if total_v > 0 { total_deg as f64 / total_v as f64 } else { 0.0 };
        
        let max_deg = g.adjacency_list.values().map(|v| v.len()).max().unwrap_or(0);
        
        // A vertex is a hub if its degree is significantly above average and exceeds threshold
        let min_hub_degree = (max_deg / 2).max(20).min(50);
        
        let mut is_hub = vec![false; total_v + 1];
        let mut hub_vertices = Vec::new();
        let mut hub_neighbors = HashMap::new();
        
        for (&u, neighbors) in &g.adjacency_list {
            if neighbors.len() >= min_hub_degree && (neighbors.len() as f64) >= avg_deg * 3.0 {
                if (u as usize) < is_hub.len() {
                    is_hub[u as usize] = true;
                }
                hub_vertices.push(u);
                hub_neighbors.insert(u, neighbors.iter().cloned().collect());
            }
        }
        
        hub_vertices.sort_unstable_by(|&a, &b| {
            g.adjacency_list[&b].len().cmp(&g.adjacency_list[&a].len())
        });
        
        HubRegistry {
            is_hub,
            hub_vertices,
            hub_neighbors,
            min_hub_degree,
        }
    }
    
    pub fn is_hub_vertex(&self, v: i32) -> bool {
        if (v as usize) < self.is_hub.len() {
            self.is_hub[v as usize]
        } else {
            false
        }
    }
}
```

---

## 3. Detailed Algorithmic Design

### 3.1 Pillar 1: Hub-Aware 2-Opt & 3-Opt Merging

1. **Active Cycle Priority Reordering:**
   - When sorting `active_cycles_number`, subcycles that contain at least one hub vertex or have edges incident to hubs are prioritized at the front.
   - This ensures the local search immediately attempts merges on the highest-connectivity components.

2. **Hub Shortcut Swap (`swap_node_hub_accelerated`):**
   - If cycle $C_1$ contains a hub vertex $H$:
     - For each cycle $C_2$, check if $C_2$ contains vertices in $N(H)$.
     - Instead of testing all pairs $(u_1, u_2) \in C_1 \times C_2$, test only the neighbors of $H$ in $C_2$, reducing complexity from $O(|C_1| \cdot |C_2|)$ to $O(|C_2|)$.
   - Verify that no mandatory degree-2 contracted edge is severed (`contractor.chain_map` check preserved).

### 3.2 Pillar 2: Hub-Component Star Cuts

When CEGAR extracts active subcycles from the SAT model:
1. For each subcycle $C$:
   - Identify incident hubs: $H(C) = \{ h \in \text{hub\_vertices} \mid \exists v \in C: (v, h) \in E \}$.
   - If $|H(C)| \ge 1$ and $|C| < |V| / 2$:
     - Compute the boundary edges connecting $C$ to its incident hubs:
       $$\partial_{\text{hub}}(C) = \{ (u, h) \in E \mid u \in C, h \in H(C) \setminus C \}$$
     - In addition to standard minimal cut SEC clauses, generate the **Hub Bridge Clause**:
       $$\bigvee_{(u, h) \in \partial_{\text{hub}}(C)} (s_{u, h} \lor s_{h, u})$$
     - If $\partial_{\text{hub}}(C)$ represents the primary egress of $C$, this clause enforces that any valid Hamiltonian cycle must cross the hub boundary, immediately pruning satellite subcycles.

---

## 4. Integration into Solver Pipeline

1. **Module Creation:** Create `src/cegar-fix/src/hub_registry.rs`.
2. **Initialization:** Instantiate `HubRegistry::new(&contracted_g)` in `main.rs` and pass `&hub_registry` to `solve_hamilton`.
3. **Local Search Integration:** Pass `&hub_registry` to `two_opt`, `merge_cycles`, and `merge_three_cycles`.
4. **CEGAR Cutting Integration:** In `get_blocking_clauses` (method 3), add Hub Bridge Star Cut generation when `hub_registry.hub_vertices.len() > 0`.

---

## 5. Verification & Testing Strategy

1. **Unit Tests (`cargo test hub_registry`):**
   - `test_hub_identification`: Verify accurate hub classification on synthetic graphs with skewed degree distributions.
   - `test_hub_prioritized_merge`: Verify fast 2-opt cycle joining using hub adjacency.
   - `test_hub_bridge_clause_generation`: Verify syntactic and semantic validity of generated Hub Bridge clauses.

2. **10 Key Regression Graphs (Zero Regression Gate):**
   - `graph45`, `graph132`, `graph161`, `graph178`, `graph183`, `graph230`, `graph248`, `graph313`, `graph339`, `graph346` must all pass with 100% `s SATISFIABLE`.

3. **Dense Hub Performance Profiling:**
   - Measure solving time and CEGAR iteration count on `graph560.col`, `graph562.col`, `graph584.col`, and `graph647.col`.
