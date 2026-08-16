# Design Document: Iterated Local Search (ILS) Patcher for Dense Hub HCP

**Document ID**: `2026-08-16-ils-patcher-design`  
**Target Systems**: `src/cegar-fix`  
**Status**: APPROVED DESIGN SPEC (Experiment 1 of 3)

---

## 1. Executive Summary & Objective

Dense Hub HCP benchmark graphs (`graph560` – `graph684`) feature 3,300+ vertices and large hub structures. While initial patching reduces subcycles by >65% in RAM ($<0.01$s), standard local search stalls at ~60–80 unmerged subcycles because greedy 2-opt/3-opt reaches local minima.

This specification designs **Iterated Local Search (ILS)** as an **independent optimization technique** (Experiment 1). By applying randomized perturbations (Double-Bridge 4-opt and non-improving edge swaps) in memory followed by fast multi-tier patching passes, ILS aims to break out of topological local minima and merge all remaining subcycles into a single Hamiltonian cycle in seconds without returning to the expensive SAT solver.

---

## 2. Architectural Design

```
                 [ Set of k subcycles from SAT / Initial Patchers ]
                                          │
                                          ▼
                                ┌───────────────────┐
                                │   ILS Main Loop   │ ◄─────────────────────────┐
                                └───────────────────┘                           │
                                          │                                     │
                                          ▼                                     │
                               [ Select Cycle to Kick ]                         │
                                          │                                     │
                                          ▼                                     │
                          [ Randomized Perturbation Kick ]                      │
                       (Double-Bridge 4-opt / Random Bridge)                    │
                                          │                                     │
                                          ▼                                     │
                             [ Fast Re-Patching Cascade ]                       │
                           (Hub + Matching + Chained LK)                        │
                                          │                                     │
                                          ▼                                     │
                                 [ Improvement Check ]                          │
                         - If k = 1: Return Hamiltonian Tour                    │
                         - If k < best_k: Update best state ────────────────────┤
                         - If stalled: Random restart / kick next ──────────────┘
```

---

## 3. Data Structures & Interface Specification

In `src/cegar-fix/src/ils_patcher.rs` (new module):

```rust
use crate::graph::Graph;
use crate::contraction::Degree2Contractor;
use crate::hub_registry::HubRegistry;

pub struct IteratedLocalSearchPatcher;

impl IteratedLocalSearchPatcher {
    /// Executes the Iterated Local Search loop to escape local minima in RAM.
    /// Returns the updated list of subcycles. If a full tour is found,
    /// returns a single cycle of length `g.adjacency_list.len()`.
    pub fn solve_via_ils(
        cycles: &[Vec<i32>],
        g: &Graph,
        contractor: &Degree2Contractor,
        hub_registry: &HubRegistry,
        max_kicks: usize,
    ) -> Vec<Vec<i32>>;

    /// Applies a 4-opt Double-Bridge perturbation or multi-edge reconnection on a target cycle.
    fn perturb_cycle(
        cycle: &[i32],
        g: &Graph,
        contractor: &Degree2Contractor,
        seed: u64,
    ) -> Option<Vec<i32>>;

    /// Helper checking if an edge is safe to break (degree-2 invariant).
    fn is_safe_to_break(u: i32, v: i32, contractor: &Degree2Contractor) -> bool;
}
```

---

## 4. Invariants & Safety Constraints

1. **Strict 100% Mathematical Soundness**:
   - `IteratedLocalSearchPatcher` only constructs valid Hamiltonian tours or merges subcycles.
   - It **never adds over-constrained or falsified SAT clauses**.
2. **Degree-2 Path Invariant**:
   - Contracted edges in `contractor.chain_map` must NEVER be severed during perturbation or splicing.
3. **Graph Adjacency Invariant**:
   - Every newly created edge in any modified cycle MUST exist in `g.adjacency_list`.
4. **Independent Activation**:
   - Integrated as a clean, self-contained module in `cegar()`, enabled by default or configurable via CLI.

---

## 5. Verification & Benchmark Plan

1. **Unit Tests (`ils_patcher.rs`)**:
   - `test_ils_double_bridge_validity`: Verifies that double-bridge perturbations produce valid simple cycles of identical vertex sets.
   - `test_ils_escapes_local_minimum`: Verifies that a synthetic 4-subcycle graph in a strict 2-opt/3-opt dead-end is solved to $k=1$ via ILS perturbation.
   - `test_ils_degree2_safety`: Verifies that degree-2 contracted edges are strictly preserved.
2. **Regression Benchmark**:
   - Run 10 Key Regression Graphs (`graph45`, `graph132`, `graph161`, etc.) $\implies 10/10$ `s SATISFIABLE`.
3. **Dense Hub Profiling**:
   - Benchmark independently on `graph560.col` and `graph562.col` to measure whether ILS converges to $k=1$ in seconds.
