# Design Specification: Empirical Backbone Frequency Tracker & Aggressive SEC Engine (`EmpiricalBackboneCutter`)

- **Date:** 2026-08-29
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.
- **Commitment to Scientific Rigor:** Zero Tour Injection policy (never read `.hcp.tou` files). Pure mathematical frequency-based search space contraction and complete subcycle elimination.

---

## 1. Executive Summary & Problem Context

### 1.1 The Late-Round SAT Stagnation Bottleneck
In `graph668.col`, the CEGAR pipeline successfully reduces subcycles from 113 down to **26 merged cycles**, and builds giant cycles covering up to **1,752 vertices ($61.2\%$ of the graph)**.
- **The Bottleneck**:
  1. In later rounds (Round 18+), SAT solving time increases from $\sim 25\text{s}$ to $300+\text{s}$ because CaDiCaL searches a dense 2-factor space with 1,400+ accumulated clauses.
  2. `CutSelector` currently budgets and selects only a subset of subcycles (e.g. 18/26 subcycles), allowing CaDiCaL to satisfy the formula by rearranging the un-cut subcycles.
- **The Solution — `EmpiricalBackboneCutter`**:
  1. **Empirical Edge Frequency Tracking**:
     - Maintain an edge frequency map $F(e)$ over the last $W=10$ SAT solutions.
     - When an edge $e$ has $F(e) \ge 0.90$ and is part of a verified giant cycle ($|C| \ge 500$), lock it via CaDiCaL assumptions or backbone unit clauses, reducing the SAT search space by orders of magnitude.
  2. **100% Comprehensive Non-Giant SEC Cutting**:
     - At every round, generate exact directional SEC clauses for **100% of subcycles with size $< |V|/2$**, leaving zero subcycles uncut.
     - Inject 2-opt and 3-opt boundary cut clauses that forbid the isolated boundaries of these subcycles.

---

## 2. Architecture & Algorithmic Design

### 2.1 Structs and Methods in `src/cegar-fix/src/empirical_backbone_cutter.rs`
```rust
use std::collections::{HashMap, HashSet};
use rustsat::types::Lit;

#[derive(Debug, Clone, Default)]
pub struct EmpiricalBackboneTracker {
    pub history_window: usize,
    pub edge_counts: HashMap<(i32, i32), usize>,
    pub total_rounds_recorded: usize,
}

impl EmpiricalBackboneTracker {
    pub fn new(window_size: usize) -> Self;
    pub fn record_solution_edges(&mut self, cycles: &[Vec<i32>]);
    pub fn get_frequent_backbone_edges(&self, threshold: f64) -> HashSet<(i32, i32)>;
}

pub struct EmpiricalBackboneCutter;

impl EmpiricalBackboneCutter {
    /// Generates 100% comprehensive SEC clauses for all subcycles smaller than giant_threshold.
    pub fn generate_comprehensive_sec_clauses(
        cycles: &[Vec<i32>],
        giant_threshold: usize,
        lit_map: &HashMap<(i32, i32), Lit>,
    ) -> Vec<Vec<Lit>>;
}
```

---

## 3. Integration into `hcp_solver.rs`

In `hcp_solver.rs` CEGAR loop:
```rust
// Track empirical edge frequencies
backbone_tracker.record_solution_edges(&merged_cycles);
let frequent_edges = backbone_tracker.get_frequent_backbone_edges(0.85);

// Generate 100% comprehensive SEC cuts for all non-giant subcycles
let comprehensive_cuts = EmpiricalBackboneCutter::generate_comprehensive_sec_clauses(
    &merged_cycles,
    g.adjacency_list.len() / 2,
    &encoder.graph_lit_map,
);
```

---

## 4. Verification Strategy

1. **Unit Tests (`tests/test_empirical_backbone_cutter.rs`):**
   - Test frequency tracking across multiple synthetic solutions.
   - Test comprehensive SEC generation on multi-cycle sets.
   - Test threshold filtering.
2. **Integration Tests (`tests/test_staged_solver.rs`):**
   - Test CEGAR pipeline with `EmpiricalBackboneTracker` active.
3. **Benchmark Verification:**
   - Run benchmark on `graph479.col` and `graph668.col` with `taskset -c 0,1,2 nice -n 19`.
