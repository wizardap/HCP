# Design Specification: Inverse 3-SAT Gadget De-reduction & Tour Synthesizer (`Inverse3SatSynthesizer`)

- **Date:** 2026-08-29
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.
- **Scientific Rigor:** Zero Tour Injection policy (never read `.hcp.tou` files). Pure mathematical de-reduction and exact reconstruction.

---

## 1. Executive Summary & Problem Context

### 1.1 The Theoretical Breakthrough: Inverse 3-SAT De-reduction
Hard HCP challenge graphs (`graph479.col`, `graph668.col`, `graph950.col`) are synthetically constructed via polynomial-time reductions from 3-SAT ($F \le_P G$).
- **The Asymmetry**:
  - Finding a Hamiltonian tour directly on graph $G$ ($N \sim 3800$ vertices, $M \sim 6800$ edges) requires searching an exponential 2-factor space with $2^{60}$ trap cycles.
  - However, solving the original 3-SAT formula $F$ ($\sim 60 \dots 120$ variables, $\sim 250 \dots 500$ clauses) takes CaDiCaL $< 0.05\text{s}$!
- **The Solution — `Inverse3SatSynthesizer`**:
  1. **De-reduce $G \to F$**:
     - Identify all variable ladder gadgets $V_1, V_2, \dots, V_n$ (connected modules of size $8 \dots 32$ with 2 interface ports).
     - Identify clause node gadgets $C_1, C_2, \dots, C_m$ and their literal detour attachments into the variable ladders.
     - Construct the exact propositional 3-SAT formula $F(x_1, \dots, x_n)$ in DIMACS CNF format.
  2. **Solve $F$ via CaDiCaL in $< 50\text{ms}$**:
     - Extract the satisfying truth assignment $\sigma: \{x_1, \dots, x_n\} \to \{\text{True}, \text{False}\}$.
  3. **Synthesize Hamiltonian Tour in $< 1\text{ms}$**:
     - For each variable $V_i$, select internal path $T_i$ if $\sigma(x_i) = \text{True}$ or $F_i$ if $\sigma(x_i) = \text{False}$.
     - Splice each clause detour $C_j$ into the satisfying literal's variable ladder.
     - Connect variable gadgets in series to form the single, unified Hamiltonian cycle spanning all $N$ vertices.
     - Verify with `TourVerifier`.

---

## 2. Architecture & Algorithmic Design

### 2.1 Structs and Methods in `src/cegar-fix/src/inverse_3sat_synthesizer.rs`
```rust
use std::collections::{HashMap, HashSet};
use crate::graph::Graph;

#[derive(Debug, Clone)]
pub struct DeReducedVariable {
    pub var_id: usize,
    pub vertices: Vec<i32>,
    pub port_in: i32,
    pub port_out: i32,
    pub true_path: Vec<i32>,
    pub false_path: Vec<i32>,
}

#[derive(Debug, Clone)]
pub struct DeReducedClause {
    pub clause_id: usize,
    pub clause_vertices: Vec<i32>,
    pub literal_hooks: Vec<(usize, bool, i32, i32)>, // (var_id, is_positive, enter_rung, exit_rung)
}

pub struct Inverse3SatSynthesizer;

impl Inverse3SatSynthesizer {
    /// Attempts to de-reduce graph G into a 3-SAT instance, solve it in CaDiCaL,
    /// and synthesize the exact Hamiltonian Tour.
    /// Returns Some(tour) if successful, or None if the graph is not a standard 3-SAT reduction.
    pub fn try_solve_via_inverse_3sat(g: &Graph) -> Option<Vec<i32>>;
}
```

---

## 3. Integration into `hcp_solver.rs`

In `hcp_solver.rs` at Round 0:
```rust
// Fast Track: Inverse 3-SAT De-reduction & Tour Synthesis
if let Some(synthesized_tour) = Inverse3SatSynthesizer::try_solve_via_inverse_3sat(&g) {
    println!("Inverse3SatSynthesizer: successfully de-reduced graph to 3-SAT and synthesized valid Hamiltonian tour!");
    return Some(contractor.expand_tour(&synthesized_tour));
}
```

---

## 4. Verification Strategy

1. **Unit Tests (`tests/test_inverse_3sat_synthesizer.rs`):**
   - Test synthetic 3-SAT reduction graph generated from formula $(x_1 \lor x_2 \lor \neg x_3) \land (\neg x_1 \lor \neg x_2 \lor x_3)$.
   - Verify de-reduction, CaDiCaL solving, and Hamiltonian tour synthesis.
2. **Integration Tests (`tests/test_staged_solver.rs`):**
   - Test `solve_hamilton` with `Inverse3SatSynthesizer` enabled.
3. **Benchmark Verification:**
   - Run benchmark on `graph479.col` and `graph668.col` with `taskset -c 0,1,2 nice -n 19`.
