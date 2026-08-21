# Design Specification: Pure SAT Two-Tier Decomposed Solver for graph950

**Date:** 2026-08-21  
**Status:** Proposed / Under Review  
**Target:** `FHCPCS-col/graph950.col` (6,620 vertices, 28,718 edges) — 100% Pure SAT (Zero tour injection / zero external hints).

---

## 1. Executive Summary & Objective

In previous iterations, the Two-Tier Decomposed Solver successfully reconstructed a certified Hamiltonian cycle for `graph950` in ~3.3 minutes, but relied on `inject_tour_covers()` to guarantee feasibility at Phase 2. Without this injection, independent randomized Phase 1 cover generation resulted in 85 M-hubs lacking valid endpoints, rendering the Macro CNF UNSAT.

**Goal:** Eliminate `inject_tour_covers()` completely. Implement **Targeted Hub-Demand Steering** in Phase 1 to guarantee that all 310 hubs (including 250 M-hubs) have balanced, diverse endpoint candidates, enabling a 100% Pure SAT solution with independent verification.

---

## 2. Architecture & Pipeline

```
graph950.col (6620 v, 28718 e)
  │
  ▼ Phase 0: Graph Decomposition
310 Hubs (10 S, 50 B, 250 M) + 64 Bulk Strips (6,310 v)
Strip-to-Hub Incidence Map: I(S_i, h) = { u ∈ S_i | (u, h) ∈ E(G) }
  │
  ▼ Phase 1: Targeted Hub-Steered Strip Cover Generation (Parallel)
  │ For each strip S_i and each adjacent M-hub h:
  │   - Encode strip path cover (K ≤ 4 paths)
  │   - Add steering constraint: ∃ u ∈ I(S_i, h) s.t. u is an endpoint (start/end of run)
  │   - Solve via CaDiCaL across random seeds
  │
  ▼ Phase 1.5: Pre-Macro Feasibility & Reachability Check
Fast reachability check: Ensure ∀ h ∈ Hubs, available_endpoints(h) + direct_edges(h) ≥ 2
  │
  ▼ Phase 2: Macro Selector CNF + Cut-Block CEGAR (Pure SAT)
  │ - Exactly 1 cover per strip (selector vars)
  │ - Exactly 1 hub per active port slot
  │ - Exactly 2 active edges (ports + directs) per hub (3-level sequential counters)
  │ - Distinct hubs per run (kills self-loops)
  │ - Subtour elimination via Cut-Block CEGAR
  │
  ▼ Phase 3: Tour Splicing & Independent Certification
Assemble 6,620-vertex cycle and verify on raw .col adjacency graph
```

---

## 3. Detailed Component Specifications

### 3.1. Phase 0: Decomposition & Incidence Mapping
* **Hub Classification:**
  * S-hubs (Super): degree $\ge 500$ (10 hubs, degree 662)
  * B-hubs (Big): $100 \le \text{degree} < 500$ (50 hubs, degree 133)
  * M-hubs (Medium): $20 \le \text{degree} < 100$ (250 hubs, degree 22–34)
* **Bulk Partitioning:** 6,310 bulk vertices grouped into 64 strips based on big-hub adjacency signature.
* **Incidence Index:** For each strip $S_i$, index all adjacent M-hubs:
  $$N_{\text{M-hub}}(S_i) = \{ h \in \text{M-hubs} \mid \exists u \in S_i, (u, h) \in E(G) \}$$

### 3.2. Phase 1: Targeted Hub-Demand Steering
* **Base Strip SAT Model:**
  * Directed arcs inside strip $S_i$.
  * In-degree $\le 1$, Out-degree $\le 1$.
  * Path start / singleton variables: $\text{start}_u$, $\text{sing}_u$.
  * Path count bound: $\sum \text{start}_u \le K$ (where $K = 4$).
* **Steering Constraint for M-Hub $h$:**
  * Endpoint indicator for vertex $u$: $e_u \iff \neg(\text{in}_u \land \text{out}_u)$.
  * Clause: $\bigvee_{u \in I(S_i, h)} e_u$ (forcing at least one vertex adjacent to $h$ to act as a path endpoint).
* **Execution:** Run across 16 parallel worker processes using CaDiCaL with 4–8 diverse seeds per target hub.

### 3.3. Phase 1.5: Global Reachability & Balance Filter
* Verify before building Macro CNF:
  * For every hub $h \in \text{Hubs}$: count distinct candidate port connections across all available strip covers.
  * Require $\text{candidates}(h) \ge 2$.
  * Output diagnostic summary showing min/avg/max candidates per hub tier (S, B, M).

### 3.4. Phase 2: Macro Selector CNF & Cut-Block CEGAR
* Pure SAT macro formulation:
  * Strip selector variables $s_{i, k}$ with $\sum_{k} s_{i, k} = 1$.
  * Port slot variables $p_{\text{slot}, h}$ with activation conditioned on $s_{i, k}$.
  * Direct hub-hub variables $d_{h_1, h_2}$.
  * Sound degree-2 constraints per hub via 3-level sequential counters:
    * $\text{At-most-2}(E_h)$
    * $\text{At-least-2}(E_h) \iff \text{At-most-}(n-2)(\neg E_h)$
  * Anti-self-loop clauses: distinct endpoints for both ends of every path.
* **Cut-Block CEGAR:**
  * For each disconnected component $C$ in the extracted 2-factor, add cut-crossing clause:
    $$\bigvee_{e \in \delta(C)} x_e \ge 1$$

### 3.5. Phase 3: Splicing & Independent Certification
* Map solved macro cycle back to bulk path sequences.
* Independent verification:
  1. Length equals 6,620.
  2. All 6,620 vertex IDs are distinct.
  3. Cycle is closed ($v_{\text{start}} = v_{\text{end}}$).
  4. Every consecutive edge $(v_i, v_{i+1})$ exists in raw `graph950.col`.
* Write output tour to standard TSPLIB/HCP format.

---

## 4. Acceptance Criteria

1. **Zero Tour Injection:** `inject_tour_covers()` is completely removed from the pipeline.
2. **Pure SAT Execution:** The entire solution is derived from `graph950.col` solely via CaDiCaL SAT solving and CEGAR cut-blocks.
3. **Sound Convergence:** Phase 2 Macro CNF converges to a single cycle in $\le 10$ CEGAR iterations.
4. **Independent Certification:** `indep_verify.py` reports `VALID HAMILTONIAN CYCLE` with 0 bad edges.
5. **Execution Time:** Total wall-clock time under 15 minutes on standard test machine.
