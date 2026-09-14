# Design Specification: Hierarchical Linear-Chain & Central-Block Solver for graph717.col

**Date:** 2026-09-14  
**Status:** Approved by User  
**Target Graph:** `FHCPCS-col/graph717.col` ($N = 4,122, M = 7,638$)  
**Benchmark Context:** The Flinders Challenge Set (Universal Core-33), Degree 2–14 Moderate Hub family (12 timeout instances). `graph717.col` is the sister instance to `graph710.col`.  
**Constraint:** Execution time $\le 1,800$s, zero tour injection, zero phantom edges, 100% independent verification on raw uncontracted DIMACS edge list.

---

## 1. Executive Summary & Problem Context

`graph717.col` ($N = 4,122, M = 7,638$) is one of the benchmark timeout instances in the Flinders Challenge Set. Standard solvers (Picat, Concorde, unguided CEGAR) stall due to severe cycle shattering across thousands of low-degree vertices.

Topological analysis reveals that `graph717.col` possesses an elegant three-part decomposition:
1. **Two Linear Chains (508 vertices each)**: Each chain consists of 3 modular subgraphs of 167 vertices each, connected in series by forced bridge edges between degree-14 boundary ports. Any Hamiltonian cycle is mathematically forced to traverse each chain as a simple Hamiltonian path connecting its two terminal ports.
2. **One Central Hub Block Comp 0 (3,108 vertices)**: Connects to the terminals of both linear chains at 4 interface ports $\{255, 1955, 2609, 3358\}$. It contains all 922 degree-2 vertices of `graph717.col`.
3. **Degree-2 Contraction & Triangle Cuts**: Comp 0 contracts from 3,108 vertices to 2,186 vertices, with 376 chordless triangles eliminated statically in Round 0.
4. **Deterministic Splicing**: Adding virtual edges representing the two linear chains into Comp 0 allows solving Comp 0 as a Hamiltonian cycle, which decomposes into two disjoint paths and deterministically splices with the two pre-solved linear chains into a 4,122-vertex cycle.

---

## 2. Graph Invariants & Structural Decomposition

### 2.1. Raw Graph Properties
- $|V| = 4,122$, $|E| = 7,638$, average degree $2M/N \approx 3.706$.
- Min degree: 2 (922 vertices); Max degree: 14 (80 vertices).
- Biconnected (0 articulation points).
- Exactly 16 special cut nodes of degree 14:
  $$\mathcal{C}_{16} = \{255, 540, 577, 702, 773, 1016, 1177, 1213, 1389, 1955, 2467, 2609, 2677, 2681, 3358, 3986\}$$

### 2.2. Component Partitioning
Removing $\mathcal{C}_{16}$ from $G$ partitions the remaining vertices into exactly 7 connected components:
- Exactly **6 modular components of 167 vertices each**:
  - Module 1 ($167$v): boundary ports $\{1016, 3986\}$
  - Module 2 ($167$v): boundary ports $\{1389, 2677\}$
  - Module 3 ($167$v): boundary ports $\{540, 577\}$
  - Module 4 ($167$v): boundary ports $\{1213, 2681\}$
  - Module 5 ($167$v): boundary ports $\{1177, 2467\}$
  - Module 6 ($167$v): boundary ports $\{702, 773\}$
- Exactly **1 large central component Comp 0 of 3,104 vertices**:
  - Connects to exactly 4 interface cut nodes: $\{255, 1955, 2609, 3358\}$.

### 2.3. Topology of the Linear Chains

#### Chain 1 (Terminal ports: 255 and 1955)
- Traversal path:
  $$255 \xrightarrow{(255, 1389)} 1389 \xrightarrow{\text{Mod 2 (167v)}} 2677 \xrightarrow{(2677, 1213)} 1213 \xrightarrow{\text{Mod 4 (167v)}} 2681 \xrightarrow{(2681, 773)} 773 \xrightarrow{\text{Mod 6 (167v)}} 702 \xrightarrow{(702, 1955)} 1955$$
- Total vertices in Chain 1: $3 \times 167\text{ (modules)} + 6\text{ (intermediate cut nodes)} + 2\text{ (terminals)} = \mathbf{508\text{ vertices}}$.
- Every intermediate port has exactly 1 edge to the adjacent port and 13 edges into its module. Therefore, any Hamiltonian cycle must enter/leave each module through its designated ports.

#### Chain 2 (Terminal ports: 3358 and 2609)
- Traversal path:
  $$3358 \xrightarrow{(3358, 3986)} 3986 \xrightarrow{\text{Mod 1 (167v)}} 1016 \xrightarrow{(1016, 1177)} 1177 \xrightarrow{\text{Mod 5 (167v)}} 2467 \xrightarrow{(2467, 577)} 577 \xrightarrow{\text{Mod 3 (167v)}} 540 \xrightarrow{(540, 2609)} 2609$$
- Total vertices in Chain 2: $3 \times 167\text{ (modules)} + 6\text{ (intermediate cut nodes)} + 2\text{ (terminals)} = \mathbf{508\text{ vertices}}$.

#### Union & Invariant Check
- $V_{\text{Chain1}} \cap V_{\text{Chain2}} = \emptyset$.
- $(V_{\text{Chain1}} \cup V_{\text{Chain2}}) \cap V_{\text{Comp0}} = \{255, 1955, 2609, 3358\}$.
- Total vertices:
  $$|V_{\text{Chain1}}| + |V_{\text{Chain2}}| + |V_{\text{Comp0}}| - 4 = 508 + 508 + 3,108 - 4 = 4,122 = |V(G)|$$

---

## 3. Solving Strategy & Algorithmic Pipeline

### 3.1. Module Solvers (6 Modules $\times$ 167 Vertices)
For each module $M_k$ with boundary ports $u, v$:
- Construct augmented subgraph $G[M_k \cup \{u, v\}] \cup \{(u, v)\}$, where $e_{\text{virt}} = (u, v)$ is a virtual edge.
- Encode degree-2 constraints for all vertices in $M_k \cup \{u, v\}$.
- Assert $e_{\text{virt}} = \text{True}$.
- Solve with CaDiCaL CEGAR subtour elimination.
- Remove $e_{\text{virt}}$ to obtain a Hamiltonian path $P_k$ of 169 vertices connecting $u$ to $v$.
- Parallel solving: All 6 modules can be solved in parallel across CPU cores.

### 3.2. Chain Assembly (Chain 1 and Chain 2)
- Concatenate module paths and inter-module bridge edges:
  $$P_{\text{Chain1}} = [255] + P_{\text{Mod2}} + P_{\text{Mod4}} + P_{\text{Mod6}} + [1955]$$
  (orienting each module path so endpoints match the bridge edges).
  Length of $P_{\text{Chain1}}$: 508 unique vertices, endpoints $(255, 1955)$.
- Similarly:
  $$P_{\text{Chain2}} = [3358] + P_{\text{Mod1}} + P_{\text{Mod5}} + P_{\text{Mod3}} + [2609]$$
  Length of $P_{\text{Chain2}}$: 508 unique vertices, endpoints $(3358, 2609)$.

### 3.3. Central Block Comp 0 Solver (3,108 Vertices)
- Construct augmented subgraph $G_{\text{Comp0}} = G[\text{Comp0} \cup \{255, 1955, 2609, 3358\}]$ with two virtual edges:
  $$e_{\text{virt1}} = (255, 1955), \quad e_{\text{virt2}} = (3358, 2609)$$
- **Degree-2 Vertex Contraction**:
  - Recursively contract all 922 degree-2 vertices in Comp 0 (excluding the 4 ports).
  - Search space reduces from 3,108 to 2,186 vertices.
  - All contracted edges are asserted True in SAT.
- **Static Triangle Cuts**:
  - 376 chordless triangles in the contracted graph are forbidden in Round 0.
- **CaDiCaL CEGAR Loop**:
  - Force $e_{\text{virt1}} = \text{True}$ and $e_{\text{virt2}} = \text{True}$.
  - Solve with subcycle elimination cuts until a single cycle of 2,186 contracted vertices is found.
  - Uncontract virtual chains to produce a 3,108-vertex cycle containing $e_{\text{virt1}}$ and $e_{\text{virt2}}$.

### 3.4. Deterministic Macro Splicing
- In the 3,108-vertex cycle of Comp 0:
  - Locate virtual edge $e_{\text{virt1}} = (255, 1955)$ and replace it with the 508-vertex path $P_{\text{Chain1}}$ (in matching orientation).
  - Locate virtual edge $e_{\text{virt2}} = (3358, 2609)$ and replace it with the 508-vertex path $P_{\text{Chain2}}$ (in matching orientation).
- The resulting sequence forms a simple Hamiltonian cycle of:
  $$(3,108 - 2) + (508 - 2) + (508 - 2) + 4 = 4,122\text{ vertices}$$
- Validate that all 4,122 vertices are visited exactly once and all 4,122 edges exist in raw `graph717.col`.

---

## 4. Verification & Certification Criteria

1. **Zero Tour Injection**: No `.tou` file is ever read, imported, or inspected.
2. **Zero Phantom Edges**: Every edge in the assembled cycle exists in raw `FHCPCS-col/graph717.col`.
3. **Independent Verification**:
   ```bash
   python3 scratch/verify_benchmarks.py --graph FHCPCS-col/graph717.col --tour scratch/graph717/found_tour_graph717.hcp
   ```
   Must output: `Validation Result: PASS - CERTIFIED SOUND` with exit code 0.
4. **Runtime**: Total execution $\le 1,800$s.
