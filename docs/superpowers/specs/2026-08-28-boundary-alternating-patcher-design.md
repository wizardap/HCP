# Design Specification: Boundary Alternating Patcher Engine (`BoundaryAlternatingPatcher`)

- **Date:** 2026-08-28
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.

---

## 1. Executive Summary & Problem Context

### 1.1 The Multi-Hop Alternating Reconnection Bottleneck
In empirical 1800s testing on `graph479.col`, the CEGAR solver condenses all 1,848 vertices into two 924-vertex cycles ($C_1, C_2$). 
However, standard 1-step 2-opt fails because the cross-edges in $\delta(C_1, C_2)$ are separated by intermediate paths across gadget modules (multi-hop offset), requiring an alternating augmenting cycle of length $2m$ ($m \in [2, 4]$).

### 1.2 The Solution: `BoundaryAlternatingPatcher`
- When active cycle count is small ($k \in [2, 4]$):
  - Extract all boundary cut edges $\delta(C_a, C_b)$ between distinct cycles.
  - Perform bounded-depth Alternating BFS to discover an augmenting alternating cycle $A = (y_1, x_1, y_2, x_2, \dots, y_m, x_m)$ where $x_i \in E(F)$ and $y_i \in E(G) \setminus E(F)$.
  - Check that no removed edge $x_i$ is a protected degree-2 contracted chain in `contractor.chain_map`.
  - Apply the symmetric difference $F' = F \oplus A$.
  - If $|F'| = 1$ and $|F'[0]| = |V(G)|$, uncontract and immediately return certified SATISFIABLE.

---

## 2. Mathematical Soundness

### 2.1 2-Factor Parity Invariant
Let $F$ be a 2-factor on $V(G)$ (i.e. every vertex has degree 2 in $F$).
An alternating cycle $A$ with respect to $F$ consists of an equal number of edges $X \subset F$ and $Y \subset E(G) \setminus F$ forming a simple cycle.
The symmetric difference $F' = (F \setminus X) \cup Y$ is guaranteed to be a valid 2-factor on $V(G)$.
If $A$ contains an odd number of cross-edges between $C_a$ and $C_b$, the two disjoint cycles $C_a$ and $C_b$ are mathematically merged into a single connected cycle.

---

## 3. Architecture & Code Changes

### 3.1 `src/cegar-fix/src/boundary_alternating_patcher.rs`
```rust
use std::collections::{HashSet, HashMap};
use crate::graph::Graph;
use crate::contraction::Degree2Contractor;

pub struct BoundaryAlternatingPatcher;

impl BoundaryAlternatingPatcher {
    /// Searches for multi-hop alternating augmenting cycles between macro-cycles (k in 2..=4)
    /// and merges them. Returns Some(merged_cycles) if cycle count is reduced.
    pub fn try_patch_macro_hemispheres(
        cycles: &[Vec<i32>],
        g: &Graph,
        contractor: &Degree2Contractor,
        max_search_depth: usize, // Default: 4
    ) -> Option<Vec<Vec<i32>>>;
}
```

### 3.2 Integration into `src/cegar-fix/src/hcp_solver.rs`
- In `cegar` solving loop (in `two_opt` and right after `HemisphereSplicer`):
  ```rust
  let sol_cycles = if sol_cycles.len() >= 2 && sol_cycles.len() <= 4 {
      if let Some(patched) = BoundaryAlternatingPatcher::try_patch_macro_hemispheres(&sol_cycles, &g, contractor, 4) {
          println!("BoundaryAlternatingPatcher: patched macro-hemispheres from {} to {} cycles", sol_cycles.len(), patched.len());
          if patched.len() == 1 && patched[0].len() == g.adjacency_list.len() {
              println!("number of subcycles found = 1 (via boundary alternating patcher)");
              let flat: Vec<i32> = patched.into_iter().flatten().collect();
              let final_tour = contractor.uncontract_cycle(&flat);
              let line = final_tour.iter().map(|i| i.to_string()).collect::<Vec<String>>().join(" ");
              let time = now - previous_time;
              let add_block_clauses_time = now - previous_time - sat_solving_time;
              println!("number of added block clauses = {}", clause_count);
              println!("add block clauses time = {:?}", add_block_clauses_time);
              println!("increment time = {:?}", time);
              println!();
              println!("solution: ");
              println!("{}\n", line);
              println!("s SATISFIABLE");
              return (count, clause_count, Some(final_tour));
          }
          patched
      } else {
          sol_cycles
      }
  } else {
      sol_cycles
  };
  ```

---

## 4. Verification Strategy

1. **Unit Tests (`tests/test_boundary_alternating_patcher.rs`):**
   - Test 3-opt and 4-opt multi-hop alternating cycle patching on synthetic 2-hemisphere topologies where simple 2-opt fails.
   - Assert degree-2 protected chain preservation.
2. **Integration Test:**
   - Verify full CEGAR loop integration in `tests/test_staged_solver.rs`.
3. **Benchmark Verification:**
   - Run benchmark on `graph479.col` to verify patching on the 2-hemisphere state.
