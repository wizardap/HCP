# Canonical Modular Decomposition Engine for Hamiltonian Cycle

**Date:** 2026-08-18  
**Topic:** Exact Graph-Theoretic Canonical Modular Decomposition for HCP  
**Status:** Approved by User  

---

## 1. Executive Summary & Problem Formulation

### 1.1 The Fundamental Bottleneck of SAT-based CEGAR
In traditional SAT-based CEGAR for the Hamiltonian Cycle Problem (Takehide Soh et al., 2016), the initial SAT formula enforces 2-factor degree constraints ($\sum e = 2$). The solver finds a 2-factor consisting of $k \ge 2$ disjoint subcycles $C_1, \dots, C_k$. Standard CEGAR forbids this partition by generating subtour elimination cut clauses $\sum_{e \in \delta(C_i)} e \ge 2$.

On complex, symmetric, or dense-hub graphs, the space of 2-factors is exponential ($2^{\Omega(N)}$), causing naive CEGAR to cycle through thousands of near-identical partitions. Ad-hoc heuristic patching on every increment adds prohibitive $O(N^2)/O(N^3)$ computational overhead ($50\text{ms} - 150\text{ms}$ per increment), regressing performance on sparse graphs.

### 1.2 The Canonical Modular Decomposition Solution
Graph theory establishes that any undirected graph $G = (V, E)$ possesses a unique **Canonical Modular Decomposition Tree** $T_M(G)$ (Gallai 1967). A subset of vertices $M \subseteq V$ is a **module** (or homogeneous set) if:
$$\forall v \in V \setminus M: \quad (\forall u, w \in M, (v, u) \in E \iff (v, w) \in E)$$

Because all vertices outside $M$ share identical adjacency relationships with every vertex in $M$:
1. A global Hamiltonian cycle enters $M$ at an entry vertex $u_{in} \in M$, traverses every vertex in $M$ along an internal Hamiltonian path $\Pi(M, u_{in}, u_{out})$, and exits at $u_{out} \in M$.
2. Each non-trivial module $M$ can be solved independently as a localized sub-Hamiltonian path problem.
3. $M$ is contracted into a macro-vertex/super-edge in the parent quotient graph $G / M$.
4. The global Hamiltonian tour is reconstructed deterministically by splicing the internal module paths into the quotient cycle in polynomial time.

---

## 2. Architecture & Data Structures

### 2.1 Modular Decomposition Tree Representation
The modular tree is structured as an explicit hierarchy of strong modules:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModularNodeType {
    /// Single vertex leaf
    Leaf(i32),
    /// Disjoint union of child modules (co-disconnected)
    Parallel(Vec<usize>),
    /// Complete join between all pairs of child modules (co-connected)
    Series(Vec<usize>),
    /// Irreducible quotient graph with prime quotient adjacency
    Prime {
        quotient_adj: HashMap<usize, HashSet<usize>>,
        children: Vec<usize>,
    },
}

#[derive(Debug, Clone)]
pub struct ModularNode {
    pub id: usize,
    pub vertices: Vec<i32>,
    pub node_type: ModularNodeType,
    pub parent: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct ModularDecompositionTree {
    pub root: usize,
    pub nodes: Vec<ModularNode>,
}
```

---

## 3. Core Algorithms

### 3.1 Algorithm 1: Canonical Strong Module Detection (Partition Refinement)
1. **Neighborhood Signature Hashing:**
   For every vertex $u \in V$, compute its neighborhood bitmask/signature relative to candidate partitions. Vertices with identical open neighborhoods ($N(u) = N(v)$) form false twins (independent modules); vertices with identical closed neighborhoods ($N[u] = N[v]$) form true twins (clique modules).
2. **Modular Partition Refinement:**
   Given candidate subset $S \subset V$, pivot vertices $p \in V \setminus S$ split $S$ into $S \cap N(p)$ and $S \setminus N(p)$. Subsets that cannot be split by any external pivot $p$ are verified strong modules.
3. **Canonical Tree Construction:**
   Strong modules are organized into a tree where child modules are partitioned by the quotient relationship (Parallel, Series, or Prime).

### 3.2 Algorithm 2: Localized Sub-Module Hamiltonian Path Solving
For each module $M$:
1. If $|M| = 1$: Trivial path `[u]`.
2. If $|M| = 2$: Path `[u, v]` if $(u, v) \in E$.
3. If $M$ is a `Series` node: Construct an alternating Hamiltonian path across child modules in $O(|M|)$ time via Chvátal-Erdős bipartition matching.
4. If $M$ is a `Prime` module ($|M| \le 300$): Solve the induced subgraph $G[M]$ with fixed candidate boundary endpoints $(u_{in}, u_{out})$ using localized SAT path formulation with unit clauses fixing entry/exit endpoints.

### 3.3 Algorithm 3: Quotient Tour Solving & Deterministic Splicing
1. **Quotient Graph Cycle Solving:**
   Solve the contracted quotient graph $G / \{M_1, \dots, M_k\}$ where each module $M_i$ acts as a macro-node.
2. **Endpoint Compatibility Matching:**
   For each directed transition $M_i \to M_{i+1}$ in the quotient cycle, select boundary vertices $u_{out}^{(i)} \in M_i$ and $u_{in}^{(i+1)} \in M_{i+1}$ such that $(u_{out}^{(i)}, u_{in}^{(i+1)}) \in E$.
3. **Deterministic Splicing:**
   Replace the macro-node traversal of $M_i$ with the internal precomputed path $\Pi(M_i, u_{in}^{(i)}, u_{out}^{(i)})$.
4. **Degree-2 Uncontraction:**
   Uncontract all contracted chains via `contractor.uncontract_cycle(&stitched_tour)` to recover original graph vertices.

---

## 4. Pipeline Integration & Safety Invariants

### 4.1 Pipeline Integration Flow
Inside `src/cegar-fix/src/hcp_solver.rs`:
```
Graph Input -> Degree-2 Contraction -> ModularDecompositionTree::build(&g)
   |
   +--> Non-trivial modules found?
          |
          +--> [YES] Solve via ModularSolver -> Verify Tour -> Print "s SATISFIABLE" (Exit early)
          |
          +--> [NO]  Fall back cleanly to standard CEGAR loop
```

### 4.2 Invariants & Soundness
- **Soundness Invariant:** Every reconstructed cycle is validated by `is_valid_hamiltonian_cycle(tour, g)` before emitting `s SATISFIABLE`.
- **Contraction Safety:** Never sever contracted chains in `contractor.chain_map`.
- **Zero Unwraps / Panics:** Handle degenerate graphs, disconnected components, and empty modules with graceful `Option`/`Result` fallbacks.

---

## 5. Verification & Test Plan

1. **Unit Testing (`src/cegar-fix/src/modular_tree.rs`):**
   - `test_modular_decomposition_true_twins`: Verification of true twin clique module detection.
   - `test_modular_decomposition_false_twins`: Verification of false twin independent set detection.
   - `test_modular_decomposition_series_join`: Verification of complete join bipartite series nodes.
   - `test_modular_path_and_splicing`: End-to-end synthetic module decomposition, path solving, and tour splicing.
2. **10 Key Regression Graphs Benchmark:**
   - Verify 100% pass rate (`s SATISFIABLE`) on `graph45`, `graph132`, `graph161`, `graph178`, `graph183`, `graph230`, `graph248`, `graph313`, `graph339`, `graph346`.
3. **Dense Hub Profiling:**
   - Profile module extraction on `graph560.col`, `graph562.col`, `graph584.col`.
