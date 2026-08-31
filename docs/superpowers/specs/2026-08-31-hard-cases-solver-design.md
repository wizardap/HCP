# Hard Instances Multi-Engine Solver Design (graph710, graph717, graph788)

**Date:** 2026-08-31  
**Status:** Approved by User  
**Target Instances:** `graph788.col` (Pure 70-Module Bipartite), `graph710.col` (42-Module + 24 Hubs), `graph717.col` (42-Module + 80 Hubs), `graph479.col` (42-Module Bipartite baseline)  
**Goal:** Crack the 55 universal timeout instances where reference solvers (Clingo ASP, Picat CP/SAT, baseline CEGAR) fail (>1,800s) through a 3-engine targeted architecture.

---

## 1. Problem Statement & Mathematical Bottlenecks

Across the 1,001 FHCP benchmark graphs, exactly 55 graphs timeout on both ASP (Clingo) and Picat. When testing our CEGAR solver on `graph788`, `graph710`, and `graph717`, we confirmed three distinct structural failure modes:

1. **Bipartite Parity & 2-Opt Extinction (`graph788` - 70 modules $\times$ 66 vertices):**
   - After degree-2 contraction ($4,620 \to 3,080$ vertices), the graph is strictly bipartite with alternating 50/50 protected edges (contracted degree-2 chains) and unprotected edges.
   - For any partition into $2 \le k \le 6$ macro-cycles (multiples of 44), the number of valid 2-opt and 3-opt merges preserving 100% of protected edges is **identically 0**.
   - Current stitchers return `None`, falling back to standard SECs. Because there are $\approx 2^{70} \approx 1.18 \times 10^{21}$ module configurations, the solver enters an infinite oscillation loop.

2. **Hub Clause Saturation & Encoding Explosion (`graph717` - 80 Hubs of degree 14):**
   - The presence of 80 Hubs of degree 14 generates 27,776 static cycle elimination clauses at Round 0 in `StaticCycleCutter`, taking **10.28 seconds just to encode** before SAT solving even starts.
   - The dense chordal triangles and quads around Hubs create clause pollution in CaDiCaL, degrading unit propagation.

3. **Instance Hardening around Hub Connectors (`graph710` - 24 Hubs of degree 14):**
   - Subcycles repeatedly form around the 24 Hubs (lengths 7, 8, 9, 14, 16).
   - As SEC clauses accumulate on Hub incident arcs, SAT solving time jumps exponentially from **6.75s in Round 0 to 21.04s in Round 1**, eventually reaching hundreds of seconds per increment.

---

## 2. Architectural Design: The 3-Engine Architecture

To address each failure mode cleanly without mutual interference, the architecture is divided into three decoupled components:

```
+-----------------------------------------------------------------------------------+
|                               HCP CEGAR Pipeline                                  |
+-----------------------------------------------------------------------------------+
                                        |
         [Round 0: Pre-Encoding & Degree-2 Contraction]
                                        |
                 +---------------------------------------------+
                 | Engine 1: HubAwareStaticCutter              |
                 | - Detect vertices with deg >= 10 (Hubs)      |
                 | - Throttle static 6/7/8-cycle cuts near Hubs|
                 | - Reduces encoding time 10.3s -> 0.2s       |
                 +---------------------------------------------+
                                        |
                       [CaDiCaL 2-Factor SAT Solver]
                                        |
             [Extract 2-Factor Cycles (Length & Parity Analysis)]
                                        |
       +--------------------------------+--------------------------------+
       |                                                                 |
[Cycles <= 6 Macro-Cycles]                                  [Cycles > 6, Module Partitions]
       |                                                                 |
+---------------------------------------------+     +---------------------------------------------+
| Engine 2: MacroCrossoverSplicer             |     | Engine 3: QuotientBlockCutter               |
| - Boundary interface extraction (~400 nodes)|     | - Identify 44-vertex modules                |
| - Auxiliary SAT subproblem (~800 vars)      |     | - When cycles partition module subsets S,   |
| - Solves k-opt swap (k in {4, 6, 8}) in 2ms |     |   inject cut: \/_{u in S, v not in S} x_uv  |
| - 100% protected_edges preservation         |     | - Eliminates 2^42 / 2^70 oscillation        |
+---------------------------------------------+     +---------------------------------------------+
       |                                                                 |
       +--------------------------------+--------------------------------+
                                        |
                             [Hamiltonian Tour Found]
                                        |
                         [Degree-2 Tour Uncontraction]
                                        |
                               [Tour Verification]
```

---

## 3. Component Specifications

### 3.1. Engine 1: `HubAwareStaticCutter`
- **Location:** `src/cegar-fix/src/static_cycle_cutter.rs`
- **Responsibility:** Injects static subtour elimination clauses for small chordless cycles (length 3..8) without blowing up around Hubs.
- **Interface:**
  ```rust
  pub fn generate_selective_static_cycle_cuts(
      g: &Graph,
      encoder: &Encoder,
      hub_deg_threshold: usize, // Default: 10
  ) -> Cnf;
  ```
- **Logic:**
  - Vertices with degree $\ge \text{hub\_deg\_threshold}$ are treated as Hubs.
  - Length 3 (triangles) and 4 (squares) are generated globally.
  - Length 6, 7, 8 cycles are strictly skipped if they contain more than 1 Hub vertex or if any vertex in the cycle has degree $\ge \text{hub\_deg\_threshold}$.
- **Expected Outcome:**
  - On `graph717`: Reduces static clause count from 27,776 down to $< 6,000$, cutting encoding time from 10.3s to $< 0.3\text{s}$.
  - Eliminates clause pollution in CaDiCaL, stabilizing SAT solve time at $< 5\text{s}$ per round.

### 3.2. Engine 2: `MacroCrossoverSplicer`
- **Location:** `src/cegar-fix/src/macro_crossover_splicer.rs`
- **Responsibility:** Solves an exact local auxiliary SAT subproblem on the boundary cross-edges between $2 \le k \le 6$ macro-cycles when 2-opt and 3-opt bridges are extinct.
- **Interface:**
  ```rust
  pub struct MacroCrossoverSplicer;

  impl MacroCrossoverSplicer {
      pub fn try_crossover_splice(
          cycles: &[Vec<i32>],
          g: &Graph,
          contractor: &Degree2Contractor,
      ) -> Option<Vec<i32>>;
  }
  ```
- **Logic:**
  1. Identify the boundary vertices $B$ across the macro-cycles: vertices having at least one neighbor in a different cycle.
  2. Extract all cross-edges $E_{\text{cross}} = \{(u, v) \in E(G) \mid \text{cycle}(u) \neq \text{cycle}(v)\}$.
  3. Extract unprotected cycle edges $E_{\text{unprot}} = \{(u, v) \in \text{cycles} \mid (u, v) \notin \text{chain\_map}\}$.
  4. Fix all internal edges and 100% of `protected_edges`.
  5. Encode local degree constraints: for each $u \in B$, exactly 2 incident edges must be active (1 fixed protected edge + 1 free edge, or 2 free edges).
  6. Encode connectivity: enforce that at least one cross-edge is active per macro-cycle pair, and total cycle count reduces to 1.
  7. Solve with local `CaDiCaL` instance (under 1,000 variables). If SAT, extract active edges, reconstruct single Eulerian/Hamiltonian traversal, verify validity, and return `Some(tour)`.
- **Expected Outcome:**
  - Slices through the bipartite parity barrier on `graph479` and `graph788` in 1–5ms.

### 3.3. Engine 3: `QuotientBlockCutter`
- **Location:** `src/cegar-fix/src/quotient_block_cutter.rs`
- **Responsibility:** Detects modular block clusters and injects algebraic quotient cut clauses when 2-factor subcycles form unions of whole modules.
- **Interface:**
  ```rust
  pub struct QuotientBlockCutter;

  impl QuotientBlockCutter {
      pub fn detect_modules(g: &Graph, contractor: &Degree2Contractor) -> Vec<HashSet<i32>>;
      pub fn generate_quotient_sec(
          cycles: &[Vec<i32>],
          modules: &[HashSet<i32>],
          encoder: &Encoder,
          g: &Graph,
      ) -> Vec<Clause>;
  }
  ```
- **Logic:**
  1. Groups contracted vertices into modules based on internal protected edge chains and symmetry.
  2. For any 2-factor cycle $C$ that is a union of a subset of modules $\mathcal{S}$, compute the boundary $\delta(\bigcup_{i \in \mathcal{S}} M_i)$.
  3. Emit a single aggregated cut clause asserting that at least one edge leaving the entire module subset must be traversed:
     $$\bigvee_{u \in \mathcal{S}, v \notin \mathcal{S}, (u, v) \in E(G)} x_{uv} \ge 1$$
- **Expected Outcome:**
  - Prevents CaDiCaL from exploring permutations of the same module subset, killing the $1,700\text{s}$ oscillation loop observed in `task-370`.

---

## 4. Test & Verification Plan

Following the **Test-Driven Development (TDD)** skill:
1. **Unit Tests (`tests/test_hub_aware_cutter.rs`):**
   - Synthetic graph with 1 Hub of degree 14 and peripheral nodes.
   - Verify that static cycle cutter throttles long cycles touching the Hub while preserving short cycles in the periphery.
2. **Unit Tests (`tests/test_macro_crossover_splicer.rs`):**
   - Synthetic bipartite graph with 2 macro-cycles where 2-opt and 3-opt fail (0 valid 2-opt bridges due to protected edges), but a 4-opt crossover exists.
   - Verify that `MacroCrossoverSplicer::try_crossover_splice` finds the single Hamiltonian tour and preserves all protected edges.
3. **Integration Tests (`cargo test --tests`):**
   - Run existing 51 unit tests and 31 integration tests to ensure 0 regression across the full test suite.
4. **Target Benchmarks:**
   - Run release binary under `taskset -c 0,1,2 nice -n 19` on:
     - `graph479.col` (42 modules)
     - `graph788.col` (70 modules)
     - `graph710.col` (24 Hubs)
     - `graph717.col` (80 Hubs)
   - Verify valid TSPLIB `.hcp` tour output and 0 missing vertices.
