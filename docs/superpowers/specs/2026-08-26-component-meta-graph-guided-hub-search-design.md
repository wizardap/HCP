# Design Specification: Component Meta-Graph & Guided Hub Search for Two-Tier HCP Solver

- **Date:** 2026-08-26
- **Status:** APPROVED (Under Superpowers SDD)
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.

---

## 1. Context & Problem Statement

### 1.1 Current State
In the Two-Tier Strip Coordinator (`TwoTierOrchestrator`), large graphs with $N > 3300$, $M/N \approx 4.34$, and 155 Hubs (`graph561.col`, `graph585.col`):
1. **Tier 1 (Strip Solver)** operates with high efficiency: all 38 strips are SATISFIED simultaneously in $\approx 100\text{ms}$ in $> 95\%$ of outer iterations.
2. **Tier 2 (Macro Coordinator)** encounters a combinatorial search bottleneck:
   - When the 38 strips are spliced through 155 hubs, they produce $15 - 25$ disconnected macro-subtours.
   - The current coordinator adds single-subtour Cut-Crossing SEC clauses ($\sum_{e \in \delta(C)} x_e \ge 2$).
   - On 155 hubs, there are tens of thousands of valid subtour partition configurations. Over 400 outer iterations, the SAT coordinator simply wanders among different 15-20 subtour configurations without converging to 1 cycle.
   - Standard 2-opt fails because many subtour pairs have zero valid cross edges in $G$ ($E(C_i, C_j) = \emptyset$).

### 1.2 Objective
Implement `ComponentMetaGraph` to:
1. Construct the meta-graph $G_{\text{meta}} = (V_{\text{component}}, E_{\text{merge}})$ in $O(|V| + |E|) < 1\text{ms}$.
2. Prune impossible 2-opt / k-opt pairs when $|E(C_i, C_j)| < 2$.
3. Compute the connected components $M_1, \dots, M_m$ of $G_{\text{meta}}$.
4. If $m > 1$ (meta-graph is disconnected), generate simultaneous **Multi-Component SEC Cuts** on each meta-component $M_a = \bigcup_{i \in M_a} C_i$, forcing the SAT coordinator to activate cross-component edges in the next iteration.

---

## 2. Mathematical Foundation & Soundness

### 2.1 Meta-Graph Definition
Let $\mathcal{C} = \{C_0, C_1, \dots, C_{K-1}\}$ be the set of $K$ disjoint subtours covering $V(G)$.
Define the undirected meta-graph $G_{\text{meta}} = (V_{\text{meta}}, E_{\text{meta}})$ where:
- $V_{\text{meta}} = \{0, 1, \dots, K-1\}$.
- $(i, j) \in E_{\text{meta}} \iff |E_G(C_i, C_j)| \ge 2$, where $E_G(C_i, C_j) = \{(u, v) \in E(G) \mid u \in C_i, v \in C_j\}$.

### 2.2 Meta-Component SEC Theorem
**Theorem:** Let $M_a \subset V_{\text{meta}}$ be a connected component of $G_{\text{meta}}$ (or any proper subset of subtours). Let $V(M_a) = \bigcup_{i \in M_a} C_i$. If $0 < |V(M_a)| < |V(G)|$, then in any valid Hamiltonian cycle $H$ of $G$:
$$\sum_{e \in \delta(V(M_a))} x_e \ge 2$$
where $\delta(V(M_a))$ is the set of edges with exactly one endpoint in $V(M_a)$.

**Proof:** Since $H$ visits all vertices of $G$, and $V(M_a)$ is a non-empty proper subset of $V(G)$, $H$ must enter $V(M_a)$ at least once and leave $V(M_a)$ at least once. Since $G$ is undirected, $H$ contains at least 2 distinct edges in the cut $\delta(V(M_a))$.

---

## 3. Architecture & Component Interfaces

### 3.1 `ComponentMetaGraph` (`src/cegar-fix/src/component_meta_graph.rs`)

```rust
use std::collections::{HashMap, HashSet};
use crate::graph::Graph;

#[derive(Debug, Clone)]
pub struct ComponentMetaGraph {
    pub num_components: usize,
    pub cross_edges: HashMap<(usize, usize), Vec<(i32, i32)>>, // (min(i,j), max(i,j)) -> edges
    pub meta_adj: Vec<Vec<usize>>,                             // component_id -> neighbor component_ids
    pub meta_components: Vec<Vec<usize>>,                      // connected components of G_meta
}

impl ComponentMetaGraph {
    /// Builds the component meta-graph in O(|V| + |E|) time.
    pub fn build(cycles: &[Vec<i32>], g: &Graph) -> Self;

    /// Checks if a 2-opt swap is structurally feasible between Ci and Cj.
    pub fn has_merge_potential(&self, c1: usize, c2: usize) -> bool;

    /// Returns true if the meta-graph is fully connected (all subtours can potentially merge).
    pub fn is_connected(&self) -> bool;

    /// Returns all meta-components (groups of subtours that can mutually connect).
    pub fn get_meta_components(&self) -> &[Vec<usize>];
}
```

### 3.2 2-Opt Fast Pruning (`src/cegar-fix/src/macro_splicer.rs`)
Before iterating through all vertex pairs $(u_1, v_1) \in C_i$ and $(u_2, v_2) \in C_j$:
```rust
if !meta_graph.has_merge_potential(i, j) {
    continue; // 0ms fast-path: no cross edges exist
}
```

### 3.3 Multi-Component SEC Generation (`src/cegar-fix/src/global_demand_coordinator.rs`)
When `meta_graph.meta_components.len() > 1`:
For each meta-component $M_a$:
1. Collect all hubs and strip ports belonging to $V(M_a)$.
2. Identify all boundary variables (Hub-Hub edges and Strip-Hub port literals) crossing between $V(M_a)$ and $V(G) \setminus V(M_a)$.
3. Add at-least-2 cardinality constraint / SEC clause on the cut variables.

---

## 4. Test Specifications

1. `test_component_meta_graph_disconnected`: Synthetic graph with 2 isolated cycles having 0 cross edges -> `is_connected() == false`, 2 meta components.
2. `test_component_meta_graph_connected`: 3 cycles connected in a triangle -> `is_connected() == true`, 1 meta component.
3. `test_2opt_fast_pruning_zero_edges`: Verify `patch_cycles_2opt` skips disconnected cycle pairs in 0ms without false splices.
4. `test_meta_component_sec_generation`: Verify multi-component cut clauses are correctly generated and prevent repeated disconnected assignments in Coordinator.

---

## 5. System & Resource Guarantees
- Single / multi-threaded test and benchmark invocations strictly use `taskset -c 0,1,2 nice -n 19`.
- Core 3 is reserved exclusively for the user.
- Wall-clock budget per instance: $T_{\max} = 1800\text{s}$.
