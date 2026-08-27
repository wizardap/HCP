# Design Specification: Dynamic Cut Selection & Clause Budgeting for CEGAR SAT Latency Control

- **Date:** 2026-08-27
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.

---

## 1. Problem Statement & Motivation

### 1.1 The SAT Latency Wall in CEGAR
On large graphs (such as `graph651.col` with $N = 3,701$ before contraction and $N = 2,933$ after contraction), the CEGAR solver produces $180 - 240$ disjoint subcycles per iteration.
- Currently, the solver generates direct exclusion clauses for **all** detected subcycles without filtering, including giant cycles of length $200 - 500$ vertices.
- In just 5–10 iterations, over $10,000$ clauses are injected into the CaDiCaL instance.
- **Latency Impact:** Per-iteration solving time explodes exponentially:
  - Iteration 0: $1.3\text{s}$ (54k base clauses)
  - Iteration 1: $15.5\text{s}$ (+1,369 clauses)
  - Iteration 2: $10.3\text{s}$ (+2,037 clauses)
  - Iteration 5+: $> 60\text{s} - 120\text{s}$ per iteration.
- As a result, the solver only completes a handful of iterations before exhausting the 1800s budget.

### 1.2 The Solution: High-Quality Cut Prioritization & Budgeting
1. **Short-Cycle Information Density:** Small cycles (length 3 to 16) generate short, powerful clauses (3 to 16 literals) that trigger immediate unit propagation and non-chronological backjumping. In contrast, cycles of length $\ge 64$ produce diffuse clauses that merely burden the two-watched-literal scheme without active propagations.
2. **Clause Budgeting:** Capping the number of new clauses added per iteration to $K_{\max} \in [30, 50]$ keeps the formula compact and bounds CaDiCaL solving latency to $\approx 0.5\text{s} - 3\text{s}$ per iteration, enabling hundreds of CEGAR rounds within 1800s.
3. **Strong Boundary Cuts for Tiny Cycles:** For tiny cycles (length $\le 8$), injecting boundary cuts $\sum_{e \in \delta(C)} x_e \ge 2$ forces the solver to cross the cut partition rather than locally shifting a single edge.

---

## 2. Mathematical Formalization & Architecture

### 2.1 Cycle Classification & Scoring
Let $\mathcal{C} = \{C_1, C_2, \dots, C_m\}$ be the set of disjoint subcycles found in the current SAT assignment ($m \le 250$).
Each cycle $C_i$ is evaluated with a score:
$$\text{Score}(C_i) = |C_i|$$
Cycles with $|C_i| > L_{\max}$ (default $L_{\max} = 64$) are excluded from static clause injection in the current round, as they naturally get broken when smaller cycles are patched or cut.

### 2.2 Cut Selection Policy
1. Filter cycles: $\mathcal{C}_{\text{cand}} = \{C \in \mathcal{C} \mid |C| \le L_{\max}\}$.
2. Sort $\mathcal{C}_{\text{cand}}$ in ascending order of $|C|$.
3. Select the top $K_{\max}$ candidate cycles: $\mathcal{C}_{\text{sel}} = \text{take}(\mathcal{C}_{\text{cand}}, K_{\max})$ (default $K_{\max} = 40$).

### 2.3 Clause Generation per Selected Cycle
For each selected cycle $C \in \mathcal{C}_{\text{sel}}$:
1. **Tiny Cycles ($|C| \le 8$):**
   - Let $\delta(C) = \{(u, v) \in E(G) \mid u \in C, v \notin C\}$ be the cut boundary.
   - If $|\delta(C)| \ge 2$:
     - Add cut crossing clause: $\bigvee_{e \in \delta(C)} x_e$.
     - If $|\delta(C)| == 2$, unit propagate both boundary edges: $[x_{e_1}], [x_{e_2}]$.
   - Add direct cycle blocking clause: $\bigvee_{e \in C} \neg x_e$.
2. **Standard Small Cycles ($8 < |C| \le 64$):**
   - Add direct cycle blocking clause: $\bigvee_{e \in C} \neg x_e$.

---

## 3. Module & Interface Design

### 3.1 `CutSelector` Struct (`src/cegar-fix/src/cut_selector.rs`)
```rust
use rustsat::types::Clause;
use crate::graph::Graph;
use crate::encoder::Encoder;

#[derive(Debug, Clone)]
pub struct CutSelectorOptions {
    pub max_cuts_per_round: usize,    // Default: 40
    pub max_cycle_len_for_cut: usize, // Default: 64
    pub small_cycle_threshold: usize, // Default: 8
    pub enable_boundary_cuts: bool,   // Default: true
}

impl Default for CutSelectorOptions {
    fn default() -> Self {
        Self {
            max_cuts_per_round: 40,
            max_cycle_len_for_cut: 64,
            small_cycle_threshold: 8,
            enable_boundary_cuts: true,
        }
    }
}

pub struct CutSelector;

impl CutSelector {
    pub fn select_and_generate_cuts(
        cycles: &[Vec<i32>],
        g: &Graph,
        encoder: &Encoder,
        options: &CutSelectorOptions,
    ) -> (Vec<Clause>, Vec<Vec<i32>>);
}
```

### 3.2 Integration into `src/cegar-fix/src/hcp_solver.rs`
- In `solve_hcp_with_cegar`:
  - When generating blocking clauses, pass `&_active_cycles` to `CutSelector::select_and_generate_cuts`.
  - Add the selected high-quality clauses to CaDiCaL.
  - Log clause selection metrics: `println!("CutSelector: selected {} high-quality cuts from {} subcycles (generated {} clauses)", selected_cycles.len(), total_cycles, clauses.len());`

---

## 4. Complexity & Performance Targets

- **Per-Round Clause Cap:** At most $40 - 80$ clauses added per round (down from $1,300 - 2,000$ clauses).
- **Formula Growth:** After 50 rounds, added clauses $\approx 2,500$ (well within CaDiCaL's fast-solving regime).
- **Target Latency:** $T_{\text{SAT}} \le 1.0\text{s} - 3.0\text{s}$ per iteration on graphs with $N \approx 3,000$.
- **Expected Rounds in 1800s:** $\ge 600 - 1,200$ rounds (up from $< 50$ rounds).

---

## 5. Verification Strategy
1. **Unit Tests (`tests/test_cut_selector.rs`):**
   - Budget capping test: verify exactly $K_{\max}$ cycles chosen when $M > K_{\max}$.
   - Priority test: verify length 3-4 cycles are prioritized over length 50+ cycles.
   - Boundary cut soundness: verify outgoing cut literals are valid graph edges.
2. **Benchmark Verification:**
   - Execute 60s benchmark on `graph651.col`.
   - Measure $T_{\text{SAT}}$ across 20+ rounds to confirm latency stability.
