# Design Specification: Two-Tier Demand-Coordinated HCP Solver for graph950

**Date:** 2026-08-21  
**Target Problem:** Hamiltonian Cycle on `FHCPCS-col/graph950.col` ($n = 6,620$, $m = 28,718$)  
**Resource Constraints:** Total runtime $\le 1800$s, Max 1–2 CPU cores, **Zero Tour Injection** (absolutely zero external tour reading).

---

## 1. Problem Context & Root Cause Analysis

### 1.1 Structural Properties of `graph950.col`
- **310 Hub Vertices ($d \ge 20$):**
  - 10 Super Hubs ($S$): degree $d = 662$ (connects to 100% of vertices in every adjacent strip).
  - 50 Big Hubs ($B$): degree $d = 133$ (connects to 100% of vertices in adjacent strip).
  - 250 Medium Hubs ($M$): degree $22 \le d \le 34$.
  - 650 Direct Hub-Hub edges.
- **6,310 Bulk Vertices ($d < 20$):**
  - Partitioned into 74 completely disconnected strips: 50 large strips (size 125), 12 small strips (size 3), 12 small strips (size 2).
  - **Zero bulk-bulk cross edges:** Bulk vertices only connect internally within their own strip or to adjacent Hubs.
  - **Single Strip Attachment:** 190 out of 250 M-Hubs attach to **only 1 strip**. 60 M-Hubs attach to **2 strips**.
  - Every large strip connects to exactly 1 S-Hub, 1 B-Hub, and 5 M-Hubs.

### 1.2 Why Previous Attempts Failed
1. **Flat CEGAR (Rust & Python)**: Exploded learned cut clauses on 6,620 vertices, BCP slowed down exponentially, oscillation plateau at ~93–101 subcycles.
2. **Hybrid 2-Factor + Patching**: 2-opt/3-opt rapidly reduced 386 to 157 cycles, then hit a hard wall due to graph sparsity (avg degree 8.6, no crossing edges).
3. **Naive Two-Tier Macro Selector**: Generated candidate path covers blindly and independently with fixed $K=4$ paths per strip. Because 190 M-Hubs require exact parity/degree matching, the macro CNF on 310 Hubs was mathematically infeasible (UNSAT in 0.2s) without a global coordinator.

---

## 2. Architecture & Design

The new design inverts the pipeline: **Global Demand Coordination runs FIRST**, calculating an exact, valid port assignment on 310 Hubs, and then commands the 74 strips to solve targeted, highly-constrained internal path covers with **Minimal UNSAT Core Feedback**.

```
                           ┌──────────────────────────┐
                           │   graph950.col (6620 v)  │
                           └─────────────┬────────────┘
                                         │
                                         ▼
                   ┌──────────────────────────────────────────┐
                   │ 1. Structural Decomposer                 │
                   │    Extract 310 Hubs, 650 Hub-Hub edges,  │
                   │    and 74 disconnected strips            │
                   └─────────────────────┬────────────────────┘
                                         │
                                         ▼
                   ┌──────────────────────────────────────────┐
                   │ 2. Global Demand-Matching Coordinator    │
                   │    • Exact-2 degree on all 310 Hubs      │
                   │    • Direct Hub-Hub edge offloading      │
                   │    • Flexible K_i in {2, 3, 4, 5}        │
                   │    • Assigns target M-Hub demands D_i    │
                   └──────────────┬───────────────────────────┘
                                  │
                  Demand vector   │   ▲ Minimal UNSAT Core
                  D_i per strip   │   │ conflict clauses
                                  ▼   │
                   ┌──────────────────┴───────────────────────┐
                   │ 3. Pinpointed Strip Path-Cover Solvers   │
                   │    • Fast internal acyclic SAT solving   │
                   │    • Assumption-based demand checking    │
                   │    • Extracts minimal core on UNSAT      │
                   └──────────────┬───────────────────────────┘
                                  │
                    74 Path Covers│ (when 100% strips are SAT)
                                  ▼
                   ┌──────────────────────────────────────────┐
                   │ 4. Macro Tour Splicer & Cut-Block CEGAR  │
                   │    • Splices 74 covers with Hub-Hub edges│
                   │    • Subtour elimination on 310 Hubs     │
                   └──────────────┬───────────────────────────┘
                                  │
                                  ▼
                   ┌──────────────────────────────────────────┐
                   │ 5. Independent Certification             │
                   │    • Verifies 6620 distinct vertices in G│
                   │    • Writes found_tour_puresat.hcp       │
                   └──────────────────────────────────────────┘
```

---

## 3. Detailed Component Specifications

### 3.1 Component 1: Structural Decomposer
- **Input**: Edge list from `FHCPCS-col/graph950.col`.
- **Classification**:
  - Classifies vertices by degree: $S$ ($\text{deg} > 300$), $B$ ($100 \le \text{deg} \le 300$), $M$ ($20 \le \text{deg} < 100$), Bulk ($\text{deg} < 20$).
  - BFS connected components on $G[\text{Bulk}]$ extracts all 74 strips.
  - Builds adjacency maps: `strip_to_hubs`, `hub_to_strips`, `hub_hub_adj`.

### 3.2 Component 2: Global Demand-Matching Coordinator (Phase 1.5)
- **Goal**: Find an integer edge selection $E_{HH} \subseteq E_{\text{Hub-Hub}}$ and strip demand vectors $\vec{D}_i = (d_{i,1}, \dots, d_{i,5})$ such that:
  1. For every Hub $h \in H$:
     $$\sum_{e \in \delta_{HH}(h)} x_e + \sum_{S_i \in \text{Strips}(h)} d_{i, h} = 2$$
  2. Direct Hub-Hub Offloading Priority: Maximizes direct Hub-Hub edges to satisfy M-Hubs with direct degree $\ge 2$, reducing active strip ports.
  3. Strip Endpoints Parity: Total endpoints for strip $S_i$:
     $$2 K_i = d_{i, S} + d_{i, B} + \sum_{j=1}^5 d_{i, M_j} \in \{4, 6, 8, 10\}$$
- **Implementation**: Formulated as a lightweight SAT instance with Sinz Sequential Counters using in-memory `Cadical195`.

### 3.3 Component 3: Pinpointed Strip Path-Cover Solver (Phase 1)
- **Goal**: Given strip $S_i$ and assigned M-hub demands $\vec{D}_i = (d_1, \dots, d_5)$, find a $K_i$-path cover on vertices of $S_i$.
- **Internal CNF Formulation**:
  - Vertex degrees in cover: $\text{deg}_{S_i}(v) \in \{1, 2\}$.
  - Endpoints: exactly $2K_i$ vertices have $\text{deg}_{S_i}(v) = 1$.
  - M-Hub Demand Selector Assumptions: For each M-Hub $M_j$ with demand $d_j > 0$, at least $d_j$ endpoints must belong to $\text{Ports}(M_j)$.
  - Acyclicity: Subtour blocking or ranking order encoding to prevent internal closed cycles.
- **Assumption-Based Feedback**:
  - Solve with assumptions: `solver.solve(assumptions=demand_lits)`.
  - If **SAT**: Extract path sequences and endpoint vertex IDs.
  - If **UNSAT**: Call `solver.get_core()` to extract the **Minimal UNSAT Core**. Add the learned conflict clause $\neg \bigwedge_{l \in \text{core}} l$ to the Global Coordinator.

### 3.4 Component 4: Macro Tour Splicer & Cut-Block CEGAR (Phase 2)
- **Goal**: Splicing the 74 path covers and active Hub-Hub edges into a single 6,620-vertex Hamiltonian cycle.
- **Subtour Elimination**:
  - If the combined 2-factor decomposes into $k > 1$ components, add a Cut-Crossing clause for each component across the 310 Hub graph.
  - Re-solve the Coordinator and update affected strips. (Convergence demonstrated in 1–5 iterations once Hub exact-2 balance is maintained).

### 3.5 Component 5: Independent Certification & Output (Phase 3)
- **Soundness Guarantee**: Independent raw Python verifier checks:
  1. Tour length == 6,620.
  2. All 6,620 vertices are distinct.
  3. Every consecutive pair $(v_i, v_{i+1}) \in E(G)$ and $(v_n, v_1) \in E(G)$.
- **Output**: Writes certified tour to `scratch/graph950/found_tour_puresat.hcp`.

---

## 4. Error Handling & Edge Cases

| Failure Mode | Detection | Mitigation |
|---|---|---|
| Strip UNSAT on demand $\vec{D}_i$ | `solve(assumptions)` returns False | Extract `get_core()` and feed learned conflict clause back to Coordinator. |
| Global Coordinator UNSAT | Coordinator returns False | Relax $K_i$ upper bound from 4 to 5 or allow alternative Hub-Hub matching. |
| Time limit approaching ($\ge 1700$s) | Wall-clock timer check | Abort gracefully and report intermediate cycle statistics. |

---

## 5. Implementation & Verification Plan

### 5.1 Step-by-Step Implementation
1. **Module 1**: `scratch/graph950/two_tier_decomposer.py` — Graph partitioner & topology analysis.
2. **Module 2**: `scratch/graph950/pinpointed_strip_solver.py` — Strip solver with assumption core extraction & unit test on Strip 0.
3. **Module 3**: `scratch/graph950/global_demand_coordinator.py` — Incremental Macro Demand Coordinator with Sinz counters.
4. **Module 4**: `scratch/graph950/two_tier_orchestrator.py` — End-to-end closed-loop solver integrating Components 1–5.
5. **Module 5**: End-to-end benchmark run on `FHCPCS-col/graph950.col` with strict 1800s timeout enforcement.

### 5.2 Success Criteria
- Valid Hamiltonian cycle over all 6,620 vertices of `graph950.col`.
- 100% Zero tour injection (no reading of `.tou` file).
- Runtime strictly $\le 1800$s on 1–2 CPU cores.
- Passed independent raw edge verification.
