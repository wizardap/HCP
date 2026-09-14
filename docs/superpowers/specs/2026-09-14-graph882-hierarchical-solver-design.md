# Design Specification: Hierarchical Modular-Chain & Central-Block Solver for graph882.col

**Date:** 2026-09-14  
**Status:** Approved by User  
**Target Graph:** `FHCPCS-col/graph882.col` ($N = 5,686, M = 9,306$)  
**Benchmark Context:** The Flinders Challenge Set (Universal Core-33), Degree 2–14 Moderate Hub family (12 timeout instances). `graph882.col` is the sister instance to `graph717.col` and `graph710.col`.  
**Constraint:** Execution time $\le 1,800$s, zero tour injection, zero phantom edges, 100% independent verification on raw uncontracted DIMACS edge list.

---

## 1. Executive Summary & Problem Context

`graph882.col` ($N = 5,686, M = 9,306$) is one of the Flinders Challenge Set timeout instances. Direct CDCL SAT and heuristics stall due to extreme cycle shattering across 1,228 degree-2 vertices and dense modular hubs.

Topological analysis reveals that `graph882.col` shares the exact same modular structural family as `graph717.col`:
1. **Outer Modular Chain (185 vertices)**:
   - Contains a classic **167-vertex module** (169 vertices with ports $\{3195, 5200\}$) identical in structure to the 6 modules of `graph717.col`.
   - Connects via bridge edge $(3195, 4509)$ to a 16-vertex buffer block `Comp 2 + {4509}`.
   - Any Hamiltonian cycle is mathematically forced to traverse this entire 185-vertex chain as a simple Hamiltonian path between endpoints `5200` and `4779`.
2. **Central Hub Block Comp 0 (5,501 vertices)**:
   - Connects to the terminals of the outer chain via edges $(5200, 2080)$ and $(4779, 5066)$.
   - Contains all 1,228 degree-2 vertices of `graph882.col`.
   - Comp 0 contracts from 5,501 to 4,273 vertices via recursive degree-2 contraction.
   - 80 chordless triangles are eliminated statically in Round 0.
3. **Deterministic Splicing & Assembly**:
   - Adding virtual edge $e_{\text{virt}} = (2080, 5066)$ into contracted Comp 0 allows solving Comp 0 as a Hamiltonian cycle via CaDiCaL CEGAR + Cocycle Cuts + 2-opt cycle absorption.
   - Replacing $e_{\text{virt}}$ with the 185-vertex outer chain path and uncontracting all 1,228 degree-2 chains yields a certified sound 5,686-vertex Hamiltonian cycle.

---

## 2. Graph Invariants & Structural Decomposition

### 2.1. Raw Graph Properties
- $|V| = 5,686$, $|E| = 9,306$, average degree $2M/N \approx 3.273$.
- Degree distribution:
  - Degree 2: 1,228 vertices (all located within Central Block Comp 0).
  - Degree 3: 2,772 vertices.
  - Degree 4: 770 vertices.
  - Degree 5: 896 vertices.
  - Degree 14: Exactly 20 hub vertices.
- Biconnected (0 articulation points, 0 bridges).

### 2.2. Outer Chain Decomposition (185 Vertices)
- Removing 2-vertex cut $\{3195, 5200\}$ isolates the **167-vertex module** (144 vertices of Comp 1 + 15 vertices of Comp 3 + 8 hubs):
  $$V_{\text{mod}} = \text{Module 167} \cup \{3195, 5200\} \quad (|V_{\text{mod}}| = 169)$$
- The buffer block consists of 15 vertices of Comp 2 + hub 4509:
  $$V_{\text{buf}} = \text{Comp 2} \cup \{4509\} \quad (|V_{\text{buf}}| = 16)$$
- Bridge edge connecting module to buffer block: $(3195, 4509) \in E(G)$.
- Outer chain vertex set:
  $$V_{\text{chain}} = V_{\text{mod}} \cup V_{\text{buf}} \quad (|V_{\text{chain}}| = 169 + 16 = 185)$$
- Chain endpoints connecting to Central Block:
  - Terminal $5200 \in V_{\text{chain}}$ connects to $2080 \in V_{\text{comp0}}$ via edge $(5200, 2080) \in E(G)$.
  - Terminal $4779 \in V_{\text{chain}}$ connects to $5066 \in V_{\text{comp0}}$ via edge $(4779, 5066) \in E(G)$.

### 2.3. Central Block Comp 0 (5,501 Vertices)
- $V_{\text{comp0}} = V(G) \setminus V_{\text{chain}} \quad (|V_{\text{comp0}}| = 5,686 - 185 = 5,501)$.
- Interface ports: $\{2080, 5066\}$.
- Invariant check:
  - $V_{\text{chain}} \cap V_{\text{comp0}} = \emptyset$.
  - $V_{\text{chain}} \cup V_{\text{comp0}} = V(G)$ ($185 + 5,501 = 5,686$).

---

## 3. Solving Strategy & Algorithmic Pipeline

```mermaid
flowchart TD
    G["graph882.col (N=5686, M=9306)"] --> D["decomposer.py"]
    D -->|185 vertices| MOD["Outer Chain (167v Module + 16v Buffer)"]
    D -->|5501 vertices| C0["Central Block Comp 0 (1228 deg-2 nodes)"]
    MOD --> S1["module_solver.py (CaDiCaL CEGAR + Cocycle Cuts)"]
    C0 --> S2["comp0_solver.py (Degree-2 Contraction + 2-opt Absorption + Cocycle Cuts)"]
    S1 -->|185v Simple Path (5200 -> 4779)| ASM["solve_graph882.py"]
    S2 -->|5501v Cycle with virt edge (2080, 5066)| ASM
    ASM -->|5686v Simple Tour| V["verify_benchmarks.py (PASS - CERTIFIED SOUND)"]
```

### 3.1. Module & Chain Solver (`module_solver.py`)
1. **Module 167 HP**:
   - Boundary ports: $5200$ and $3195$.
   - Solve Hamiltonian path $P_{\text{mod}}$ of 169 vertices connecting $5200 \to 3195$ using CaDiCaL CEGAR with pure cocycle cuts. (Verified runtime: $< 0.5$s).
2. **Buffer Block 16 HP**:
   - Boundary ports: $4509$ and $4779$.
   - Solve Hamiltonian path $P_{\text{buf}}$ of 16 vertices connecting $4509 \to 4779$. (Verified runtime: $< 0.1$s).
3. **Chain Assembly**:
   - Connect $P_{\text{mod}}$ and $P_{\text{buf}}$ via edge $(3195, 4509)$:
     $$P_{\text{chain}} = P_{\text{mod}} + P_{\text{buf}} \quad (185\text{ vertices, endpoints } 5200 \to 4779)$$
   - Cache solved chain in `scratch/graph882/chain_path.json`.

### 3.2. Central Block Solver (`comp0_solver.py`)
1. **Augmented Graph**:
   - $G_{\text{comp0}} = G[V_{\text{comp0}}] \cup \{e_{\text{virt}}\}$, where $e_{\text{virt}} = (2080, 5066)$.
2. **Degree-2 Contraction**:
   - Recursively contract 1,228 degree-2 vertices down to 4,273 contracted vertices.
   - Maintain map from contracted edges $(u, v)$ to the chain of internal degree-2 vertices $[w_1, w_2, \dots]$.
3. **Static Triangle Cuts**:
   - Forbid all 80 chordless triangles in Round 0.
4. **CEGAR Loop with 2-Opt Cycle Absorption**:
   - Assert $e_{\text{virt}} = \text{True}$.
   - At each iteration, find connected cycle components.
   - Run 2-opt cycle absorption to actively merge subcycles where 2-opt swap moves exist.
   - For remaining subcycles $C$, add pure cocycle cuts:
     $$\bigvee_{e \in \delta(C)} x_e \ge 1$$
     (Requiring zero auxiliary variables, forcing at least 2 crossing edges due to parity).
   - Once converged to a single cycle of 4,273 vertices, expand contracted edges into the full 5,501-vertex Hamiltonian cycle containing $e_{\text{virt}}$.
   - Cache cycle in `scratch/graph882/comp0_cycle.json`.

### 3.3. Full Tour Assembly & Verification (`solve_graph882.py`)
1. **Splicing**:
   - Locate $e_{\text{virt}} = (2080, 5066)$ in the 5,501-vertex cycle of Comp 0.
   - Replace $e_{\text{virt}}$ with the 185-vertex outer chain path $P_{\text{chain}}$:
     $$\dots \to 2080 \to 5200 \xrightarrow{P_{\text{chain}}} 4779 \to 5066 \to \dots$$
   - Total length: $(5,501 - 1) + 185 = 5,686$ vertices.
2. **Export & Certification**:
   - Save tour to `scratch/graph882/found_tour_graph882.hcp`.
   - Run independent verification:
     ```bash
     python3 scratch/verify_benchmarks.py --graph FHCPCS-col/graph882.col --tour scratch/graph882/found_tour_graph882.hcp
     ```
   - Requirement: `Validation Result: PASS - CERTIFIED SOUND` with exit code 0.

---

## 4. Verification & Certification Criteria

1. **Zero Tour Injection**: No `.tou` reference files inspected or loaded.
2. **Zero Phantom Edges**: Every consecutive pair of vertices $(u, v)$ in the 5,686-vertex tour must exist in raw `FHCPCS-col/graph882.col`.
3. **100% In-Memory Execution**: The full solver must be able to run end-to-end de novo directly from the raw graph.
4. **Runtime Limit**: Total execution time $\le 1,800$s (expected runtime $\approx 200 - 400$s).
