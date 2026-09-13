# Design Specification: Two-Half Two-Tier Hierarchical Cluster Solver for graph990.col

**Date:** 2026-09-13  
**Status:** Approved by User  
**Target Graph:** `FHCPCS-col/graph990.col` ($N = 8,020, M = 35,018$)  
**Benchmark Constraint:** Execution time $\le 1,800$s (target $< 10$ minutes wall-clock using 4-core parallel CaDiCaL CEGAR), zero tour injection, 100% independent verification on raw edge list.

---

## 1. Executive Summary & Context

`graph990.col` is the largest ($N = 8,020, M = 35,018$) and final instance in the dense parametric hub family ($m = 4.5n - 1072$) of the Flinders Hamiltonian Cycle Project Challenge Set (FHCPCS), succeeding `graph950` ($6,620$), `graph963` ($7,020$), `graph975` ($7,420$), and `graph982` ($7,620$).

Standard flat SAT formulations and general-purpose heuristics fail on `graph990.col` due to subcycle shattering across dense bipartite clusters and high max degree ($\Delta = 802$). Building upon our certified solutions for `graph950`, `graph963`, `graph975`, and `graph982`, this specification formalizes the two-half two-tier hierarchical solver architecture for `graph990.col`.

---

## 2. Graph Structural Properties & Mathematical Invariants

### 2.1. Top-Degree Super-Hubs
- $|V| = 8,020$, $|E| = 35,018$.
- Maximum degree $\Delta = 802$.
- Exactly **10 super-hubs** of degree 802:
  - $\text{Half}_1$: $\mathcal{H}_1 = \{3076, 3517, 3728, 5293, 6726\}$
  - $\text{Half}_2$: $\mathcal{H}_2 = \{2205, 3905, 4178, 7717, 7858\}$

### 2.2. Perfect 2-Bridge Cut & Half Partition
A multi-source Voronoi BFS rooted at the 10 super-hubs splits $V(G)$ into two equal halves of exactly $4,010$ vertices each.
The edge cut $\delta(\text{Half}_1, \text{Half}_2)$ contains **strictly 2 edges**:
$$e_1 = (3517, 7858) \quad \text{and} \quad e_2 = (3728, 4178)$$
where $3517, 3728 \in \text{Half}_1$ and $7858, 4178 \in \text{Half}_2$.

### 2.3. Fundamental Theorem of the 2-Bridge Cut
**Theorem:** *In any graph $G$ where an edge cut separating $V_1$ and $V_2$ consists of exactly 2 edges $e_1 = (u_1, v_1)$ and $e_2 = (u_2, v_2)$, every Hamiltonian cycle $C$ in $G$ must contain both $e_1$ and $e_2$. Furthermore, $C \cap V_1$ is a single Hamiltonian path in $V_1$ between $u_1$ and $u_2$, and $C \cap V_2$ is a single Hamiltonian path in $V_2$ between $v_1$ and $v_2$.*

**Corollary:** The global Hamiltonian cycle problem on `graph990.col` reduces without loss of generality or completeness to:
1. Finding a Hamiltonian path $P_1$ covering all $4,010$ vertices of $\text{Half}_1$ with endpoints $u_{start} = 3517, u_{end} = 3728$.
2. Finding a Hamiltonian path $P_2$ covering all $4,010$ vertices of $\text{Half}_2$ with endpoints $v_{start} = 4178, v_{end} = 7858$.
3. Concatenating $P_1, e_2, P_2, e_1$ into a certified 8,020-vertex Hamiltonian cycle.

---

## 3. Structural Decomposition & Macro Chains

### 3.1. Internal Accounting per Half
Inside each half of $4,010$ vertices:
- **155 Hubs** (degree $\ge 20$): 5 super-hubs (deg 802) and 150 boundary hubs (deg 161, 34, 30).
- **3,855 Bulk Vertices** grouped into 37 connected components:
  - 25 large strips of length 153 ($25 \times 153 = 3,825$ vertices).
  - 6 tiny strips of length 3 ($6 \times 3 = 18$ vertices).
  - 6 tiny strips of length 2 ($6 \times 2 = 12$ vertices).
- **5 Groups $\times$ 800 Vertices Bulk**:
  - Each group bulk consists of 5 large strips ($5 \times 153 = 765$), 30 intermediate boundary hubs ($30$), 1 dedicated tiny strip of length 3 ($3$), and 1 dedicated tiny strip of length 2 ($2$):
    $$765 + 30 + 3 + 2 = 800 \text{ vertices}$$
  - $5 \times 800 = 4,000$ vertices bulk per half.
- **Macro Remainder (10 Vertices per Half)**:
  - 5 super-hubs + 1 leftover strip of len 3 + 1 leftover strip of len 2 = $5 + 3 + 2 = 10$ vertices.
  - $4,000 + 10 = 4,010$ vertices.

### 3.2. Half 1 Macro Hamiltonian Chain (4,010 Vertices)
- **Super-Hubs**: $\{3076, 3517, 3728, 5293, 6726\}$
- **Macro Nodes**: $\{78, 1225, 2062, 4974, 5263\}$
- **Chain Sequence**:
  1. Start at bridge endpoint **3517**
  2. Edge $(3517, 7644)$ $\to$ **Group 5293 bulk** ($7644 \to 5381$, 800v)
  3. Edge $(5381, 4974) \to 4974$
  4. Edge $(4974, 5293) \to 5293$
  5. Edge $(5293, 5263) \to 5263$
  6. Edge $(5263, 3862)$ $\to$ **Group 3728 bulk** ($3862 \to 3071$, 800v)
  7. Edge $(3071, 3076) \to 3076$
  8. Edge $(3076, 78) \to 78$
  9. Edge $(78, 3494)$ $\to$ **Group 3076 bulk** ($3494 \to 4316$, 800v)
  10. Edge $(4316, 3455)$ $\to$ **Group 3517 bulk** ($3455 \to 3729$, 800v)
  11. Edge $(3729, 1225) \to 1225$
  12. Edge $(1225, 3391)$ $\to$ **Group 6726 bulk** ($3391 \to 6248$, 800v)
  13. Edge $(6248, 2062) \to 2062$
  14. Edge $(2062, 6726) \to 6726$
  15. Edge $(6726, 3728) \to$ **3728** (bridge endpoint to Half 2).

All 14 macro connecting edges are verified existing edges in $E(G)$.

### 3.3. Half 2 Macro Hamiltonian Chain (4,010 Vertices)
- **Super-Hubs**: $\{2205, 3905, 4178, 7717, 7858\}$
- **Macro Nodes**: $\{567, 1040, 3950, 6068, 6262\}$
- **Chain Sequence**:
  1. Start at bridge endpoint **4178**
  2. Edge $(4178, 304)$ $\to$ **Group 2205 bulk** ($304 \to 1029$, 800v)
  3. Edge $(1029, 6262) \to 6262$
  4. Edge $(6262, 2205) \to 2205$
  5. Edge $(2205, 3950) \to 3950$
  6. Edge $(3950, 1272)$ $\to$ **Group 7858 bulk** ($1272 \to 6331$, 800v)
  7. Edge $(6331, 3905) \to 3905$
  8. Edge $(3905, 6068) \to 6068$
  9. Edge $(6068, 4011)$ $\to$ **Group 3905 bulk** ($4011 \to 5747$, 800v)
  10. Edge $(5747, 340)$ $\to$ **Group 4178 bulk** ($340 \to 4340$, 800v)
  11. Edge $(4340, 1040) \to 1040$
  12. Edge $(1040, 1121)$ $\to$ **Group 7717 bulk** ($1121 \to 7436$, 800v)
  13. Edge $(7436, 567) \to 567$
  14. Edge $(567, 7717) \to 7717$
  15. Edge $(7717, 7858) \to$ **7858** (bridge endpoint to Half 1).

All 14 macro connecting edges are verified existing edges in $E(G)$.

---

## 4. Port Configuration Targets

### Half 1 Targets:
1. `(5293, 7644, 5381, {"clusters": [5, 6, 11, 16, 18], "tiny": [26, 35]})`
2. `(3728, 3862, 3071, {"clusters": [4, 7, 9, 17, 23], "tiny": [27, 33]})`
3. `(3076, 3494, 4316, {"clusters": [8, 10, 14, 15, 24], "tiny": [29, 34]})`
4. `(3517, 3455, 3729, {"clusters": [0, 2, 3, 13, 22], "tiny": [30, 31]})`
5. `(6726, 3391, 6248, {"clusters": [1, 12, 19, 20, 21], "tiny": [28, 36]})`

### Half 2 Targets:
1. `(2205, 304, 1029, {"clusters": [2, 9, 10, 17, 20], "tiny": [29, 31]})`
2. `(7858, 1272, 6331, {"clusters": [1, 4, 5, 7, 12], "tiny": [28, 34]})`
3. `(3905, 4011, 5747, {"clusters": [8, 13, 16, 23, 24], "tiny": [25, 36]})`
4. `(4178, 340, 4340, {"clusters": [0, 11, 15, 19, 22], "tiny": [27, 32]})`
5. `(7717, 1121, 7436, {"clusters": [3, 6, 14, 18, 21], "tiny": [30, 35]})`

---

## 5. Parallel Execution Strategy

With 4 CPU cores available (`nproc = 4`), the 10 groups will be solved via a `multiprocessing.Pool(4)`:
- Each worker executes `solve_cluster_path` with incremental CaDiCaL CEGAR.
- Solved paths are persisted directly to JSON caches:
  - `scratch/graph990/half1_group_paths.json`
  - `scratch/graph990/half2_group_paths.json`
- Group 5293 already solved experimentally in 214s; all 10 groups in parallel will take $\sim 5-8$ minutes wall-clock time, well within the 1,800s limit.

---

## 6. Verification & Certification Protocol

1. **Tour Soundness Certification**:
   The final assembled tour `scratch/graph990/found_tour_graph990.hcp` must contain exactly 8,020 vertices, visit every vertex exactly once, and have every consecutive edge $(u, v) \in E(G)$ verified against raw `FHCPCS-col/graph990.col`.
2. **Independent Verifier**:
   `python3 scratch/verify_benchmarks.py --graph FHCPCS-col/graph990.col --tour scratch/graph990/found_tour_graph990.hcp`
   must return exit code 0 and output `Validation Result: PASS - CERTIFIED SOUND`.
3. **Zero Tour Injection**:
   No `.tou` file is ever read, referenced, or inspected.
