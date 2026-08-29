# Design Specification: Gadget Interface Port Truth Assignment & Flow Synchronizer (`InterfacePortSynchronizer`)

- **Date:** 2026-08-29
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.
- **Commitment to Scientific Rigor:** No overpromising. Empirical verification and exact timing are required.

---

## 1. Executive Summary & Mathematical Foundation

### 1.1 The Gadget Dual-Path Reality
In reduction HCP graphs (`graph479.col`, `graph668.col`, `graph950.col`), each variable/clause gadget $M_k$ contains:
- External interface ports $\{A_k, B_k\}$ connecting $M_k$ to the rest of the graph.
- Exactly two distinct internal Hamiltonian paths spanning $V(M_k)$:
  - **State $T_k$ (True Path)**: Entry at $A_k \to$ internal ladder $\to$ exit at $B_k$.
  - **State $F_k$ (False Path)**: Entry at $A_k \to$ alternate internal ladder $\to$ exit at $B_k$.
- **The Defect in Pure 2-Factor Formulation**:
  A general 2-factor allows a third, degenerate state: the **Perimeter Loop** (an isolated 16-cycle traversing the perimeter without visiting external ports).
- **The Solution — `InterfacePortSynchronizer`**:
  For each detected gadget module $M_k$:
  1. Compute internal Hamiltonian paths $T_k$ and $F_k$ using local SAT/DFS in $< 1\text{ms}$.
  2. Allocate a single binary choice literal $x_k \in \{0, 1\}$.
  3. Encode the channel selector:
     $$x_k \implies \bigwedge_{e \in T_k} e \quad \text{and} \quad \neg x_k \implies \bigwedge_{e \in F_k} e$$
  4. Encode port flow conservation: exactly one external edge enters $A_k$ and exits $B_k$.
  5. This strictly restricts the SAT solver to valid $T/F$ state combinations across the $K$ gadgets, collapsing the search space from thousands of edge permutations down to a compact $K$-variable Boolean instance.

---

## 2. Architecture & Algorithmic Design

### 2.1 Structs and Methods in `src/cegar-fix/src/interface_port_synchronizer.rs`
```rust
use std::collections::{HashMap, HashSet};
use rustsat::instances::Cnf;
use crate::graph::Graph;
use crate::encoder::Encoder;

#[derive(Debug, Clone)]
pub struct GadgetDualPath {
    pub module_id: usize,
    pub vertices: Vec<i32>,
    pub ports: [i32; 2],
    pub true_path_edges: Vec<(i32, i32)>,
    pub false_path_edges: Vec<(i32, i32)>,
}

pub struct InterfacePortSynchronizer;

impl InterfacePortSynchronizer {
    /// Detects gadget modules and extracts dual internal Hamiltonian paths (T and F).
    pub fn extract_gadget_dual_paths(
        g: &Graph,
        max_module_size: usize,
    ) -> Vec<GadgetDualPath>;

    /// Injects gadget choice literals x_k and dual-path channeling constraints into base CNF.
    pub fn encode_interface_port_synchronization(
        dual_paths: &[GadgetDualPath],
        encoder: &mut Encoder,
        cnf: &mut Cnf,
    );
}
```

---

## 3. Integration into `hcp_solver.rs`

In `hcp_solver.rs` at Round 0:
```rust
// Interface Port Truth Assignment & Flow Synchronizer
let dual_paths = InterfacePortSynchronizer::extract_gadget_dual_paths(&g, 32);
if dual_paths.len() >= 4 {
    println!("InterfacePortSynchronizer: detected {} gadget modules with dual T/F paths, injecting flow synchronization clauses", dual_paths.len());
    InterfacePortSynchronizer::encode_interface_port_synchronization(&dual_paths, &mut encoder, &mut cnf);
}
```

---

## 4. Verification Strategy

1. **Unit Tests (`tests/test_interface_port_synchronizer.rs`):**
   - Test extraction of $T$ and $F$ internal Hamiltonian paths on synthetic 16-node ladders.
   - Test channel selector clauses: asserting $x_k \implies T_k$ and $\neg x_k \implies F_k$.
   - Test that perimeter loops are rendered UNSAT by the channeling constraints.
2. **Integration Tests (`tests/test_staged_solver.rs`):**
   - Test full CEGAR solver with `InterfacePortSynchronizer` on multi-gadget graphs.
3. **Benchmark Verification:**
   - Run benchmark on `graph479.col` and `graph668.col` with `taskset -c 0,1,2 nice -n 19`.
