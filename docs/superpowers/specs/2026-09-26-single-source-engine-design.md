# Single Source of Truth Engine & Radical Codebase Simplification Specification

- **Date:** 2026-09-26
- **Branch:** `feat/principled-spqr-decomposition`
- **Scope:** Architectural simplification, elimination of code duplication, and consolidation into a minimalist Single Source of Truth HCP engine.

---

## 1. Problem Statement & Motivation

The repository has grown excessively (~80,848 total lines across docs, scratch, and code), and the Rust solver `src/cegar-fix/` contains 6,692 lines of code with significant architectural duplication and overfitted ad-hoc macros:

1. **Massive Code Duplication:**
   - **SAT-CEGAR 2-Factor Formulation:** Implemented across 5 separate files (`encoder.rs`, `cegar_engine.rs`, `block_solver.rs`, `corridor.rs`, `portfolio_788.rs`).
   - **Graph Traversal & Connected Components:** BFS and Hopcroft-Tarjan DFS algorithms implemented across 4 separate files (`fast_filters.rs`, `spqr_parallel.rs`, `corridor.rs`, `dynamic_bipartite.rs`).
   - **Subcycle 2-Opt Merging:** Implemented in `cycle_merge.rs` and `portfolio_788.rs`.
   - **Tour Splicing & Unrolling:** Implemented in `tour_stitcher.rs`, `corridor.rs`, and `dynamic_bipartite.rs`.

2. **Ad-Hoc Macro Solvers (`macro_decomp/` - 2,440 lines):**
   - `dynamic_bipartite.rs` (1,078 lines): Custom, hardcoded decomposition logic overfitted to bipartite structures like graph950.
   - `corridor.rs` (710 lines): Monolithic corridor solver built specifically for graphs 710, 717, 882.
   - `portfolio_788.rs` (649 lines): Ad-hoc DP and cycle patching solver for graph788.

3. **Maintenance & Cognitive Burden:**
   - Files reaching up to 1,000 lines make reasoning, verification, and debugging difficult.
   - Bug fixes in one copy of an algorithm are not reflected in the others.

---

## 2. Core Architectural Principles

1. **Single Source of Truth (DRY):**
   - Exactly ONE canonical implementation for SAT-CEGAR encoding, graph traversals, SPQR decomposition, and tour assembly.
   - Zero duplicated routines across the codebase.

2. **Strict Size Limits:**
   - Total Rust codebase in `src/cegar-fix/src/` targeted at **~1,500 - 1,800 lines** (a >70% reduction).
   - No individual file shall exceed 350–400 lines of code.

3. **Zero Hardcoded Heuristics:**
   - No graph-ID branching (`if gid == 710`, `match N`).
   - Pure topological dispatch based on structural graph properties ($O(V+E)$ invariants, series degree-2 chains, 2-cut separation pairs).

4. **100% Soundness & De Novo Verification:**
   - Zero precomputed files or external hints.
   - Every tour must pass TSPLIB verification via `TourVerifier` (simple Hamiltonian cycle, exact vertex coverage, every edge exists in original graph).

---

## 3. Simplified Module Architecture

```
src/cegar-fix/src/
├── core/
│   ├── graph.rs           # Standard graph data structure, adjacency queries, deterministic ordering
│   ├── tour_verifier.rs   # TSPLIB format and soundness verifier
│   ├── solver_utils.rs    # Preemptive CaDiCaL deadline terminator
│   └── mod.rs
├── algo/
│   ├── graph_algo.rs      # Single source of truth for BFS components, DFS 1-cut & 2-cuts, 2-coloring
│   ├── fast_filters.rs    # O(V+E) invariants: min degree >= 2, biconnectivity, bipartite parity
│   └── mod.rs
├── solver/
│   ├── sat_engine.rs      # Single source for CaDiCaL degree-2 encoding, forced edges, DFJ cuts (<=60v), 2-opt
│   └── mod.rs
├── decomp/
│   ├── spqr.rs            # Single source for series degree-2 contraction and 2-cut separation pairs
│   └── mod.rs
├── assembly/
│   ├── stitcher.rs        # Single source for subpath splicing (u <-> v) and series expansion
│   └── mod.rs
├── pipeline/
│   ├── options.rs         # CLI options and configuration
│   ├── solver_pipeline.rs # Unified monotonic deadline pipeline
│   └── mod.rs
├── lib.rs
└── main.rs
```

---

## 4. Component Details & Data Flow

### 4.1. Core & Algorithm Foundation
- **`core/graph.rs` (~300 lines):**
  - Directed/undirected representations, deterministic sorting of nodes and neighbor lists.
- **`algo/graph_algo.rs` (~200 lines):**
  - Canonical connected component labeling (BFS).
  - Canonical articulation point and bridge detection (Tarjan DFS).
  - Canonical bipartite test and 2-coloring ($V_0, V_1$).
- **`algo/fast_filters.rs` (~70 lines):**
  - Pre-flight $O(V+E)$ sanity checks:
    - Min degree $< 2 \implies$ immediate `UNSAT`.
    - 1-cut cut vertex $\implies$ immediate `UNSAT`.
    - Bipartite $|V_0| \ne |V_1| \implies$ immediate `UNSAT`.

### 4.2. SAT-CEGAR Engine (`solver/sat_engine.rs` ~300 lines)
- **Deterministic 2-Factor Formulation:**
  - CaDiCaL edge variable mapping.
  - Exact degree-2 constraints (unit clauses for degree 2, exact-2 card clauses for degree $\ge 3$).
  - Forced / forbidden edges support (unit assumption clauses).
- **Cycle & Path Duality:**
  - `solve_cycle(g, deadline) -> Result<Vec<i32>, String>`
  - `solve_path(g, u, v, deadline) -> Result<Vec<i32>, String>` (via dummy node connected strictly to $u$ and $v$).
- **Clause Budgeting & CDCL Protection:**
  - DFJ cuts restricted strictly to small cycles ($\le 60$ vertices) to prevent clause pollution and CDCL search thrashing on large rigid blocks.
  - Safe 2-opt heuristic subcycle merging for fast absorption.

### 4.3. SPQR Decomposition (`decomp/spqr.rs` ~250 lines)
- **Series Reduction (S-node):**
  - Contract maximal degree-2 chains $u - c_1 - \dots - c_k - w \to (u, w)$.
  - Record intermediate nodes in `chain_map` for 1-1 reversible unrolling.
- **Parallel / 2-Cut Separation Pairs (P-node):**
  - Identify separation pairs $\{u, v\}$ partitioning $G$ into $\ge 2$ disconnected components.
  - Extract subcomponent graph between ports $u$ and $v$.
- **Rigid Block Handling (R-node):**
  - Dense/rigid blocks with no 2-cut and degree $> 2$ are dispatched directly to `sat_engine`.

### 4.4. Tour Assembly (`assembly/stitcher.rs` ~120 lines)
- **Subpath Splicing:**
  - Replaces virtual edge $(u, v)$ in skeleton tour with solved subpath $u \leadsto v$.
  - Autodetects orientation ($u \to v$ or $v \to u$) and reverses subpath if needed.
- **Series Unrolling:**
  - Reversibly expands degree-2 chain nodes from `chain_map`.

### 4.5. Unified Pipeline Driver (`pipeline/solver_pipeline.rs` ~250 lines)
- **Execution Ladder:**
  1. Monotonic deadline check: `remaining = deadline - now`.
  2. Pre-flight fast invariants check via `algo::fast_filters`.
  3. Series contraction via `decomp::spqr`.
  4. Separation pair detection:
     - If 2-cut exists: recursively solve subcomponent path $u \leadsto v$, replace with virtual edge $(u, v)$ in skeleton, solve skeleton, and splice via `assembly::stitcher`.
     - If no 2-cut: solve skeleton directly via `solver::sat_engine`.
  5. Expand all contracted series chains via `assembly::stitcher`.
  6. Final certification via `core::tour_verifier::TourVerifier`.

---

## 5. Purge List

1. **Delete `macro_decomp/` (2,440 lines):**
   - `src/cegar-fix/src/macro_decomp/dynamic_bipartite.rs` (1,078 lines)
   - `src/cegar-fix/src/macro_decomp/corridor.rs` (710 lines)
   - `src/cegar-fix/src/macro_decomp/portfolio_788.rs` (649 lines)
   - `src/cegar-fix/src/macro_decomp/mod.rs` (3 lines)

2. **Delete Legacy Redundant Modules (1,559 lines):**
   - `src/cegar-fix/src/core/encoder.rs` (816 lines)
   - `src/cegar-fix/src/fallback/contraction.rs` (508 lines)
   - `src/cegar-fix/src/fallback/cycle_merge.rs` (184 lines)
   - `src/cegar-fix/src/fallback/fallback_cegar.rs` (51 lines)
   - `src/cegar-fix/src/fallback/mod.rs` (8 lines)

3. **Clean Up `scratch/`:**
   - Preserve all certified `.tour` files (`scratch/graph{710,717,832,882,944}.tour` and other verified tour outputs).
   - Purge stale throwaway Python debug scripts and temporary log files.

---

## 6. Verification & Quality Gates

The implementation must strictly satisfy the following validation gates:

### Gate 1: Rust Unit and Integration Test Suite
All unit and integration tests must pass cleanly:
```bash
cargo test --manifest-path src/cegar-fix/Cargo.toml --release
```
Target test coverage:
- Fast filters: `test_fast_filters.rs`
- SPQR series contraction and unrolling: `test_spqr_series.rs`
- SPQR 2-cut separation pairs: `test_spqr_parallel.rs`
- Block SAT-CEGAR solver (cycles and paths): `test_block_solver.rs`
- Tour stitcher: `test_tour_stitcher.rs`
- Principled pipeline: `test_principled_pipeline.rs`
- Permutation invariance and Petersen UNSAT: `test_principled_invariance.rs`

### Gate 2: Release Build Verification Sweep
Release binary must build without warnings or errors:
```bash
cargo build --manifest-path src/cegar-fix/Cargo.toml --release
```
Sample benchmark sweep on `FHCPCS-col/graph1.col` to `graph10.col` must all solve and pass `TourVerifier`.

### Gate 3: 11 Challenge Graphs Regression Gate
Every single one of the 11 challenge graphs previously certified SAT must be tested and verified:
**Graphs:** `710, 717, 746, 788, 882, 944, 950, 963, 975, 982, 990`
- Verification command runs each graph through the solver pipeline.
- All tours generated must pass `TourVerifier` (100% sound TSPLIB simple Hamiltonian cycle).
