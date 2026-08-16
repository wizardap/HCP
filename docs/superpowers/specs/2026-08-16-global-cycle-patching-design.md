# Design Document: Global Cycle Patching Framework for Dense Hub HCP

**Document ID**: `2026-08-16-global-cycle-patching-design`  
**Target Systems**: `src/cegar-fix`  
**Status**: APPROVED DESIGN SPEC  

---

## 1. Executive Summary & Problem Context

The Flinders Hamiltonian Cycle Project Challenge Set (FHCPCS, 1001 graphs) contains 75 instances that timed out in the official baseline CEGAR solver. A major cluster among these timeouts is the **Dense Hub Graphs** (`graph560` – `graph684`), characterized by:
- $3\,300$ to $3\,700$ vertices and $14\,000$ to $15\,000$ edges.
- $5$ super-hubs ($deg(h) \ge 660$) acting as star centers for dozens to hundreds of small satellite subcycles ($C_1, C_2, \dots, C_m$).
- Standard pairwise 2-opt and 3-opt heuristic search fails because satellite cycles do not have direct cross-edges between each other; they connect almost exclusively to the hubs.
- Consequently, CEGAR generates tens of thousands of incremental blocking cuts without merging cycles, leading to timeouts (>60s).

This document specifies the **Global Cycle Patching Framework**, a modular 3-phase optimization architecture to systematically eliminate cycle fragmentation while preserving 100% mathematical soundness.

---

## 2. Global Architecture & 3-Phase Roadmap

```
                       SAT Solver (CaDiCaL)
                                │
                      [ Set of k subcycles ]
                                │
        ┌───────────────────────┼───────────────────────┐
        ▼                       ▼                       ▼
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│     Phase 1     │     │     Phase 2     │     │     Phase 3     │
│  Multi-Subcycle │ ──► │   Max-Matching  │ ──► │  Chained k-Opt  │
│   Hub Patching  │     │  Global Patching│     │  Lin-Kernighan  │
└─────────────────┘     └─────────────────┘     └─────────────────┘
        │                       │                       │
        └───────────────────────┼───────────────────────┘
                                │
                     [ 1 Hamiltonian Tour ]
                    (Or Reduced Cycle Set)
```

### Phase 1: Multi-Subcycle Hub Patching (Primary Scope)
- **Star-Topology Splicing**: Sequentially splices multiple satellite subcycles $C_i$ incident to super-hubs directly into the main cycle $C_{\text{main}}$ in a single $O(\sum |C_i|)$ pass.
- **Immediate Termination**: If $C_{\text{main}}$ absorbs all subcycles to reach length $N$, the solver outputs `s SATISFIABLE` immediately at increment 0/1.

### Phase 2: Maximum Matching Global Patching (Future Phase)
- **Merge Compatibility Graph $\mathcal{H}$**: Nodes represent remaining subcycles, weighted edges represent valid 2-opt/3-opt connections.
- **Simultaneous Disjoint Merges**: Solves Maximum Weight Matching to execute all non-interfering pairwise merges in parallel, reducing cycle count $k \to k/2$ in one global step.

### Phase 3: Chained $k$-Opt Variable Depth Search (Future Phase)
- **Lin-Kernighan Extension**: Explores alternating edge-switch chains of variable depth ($k \ge 4$) to bridge multi-cycle bottlenecks where pairwise or triplet merges are insufficient.

---

## 3. Detailed Specification: Phase 1 (Multi-Subcycle Hub Patching)

### 3.1 Data Structures & Signatures

In `src/cegar-fix/src/patching.rs` (new module):

```rust
use std::collections::{HashSet, HashMap};
use crate::graph::Graph;
use crate::contraction::Degree2Contractor;
use crate::hub_registry::HubRegistry;

pub struct HubPatcher;

impl HubPatcher {
    /// Attempts to patch satellite subcycles into a primary cycle via incident hubs.
    /// Returns the updated list of subcycles. If all subcycles are successfully merged,
    /// returns a single cycle of length `g.adjacency_list.len()`.
    pub fn patch_cycles_via_hubs(
        cycles: &[Vec<i32>],
        g: &Graph,
        contractor: &Degree2Contractor,
        hub_registry: &HubRegistry,
    ) -> Vec<Vec<i32>>;

    /// Checks if a single satellite subcycle can be spliced into the main cycle at hub `h`.
    /// Splicing requires breaking one cycle edge in `main_cycle` at `h` and one edge in `satellite_cycle`,
    /// then reconnecting them in valid orientation.
    fn try_splice_subcycle_at_hub(
        main_cycle: &mut Vec<i32>,
        satellite_cycle: &[i32],
        hub: i32,
        g: &Graph,
        contractor: &Degree2Contractor,
    ) -> bool;
}
```

### 3.2 Splicing Algorithm (Karp-style Hub Rewiring)

Given:
- Main cycle $C_{\text{main}} = [v_0, v_1, \dots, v_{k-1}]$, where $v_p = h$ is a super-hub.
  - Predecessor of $h$: $v_{p-1} = \text{pred}$ (index $(p-1+k) \pmod k$).
  - Successor of $h$: $v_{p+1} = \text{succ}$ (index $(p+1) \pmod k$).
- Satellite subcycle $C_{\text{sat}} = [w_0, w_1, \dots, w_{r-1}]$.

**Algorithm:**
1. Scan all adjacent pairs $(w_j, w_{j+1})$ along $C_{\text{sat}}$ (where $j+1$ wraps around to $0$):
   - **Guard**: Check that $(w_j, w_{j+1})$ is NOT a contracted degree-2 path:
     `!contractor.chain_map.contains_key(&(w_j, w_{j+1})) && !contractor.chain_map.contains_key(&(w_{j+1}, w_j))`.
2. **Case A (Splicing between $h$ and $\text{succ}$)**:
   - Check if edge $(h, \text{succ})$ is safe to break:
     `!contractor.chain_map.contains_key(&(h, \text{succ})) && !contractor.chain_map.contains_key(&(\text{succ}, h))`.
   - Check if $(h, w_j) \in E(G)$ and $(w_{j+1}, \text{succ}) \in E(G)$:
     - Splice orientation 1: Insert sequence $[w_j, w_{j-1}, \dots, w_0, w_{r-1}, \dots, w_{j+1}]$ between $h$ and $\text{succ}$.
   - Alternatively, check if $(h, w_{j+1}) \in E(G)$ and $(w_j, \text{succ}) \in E(G)$:
     - Splice orientation 2: Insert sequence $[w_{j+1}, w_{j+2}, \dots, w_{r-1}, w_0, \dots, w_j]$ between $h$ and $\text{succ}$.
3. **Case B (Splicing between $\text{pred}$ and $h$)**:
   - Symmetric check using $\text{pred}$ and $h$.
4. **Validation**:
   - If a valid splice orientation is found, reconstruct $C_{\text{main}}$, verify that every newly introduced adjacent pair is an edge in $G$, and mark $C_{\text{sat}}$ as absorbed.

### 3.3 Integration into Solver Pipeline

In `src/cegar-fix/src/hcp_solver.rs`:
- Immediately after CaDiCaL returns a solution yielding $k$ subcycles ($k > 1$):
  1. Call `HubPatcher::patch_cycles_via_hubs(&sol_cycles, &g, contractor, hub_registry)`.
  2. If `patched_cycles.len() == 1 && patched_cycles[0].len() == g.adjacency_list.len()`:
     - Found Hamiltonian tour $\implies$ Return immediately with `s SATISFIABLE`.
  3. If cycle count was reduced ($1 < \text{len} < k$):
     - Pass the reduced cycle set into existing 2-opt / 3-opt pipeline.
  4. If no patching was possible:
     - Fall back to standard 2-opt / 3-opt / CEGAR blocking clauses.

---

## 4. Invariants & Safety Constraints

1. **Strict 100% Mathematical Soundness**:
   - `HubPatcher` only constructs valid Hamiltonian tours or merges subcycles.
   - It **never adds over-constrained or falsified SAT clauses**.
2. **Degree-2 Path Invariant**:
   - Contracted edges in `contractor.chain_map` must NEVER be severed during splicing.
3. **Graph Invariant**:
   - Every edge in any constructed tour must exist in `g.adjacency_list`.
4. **Zero Regressions**:
   - All 10 Key Regression Graphs (`graph45`, `graph132`, `graph161`, `graph178`, `graph183`, `graph230`, `graph248`, `graph313`, `graph339`, `graph346`) must maintain 100% `s SATISFIABLE`.

---

## 5. Verification & Benchmark Strategy

1. **Unit Testing**:
   - `test_hub_patcher_single_splice`: Verify splicing a single 3-node cycle into a 6-node cycle via a central hub.
   - `test_hub_patcher_multi_satellite`: Verify splicing 5 disjoint satellite cycles into 1 main cycle in a single pass.
   - `test_hub_patcher_degree2_guard`: Verify that contracted degree-2 chains are never broken by splicing.
2. **Integration Verification**:
   - Run full 14 unit test suite (`cargo test`).
   - Run 10 Key Regression benchmarks.
3. **Dense Hub Profiling**:
   - Profile `graph560.col`, `graph562.col`, `graph584.col`, and measure cycle reduction and execution time.
