# Design Specification: 5-Cluster Macro-Ring Hierarchical Decomposition Solver for graph746.col

**Date:** 2026-09-13  
**Status:** Approved by User  
**Target Graph:** `FHCPCS-col/graph746.col` ($N = 4,286, M = 18,286$)  
**Benchmark Context:** The final remaining Dense Hub instance in the Universal Core-33 timeout benchmark set.  
**Constraint:** Execution time $\le 1,800$s (target $< 3$ minutes wall-clock using 4-core parallel CaDiCaL CEGAR), zero tour injection, zero phantom edges, 100% independent verification on raw uncontracted DIMACS edge list.

---

## 1. Executive Summary & Problem Context

`graph746.col` ($N = 4,286, M = 18,286$) has historically timed out at 1,800s across all standard solvers (Picat, ASP, baseline CEGAR) due to massive subcycle shattering induced by dense hub vertices.

Unlike the bipartite two-half ladder instances (`graph950`, `graph963`, `graph975`, `graph982`, `graph990`) which contain 10 super-hubs separated by a 2-bridge cut, `graph746.col` consists of **5 symmetric super-hubs** arranged in a 5-cluster macro-ring topology. This specification details the hierarchical decomposition, intra-cluster SAT encoding, parallel CaDiCaL CEGAR execution, and deterministic macro-ring tour assembly.

---

## 2. Graph Structural Properties & Mathematical Invariants

### 2.1. Degree Invariants
- $|V| = 4,286$, $|E| = 18,286$, density $m/n \approx 4.2664$.
- Exactly **5 super-hubs** of maximum degree 857–858:
  $$\mathcal{H}_{super} = \{1430, 3641, 3735, 3790, 3960\}$$
  - Super-hub 1430: degree 857
  - Super-hub 3641: degree 857
  - Super-hub 3735: degree 858
  - Super-hub 3790: degree 858
  - Super-hub 3960: degree 857
- Exactly **25 boundary hubs** of degree 172.
  - 23 boundary hubs are dedicated to a single super-hub.
  - 2 boundary hubs act as dual-connectors between super-hubs:
    - Vertex 3547 connects to super-hubs 1430 and 3735.
    - Vertex 3146 connects to super-hubs 3641 and 3790.
- Exactly **250 intermediate hubs** of degree 16 (50 per group).
- Exactly **4,006 bulk interior vertices** partitioning into 63 connected components:
  - 25 large strips of length 144 ($25 \times 144 = 3,600$ vertices).
  - 25 medium strips of length 15 ($25 \times 15 = 375$ vertices).
  - 6 tiny strips of length 3 ($6 \times 3 = 18$ vertices).
  - 6 tiny strips of length 2 ($6 \times 2 = 12$ vertices).
  - 1 tiny strip of length 1 ($1 \times 1 = 1$ vertex: vertex 2433).

---

## 3. Structural Decomposition: 5 Groups $\times$ 855 Vertices + 11 Macro Nodes

### 3.1. Exact Group Bulk Allocation (855 Vertices per Group)
Each of the 5 super-hubs controls:
- 5 large strips of length 144 ($5 \times 144 = 720$ vertices)
- 5 medium strips of length 15 ($5 \times 15 = 75$ vertices)
- 5 boundary hubs of degree 172 ($5$ vertices)
- 50 intermediate hubs of degree 16 ($50$ vertices)
- 1 dedicated tiny strip of length 3 ($3$ vertices)
- 1 dedicated tiny strip of length 2 ($2$ vertices)
$$\text{Group Bulk Size} = 720 + 75 + 5 + 50 + 3 + 2 = \mathbf{855 \text{ vertices}}$$
$$5 \text{ groups} \times 855 \text{ vertices} = 4,275 \text{ vertices}$$

### 3.2. Macro Corridor Remainder (11 Vertices)
$$4,286 - 4,275 = \mathbf{11 \text{ vertices}}$$
The 11 macro corridor vertices consist of:
- The 5 super-hubs: $\{1430, 3641, 3735, 3790, 3960\}$
- The 6 connector vertices:
  - Tiny strip of length 3: $\{2361, 3106, 3566\}$
  - Tiny strip of length 2: $\{1321, 3692\}$
  - Tiny strip of length 1: $\{2433\}$

---

## 4. Port Configuration Targets & Macro Cycle Traversal

Every group bulk has **strictly two external ports** connecting outside its interior.

### 4.1. Group Configuration Targets
1. **Group 1430** ($855$v):
   - $u_{in} = 3003$, $u_{out} = 2623$
   - `cfg = {"large": [3, 8, 9, 13, 16], "med": [27, 29, 30, 34, 46], "tiny": [50, 57]}`
2. **Group 3790** ($855$v):
   - $u_{in} = 2165$, $u_{out} = 1264$
   - `cfg = {"large": [4, 10, 14, 17, 18], "med": [31, 33, 39, 43, 47], "tiny": [51, 59]}`
3. **Group 3960** ($855$v):
   - $u_{in} = 1025$, $u_{out} = 3498$
   - `cfg = {"large": [1, 6, 7, 19, 23], "med": [38, 40, 42, 48, 49], "tiny": [54, 61]}`
4. **Group 3641** ($855$v):
   - $u_{in} = 3146$, $u_{out} = 2397$
   - `cfg = {"large": [5, 11, 12, 15, 24], "med": [25, 32, 37, 44, 45], "tiny": [53, 56]}`
5. **Group 3735** ($855$v):
   - $u_{in} = 46$, $u_{out} = 3547$
   - `cfg = {"large": [0, 2, 20, 21, 22], "med": [26, 28, 35, 36, 41], "tiny": [52, 58]}`

### 4.2. Full 16-Stage Macro Hamiltonian Cycle Sequence
The complete cycle traverses all 5 group bulks and all 11 macro corridor vertices:
1. Macro vertex **1430**
2. Edge $(1430, 3566) \to 3566$
3. Edge $(3566, 3003) \to$ **Group 1430 bulk** ($3003 \to 2623$, 855v)
4. Edge $(2623, 2165) \to$ **Group 3790 bulk** ($2165 \to 1264$, 855v)
5. Edge $(1264, 3692) \to 3692$
6. Edge $(3692, 1025) \to$ **Group 3960 bulk** ($1025 \to 3498$, 855v)
7. Edge $(3498, 3106) \to 3106$
8. Edge $(3106, 3960) \to 3960$
9. Edge $(3960, 3735) \to 3735$
10. Edge $(3735, 2433) \to 2433$
11. Edge $(2433, 3790) \to 3790$
12. Edge $(3790, 3146) \to$ **Group 3641 bulk** ($3146 \to 2397$, 855v)
13. Edge $(2397, 2361) \to 2361$
14. Edge $(2361, 3641) \to 3641$
15. Edge $(3641, 1321) \to 1321$
16. Edge $(1321, 46) \to$ **Group 3735 bulk** ($46 \to 3547$, 855v)
17. Closing edge $(3547, 1430) \to$ **1430** (completes the cycle).

**Verification of Invariants:**
- Bulk vertices: $5 \times 855 = 4,275$ vertices.
- Macro corridor vertices: $\{1430, 3566, 3692, 3106, 3960, 3735, 2433, 3790, 2361, 3641, 1321\} = 11$ vertices.
- Total vertices in cycle: $4,275 + 11 = \mathbf{4,286}$ vertices.
- All 16 connecting edges are verified existing edges in raw $E(G)$.

---

## 5. Parallel Execution Strategy & Performance

With 4 CPU cores available (`nproc = 4`):
- `multiprocessing.Pool(4)` solves the 5 group bulks concurrently.
- Each worker executes `solve_cluster_path` with incremental CaDiCaL CEGAR subtour cuts.
- As confirmed experimentally on Group 1430, convergence occurs in $\approx 60$s (199 iterations).
- Total parallel solve time for all 5 groups will be $\approx 1 - 2$ minutes wall-clock.
- Solved paths are persisted incrementally to `scratch/graph746/group_paths.json`.

---

## 6. Verification & Certification Protocol

1. **Deterministic Tour Output**:
   Write assembled cycle to `scratch/graph746/found_tour_graph746.hcp`.
2. **Independent Soundness Certification**:
   Run:
   ```bash
   python3 scratch/verify_benchmarks.py --graph FHCPCS-col/graph746.col --tour scratch/graph746/found_tour_graph746.hcp
   ```
   Must output: `Validation Result: PASS - CERTIFIED SOUND` with exit code 0.
3. **Zero Tour Injection & Zero Phantom Edges**:
   - Strictly no reference `.tou` files read or created.
   - 100% of the 4,286 consecutive edges verified directly against `FHCPCS-col/graph746.col`.
