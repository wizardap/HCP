# Design Specification: Static Substructure Cycle Cutter Engine (`StaticCycleCutter`)

- **Date:** 2026-08-28
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.

---

## 1. Executive Summary & Problem Context

### 1.1 The Static Small-Cycle Redundancy Bottleneck
In empirical benchmarks on challenge instances (`graph479.col`, `graph668.col`, etc.):
- Following degree-2 contraction, graphs possess hundreds of small induced cycles (e.g. 519 static 4-cycles on `graph479.col`).
- Unconstrained SAT solving at Round 0 blindly generates hundreds of independent 4-cycles ($117$ on Round 0, $67$ on Round 1, $86$ on Round 2).
- Dynamic CEGAR requires 20–30 expensive solver iterations just to iteratively discover and refute these static 4-cycles.

### 1.2 The Solution: `StaticCycleCutter`
- Before the CEGAR solving loop begins (at Round 0), statically enumerate all induced 4-cycles (and 3-cycles/triangles) in the contracted graph in $O(|V| \cdot \Delta^2)$ time ($< 5\text{ms}$).
- For each detected static small cycle $(u, v, w, z)$:
  - Generate directional subtour elimination clauses:
    $$(\neg x_{u,v} \lor \neg x_{v,w} \lor \neg x_{w,z} \lor \neg x_{z,u}) \quad \text{and} \quad (\neg x_{u,z} \lor \neg x_{z,w} \lor \neg x_{w,v} \lor \neg x_{v,u})$$
  - Add these clauses directly to `base_cnf`.
- This permanently eliminates 100% of all static 4-cycles from Round 0 onwards with zero overhead, allowing the SAT solver to construct macro-scale structures immediately.

---

## 2. Mathematical Soundness & Completeness

### 2.1 Invariant
Let $C = (v_1, v_2, \dots, v_k)$ be an induced cycle in $G$ with $k < |V(G)|$.
In any valid Hamiltonian tour $T$, $T$ cannot contain all $k$ edges of $C$ simultaneously (otherwise $T = C$, which contradicts $|T| = |V(G)| > k$).
Therefore, asserting $\bigvee_{i=1}^k \neg x_{v_i, v_{i+1}}$ for both traversal directions of $C$ is 100% sound and preserves all valid Hamiltonian cycles.

---

## 3. Architecture & Code Changes

### 3.1 `src/cegar-fix/src/static_cycle_cutter.rs`
```rust
use std::collections::HashSet;
use rustsat::instances::Cnf;
use rustsat::types::Clause;
use rustsat::clause;
use crate::graph::Graph;
use crate::encoder::Encoder;

pub struct StaticCycleCutter;

impl StaticCycleCutter {
    /// Statically finds all induced 3-cycles and 4-cycles in graph G and generates subtour elimination clauses.
    pub fn generate_static_small_cycle_cuts(
        g: &Graph,
        encoder: &Encoder,
    ) -> Cnf;
}
```

### 3.2 Integration into `src/cegar-fix/src/hcp_solver.rs`
- In `solve_hamilton`:
  - After degree-2 path mandatory edge constraints and Snark bridge locking:
    ```rust
    let static_cuts = StaticCycleCutter::generate_static_small_cycle_cuts(&g, &encoder);
    if !static_cuts.is_empty() {
        println!("StaticCycleCutter: injected {} static small-cycle elimination clauses at Round 0", static_cuts.len());
        cnf.extend(static_cuts);
    }
    ```

---

## 4. Verification Strategy

1. **Unit Tests (`tests/test_static_cycle_cutter.rs`):**
   - Test detection of 3-cycles and 4-cycles on synthetic ladder and grid graphs.
   - Assert that generated clauses contain valid literals and prevent small subtours on a toy SAT formula.
2. **Benchmark Verification:**
   - Run benchmark on `graph479.col` and verify that Round 0 contains 0 subcycles of length 4.
