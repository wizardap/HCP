# Design Specification: Twin Giant Cycle Bridge Splicer & Bicomponent Cut Engine (`TwinGiantCycleSplicer`)

- **Date:** 2026-08-29
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.
- **Commitment to Scientific Rigor:** Zero Tour Injection policy (never read `.hcp.tou` files). Pure mathematical graph splicing and exact SAT cut constraints.

---

## 1. Executive Summary & Problem Context

### 1.1 Empirical Discovery in Late-Round CEGAR on `graph668.col`
In deep CEGAR rounds (e.g. Round 16), CaDiCaL produces a **Bicomponent Giant Cycle** configuration:
- Cycle $C_1$: 1,069 vertices ($37.4\%$ of graph)
- Cycle $C_2$: 1,160 vertices ($40.5\%$ of graph)
- Together, $C_1 \cup C_2$ cover **$77.9\%$ of the entire graph**!
- All 16-cycles and gadget subcycles have been completely eliminated.
- **The Bottleneck**:
  Existing stitchers treated cycles with $|C| \ge |V| / 4$ symmetrically or searched only for small cycle absorptions into a single dominant cycle ($|C| > |V|/2$).
  When two giant cycles co-exist, they cannot be absorbed greedily one vertex at a time.
- **The Solution — `TwinGiantCycleSplicer`**:
  1. **Direct 2-Opt & 3-Opt Splicing across Twin Giants**:
     - Specifically scan the cross-product of arcs $(u_1, v_1) \in C_1$ and $(u_2, v_2) \in C_2$.
     - Discover direct 2-opt reversals or 3-cycle triangle merges through intermediate cycles $C_k$.
     - When found, immediately fuse $C_1 \cup C_2$ (and $C_k$) into a single super-giant cycle of $> 2,300$ vertices.
  2. **Bicomponent Cut Clause Injection**:
     - When twin giant cycles exist and cannot be spliced in $O(1)$ post-processing, inject the exact bicomponent cut clause:
       $\bigvee_{(u, v) \in \delta^+(C_1 \to C_2)} x_{uv}$
     - This forces CaDiCaL in the next increment to route Hamiltonian paths across the $C_1 \leftrightarrow C_2$ interface.

---

## 2. Architecture & Algorithmic Design

### 2.1 Structs and Methods in `src/cegar-fix/src/twin_giant_splicer.rs`
```rust
use crate::graph::Graph;
use rustsat::types::{Clause, Lit};
use std::collections::{HashMap, HashSet};

pub struct TwinGiantSplicer;

impl TwinGiantSplicer {
    /// Attempts direct 2-opt or 3-opt intermediate splicing between two giant cycles.
    /// Returns Some(merged_cycles) if a merge was achieved.
    pub fn try_splice_twin_giants(
        cycles: &[Vec<i32>],
        g: &Graph,
        total_v: usize,
    ) -> Option<Vec<Vec<i32>>>;

    /// Generates exact bicomponent cut clauses between two giant cycles C1 and C2.
    pub fn generate_bicomponent_cut_clauses(
        c1: &[i32],
        c2: &[i32],
        g: &Graph,
        graph_lit_map: &HashMap<(i32, i32), Lit>,
    ) -> Vec<Clause>;
}
```

---

## 3. Integration into `giant_cycle_stitcher.rs` and `hcp_solver.rs`

1. **In `GiantCycleStitcher::repair_until_fixed_point`**:
   Add Step 8: `TwinGiantSplicer::try_splice_twin_giants`.
2. **In `hcp_solver.rs`**:
   When the top two cycles in `sol_cycles` both have $|C_i| \ge |V| / 4$:
   Generate bicomponent cut clauses via `TwinGiantSplicer::generate_bicomponent_cut_clauses` and add to `working_cnf` and `accumulated_cut_cnfs`.

---

## 4. Verification Strategy

1. **Unit Tests (`tests/test_twin_giant_splicer.rs`):**
   - Test direct 2-opt splice between two large cycles.
   - Test 3-way intermediate bridge splice $C_1 \leftrightarrow C_{\text{bridge}} \leftrightarrow C_2$.
   - Test bicomponent cut clause generation.
2. **Integration Tests (`tests/test_staged_solver.rs`):**
   - Test CEGAR loop integration with twin giant components.
3. **Benchmark Verification:**
   - Run benchmark on `graph479.col` and `graph668.col` with `taskset -c 0,1,2 nice -n 19`.
