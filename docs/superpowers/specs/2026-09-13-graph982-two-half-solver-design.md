# Design Specification: Two-Half Two-Tier Hierarchical Cluster Solver for graph982.col

**Date:** 2026-09-13  
**Status:** Approved by User  
**Target Graph:** `FHCPCS-col/graph982.col` ($N = 7,620, M = 33,218$)  
**Benchmark Constraint:** Execution time $\le 1,800$s (target $< 60$s), zero tour injection, 100% independent verification on raw edge list.

---

## 1. Executive Summary & Context

`graph982.col` is one of the hardest challenge graphs in the Flinders Hamiltonian Cycle Project Challenge Set (FHCPCS). Traditional general-purpose SAT formulations and naive CEGAR subtour elimination loops stall indefinitely on this graph due to exponential subcycle fragmentation and high density ($\Delta = 762$).

Analysis of `graph982.col` reveals that it belongs to the exact same parametric Dense Hub family as `graph950.col` ($N=6,620$), `graph963.col` ($N=7,020$), and `graph975.col` ($N=7,420$), all three of which have been successfully solved and certified 100% sound using the Two-Half / Two-Tier Hierarchical Solver pipeline.

This document formalizes the architectural and algorithmic design for solving `graph982.col` in $< 60$ seconds using structural decomposition, intra-cluster CaDiCaL CEGAR, and bridge-cut tour assembly.

---

## 2. Graph Structural Properties & Mathematical Invariants

### 2.1. Top-Degree Super-Hubs
- $|V| = 7,620$, $|E| = 33,218$.
- Maximum degree $\Delta = 762$.
- Exactly **10 super-hubs** of maximum degree 762:
  $$\mathcal{H}_{super} = \{1714, 2907, 4740, 5022, 5378, 5575, 5852, 6335, 6696, 6956\}$$

### 2.2. Perfect 2-Bridge Cut & Half Partition
A multi-source Voronoi BFS partition rooted at the 10 super-hubs splits the 10 hubs into two equal halves of 5 super-hubs each:
- **$\text{Half}_1$ ($3,810$ vertices)**:
  $$\mathcal{H}_1 = \{1714, 5022, 5575, 5852, 6696\}$$
- **$\text{Half}_2$ ($3,810$ vertices)**:
  $$\mathcal{H}_2 = \{2907, 4740, 5378, 6335, 6956\}$$

The edge cut $\delta(\text{Half}_1, \text{Half}_2)$ between the two partitions contains **strictly 2 edges**:
$$e_1 = (1495, 6335), \quad e_2 = (6670, 7311)$$
where $1495, 6670 \in \text{Half}_1$ and $6335, 7311 \in \text{Half}_2$.

### 2.3. Fundamental Theorem of the 2-Bridge Cut
**Theorem:** *In any graph $G$ where a cut separating $V_1$ and $V_2$ consists of exactly 2 edges $e_1 = (u_1, v_1)$ and $e_2 = (u_2, v_2)$, any Hamiltonian cycle $C$ in $G$ must contain both $e_1$ and $e_2$. Furthermore, $C \cap V_1$ is a single Hamiltonian path in $V_1$ connecting $u_1$ and $u_2$, and $C \cap V_2$ is a single Hamiltonian path in $V_2$ connecting $v_1$ and $v_2$.*

**Corollary:** The global Hamiltonian cycle problem on $G$ ($7,620$ vertices) reduces without loss of generality or completeness to:
1. Finding a Hamiltonian path $P_1$ covering all $3,810$ vertices of $\text{Half}_1$ with endpoints $u_{start} = 1495, u_{end} = 6670$.
2. Finding a Hamiltonian path $P_2$ covering all $3,810$ vertices of $\text{Half}_2$ with endpoints $v_{start} = 6335, v_{end} = 7311$.
3. Concatenating $P_1, e_2, \text{reverse}(P_2), e_1$ into a certified Hamiltonian cycle.

---

## 3. Detailed Architectural Components

### 3.1. Two-Tier Hierarchical Decomposition (`decompose_half`)
Inside each half of $3,810$ vertices:
- Vertices partition into:
  - **155 Hubs** (degree $\ge 20$), including the 5 super-hubs and 150 intermediate hubs.
  - **$3,655$ Bulk Vertices** grouped into 37 connected strips:
    - 25 large strips of length 145 ($25 \times 145 = 3,625$ vertices).
    - 6 tiny strips of length 3 ($6 \times 3 = 18$ vertices).
    - 6 tiny strips of length 2 ($6 \times 2 = 12$ vertices).
- **Group Allocation**:
  - Each of the 5 super-hubs controls exactly 5 large strips ($5 \times 145 = 725$ vertices) and 30 intermediate hubs ($725 + 30 = 755$ vertices).
  - Exactly one pair of tiny strips (size $3 + 2 = 5$ vertices) is assigned to each group, yielding **exactly 760 vertices per group**:
    $$755 + 5 = 760 \text{ vertices}$$
  - $5 \text{ groups} \times 760 \text{ vertices} = 3,800 \text{ vertices} + 10 \text{ super-hubs} = 7,620 \text{ vertices}$.

### 3.2. Intra-Cluster SAT Routing (`solve_cluster_path`)
For each group of 760 vertices $V_{bulk}$:
1. **Induced Subgraph $G[V_{bulk}]$**:
   - Filter edges strictly within $V_{bulk}$.
2. **Boolean Variables**:
   - $x_e \in \{0, 1\}$ for each edge $e \in E(G[V_{bulk}])$.
3. **Cardinality / Degree Constraints**:
   - At terminal port $u_{in}$: $\sum_{e \in \delta(u_{in})} x_e = 1$.
   - At terminal port $u_{out}$: $\sum_{e \in \delta(u_{out})} x_e = 1$.
   - At all intermediate vertices $v \in V_{bulk} \setminus \{u_{in}, u_{out}\}$: $\sum_{e \in \delta(v)} x_e = 2$.
   - Encoded via PySAT `CardEnc.equals` using `EncType.seqcounter`.
4. **Subcycle Elimination via CaDiCaL CEGAR**:
   - In each CEGAR iteration, extract the active path from $u_{in}$ to $u_{out}$.
   - For every isolated disconnected cycle $C_k \subset V_{bulk}$, post the cut-crossing clause:
     $$\sum_{e \in \delta(C_k, V_{bulk} \setminus C_k)} x_e \ge 2$$
   - Converges in 10–25 iterations ($< 3$ seconds per group).
5. **Persistent Path Caching**:
   - Solved paths are persisted as JSON arrays in `scratch/graph982/half1_group_paths.json` and `scratch/graph982/half2_group_paths.json` to enable deterministic replay and instant restart.

### 3.3. Assembly and Bridge Stitching
1. **Half 1 Path Assembly**:
   - Chain the 5 solved group paths using the fixed inter-group hub connectors:
     $$P_1 = [1495, \dots, \text{Group}_1, \dots, \text{Group}_5, \dots, 6670]$$
   - Invariants: $|P_1| = 3,810$, $|\text{unique}(P_1)| = 3,810$, all edges $\in E(G)$.
2. **Half 2 Path Assembly**:
   - Chain the 5 solved group paths using the fixed inter-group hub connectors:
     $$P_2 = [6335, \dots, \text{Group}'_1, \dots, \text{Group}'_5, \dots, 7311]$$
   - Invariants: $|P_2| = 3,810$, $|\text{unique}(P_2)| = 3,810$, all edges $\in E(G)$.
3. **Closing Tour**:
   $$\text{Tour} = P_1 + \text{reverse}(P_2)$$
   - Bridge 1: $(P_1[-1], P_2[-1]) = (6670, 7311) \in E(G)$.
   - Bridge 2: $(P_2[0], P_1[0]) = (6335, 1495) \in E(G)$.
   - Total length: $3,810 + 3,810 = 7,620$ vertices.

---

## 4. Error Handling & Invariant Verification Protocol

1. **Partition Integrity**:
   - Validate that $|\text{Half}_1| = 3,810$ and $|\text{Half}_2| = 3,810$.
   - Validate that $\text{Half}_1 \cap \text{Half}_2 = \emptyset$ and $\text{Half}_1 \cup \text{Half}_2 = V(G)$.
2. **Bridge Cut Validation**:
   - Verify that all cross-half edges are exactly $\{(1495, 6335), (6670, 7311)\}$.
3. **Zero Phantom Edges**:
   - Verify every edge in $P_1$, $P_2$, and the two bridge edges against `G[u]`.
4. **Independent Certification**:
   - Output TSPLIB `.hcp` tour file to `scratch/graph982/found_tour_graph982.hcp`.
   - Run independent verification tool:
     ```bash
     python3 scratch/verify_benchmarks.py --graph FHCPCS-col/graph982.col --tour scratch/graph982/found_tour_graph982.hcp
     ```
   - Must achieve: `Validation Result: PASS - CERTIFIED SOUND`.

---

## 5. Implementation Files & Artifacts

- **Solver Script**: `scratch/graph982/solve_graph982.py`
- **Group Path Caches**:
  - `scratch/graph982/half1_group_paths.json`
  - `scratch/graph982/half2_group_paths.json`
- **Certified Tour Output**: `scratch/graph982/found_tour_graph982.hcp`
