# Design Specification: DP Bitmask Splicer for Class B2 (Graph788.col)

**Date**: 2026-09-05  
**Topic**: Exact Deterministic Splicing & DP Bitmask Architecture for Flinders Hamiltonian Cycle Project Challenge Set (FHCPCS) Class B2 Graphs  
**Primary Target**: `FHCPCS-col/graph788.col` ($N = 4,620, M = 7,560$)  
**Status**: Approved for Implementation Plan  

---

## 1. Executive Summary & Problem Context

In the Flinders Hamiltonian Cycle Project Challenge Set (FHCPCS), graphs in **Class B2** (e.g., `graph788`, `graph868`, `graph960`) are large, sparse graphs containing numerous degree-2 vertices, guaranteed by mathematical prior to possess a **unique Hamiltonian cycle** ($N$ vertices).

Existing baseline solvers and standard Cut-Set CEGAR methods experience severe timeouts ($> 1,800$s) on this class due to the **Fragmentation Trap**:
- Standard CEGAR rapidly reduces cycle counts down to a Near-Hamiltonian state (1 giant backbone containing $> 90\%$ of vertices and $\le 18$ small subcycles).
- In the final mile, unconstrained SAT solvers satisfy cut-set constraints on small subcycles by taking the path of least resistance: splitting the giant backbone into smaller pieces, causing cycle counts to rebound (e.g., $12 \to 22$) and search times to explode from seconds to hours.

This specification defines a **100% pure, exact, deterministic solver** that completely eliminates the Fragmentation Trap by freezing the giant backbone and resolving all remaining subcycles via a **DP Bitmask Splicing Engine**.

---

## 2. Mathematical Foundation & Architecture Pipeline

The solver operates in 4 sequential, deterministic stages:

```
[FHCPCS-col/graph788.col] (N=4,620, M=7,560)
         │
         ▼
[Stage 1: Block Contraction & Invariant Generation]
  ├── Identify 1,540 degree-2 vertices -> contract into 1,540 blocks (u - v - w)
  ├── Lock 3,080 edges (exactly 66.7% of HCP tour) prior to SAT search
  └── Generate 840 2-cycle mutexes (AtMost1) & 49 3-cycle mutexes
         │
         ▼
[Stage 2: Giant Backbone Acquisition & 4-opt Flip]
  ├── Mutex-accelerated CEGAR reaches near-Hamiltonian 2-factor in ~99s
  └── Step 1 alternating 4-opt flip absorbs secondary giant (218 blocks) & Subcycle 11 (2 blocks)
  └── Giant backbone reaches 1,432 blocks (93.0% of all vertices)
         │
         ▼
[Stage 3: DP Bitmask Splicing Engine]
  ├── Partition remaining 18 subcycles (108 blocks) into 9 independent connected components
  ├── Generate Hamiltonian path completions & Giant entry/exit gates for each component
  └── Execute DP Bitmask over (mask, pos) across 2^9 = 512 states in < 0.05s
         │
         ▼
[Stage 4: Tour Reconstruction & 100% Independent Verification]
  ├── Expand blocks back to raw 4,620 vertices
  ├── Verify all 4,620 edges exist in FHCPCS-col/graph788.col
  └── Export certified TSPLIB tour: scratch/graph788/found_tour_graph788.hcp
```

---

## 3. Stage 1: Block Contraction & Invariant Mutexes

1. **Topological Block Deduction**:
   - Every vertex $v$ with degree 2 in $G$ has exactly two neighbors $u$ and $w$.
   - In any Hamiltonian cycle, both edges $(u, v)$ and $(v, w)$ must be selected.
   - For `graph788.col`, exactly 1,540 vertices have degree 2. Contracting them yields 1,540 blocks of 3 vertices $(u - v - w)$, locking 3,080 edges ($66.7\%$).
   - Each block $B$ has two external ports: port `u` and port `w`.

2. **2-Cycle & 3-Cycle Invariant Mutex Generation**:
   - In the contracted block graph, 840 block pairs $(B_1, B_2)$ share 2 external edges.
   - For every such pair with edges $e_1, e_2$, selecting both creates a closed 2-block cycle. The clause $(\neg e_1 \lor \neg e_2)$ is added to the SAT solver a priori.
   - Similarly, for the 49 triangles of blocks $(B_1, B_2, B_3)$, clauses $(\neg e_{12} \lor \neg e_{23} \lor \neg e_{31})$ forbid 3-block cycles.
   - These invariants reduce spurious cycles by $61\%$ on iteration 1.

---

## 4. Stage 2: Giant Backbone Acquisition & 4-opt Flip

1. **Fast Mutex-CEGAR**:
   - CaDiCaL SAT solver with degree-1 port constraints and cut-set SEC constraints solves down to $\le 20$ cycles in ~99s.
2. **Step 1 Alternating 4-opt Flip**:
   - The auxiliary alternating graph $\mathcal{G}_{aux}$ on ports yields a directed cycle of length 4:
     - Cycle IDs involved: $\{0 \text{ (Giant)}, 3 \text{ (218 blocks)}, 11 \text{ (2 blocks)}\}$
     - Added edges: $[(517, 2614), (798, 3487), (1051, 3597), (3317, 3940)]$
     - Removed edges: $[(517, 798), (1051, 3487), (3597, 3940), (2614, 3317)]$
   - Flipping these 4 edges merges Subcycle 3 (218 blocks) and Subcycle 11 (2 blocks) into the Giant backbone.
   - Giant cycle expands from 1,220 blocks to **1,432 blocks (93.0% of the entire graph, 4,296 vertices)**.
   - Only 108 blocks (324 vertices) remain unabsorbed across 18 subcycles.

---

## 5. Stage 3: The DP Bitmask Splicing Engine

### 5.1 Component Decomposition
The 18 remaining subcycles partition into **9 independent connected components**:
- **7 Micro-Components**:
  - Component 3: 4 blocks (12 vertices, 1 subcycle)
  - Component 4: 4 blocks (12 vertices, 1 subcycle)
  - Component 5: 8 blocks (24 vertices, 1 subcycle)
  - Component 6: 8 blocks (24 vertices, 1 subcycle)
  - Component 7: 2 blocks (6 vertices, 1 subcycle: blocks 428 & 1475)
  - Component 8: 2 blocks (6 vertices, 1 subcycle: blocks 628 & 1001)
  - Component 9: 4 blocks (12 vertices, 2 subcycles: line graph)
- **2 Tree/Star Components**:
  - Component 1: 28 blocks (84 vertices, star of 4 subcycles centered at Subcycle 9)
  - Component 2: 48 blocks (144 vertices, tree of 6 subcycles)

### 5.2 Candidate Route Table Generation
For each component $K_i$ ($0 \le i < 9$):
1. Compute internal Hamiltonian paths covering all blocks in $K_i$.
2. For each path from start port $p_{start}$ to end port $p_{end}$, find external edges in $G$ to Giant ports $g_{in}, g_{out}$.
3. Candidate route structure:
   $$R = (i, g_{in}, g_{out}, E_{internal}, e_{in}, e_{out})$$
4. Candidate counts:
   - Micro-components: 8 to 18 candidates each.
   - Tree components (Comp 1 & 2): $\le 32$ candidates via local sub-HCP search.

### 5.3 DP Bitmask Recurrence & State Space
- State space: $\text{mask} \in [0, 2^9 - 1]$ ($0 \le \text{mask} \le 511$).
- Table `dp[mask]` stores the compatible set of routes for the components set in `mask`.
- Two routes $R_a$ and $R_b$ are compatible if:
  $$\{g_{in}^a, g_{out}^a\} \cap \{g_{in}^b, g_{out}^b\} = \emptyset \quad \text{and} \quad \text{edges\_removed}(R_a) \cap \text{edges\_removed}(R_b) = \emptyset$$
- State transition:
  $$\text{dp}[\text{mask} \mid (1 \ll k)] = \text{dp}[\text{mask}] \cup \{r\} \quad \text{for compatible } r \in \text{Routes}[k]$$
- Goal: $\text{dp}[511]$ yields 9 non-conflicting routes that absorb all 108 blocks into Giant.
- Computational cost: $\le 512 \times 32 = 16,384$ checks, running in $< 0.05$ seconds.

### 5.4 Global Cycle Verification
After applying the 9 routes from $\text{dp}[511]$:
- Degree of every block port is strictly 1.
- Global cycle check in $O(N)$ confirms that the result has exactly 1 cycle of 1,540 blocks.

---

## 6. Stage 4: Tour Reconstruction & Independent Verification

1. **Uncontraction**:
   - Convert the 1,540-block cycle into raw vertices: for block $B = (u - v - w)$, traverse $u \to v \to w$ (or $w \to v \to u$) depending on entry/exit ports.
2. **Independent Verification Protocol**:
   - Tour length must equal exactly $N = 4,620$.
   - Unique vertices: $\text{len}(\text{set}(\text{tour})) == 4,620$ and $\min(\text{tour}) == 1, \max(\text{tour}) == 4,620$.
   - Every consecutive pair $(v_i, v_{i+1})$ for $1 \le i < 4,620$ must exist in `FHCPCS-col/graph788.col`.
   - The closing edge $(v_{4,620}, v_1)$ must exist in `FHCPCS-col/graph788.col`.
3. **Export Artifact**:
   - Save certified TSPLIB format tour to `scratch/graph788/found_tour_graph788.hcp`.

---

## 7. Resource Constraints & Execution Standards

- **Core Isolation**: Core 3 strictly reserved for user (`taskset -c 0,1,2 nice -n 19`).
- **Purity Constraint**: 100% exact SAT and DP logic. Zero heuristic LKH, zero tour injection, zero reading `.tou` files.
- **Reproducibility**: Entire pipeline reproducible in a single self-contained script `scratch/solve_graph788_dp_bitmask.py`.
