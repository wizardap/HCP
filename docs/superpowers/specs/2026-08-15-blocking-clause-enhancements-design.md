# Design Document: Enhancing CEGAR Blocking Clauses (Three Techniques: A1, A2, A3)

## 1. Overview and Problem Statement
In the SAT-based CEGAR framework for the Hamiltonian Cycle Problem (HCP), the solver alternates between finding a 2-factor (a set of disjoint simple cycles covering all vertices) using the CaDiCaL SAT solver, and adding blocking clauses to eliminate subcycles ($|C| < |V|$).

Analysis of 75 baseline timeout instances on the FHCPCS benchmark revealed two major algorithmic bottlenecks:
1. **Dense / Super-Hub Graphs (74.7% of timeouts)**: Subcycles containing high-degree vertices generate huge Cut-set clauses ($\delta^+(C)$ containing hundreds of positive literals). These clauses provide weak unit propagation and degrade CDCL search performance.
2. **Ultra-Large Sparse Graphs (25.3% of timeouts)**: The solver falls into subcycle ping-pong churn (>15,000 increments) due to internal edge reconfigurations within the same subcycle vertex sets.

This design document specifies three complementary mathematical enhancements to blocking clause generation (Techniques A1, A2, and A3) to sharpen constraint propagation while strictly preserving correctness and zero regressions.

---

## 2. Mathematical Specification of the Three Techniques

### 2.1 Technique A1: Boundary Minimal Cut Reduction
* **Goal**: Optimize Cut-set clause construction by identifying and filtering boundary vertices, using $O(1)$ set lookups.
* **Formulation**:
  For any subcycle $C \subset V$:
  Let $S_{set}$ be the hash set representation of $C$ (or $V \setminus C$).
  A vertex $u \in C$ is a *boundary vertex* ($\partial C$) iff $\exists v \in \text{Adj}(u)$ such that $v \notin S_{set}$.
  - The forward cut clause $\text{Cut}^+(C) = \bigvee_{u \in \partial C, v \notin C, (u,v) \in E} x_{uv}$.
  - The reverse cut clause $\text{Cut}^-(C) = \bigvee_{u \in \partial C, v \notin C, (v,u) \in E} x_{vu}$.
  Interior vertices $u \in C$ where $\text{Adj}(u) \subseteq C$ are skipped immediately, reducing clause construction overhead and avoiding redundant iterations.

### 2.2 Technique A2: Induced Subgraph SECs (Subtour Elimination Constraints)
* **Goal**: Forbid all internal subtours and chords within small cycle vertex sets ($|C| \le 6$) in a single CEGAR step.
* **Formulation**:
  Let $C = \{v_0, v_1, \dots, v_{k-1}\}$ be a subcycle with $k = |C| \le 6$.
  Consider the induced subgraph $G[C] = (C, E[C])$, where $E[C] = \{(u,v) \in E \mid u, v \in C\}$.
  - If $|E[C]| = k$ (no chords): $G[C]$ contains only the forward and reverse cycles. We add standard bidirectional cycle exclusion clauses:
    $$\neg x_{v_0,v_1} \vee \neg x_{v_1,v_2} \vee \dots \vee \neg x_{v_{k-1},v_0}$$
    $$\neg x_{v_0,v_{k-1}} \vee \dots \vee \neg x_{v_1,v_0}$$
  - If $|E[C]| > k$ (internal chords exist):
    Any valid Hamiltonian cycle can select at most $k-1$ edges in $G[C]$ if $C \neq V$.
    For all simple cycles $C' \subseteq G[C]$ formed using chord edges, we generate their corresponding cycle exclusion clauses.
    This prevents CaDiCaL from flipping to symmetric permutations on the exact same vertex set $C$.

### 2.3 Technique A3: Complementary Cut Symmetry
* **Goal**: When a subcycle spans the majority of the graph ($|C| > |V| / 2$), construct the cut on the complementary set $S = V \setminus C$.
* **Formulation**:
  By the conservation of vertex degree in any 2-factor:
  $$\delta^+(C) = \delta^-(V \setminus C) \quad \text{and} \quad \delta^-(C) = \delta^+(V \setminus C)$$
  When $|C| > |V| / 2$, let $S = V \setminus C$. Since $|S| < |V| / 2$, enumerating the boundary cut from $S$ requires iterating over fewer vertices, producing tighter clauses with lower memory and construction overhead.

---

## 3. Architecture and Data Flow

```
+-------------------------------------------------------------+
|                      CEGAR Main Loop                        |
|                                                             |
| 1. solver.solve() -> sat_solution (edges selected)          |
| 2. Extract disjoint 2-factor cycles: sol_cycles             |
| 3. Run Solution Constructor (2-opt & 3-opt candidate merge) |
|    - If full Hamiltonian cycle found (|C| = 1) -> EXIT SAT  |
+------------------------------+------------------------------+
                               |
                   |C| > 1 (Subcycles remain)
                               |
                               v
+-------------------------------------------------------------+
|               Enhanced Blocking Clause Generator            |
|                                                             |
| For each cycle C in sol_cycles:                             |
|   1. [Technique A3] If |C| > |V|/2:                         |
|        Target set S = V \ C                                 |
|      Else:                                                  |
|        Target set S = C                                     |
|                                                             |
|   2. [Technique A1] Boundary Extraction:                    |
|        boundary = { u in S | exists v in Adj(u), v not in S}|
|        cut_clause_out = \/_((u,v) in delta^+(S)) x_uv       |
|        cut_clause_in  = \/_((v,u) in delta^-(S)) x_vu       |
|                                                             |
|   3. [Technique A2] Induced Subgraph SECs:                  |
|        If |C| <= 6:                                         |
|          Find all simple cycles in G[C]                     |
|          Generate exclusion clause for each cycle in G[C]   |
+------------------------------+------------------------------+
                               |
                               v
+-------------------------------------------------------------+
|           Add generated clauses to CaDiCaL Solver           |
+-------------------------------------------------------------+
```

---

## 4. Implementation Details in Rust

### 4.1 Modified Files
* `src/cegar-fix/src/hcp_solver.rs`:
  - Enhance `get_blocking_clauses`: integrate A1 (boundary filtering), A2 (induced subgraph SECs), and A3 (complementary cuts).
  - Add helper `find_induced_cycles(vertices: &[i32], g: &Graph) -> Vec<Vec<i32>>` for finding chord cycles in $G[C]$ when $|C| \le 6$.
  - Update `asp_blocking_clauses` to use `HashSet` and boundary sets.

---

## 5. Verification and Acceptance Criteria
1. **Unit Testing**:
   - Verification that chord cycles on small induced subgraphs (triangles, squares, pentagons with chords) generate correct exclusion clauses.
   - Verification that complementary cuts produce identical cut edges to forward cuts.
2. **Benchmark Verification**:
   - All standard benchmark testcases (`graph45`, `graph132`, `graph161`, `graph178`, `graph183`, `graph230`, `graph248`, `graph313`, `graph339`) solve without regressions.
   - Profile performance on timeout instances `graph560.col`, `graph584.col`, `graph647.col`.
