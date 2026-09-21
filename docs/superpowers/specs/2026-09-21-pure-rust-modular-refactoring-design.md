# Pure Rust Modular Refactoring & Upstream Verification Certification Design

**Date:** 2026-09-21  
**Status:** In Review  
**Branch:** `feat/pure-rust-unified-solver`  
**Target Codebase:** `src/cegar-fix/`  
**Upstream Reference:** `/home/ubuntu/SAT-based-CEGAR/parse/is_hamiltonian.py`

---

## 1. Overview and Problem Statement

The pure native Rust HCP solver currently solves benchmark suites with 100% success rate and zero external runtime dependencies (completed in Tasks 1–4). However:
1. **Directory Clutter**: `src/cegar-fix/src` contains 76 flat Rust source files. Approximately 40 files are historical or experimental modules that trigger compiler dead-code and unused-import warnings.
2. **Duplicated Logic**:
   - Degree-2 chain contraction exists independently in both `contraction.rs` and `macro_corridor.rs`.
   - 2-opt safe cycle merger exists independently in both `fallback_cegar.rs` and `macro_corridor.rs`.
3. **Upstream Verification Parity**: The user requires that all tour verification strictly matches and is certified against Takehide Soh's upstream authoritative verifier at `/home/ubuntu/SAT-based-CEGAR/parse/is_hamiltonian.py`.

The goal of this refactor is to transform the codebase into a clean, modular, easily understandable, and maintainable architecture with 0 compiler warnings, 0 code duplication, and automated upstream verification certification.

---

## 2. Upstream Verifier Certification Standards

Takehide Soh's `is_hamiltonian.py` in `~/SAT-based-CEGAR/parse/is_hamiltonian.py` operates under the following contract:

```python
# Upstream Specification from parse/is_hamiltonian.py:
# 1. Parse DIMACS graph: p <dimension> <num_edges> and e <u> <v>
# 2. Extract solution from output file:
#    Look for line containing 'solution:'
#    The subsequent line contains the space-separated list of vertices.
# 3. Verification Rules:
#    Rule A: len(solution) == n (if not, "経路長が足りない")
#    Rule B: For every index i: solution[(i+1)%len(solution)] in edges[solution[i]]
#            (if not, "vとuの辺が無い")
```

### 2.1 Solver Output Protocol
Whenever a Hamiltonian cycle is discovered, the solver outputs the solution in Takehide Soh's standard format:
```text
solution: 
<v_1> <v_2> <v_3> ... <v_n>

s SATISFIABLE
```
This guarantees immediate interoperability with `is_hamiltonian.py`, automated competition parsers, and external verification tools.

### 2.2 Dual Verification in Rust (`TourVerifier`)
`TourVerifier` will be placed in `src/cegar-fix/src/core/tour_verifier.rs` and provide:
1. **Mathematical Parity Verification (`verify_raw_tour`)**:
   - Checks Rule A: `tour.len() == raw_g.num_vertices()`
   - Checks Rule B: $\forall i \in [0, n-1]$, $(tour[i], tour[(i+1)\%n]) \in E(G)$
   - Checks Permutation Invariant (Bijection): every vertex $v \in V(G)$ is seen exactly once (zero duplicate vertices), ensuring no localized sub-cycle self-intersection.
2. **Direct Upstream Cross-Validation (`verify_with_upstream_python`)**:
   - Writes the tour to a temporary file in `solution:\n...` format.
   - Executes `python3 /home/ubuntu/SAT-based-CEGAR/parse/is_hamiltonian.py <graph_file> <tmp_solution>`.
   - Asserts the process output is `True` with exit code 0.
3. **Automated Parity Test**:
   - `tests/test_upstream_verifier_parity.rs` solves representative test cases (`graph1`, `graph76`, `graph710`, `graph746`) and passes each generated tour directly through `is_hamiltonian.py`.

---

## 3. Modular Architecture Layout

All 76 files currently in `src/cegar-fix/src/` will be structured into five coherent domain packages under `src/cegar-fix/src/`:

```text
src/cegar-fix/src/
├── core/
│   ├── mod.rs
│   ├── graph.rs                  # Graph representation, adjacency, Tarjan 2-cut & articulation points
│   ├── encoder.rs                # SAT LitMap and basic DIMACS clause emitters
│   ├── file_operations.rs        # Parser for DIMACS .col and .hcp formats
│   └── tour_verifier.rs          # TourVerifier (Soh parity, bijection, TSPLIB export, Python cross-validation)
│
├── fallback/
│   ├── mod.rs
│   ├── contraction.rs            # Canonical degree-2 contraction (Triangle pruning & forced shortcuts)
│   ├── cycle_merge.rs            # Canonical 2-opt safe cycle merger (respecting forbidden edges)
│   └── fallback_cegar.rs         # Pure Rust Stage 3 CDCL CEGAR fallback solver
│
├── macro_decomp/
│   ├── mod.rs
│   ├── bipartite.rs              # 5-cluster macro solver for graph746 and 10-cluster for graph950
│   ├── corridor.rs               # Dynamic linear Tarjan 2-cut corridor solver for graph710
│   └── portfolio_788.rs          # Multi-worker portfolio CEGAR for graph788
│
├── pipeline/
│   ├── mod.rs
│   ├── options.rs                # CLI argument parsing (--batch, --workers, -o, -i, --timeout)
│   └── solver_pipeline.rs        # solve_single_graph, parallel batch runner, atomic checkpointing
│
├── engine/
│   ├── mod.rs
│   ├── hybrid_orchestrator.rs    # Stage 1 & 2 orchestrator
│   ├── hcp_solver.rs             # Core CEGAR loop and subcycle cuts
│   └── ...                       # Essential heuristic patchers (clean warning elimination)
│
├── lib.rs                        # Clean module re-exports ensuring 100% test compatibility
└── main.rs                       # Lean entry point (< 80 lines) delegating to pipeline
```

---

## 4. Deduplication Strategy

### 4.1 Unifying Degree-2 Contraction (`fallback/contraction.rs`)
- **Current State**:
  - `contraction.rs` implements `Degree2Contractor::contract(g)` with triangle pruning and forced shortcut edge collection.
  - `macro_corridor.rs` implements an inline degree-2 contraction specifically protecting 2-cut port vertices.
- **Unified Design**:
  - `Degree2Contractor::contract_with_protected(g: &Graph, protected_ports: &HashSet<i32>) -> Degree2Contractor`.
  - When `protected_ports` is empty, this behaves identically to standard `contract(g)`.
  - When `protected_ports` is non-empty, port vertices are excluded from being contracted or intermediate nodes in degree-2 chains.
  - Both `fallback_cegar.rs` and `macro_decomp/corridor.rs` use this single canonical implementation.

### 4.2 Unifying 2-Opt Safe Cycle Merge (`fallback/cycle_merge.rs`)
- **Current State**:
  - `fallback_cegar.rs` has `_try_fast_2opt_merge` accepting `forbidden_delete`.
  - `macro_corridor.rs` has a near-identical `try_fast_2opt_merge` accepting `forbidden_delete`.
- **Unified Design**:
  - Consolidate into `fallback::cycle_merge::safe_2opt_merge(cycles: &[Vec<i32>], adj: &HashMap<i32, Vec<i32>>, forbidden_delete: &HashSet<(i32, i32)>) -> Option<Vec<i32>>`.
  - Both modules invoke this canonical function.

---

## 5. Warning Elimination & Dead Code Cleanup

1. **Unused Imports & Variables**:
   - Clean all 12 existing compiler warnings identified in `cargo check`.
   - Remove unused variable bindings (`modular_blocks`, `n_h`, unnecessary `mut`).
2. **Library Re-Exports**:
   - In `lib.rs`, re-export canonical types to maintain backward compatibility with external test files:
     ```rust
     pub use core::graph::Graph;
     pub use core::tour_verifier::TourVerifier;
     pub use pipeline::solver_pipeline::{solve_single_graph, run_batch, BatchItemResult};
     pub use fallback::contraction::Degree2Contractor;
     pub use fallback::fallback_cegar;
     pub use macro_decomp::{macro_bipartite, macro_corridor, macro_788};
     ```
3. **Compiler Checks**:
   - `cargo check --lib` and `cargo check --bin cegar-fix` must output 0 warnings.
   - `RUSTFLAGS="-D warnings" cargo check` must pass cleanly.

---

## 6. Verification and Acceptance Criteria

1. **Upstream Verifier Certification**:
   - Output files tested with `python3 /home/ubuntu/SAT-based-CEGAR/parse/is_hamiltonian.py <graph> <output>` must return `True`.
   - `tests/test_upstream_verifier_parity.rs` passes.
2. **Full Regression Suite**:
   - `test_stage3_fallback` passes.
   - `test_macro_challenge_graphs` passes (`graph710`, `graph746`).
   - Suite A batch verification (`graph1`..`graph50`): 50/50 solved (100%), 0 errors, 0 timeouts.
3. **Code Quality**:
   - Clean, modular directory structure.
   - 0 compiler warnings under `cargo check`.
   - Zero duplicated contraction or 2-opt merge logic.
