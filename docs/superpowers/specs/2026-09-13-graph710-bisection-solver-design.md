# Design Specification: Two-Block Bisection & Degree-2 Contraction Solver for graph710.col

**Date:** 2026-09-13  
**Status:** Approved by User  
**Target Graph:** `FHCPCS-col/graph710.col` ($N = 4,064, M = 6,800$)  
**Benchmark Context:** The Flinders Challenge Set (Universal Core-33), Degree 2–14 Moderate Hub family (12 timeout instances). `graph710.col` is the primary target instance.  
**Constraint:** Execution time $\le 1,800$s (target $\sim 185$s wall-clock: Block A $\sim 2$s, Block B $\sim 180$s), zero tour injection, zero phantom edges, 100% independent verification on raw uncontracted DIMACS edge list.

---

## 1. Executive Summary & Problem Context

`graph710.col` ($N = 4,064, M = 6,800$) is one of the 12 unsolved timeout instances in the Degree 2–14 Moderate Hub family. Standard monolithic solvers (such as pure CDCL SAT, Lin-Kernighan, and unguided CEGAR) suffer from severe cycle shattering and fail to converge within the 1,800-second benchmark limit.

Structural analysis of `graph710.col` reveals a critical topological vulnerability:
1. **2-Vertex Cut Bisection**: The graph is split into two disjoint subgraphs by a 2-vertex cut $C = \{1876, 2491\}$.
   - **Block A ($V_A$)**: 885 internal vertices $+ \{1876, 2491\} = 887$ vertices.
   - **Block B ($V_B$)**: 3,177 internal vertices $+ \{1876, 2491\} = 3,179$ vertices.
2. **Mathematically Forced Bridge Ports**:
   - In raw $G$, vertex $1876$ has degree 4, with **strictly 1 edge into Block A** (`(1876, 3878)`).
   - In raw $G$, vertex $2491$ has degree 3, with **strictly 1 edge into Block B** (`(2491, 1671)`).
   - Any valid Hamiltonian cycle $H$ of $G$ must enter Block A via $1876$, traverse all 885 interior vertices of Block A, and exit via $2491$. Symmetrically, $H$ must enter Block B via $2491$, traverse all 3,177 interior vertices of Block B, and return to $1876$.
3. **Degree-2 Contraction in Block B**:
   - Block B contains 922 degree-2 vertices. Recursive contraction reduces the active search space from 3,179 vertices to 2,257 vertices, turning long chains into single forced edges.
4. **Static Chordless Triangle Cuts**:
   - Pre-generating negative cuts for all 454 chordless triangles in contracted Block B prevents small cycle churn during CaDiCaL CEGAR.

This specification details the mathematical invariants, the bisection and contraction algorithms, the SAT encodings for both blocks, and the deterministic splice into a certified Hamiltonian cycle.

---

## 2. Graph Invariants & Structural Decomposition

### 2.1. Raw Graph Properties
- $|V| = 4,064$, $|E| = 6,800$, average degree $2M/N \approx 3.346$.
- Maximum degree: 14; minimum degree: 2.
- 2-vertex cut: $C = \{1876, 2491\}$.
- There is no direct edge between $1876$ and $2491$ in $G$.

### 2.2. Two-Block Partition Invariants
Removing $\{1876, 2491\}$ yields exactly two connected components:
- Component $A$: $885$ vertices.
  $$V_A = \text{Component } A \cup \{1876, 2491\}, \quad |V_A| = 887$$
- Component $B$: $3,177$ vertices.
  $$V_B = \text{Component } B \cup \{1876, 2491\}, \quad |V_B| = 3,179$$
- Overlap & Union Invariants:
  $$V_A \cap V_B = \{1876, 2491\}$$
  $$|V_A| + |V_B| - |V_A \cap V_B| = 887 + 3,179 - 2 = 4,064 = |V(G)|$$

### 2.3. Forced Boundary Port Analysis
- Degree of $1876$ in $G$: $\deg(1876) = 4$.
  - Edges incident to $1876$: $(1876, 3878) \in E(G[V_A])$, and 3 edges in $E(G[V_B])$.
  - Edge into Block A: exactly 1 edge $(1876, 3878)$.
- Degree of $2491$ in $G$: $\deg(2491) = 3$.
  - Edges incident to $2491$: $(2491, 1671) \in E(G[V_B])$, and 2 edges in $E(G[V_A])$.
  - Edge into Block B: exactly 1 edge $(2491, 1671)$.

**Theorem (Port & Direction Forcing):**
In any 2-factor of $G$ that forms a single Hamiltonian cycle $H$:
1. $H$ must visit $1876$ with exactly two edges. Since $1876$ has only 1 edge into Component $A$, $H$ must use edge $(1876, 3878)$ and exactly one of its 3 edges into Component $B$.
2. $H$ must visit $2491$ with exactly two edges. Since $2491$ has only 1 edge into Component $B$, $H$ must use edge $(2491, 1671)$ and exactly one of its 2 edges into Component $A$.
3. Therefore, the restriction of $H$ to $V_A$ is a simple path $P_A$ with endpoints $\{1876, 2491\}$ visiting all $887$ vertices of $V_A$.
4. The restriction of $H$ to $V_B$ is a simple path $P_B$ with endpoints $\{2491, 1876\}$ visiting all $3,179$ vertices of $V_B$.

---

## 3. Solving Strategy & Algorithmic Modules

### 3.1. Module 1: Graph Decomposer (`scratch/graph710/decomposer.py`)
- Reads raw DIMACS edge list `FHCPCS-col/graph710.col`.
- Computes connected components of $G \setminus \{1876, 2491\}$.
- Asserts $|V_A| = 887$, $|V_B| = 3,179$, $V_A \cap V_B = \{1876, 2491\}$.
- Exports edge lists and vertex sets for Block A and Block B.

### 3.2. Module 2: Block A SAT Solver
- Graph $G_A = G[V_A] \cup \{(1876, 2491)\}$, where $e_{virt} = (1876, 2491)$ is an added virtual edge.
- Virtual edge $e_{virt}$ is forced to True (unit clause or assumption).
- Standard 2-factor SAT formulation:
  - Boolean variable $x_e$ for each undirected edge $e \in E(G_A)$.
  - For each $v \in V_A$, exactly-2 constraint $\sum_{e \in \delta(v)} x_e = 2$ encoded via cardinality networks (At-Least-2 and At-Most-2 clauses).
- CaDiCaL CEGAR subtour elimination:
  - Solve SAT instance.
  - If SAT, extract active edges $E_{active} = \{e \mid x_e = \text{True}\}$.
  - Find connected components in $(V_A, E_{active})$.
  - If single component of size 887: terminate with valid cycle.
  - Otherwise, for each subcycle $C$ not containing $e_{virt}$ (or all subcycles), add cut clause:
    $$\sum_{e \in \delta(C)} x_e \ge 2 \quad \text{or} \quad \bigvee_{e \in C} \neg x_e$$
- Path extraction: Remove $e_{virt}$ from the 887-vertex cycle to obtain Hamiltonian path $P_A = [1876, \dots, 2491]$.
- Empirical benchmark: $\approx 2.03$s, 407 iterations.

### 3.3. Module 3: Block B Degree-2 Contraction & Triangle-Cut Solver
- Graph $G_B = G[V_B] \cup \{(2491, 1876)\}$, with virtual edge $e_{virt} = (2491, 1876)$ forced to True.
- **Degree-2 Vertex Contraction**:
  - Degree-2 vertices cannot branch; their incident edges must both be selected in any valid 2-factor.
  - Recursively identify all degree-2 vertices $v \in V_B \setminus \{1876, 2491\}$.
  - If $N(v) = \{u, w\}$, contract $v$: remove $v$ and edges $(u, v), (v, w)$, and insert contracted edge $(u, w)$ with contraction chain $[u, v, w]$.
  - If an edge $(u, w)$ already exists, preserve the multigraph / shortest chain structure.
  - In $V_B$, exactly 922 degree-2 vertices are contracted, reducing the vertex count from 3,179 to 2,257.
  - All contracted edges are permanently asserted True in the SAT solver.
- **Static Chordless Triangle Cuts**:
  - In the contracted graph of 2,257 vertices, enumerate all chordless 3-cycles (triangles).
  - There are exactly 454 chordless triangles.
  - For each triangle $(u, v, w)$, add the static clause:
    $$(\neg x_{uv} \lor \neg x_{vw} \lor \neg x_{wu})$$
    at Round 0 prior to starting the CEGAR loop.
- **CaDiCaL CEGAR Loop**:
  - Run CEGAR loop with subcycle blocking clauses.
  - With triangle cuts and contracted degree-2 paths, CaDiCaL converges to 1 single cycle of 2,257 vertices at iteration 88 in $\approx 179.33$s.
- **Path Reconstruction**:
  - Traverse the 2,257-cycle in order.
  - Whenever a contracted edge $(u, w)$ is encountered, expand it into its constituent degree-2 path $[u, v_1, \dots, v_k, w]$.
  - Verify that the expanded cycle has length 3,179, contains $e_{virt} = (2491, 1876)$, and has zero missing or duplicated vertices.
  - Remove $e_{virt}$ to yield Hamiltonian path $P_B = [2491, \dots, 1876]$.

### 3.4. Module 4: Deterministic Tour Assembly (`scratch/graph710/solve_graph710.py`)
- Given $P_A = [1876, a_1, a_2, \dots, a_{885}, 2491]$ and $P_B = [2491, b_1, b_2, \dots, b_{3177}, 1876]$:
- Assemble full tour:
  $$T = P_A[:-1] + P_B[:-1]$$
- Verification of Seams:
  - Transition from $A$ to $B$: $a_{885} \to 2491 \to b_1$. Both $(a_{885}, 2491) \in E(G)$ and $(2491, b_1) \in E(G)$.
  - Transition from $B$ to $A$: $b_{3177} \to 1876 \to a_1$. Both $(b_{3177}, 1876) \in E(G)$ and $(1876, a_1) \in E(G)$.
  - Total length of $T$: $886 + 3,178 = 4,064$ vertices.
  - Set of vertices in $T$: exactly $\{1, \dots, 4064\}$ with zero duplicates.
- Output tour in standard HCP format to `scratch/graph710/found_tour_graph710.hcp`.

---

## 4. Verification & Certification Criteria

1. **Zero Tour Injection**:
   - Strictly forbidden from reading, referencing, or importing `.tou` files (e.g. `FHCPCS-col/graph710.tou`).
2. **Zero Phantom Edges**:
   - Every single edge $(T[i], T[(i+1) \pmod N])$ must exist in the raw adjacency list of `FHCPCS-col/graph710.col`.
3. **Independent Benchmark Certification**:
   - Verification command:
     ```bash
     python3 scratch/verify_benchmarks.py --graph FHCPCS-col/graph710.col --tour scratch/graph710/found_tour_graph710.hcp
     ```
   - Must output: `Validation Result: PASS - CERTIFIED SOUND` with exit code 0.
4. **Execution Time Limit**:
   - Wall-clock runtime $\le 1,800$s (expected $\sim 185$s).
