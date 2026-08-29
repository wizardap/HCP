# Design Specification: CNF Subsumer & Polarity-Guided Backbone Engine (`CnfSubsumerAndPhaseGuider`)

- **Date:** 2026-08-29
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.
- **Commitment to Scientific Rigor:** Zero Tour Injection policy (never read `.hcp.tou` files). Pure mathematical clause pruning and CDCL polarity-guided heuristic steering.

---

## 1. Executive Summary & Problem Context

### 1.1 The Accumulated Clause Overhead in Deep CEGAR Rounds
In `graph668.col`, CEGAR and `EmpiricalBackboneTracker` successfully formed a **1,972-vertex giant cycle ($68.9\%$ of the graph)** and pruned 16-cycles down to **only 4 cycles**.
- **The Bottleneck**:
  By Round 14, over 3,484 block and SEC clauses accumulate. Re-feeding 3,000+ clauses to CaDiCaL causes solve times to climb to $300\text{s}+$.
  Many older 16-cycle and 24-cycle clauses are strictly subsumed by the comprehensive boundary cuts added in later rounds.
- **The Solution — `CnfSubsumerAndPhaseGuider`**:
  1. **Clause Deduplication & Subsumption**:
     - Sort and filter all accumulated cut clauses: if clause $A \subseteq B$, clause $B$ is redundant and eliminated.
     - Prune inactive subcycle clauses from rounds $> 10$ prior if their vertices have already been absorbed into the giant cycle.
     - Compresses 3,500+ clauses down to $\sim 400$ essential conflict cuts.
  2. **Polarity / Phase-Guided Backbone Steering**:
     - Set initial search polarity in CaDiCaL workers for high-frequency backbone edges ($f(e) \ge 0.85$), guiding VSIDS branching heuristics directly into the target 2-factor manifold.

---

## 2. Architecture & Algorithmic Design

### 2.1 Structs and Methods in `src/cegar-fix/src/cnf_subsumer.rs`
```rust
use rustsat::instances::Cnf;
use rustsat::types::{Clause, Lit};
use std::collections::HashSet;

pub struct CnfSubsumer;

impl CnfSubsumer {
    /// Deduplicates and removes subsumed clauses from a collection of CNF cuts.
    pub fn prune_and_subsume_cuts(cnfs: &[Cnf]) -> Cnf;
}
```

---

## 3. Integration into `hcp_solver.rs`

In `hcp_solver.rs` before `ParallelSatPortfolio::solve_portfolio` and during `SolverReseeder`:
```rust
if accumulated_cut_cnfs.len() > 500 {
    let pruned_cnf = CnfSubsumer::prune_and_subsume_cuts(&accumulated_cut_cnfs);
    println!("CnfSubsumer: pruned {} accumulated cut sets down to {} essential clauses",
        accumulated_cut_cnfs.len(), pruned_cnf.len());
    working_cnf = base_cnf.clone();
    working_cnf.extend(pruned_cnf);
}
```

---

## 4. Verification Strategy

1. **Unit Tests (`tests/test_cnf_subsumer.rs`):**
   - Test clause deduplication ($A = B$).
   - Test proper subset subsumption ($A \subset B \implies$ discard $B$).
   - Test empty and single-clause edge cases.
2. **Integration Tests (`tests/test_staged_solver.rs`):**
   - Test CEGAR pipeline with `CnfSubsumer` active.
3. **Benchmark Verification:**
   - Run benchmark on `graph479.col` and `graph668.col` with `taskset -c 0,1,2 nice -n 19`.
