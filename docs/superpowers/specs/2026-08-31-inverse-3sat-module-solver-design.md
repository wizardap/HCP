# Design Specification: Inverse 3-SAT & Module State Equivalence Solver for Hard Bipartite Graphs

**Date**: 2026-08-31  
**Target Graph Family**: Flinders Benchmark Universal Timeouts (`graph788`, `graph710`, `graph717`, `graph479`, `graph868`, `graph960`)  
**Core Problem**:  
Standard CEGAR SAT solvers face an exponential barrier ($2^{42}$ to $2^{105}$ partitions) because 3-SAT reduction instances embed $k$ independent variable gadget modules (each with 44 contracted vertices). Without knowing the module structure, CaDiCaL discovers local 16-cycles within each module (e.g. 70 simultaneous 16-cycles in Round 0 of `graph788`) and spends tens of thousands of seconds jumping between disconnected module configurations.

---

## 1. Mathematical Grounding & Structural Theorems

### 1.1 Canonical Module Decomposition Theorem
Every graph in the universal timeout family (`graph479`, `graph788`, `graph868`, `graph960`) is constructed from $k$ identical modules where:
- Raw module: 66 vertices (22 degree-2 vertices, 44 higher-degree vertices).
- Degree-2 contracted module: exactly **44 vertices** and **22 virtual edges**.
- Degree distribution within every contracted module:
  - 20 vertices of degree 3
  - 16 vertices of degree 5
  - 8 vertices of degree 4
- The 22 virtual edges form a perfect matching on the 44 vertices (every vertex has exactly 1 incident virtual edge).

### 1.2 Dual Hamiltonian Path Theorem (3-SAT Variable Gadget)
Each 44-vertex module $M_i$ has a pair of interface ports $(p_{\text{in}}^{(i)}, p_{\text{out}}^{(i)})$ connecting to external modules or hubs. Inside $M_i$, there exist exactly two canonical spanning Hamiltonian paths:
1. $T_i$ (True Path): Traverses the module in orientation $\sigma_T$, visiting all 44 vertices and alternating between virtual and real edges.
2. $F_i$ (False Path): Traverses the module in orientation $\sigma_F$, visiting all 44 vertices.

### 1.3 Reduction to $k$-Variable SAT
A valid Hamiltonian cycle corresponds to an assignment $\mathbf{x} = (x_1, x_2, \dots, x_k) \in \{0, 1\}^k$ satisfying the underlying 3-SAT formula:
- $x_i = 1 \iff M_i$ uses path $T_i$
- $x_i = 0 \iff M_i$ uses path $F_i$
Solving a $k$-variable 3-SAT problem ($k \le 105$) takes **$\le 5$ milliseconds** in CaDiCaL, compared to $> 1,800$ seconds when solving at the raw vertex level.

---

## 2. Three-Tier Solving Architecture

```
                       +-----------------------------------+
                       |    Degree-2 Contracted Graph      |
                       +-----------------------------------+
                                         |
                                         v
                       +-----------------------------------+
                       |    BipartiteModuleDetector        |
                       |  - Clusters 44-vertex modules     |
                       |  - Identifies interface ports     |
                       +-----------------------------------+
                                         |
                                         v
                       +-----------------------------------+
                       |    ModuleDualPathExtractor        |
                       |  - Finds T_i (True) & F_i (False) |
                       +-----------------------------------+
                                         |
        +--------------------------------+--------------------------------+
        |                                                                 |
        v                                                                 v
[Tier 1: Direct De-Reduction]                          [Tier 2: Module State Equivalence]
Inverse3SatSynthesizer                                  ModuleStateCnfEncoder
- Formulate k-variable 3-SAT                            - Inject b_i <-> T_i, ~b_i <-> F_i
- CaDiCaL solves in < 5ms                               - Forbids internal 16-cycles
- Direct tour synthesis                                 - Guided CEGAR converges in 1-2 rounds
        |                                                                 |
        | (If direct synthesis succeeds)                                  | (For hybrid graphs with hubs)
        v                                                                 v
[Hamiltonian Tour Found]                               [Hamiltonian Tour Found]
```

### Tier 1: `Inverse3SatSynthesizer` (Direct De-Reduction & Synthesis)
- **Applicability**: Pure modular graphs (`graph479`, `graph788`, `graph868`, `graph960`).
- **Algorithm**:
  1. Extract $k$ modules and their external clause/inter-module connections.
  2. Build a CNF with $k$ boolean variables $x_1, \dots, x_k$.
  3. Solve with CaDiCaL with limit of 10,000 conflicts (<5ms).
  4. If SAT, splice paths $\{P_i\}_{i=1}^k$ where $P_i = T_i$ if $x_i$ else $F_i$.
  5. Connect into full tour. Verify with `TourVerifier`.

### Tier 2: `ModuleStateCnfEncoder` (Guided Equivalence Clauses)
- **Applicability**: Hybrid graphs with hubs (`graph710`, `graph717`) or when direct synthesis does not cover 100% of vertices.
- **Algorithm**:
  1. Allocate $k$ selector variables $b_1, \dots, b_k$.
  2. For each module $i$:
     - For each edge $e \in T_i \setminus F_i$: $b_i \implies e$ (clause `!b_i, lit_e`).
     - For each edge $e \in F_i \setminus T_i$: $\neg b_i \implies e$ (clause `b_i, lit_e`).
     - For edges not in $T_i \cup F_i$: forbidden inside module.
  3. This completely eliminates intra-module fragmentation (such as the 70 length-16 subcycles in `graph788`), forcing SAT search to only navigate valid global states.

### Tier 3: `MacroCrossoverSplicer` with Variable Flip
- **Applicability**: When 2 to 6 macro-cycles remain along module boundaries.
- **Algorithm**:
  - Solves an auxiliary local SAT instance allowing module state flips $(b_i \leftrightarrow \neg b_i)$ to bridge boundary cycles.

---

## 3. Detailed Component Design

### 3.1 `BipartiteModuleDetector` (`src/cegar-fix/src/bipartite_module_detector.rs`)
- In bipartite graphs, common-neighbor counts between endpoints of edges are always 0 (triangle-free).
- Therefore, we cluster using **Virtual Edge Quotient Projection**:
  - Each of the 22 virtual edges in a module is a node in the quotient graph.
  - Two virtual edges $e_1 = (u_1, w_1)$ and $e_2 = (u_2, w_2)$ are connected if there is a real edge between $\{u_1, w_1\}$ and $\{u_2, w_2\}$.
  - The 22 virtual edges inside a module form a dense 22-vertex connected component with high internal edge density ($> 50$ edges) and very low cut-edge boundary ($\le 4$ edges leaving the component).
  - Strongly connected components or graph cut separates all $k$ modules in $O(|V| + |E|)$.

### 3.2 `ModuleDualPathExtractor` (`src/cegar-fix/src/module_dual_path_extractor.rs`)
- Inputs: 44 module vertices, 22 virtual edges, induced subgraph.
- Finds vertices with external degree $> 0$ (boundary ports).
- Runs bounded DFS ($N=44$, degree $\le 5$, search space $< 500$ steps) to find:
  - $T_i$: Hamiltonian path starting at port $A$, ending at port $B$.
  - $F_i$: Alternative Hamiltonian path starting at port $A$, ending at port $B$ (or alternative port pair).

### 3.3 `ModuleStateCnfEncoder` (`src/cegar-fix/src/module_state_cnf_encoder.rs`)
- Encodes module selector literals $b_1, \dots, b_k$.
- Generates implication clauses linking $b_i$ to edge choice variables in the master `base_cnf`.

---

## 4. Test & Verification Plan

1. **Unit Test 1: Bipartite Module Detection** (`tests/test_bipartite_module_detector.rs`):
   - Synthetic graph of two 44-vertex modules connected by 2 cross-edges.
   - Assert `BipartiteModuleDetector` detects exactly 2 modules of size 44.
2. **Unit Test 2: Dual Path Extraction** (`tests/test_module_dual_path_extractor.rs`):
   - Assert both $T_i$ and $F_i$ span all 44 vertices and use 100% of virtual edges.
3. **Unit Test 3: Module State Equivalence Encoding** (`tests/test_module_state_cnf_encoder.rs`):
   - Verify that fixing $b_i = 1$ forces all $T_i$ edges, and $b_i = 0$ forces all $F_i$ edges.
4. **Benchmark Verification (Core 0,1,2, nice -n 19)**:
   - Run on `graph479.col` (42 modules).
   - Run on `graph788.col` (70 modules).
   - Run on `graph710.col` (24 hubs + modules).
   - Run on `graph717.col` (80 hubs + modules).
   - Verify all tours with independent verifier.
