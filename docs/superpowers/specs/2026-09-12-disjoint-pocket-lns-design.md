# Design Specification: Disjoint-Pocket Corridor LNS & Backbone Phase-Hinting

**Date:** 2026-09-12  
**Status:** Under Review  
**Target:** `src/cegar-fix/src/disjoint_pocket_lns.rs`, `src/cegar-fix/src/hcp_solver.rs`  
**Related Documents:**
- Spec: [`docs/superpowers/specs/2026-09-12-corridor-cluster-mtz-design.md`](file:///home/ubuntu/HCP/docs/superpowers/specs/2026-09-12-corridor-cluster-mtz-design.md)
- Plan: [`docs/superpowers/plans/2026-09-12-corridor-cluster-mtz.md`](file:///home/ubuntu/HCP/docs/superpowers/plans/2026-09-12-corridor-cluster-mtz.md)
- Phenomenon Report: [`docs/research/phenomena/2026-09-12-graph868-corridor-parity-cliff-phenomenon.md`](file:///home/ubuntu/HCP/docs/research/phenomena/2026-09-12-graph868-corridor-parity-cliff-phenomenon.md)
- Benchmark Telemetry: [`scratch/log_graph868_cluster_mtz_1800s.txt`](file:///home/ubuntu/HCP/scratch/log_graph868_cluster_mtz_1800s.txt)

---

## 1. Executive Summary & Problem Formulation

### 1.1 The Structural Parity & Dispersed Docking Barrier
The 1,800-second benchmark on `FHCPCS-col/graph868.col` with `AlternatingPortEngine` and `PortCorridorLns` revealed that:
1. `AlternatingPortEngine` successfully compressed subcycles from 94 down to 37 cycles, expanding the Giant Cycle to **3,272 vertices (88.5% of the graph)**.
2. At 37 remaining cycles, exhaustive topological analysis (`test_external_2opt.rs`) proved that **exactly zero external 2-opt moves exist** between any of the 30 satellite cycles and the Giant Cycle.
3. Every satellite cycle connects to the Giant Cycle through docking cross-edges that are widely dispersed across the ring (e.g. at ring positions 492, 513, 707, 723, 896, 3356).
4. As a consequence, any single contiguous corridor of length $\le 30$ blocks covers at most 1 docking position, leaving the remaining ports disconnected and rendering local CaDiCaL instances structurally `UNSAT` (0 Hamiltonian paths from $P_{entry}$ to $P_{exit}$ visiting all unfrozen blocks).

### 1.2 The Disjoint-Pocket Architecture
To enable 4-opt and 6-opt cycle splicing across arbitrarily large ring distances without expanding corridor width, we design **Disjoint-Pocket Corridor LNS** (`DisjointPocketLns`):
- Instead of a single contiguous interval on the Giant Cycle, we extract **two separate, localized pockets** ($Pocket_1$ and $Pocket_2$) centered around two distinct docking ports $(d_1, d_2)$ of the target satellite cycle $C_{sat}$.
- The remainder of the Giant Cycle is partitioned into **two immutable Macro-Arcs** ($Arc_1$ from $Pocket_{1, exit}$ to $Pocket_{2, entry}$, and $Arc_2$ from $Pocket_{2, exit}$ to $Pocket_{1, entry}$).
- CaDiCaL is invoked on the union $\Omega = Pocket_1 \cup Pocket_2 \cup C_{sat}$ ($\approx 8 \sim 14$ blocks, $\le 30$ Boolean variables).
- In addition, to prevent CaDiCaL from suffering "phase drift" during global CEGAR increments, we inject the 3,272 edges of the Giant Cycle as **Backbone Phase Hints** (`solver.phase(lit)`) into CaDiCaL.

---

## 2. Mathematical Formulation & Structural Invariants

### 2.1 Disjoint Pocket Topology

Let:
- $C_{giant} = [P_0, P_1, \dots, P_{2L-1}]$ be the Giant Cycle in port coordinate order ($L$ blocks, $2L$ ports).
- $C_{sat}$ be the satellite cycle of $K_{sat}$ blocks ($2K_{sat}$ ports).
- $D = \{ (p_{sat}, p_{giant}) \in C_{sat} \times C_{giant} \mid (port\_to\_node[p_{sat}], port\_to\_node[p_{giant}]) \in E(G) \}$ be the set of docking edges.

```
                    ┌────────────────────────────────────────────────────────┐
                    │               Satellite Cycle C_sat                     │
                    │               (K_sat blocks, unfrozen)                 │
                    └──────────┬──────────────────────────────▲──────────────┘
                               │ e_dock1                      │ e_dock2
                               ▼                              │
┌──────────────────────────┐ ┌───────────────────┐ ┌──────────┴────────┐ ┌──────────────────────────┐
│  Macro-Arc 2 (Frozen)    │ │ Pocket 1 (3 blk)  │ │ Macro-Arc 1 (Froz) │ │ Pocket 2 (3 blk)         │
│  P_{2,exit} -> P_{1,in}  │>│ P_{1,in}->P_{1,out│>│ P_{1,out}->P_{2,in │>│ P_{2,in} -> P_{2,exit}   │
└──────────────────────────┘ └───────────────────┘ └────────────────────┘ └──────────┬───────────────┘
            ▲                                                                        │
            └────────────────────────────────────────────────────────────────────────┘
```

1. **Docking Pair Selection**:
   Find pairs of docking edges $((p_{s1}, p_{g1}), (p_{s2}, p_{g2})) \in D \times D$ such that:
   - $p_{s1}.block \ne p_{s2}.block$ (connecting distinct blocks in $C_{sat}$).
   - Ring separation: $d_{ring}(pos(p_{g1}), pos(p_{g2})) \ge 2 \times B + 4$ (where $B=1$ is the pocket half-width in blocks), ensuring $Pocket_1$ and $Pocket_2$ are strictly disjoint and separated by non-empty Macro-Arcs.

2. **Pocket Boundaries**:
   For a chosen docking pair with giant positions $pos_1 < pos_2$:
   - **$Pocket_1$**:
     - Center block: $g_1.block$.
     - Span: from block $pos_1/2 - B$ to $pos_1/2 + B$ (clamped modulo $L$).
     - Parity-aligned: Entry port $P_{1, entry}$ is odd ($2k+1$); Exit port $P_{1, exit}$ is even ($2m$).
   - **$Pocket_2$**:
     - Center block: $g_2.block$.
     - Span: from block $pos_2/2 - B$ to $pos_2/2 + B$ (clamped modulo $L$).
     - Parity-aligned: Entry port $P_{2, entry}$ is odd ($2k'+1$); Exit port $P_{2, exit}$ is even ($2m'$).
   - Invariant: $Pocket_1 \cap Pocket_2 = \emptyset$, $P_{1, entry} \ne P_{1, exit}$, $P_{2, entry} \ne P_{2, exit}$.

3. **Macro-Arc Invariants**:
   - $Arc_1$: the subpath of $C_{giant}$ from $(P_{1, exit} + 1)$ to $(P_{2, entry} - 1)$.
   - $Arc_2$: the subpath of $C_{giant}$ from $(P_{2, exit} + 1)$ to $(P_{1, entry} - 1)$.
   - Both $Arc_1$ and $Arc_2$ consist of complete, uncut blocks.
   - The total number of cut edges on $C_{giant}$ is **exactly 4**:
     1. $(P_{1, in\_prev}, P_{1, entry})$
     2. $(P_{1, exit}, P_{1, exit\_next})$
     3. $(P_{2, in\_prev}, P_{2, entry})$
     4. $(P_{2, exit}, P_{2, exit\_next})$

---

## 3. SAT Encoding & Solving Architecture

### 3.1 Unfrozen Scope
$$\Omega = blocks(Pocket_1) \cup blocks(Pocket_2) \cup blocks(C_{sat})$$
Total blocks: $K = |\Omega| \le 3 + 3 + 8 = 14$ blocks.

### 3.2 Clausal Constraints
1. **Degree-2 Chain Mutex**:
   For all contracted blocks in $\Omega$:
   $$(l_{uw} \lor l_{wu}) \land (\neg l_{uw} \lor \neg l_{wu})$$
   Enforces each block is traversed in exactly one direction.

2. **Macro-Arc and External Freezing Assumptions**:
   - Every block $b \notin \Omega$ has all its incident active edges asserted as unit assumptions:
     $$x_{u \to v} = 1 \quad \text{for active } (u, v)$$
     $$x_{u \to w} = 0 \quad \text{for inactive } (u, w)$$
   - The 4 boundary cuts are asserted active:
     - $x_{P_{1, in\_prev} \to P_{1, entry}} = 1$
     - $x_{P_{1, exit} \to P_{1, exit\_next}} = 1$
     - $x_{P_{2, in\_prev} \to P_{2, entry}} = 1$
     - $x_{P_{2, exit} \to P_{2, exit\_next}} = 1$
   - All other satellite cycles $C_{other} \ne C_{sat}$ are fully frozen to their active edges.

3. **Local Multi-Pocket CEGAR**:
   - Solve with CaDiCaL using the assumptions.
   - If `Sat`:
     - Extract active edges and decompose into cycles via `AlternatingPortEngine::get_cycles`.
     - Target length: $L_{target} = |C_{giant}| + |C_{sat}|$.
     - If a cycle of length $L_{target}$ is found: return `Some(merged_cycle)`.
     - If internal subcycles formed strictly within $\Omega$: add standard subcycle cut clause:
       $$\bigvee_{u \in C_{sub}, v \in N(u) \setminus C_{sub}} x_{u \to v}$$
     - Repeat up to 8 iterations.

---

## 4. Global Backbone Phase-Hinting in `hcp_solver.rs`

In `hcp_solver.rs`, when CEGAR increments proceed:
1. When Giant Cycle covers $\ge 80\%$ of vertices:
   For every edge $(u, v)$ in the Giant Cycle:
   $$\text{solver.phase}(lit(u, v))$$
2. This biases CaDiCaL's variable selection heuristics (VSIDS / CHB) to branch toward preserving existing Giant Cycle edges unless a conflict clause strictly forbids it.
3. This eliminates the 500s+ search stall caused by phase drift in Rounds 9–10.

---

## 5. Implementation & Verification Plan

### 5.1 Modules & Files
- `src/cegar-fix/src/disjoint_pocket_lns.rs`: New module implementing `DisjointPocketLns`.
- `src/cegar-fix/src/lib.rs`: Expose `pub mod disjoint_pocket_lns;`.
- `src/cegar-fix/src/hcp_solver.rs`: Wire `DisjointPocketLns::repair` and Backbone Phase-Hinting.
- `src/cegar-fix/tests/test_disjoint_pocket_lns.rs`: Unit test validating 4-opt absorption on non-contiguous docking ports.

### 5.2 Verification Gates
1. Unit test `cargo test --test test_disjoint_pocket_lns` passes.
2. Full workspace test suite `cargo test --tests` passes 100%.
3. Clean compilation `cargo build --release` with 0 warnings.
4. Benchmark validation on `graph868.col` with `taskset -c 0,1`.
