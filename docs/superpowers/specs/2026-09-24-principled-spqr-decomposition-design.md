# Design Spec: Principled SPQR-Tree Decomposition & Zero-Hardcode HCP Architecture

**Date:** 2026-09-24  
**Status:** Approved  
**Branch:** `feat/principled-spqr-decomposition`  
**Scope:** `src/cegar-fix/src/decomp/`, `src/cegar-fix/src/solver/`, `src/cegar-fix/src/assembly/`, `src/cegar-fix/src/pipeline/`, purge of `hcp_solver/families/` and precomputed cache files.

---

## 1. Background & Problem Statement

### 1.1. Current State & The "Overfitting" Anti-Pattern
Previous iterations of the solver successfully purged hardcoded vertex IDs (such as `vec![1430, 3641, ...]`) and hardcoded graph ID routing (`HCPRouter.solve_file`). However, the architectural foundation still retained **instance-specific overfitting (benchmark exploitation)**:
1. **Ad-hoc Macro Solvers in Rust:**
   - `dynamic_bipartite.rs`: Hand-tailored specifically for the dense bipartite hub and strip structures of graphs 746, 950, 963, 975, 982, and 990.
   - `corridor.rs`: Hand-tailored specifically for the 2-cut bridge corridors of graphs 710, 717, 882, and 944.
   - `portfolio_788.rs`: Hand-tailored specifically for graph 788's alternating pair degree-2 structure.
2. **Blatant Hardcoding in Python Legacy Code:**
   - `hcp_solver/families/corridor_solver.py`: Explicitly maps `SUPPORTED_GRAPHS = {710, 717, 882, 944}`, maps vertex counts `size_map = {4064: 710, ...}`, and reads precomputed tour fragments from disk (`data/corridors/graph710/block_paths.json`, `scratch/graph717/found_tour_graph717.hcp`).
3. **Generalization Failure:**
   - If an arbitrary graph of size $N = 5,000$ with an unseen topology is provided (e.g. 3-regular expander, random graph, or 3D grid), none of the ad-hoc macro signatures match. The solver falls back to raw CDCL SAT-CEGAR and inevitably times out.

### 1.2. Design Objective
Replace all ad-hoc macro solvers, precomputed caches, and instance-specific heuristics with a **100% de novo, mathematically sound, zero-hardcode pipeline** based on classical structural graph decomposition (Tutte's SPQR-tree / Hopcroft-Tarjan triconnected reduction, Bipartite parity, and Modular reduction).

---

## 2. Mathematical Principles & Theoretical Invariants

The design grounds all graph reductions in proven theorems of structural graph theory:

### 2.1. 1-Cut (Articulation Point) Invariant
- **Theorem:** If a graph $G$ has an articulation point (cut-vertex), deleting it leaves $\ge 2$ connected components. A simple cycle must enter and leave each component through distinct vertices. Thus, a cut-vertex graph cannot contain a Hamiltonian cycle.
- **Application:** A standard Tarjan DFS $O(V+E)$ detects articulation points. If any exist, the solver immediately certifies and returns `UNSATISFIABLE`.

### 2.2. Tutte's SPQR-Tree & 2-Cut (Separation Pair) Theorem
- **Theorem (Tutte 1966, Hopcroft-Tarjan 1973):** Every 2-connected graph has a unique canonical decomposition into a tree of triconnected components (SPQR-tree):
  - **S (Series):** Chains of degree-2 vertices.
  - **P (Parallel):** Multiple edges or disjoint subgraphs connected across the same separation pair $\{u, v\}$.
  - **Q (Trivial):** A single edge.
  - **R (Rigid):** True 3-connected (triconnected) components having no 2-cut.
- **Hamiltonian 2-Cut Lemma:** Let $\{u, v\}$ be a 2-cut separating $G$ into components $G_1, G_2$. Any Hamiltonian cycle $H$ on $G$ must traverse between $G_1$ and $G_2$ across $\{u, v\}$ exactly twice. Therefore:
  - $H \cap G_1$ is a Hamiltonian path between $u$ and $v$ visiting all vertices in $V(G_1)$.
  - $H \cap G_2$ is a Hamiltonian path between $u$ and $v$ visiting all vertices in $V(G_2)$.
  - $H = (H \cap G_1) \cup (H \cap G_2)$ is a sound Hamiltonian cycle for $G$.
- **Sound Divide & Conquer Reduction:**
  1. Detect separation pair $\{u, v\}$.
  2. If a component $G_{\text{sub}}$ is smaller or has corridor topology, solve a Hamiltonian path $u \leadsto v$ on $G_{\text{sub}}$.
  3. Contract $G_{\text{sub}}$ into a single virtual shortcut edge $(u, v)$ in the skeleton graph $G_{\text{skeleton}}$.
  4. Solve a Hamiltonian cycle on $G_{\text{skeleton}}$.
  5. Expand virtual edges back to sound paths.

### 2.3. Bipartite Parity & Twin Modular Reduction
- **Bipartite Parity Theorem:** In a bipartite graph $G = (V_1 \cup V_2, E)$, every step in an alternating cycle transitions between $V_1$ and $V_2$. Therefore, $G$ is Hamiltonian only if $|V_1| = |V_2|$. If $|V_1| \neq |V_2|$, return `UNSATISFIABLE`.
- **Twin / Modular Equivalence:** Vertices $u, w$ with identical open neighborhoods $N(u) = N(w)$ possess topological symmetry. In SAT encoding, this symmetry is broken dynamically via canonical ordering clauses without assuming domain-specific strip geometry.

---

## 3. System Architecture & Modular Structure

```
                                  Input Graph G = (V, E)
                                            │
                                            ▼
                        ┌───────────────────────────────────────┐
                        │        Stage 0: Fast Filters          │
                        │ - Min degree < 2 check -> UNSAT       │
                        │ - Disconnected check -> UNSAT         │
                        │ - 1-cut Tarjan DFS -> UNSAT           │
                        │ - Bipartite parity (|V1|!=|V2|)->UNSAT│
                        └───────────────────┬───────────────────┘
                                            ▼ (2-Connected & Balanced)
                        ┌───────────────────────────────────────┐
                        │      Stage 1: SPQR Decomposition      │
                        │       (decomp::spqr module)           │
                        │                                       │
                        │ 1a. Series: Degree-2 Chain Contraction│
                        │     u - v1 - v2 - w  ==>  (u, w)      │
                        │                                       │
                        │ 1b. Parallel: 2-Cut Separation Pairs  │
                        │     Isolate subcomponents G_sub       │
                        └───────────────────┬───────────────────┘
                                            │
                                            ▼
                        ┌───────────────────────────────────────┐
                        │      Stage 2: Core SAT Block Solver   │
                        │      (solver::block_solver module)    │
                        │                                       │
                        │ - solve_hamiltonian_path(G_sub, u, v) │
                        │ - solve_hamiltonian_cycle(G_skeleton) │
                        │ - CaDiCaL + DFJ Cuts + Terminator     │
                        └───────────────────┬───────────────────┘
                                            │
                                            ▼
                        ┌───────────────────────────────────────┐
                        │   Stage 3: Recursive Tour Assembly    │
                        │     (assembly::tour_stitcher)         │
                        │                                       │
                        │ - Expand virtual edges                │
                        │ - Splice subcomponent paths at ports  │
                        └───────────────────┬───────────────────┘
                                            │
                                            ▼
                        ┌───────────────────────────────────────┐
                        │   Stage 4: Independent Verification   │
                        │       (core::tour_verifier)           │
                        │                                       │
                        │ - Verify |V| unique vertices          │
                        │ - Verify each (u, v) in E_original    │
                        │ - Export TSPLIB .hcp                  │
                        └───────────────────────────────────────┘
```

### 3.1. New Modules in `src/cegar-fix/src/`

#### 1. `decomp::spqr` (`src/cegar-fix/src/decomp/spqr.rs`)
- Replaces `corridor.rs` and ad-hoc 2-cut heuristics.
- Implements separation pair discovery and degree-2 chain extraction.
- Data structures:
  ```rust
  pub struct SeparationPair {
      pub u: i32,
      pub v: i32,
      pub components: Vec<Vec<i32>>,
  }
  
  pub struct SpqrDecomposition {
      pub chain_contractions: HashMap<(i32, i32), Vec<i32>>,
      pub separation_pairs: Vec<SeparationPair>,
      pub rigid_skeleton: Graph,
  }
  ```

#### 2. `decomp::modular_bipartite` (`src/cegar-fix/src/decomp/modular_bipartite.rs`)
- Replaces `dynamic_bipartite.rs` and `portfolio_788.rs`.
- Validates 2-colorability and equal partition sizes.
- Extracts twin vertex classes ($N(u) = N(w)$) to generate symmetry-breaking unit/binary clauses for CDCL SAT.

#### 3. `solver::block_solver` (`src/cegar-fix/src/solver/block_solver.rs`)
- Standardized, general-purpose SAT-CEGAR interfaces:
  ```rust
  pub fn solve_hamiltonian_cycle(
      g: &Graph,
      timeout_secs: f64
  ) -> Result<Vec<i32>, SolverError>;

  pub fn solve_hamiltonian_path(
      g: &Graph,
      port_u: i32,
      port_v: i32,
      timeout_secs: f64
  ) -> Result<Vec<i32>, SolverError>;
  ```
- Equipped with:
  - Strict preemptive deadline checking via `attach_terminator`.
  - DFJ subcycle elimination cuts.
  - Safe 2-opt cycle merging (never overwriting raw subcycles for cut generation).

#### 4. `assembly::tour_stitcher` (`src/cegar-fix/src/assembly/tour_stitcher.rs`)
- Recursively expands contracted virtual edges:
  - Degree-2 chain virtual edges: replaces $(u, w)$ with $u \to v_1 \to \dots \to w$.
  - 2-cut component virtual edges: replaces $(u, v)$ with the sound Hamiltonian path $u \leadsto v$ of the subcomponent.
- Guarantees cycle closure and correct orientation across interface ports.

### 3.2. Purge & Deprecation Plan
- Delete `hcp_solver/families/` (`corridor_solver.py`, `block_splicer.py`, `dense_bipartite.py`).
- Delete `data/corridors/` and all static JSON precomputed tour data.
- Deprecate `macro_decomp/` in `src/cegar-fix`, migrating valid mathematical logic to `decomp/` and deleting benchmark-specific filenames (`portfolio_788.rs`).

---

## 4. Execution Flow & Algorithmic Details

1. **Load Graph:** Graph $G = (V, E)$ loaded from file or memory.
2. **Fast Soundness Invariants:**
   - If $\exists v \in V: \text{deg}(v) < 2 \implies$ Return `UNSATISFIABLE`.
   - If $G$ has $> 1$ connected components $\implies$ Return `UNSATISFIABLE`.
   - If $G$ has cut-vertices (via Tarjan DFS) $\implies$ Return `UNSATISFIABLE`.
   - If $G$ is 2-colorable and $|V_1| \neq |V_2| \implies$ Return `UNSATISFIABLE`.
3. **SPQR Series Contraction:**
   - Contract every maximal chain $u - c_1 - \dots - c_k - w$ of degree-2 vertices to a virtual edge $(u, w)$.
   - Store mapping in `chain_map`.
4. **SPQR Separation Pair Decomposition:**
   - Scan for 2-cuts $\{u, v\}$ separating $G$ into $\ge 2$ components.
   - For each separable component $C$ where $|C| < |V| - 2$:
     - Construct subgraph $G_C = G[C \cup \{u, v\}]$.
     - Solve Hamiltonian path $u \leadsto v$ on $G_C$ using `solve_hamiltonian_path`.
     - If path found, replace $C$ in the main graph with a virtual edge $(u, v)$.
5. **Skeleton Solving:**
   - Solve Hamiltonian cycle on remaining skeleton graph $G_{\text{skeleton}}$ using `solve_hamiltonian_cycle`.
6. **Recursive Assembly:**
   - Traverse the skeleton tour.
   - Replace virtual edges with their corresponding subcomponent paths and degree-2 intermediate nodes.
7. **Strict Independent Certification:**
   - Execute `TourVerifier::verify(&raw_g, &tour)`.
   - Ensure $|\text{tour}| = |V(G)|$, all vertices are unique, and all traversed edges exist in the original input graph.
   - Export TSPLIB `.hcp`.

---

## 5. Testing & Verification Strategy

### 5.1. Permutation & Isomorphism Invariance Tests
- **Objective:** Prove zero dependence on vertex numbering, file line order, or specific vertex IDs.
- **Protocol:**
  - For any input graph $G$, generate random bijection $\pi: V \to V$ and shuffle the edge list to produce $G' \cong G$.
  - Run the solver on $G'$.
  - Verify that $\pi^{-1}(\text{tour}')$ is a certified valid tour of $G$.

### 5.2. Unit & Property-Based Tests
- `test_spqr_series_cycle`: Cycle graphs $C_n$ ($n \in [4, 100]$) reduce cleanly to 3-cycles and solve.
- `test_spqr_separation_pair`: Synthetic 2-cut graphs composed of two rigid blocks connected at 2 cut vertices are separated, solved as sub-paths, and stitched seamlessly.
- `test_hamiltonian_path_endpoints`: Validates that `solve_hamiltonian_path` strictly begins at $u$, terminates at $v$, and visits $100\%$ of vertices in the subgraph.

### 5.3. Adversarial / Negative Soundness Tests
- Verify that non-Hamiltonian graphs with 2-cuts or degree-2 chains are correctly reported as `UNSATISFIABLE`:
  - **Petersen Graph** (smallest 3-regular non-Hamiltonian graph).
  - **Herschel Graph** (bipartite non-Hamiltonian planar graph).
  - **Chvátal Graph** (triangle-free non-Hamiltonian graph).
  - Graphs with a 2-cut where one side admits no Hamiltonian path between ports.

### 5.4. Benchmark Suites
- **Suite A (graph1 .. graph400):** Must achieve 100% pass rate without regressions.
- **Massive Challenge Graphs ($N \ge 4000$):** Measure pure de novo performance without any precomputed files, hardcoded parameters, or domain-specific assumptions.
