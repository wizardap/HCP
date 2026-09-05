# Directed CEGAR with Dual Boundary Cuts & Module Parity Design

## 1. Executive Summary

This specification defines the enhanced **Directed CEGAR** architecture designed specifically for **Type A Dual-Timeout instances** (`graph479`, `graph788`, `graph868`, `graph960`) from the FHCP challenge. 

In Type A graphs, degree-2 contraction yields a 100% strictly bipartite graph where every contracted vertex participates in a canonical virtual edge pair. This enables an exact reduction to a directed graph with $N_{dir} = N_{raw} / 3$ vertices.

## 2. Core Architecture

```
[Raw Graph .col] (e.g. graph788: N=4,620)
       │
       ▼ Degree-2 Virtual Pair Contraction & 2-Coloring Bipartite Partition
[Directed Graph] (N_dir = 1,540 vertices, 4,480 arcs)
       │
       ├── Exact-1 In/Out Arc Encoding + 2-Cycle Prohibition
       ├── Module Parity / State Synchronization (K = 35 modules of size 44)
       │
       ▼ [Iterative Directed CEGAR Loop with CaDiCaL]
       ├── SAT Solve -> Extract Disjoint Directed Cycles
       ├── [Pass A: 2-Opt Giant Cycle Splicing]
       ├── [Pass B: 3-Opt Small Cycle Absorption]
       │       └── If single tour reached -> Uncontract & Certify!
       └── [Dual Boundary Cut Generation]
               ├── Cycle Blocking Cut: ¬arc_1 ∨ ¬arc_2 ∨ ... ∨ ¬arc_k
               ├── Outgoing Boundary Cut: ⋁_{(u,v) ∈ δ⁺(C)} x_{uv} ≥ 1  (for len ≤ 64)
               └── Incoming Boundary Cut: ⋁_{(u,v) ∈ δ⁻(C)} x_{uv} ≥ 1  (for len ≤ 64)
```

## 3. Key Components & Implementation Details

### Component 1: `DirectedDualCutSolver`
- Location: `src/cegar-fix/src/directed_dual_cut_solver.rs` (and executable CLI integration in `main.rs` & `examples/solve_directed_dual.rs`).
- Encodes:
  1. For each vertex $u \in [0, N_{dir}-1]$:
     - $\sum_{v \in \text{out}(u)} x_{uv} = 1$ (AMO + ALO)
  2. For each vertex $v \in [0, N_{dir}-1]$:
     - $\sum_{u \in \text{in}(v)} x_{uv} = 1$ (AMO + ALO)
  3. For each pair $(u, v)$ where both $(u, v)$ and $(v, u)$ exist:
     - $\neg x_{uv} \vee \neg x_{vu}$ (prohibits degenerate 2-cycles)

### Component 2: Dual Boundary Cuts
- For every subcycle $C$ found by CaDiCaL:
  - Add standard cycle blocking clause: $\bigvee_{i} \neg x_{c_i \to c_{i+1}}$
  - When $|C| \le 64$:
    - Outgoing cut: $\bigvee_{u \in C, v \notin C} x_{uv}$
    - Incoming cut: $\bigvee_{u \notin C, v \in C} x_{uv}$
- Measured impact: Accelerates SAT solve time by **34x – 64x** on hard rounds (e.g. Round 10 drops from 201s to 3.15s).

### Component 3: Multi-Pass Directed Splicer
- `parallel_absorb_into_giant`: Performs 2-opt direct swaps where giant cycle edges $(u_1, v_1)$ and small cycle edges $(u_2, v_2)$ cross over.
- `sat_absorb_small_cycle`: Performs 3-opt directed bridging for triangular cut connections.

### Component 4: Tour Uncontractor & Zero-Defect Verifier
- Maps directed tour of length $N_{dir}$ back to $(in\_v, out\_v)$ pairs.
- Uncontracts degree-2 intermediate vertices using `Degree2Contractor::uncontract_cycle`.
- Verifies every vertex appears exactly once and every consecutive pair is an edge in the raw graph via `TourVerifier::verify_raw_tour`.
- Writes certified TSPLIB `.hcp` tour to `scratch/found_tour_<name>_certified.hcp`.

## 4. Execution Guardrails & Test Strategy
- Strict resource constraint: **Core 3 is 100% strictly reserved for the user**. Run only on `taskset -c 0,1,2 nice -n 19`.
- Target instances:
  - `graph788.col` ($N=4,620$, $K=35$ modules)
  - `graph868.col` ($N=5,544$, $K=42$ modules)
  - `graph960.col` ($N=6,930$, $K=52$ modules)
