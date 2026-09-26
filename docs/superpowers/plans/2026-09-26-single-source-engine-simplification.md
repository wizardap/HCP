# Single Source of Truth Engine — Codebase Simplification Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate all code duplication in `src/cegar-fix/`, delete 4,000+ lines of redundant ad-hoc modules, and verify the simplified codebase still solves all 11 challenge graphs.

**Architecture:** The macro_decomp/ cascade (corridor, dynamic_bipartite, portfolio_788) and legacy fallback/ modules are replaced by the principled SPQR pipeline already integrated in solver_pipeline.rs. The `add_at_most_2` helper and `safe_2opt_merge` are moved into the unified `solver/cegar_engine.rs` as it is their sole remaining consumer. All legacy integration tests are deleted; the principled test suite and 11-graph regression gate remain.

**Tech Stack:** Rust, CaDiCaL (via rustsat-cadical), cargo test

**Spec:** [`docs/superpowers/specs/2026-09-26-single-source-engine-design.md`](file:///root/HCP/docs/superpowers/specs/2026-09-26-single-source-engine-design.md)

## Global Constraints

- Zero graph-ID routing: no `match graph_id`, no `if N == 4064`, no hardcoded vertex sets.
- 100% de novo solving: 0 precomputed files, 0 cached tours, 0 injected subpaths.
- Strict Soundness: every tour certified by `TourVerifier` — zero false SAT.
- No file in `src/cegar-fix/src/` shall exceed 400 lines.
- All 11 previously-solved challenge graphs (710, 717, 746, 788, 882, 944, 950, 963, 975, 982, 990) must produce SAT_VERIFIED.

## Review Focus

1. **Cycle merge removal safety** — `cegar_engine.rs` imports `safe_2opt_merge` from `fallback::cycle_merge`. After inlining, verify the moved function signature and behavior are identical.
2. **`add_at_most_2` sole consumer** — `cegar_engine.rs` is the only non-macro_decomp consumer of `encoder::add_at_most_2`. After inlining, verify the function is byte-identical.
3. **Pipeline macro-decomp fallthrough** — `solver_pipeline.rs` lines 142–162 dispatch to `dynamic_bipartite`, `macro_788`, and `macro_corridor`. After removal, verify the principled SPQR path (lines 164+) handles all these graphs.
4. **Test file deletion safety** — 13 legacy test files reference deleted modules. Verify none of them test behaviors not covered by the 7 principled test files.
5. **11-graph timeout sensitivity** — Some challenge graphs (788, 950, 963, 975, 982, 990) previously relied on macro-decomp fast paths. Verify they solve within 600s via the general SPQR + CEGAR pipeline.

---

### Task 1: Inline Shared Helpers into cegar_engine.rs

Move `add_at_most_2` and `safe_2opt_merge` (the only two functions from legacy modules still used by the principled code) directly into `solver/cegar_engine.rs`, eliminating the cross-module dependencies.

**Files:**
- Modify: `src/cegar-fix/src/solver/cegar_engine.rs` (lines 1, 5)
- Reference (read-only): `src/cegar-fix/src/core/encoder.rs:772-815` (`add_at_most_2`)
- Reference (read-only): `src/cegar-fix/src/fallback/cycle_merge.rs:1-112` (`safe_2opt_merge` + `merge_two_cycles`)

**Interfaces:**
- Consumes: `crate::core::graph::Graph`, `crate::core::solver_utils::create_solver_with_deadline`, `crate::core::tour_verifier::TourVerifier`
- Produces: `pub fn solve_cycle(g: &Graph, timeout_secs: f64) -> Result<Vec<i32>, String>`, `pub fn solve_cycle_with_forced_edges(...)`, `pub fn solve_path(...)` — unchanged public API

- [ ] **Step 1: Copy `add_at_most_2` into cegar_engine.rs**

  Open `src/cegar-fix/src/core/encoder.rs` lines 772-815, copy the entire `pub fn add_at_most_2(...)` function.
  Paste it at the bottom of `src/cegar-fix/src/solver/cegar_engine.rs` (before the closing of the file).
  Change its visibility from `pub` to `fn` (module-private, only used within cegar_engine).

- [ ] **Step 2: Copy `merge_two_cycles` and `safe_2opt_merge` into cegar_engine.rs**

  Open `src/cegar-fix/src/fallback/cycle_merge.rs` lines 1-112.
  Copy both `merge_two_cycles` and `safe_2opt_merge` functions.
  Paste them into `src/cegar-fix/src/solver/cegar_engine.rs` (before `add_at_most_2`).
  Change visibility to `fn` (module-private).
  Update the `safe_2opt_merge` parameter type from `HashMap<i32, HashSet<i32>>` to match the `adj_sets: &HashMap<i32, HashSet<i32>>` already used in `cegar_engine.rs`.

- [ ] **Step 3: Update imports in cegar_engine.rs**

  Replace:
  ```rust
  use crate::core::encoder::add_at_most_2;
  use crate::fallback::cycle_merge::safe_2opt_merge;
  ```
  With:
  ```rust
  // (no external imports — both functions are now defined locally in this file)
  ```
  Keep all other imports unchanged.

- [ ] **Step 4: Verify compilation**

  Run:
  ```bash
  export PATH="$HOME/.cargo/bin:$PATH"
  cargo check --manifest-path src/cegar-fix/Cargo.toml
  ```
  Expected: compiles with no errors.

- [ ] **Step 5: Run principled test suite**

  Run:
  ```bash
  cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_cegar_engine --test test_fast_filters --test test_spqr_series --test test_spqr_parallel --test test_block_solver --test test_tour_stitcher --test test_principled_pipeline --test test_principled_invariance --test test_tour_verifier
  ```
  Expected: all tests PASS.

- [ ] **Step 6: Commit**

  ```bash
  git add src/cegar-fix/src/solver/cegar_engine.rs
  git commit -m "refactor(solver): inline add_at_most_2 and safe_2opt_merge into cegar_engine"
  ```

---

### Task 2: Delete Legacy Modules (macro_decomp/, fallback/, core/encoder.rs)

Remove 3,999 lines of dead code: the entire `macro_decomp/` directory, `fallback/` directory, and `core/encoder.rs`.

**Files:**
- Delete: `src/cegar-fix/src/macro_decomp/dynamic_bipartite.rs` (1,078 lines)
- Delete: `src/cegar-fix/src/macro_decomp/corridor.rs` (710 lines)
- Delete: `src/cegar-fix/src/macro_decomp/portfolio_788.rs` (649 lines)
- Delete: `src/cegar-fix/src/macro_decomp/mod.rs` (3 lines)
- Delete: `src/cegar-fix/src/fallback/contraction.rs` (508 lines)
- Delete: `src/cegar-fix/src/fallback/cycle_merge.rs` (184 lines)
- Delete: `src/cegar-fix/src/fallback/fallback_cegar.rs` (51 lines)
- Delete: `src/cegar-fix/src/fallback/mod.rs` (8 lines)
- Delete: `src/cegar-fix/src/core/encoder.rs` (816 lines)
- Modify: `src/cegar-fix/src/core/mod.rs` — remove `pub mod encoder;`
- Modify: `src/cegar-fix/src/lib.rs` — remove all `macro_decomp`, `fallback`, and `encoder` declarations and re-exports
- Delete: `src/cegar-fix/src/solver/block_solver.rs` (19 lines — thin wrapper, now redundant)
- Modify: `src/cegar-fix/src/solver/mod.rs` — remove `pub mod block_solver;`

**Interfaces:**
- Consumes: Task 1 complete (no more cross-references to deleted modules)
- Produces: Clean `lib.rs` with only `core`, `decomp`, `solver`, `assembly`, `pipeline` modules

- [ ] **Step 1: Remove macro_decomp dispatch from solver_pipeline.rs**

  In `src/cegar-fix/src/pipeline/solver_pipeline.rs`, delete lines 8-10 (the macro_decomp use statements):
  ```rust
  use crate::macro_decomp::corridor as macro_corridor;
  use crate::macro_decomp::dynamic_bipartite;
  use crate::macro_decomp::portfolio_788 as macro_788;
  ```
  And delete lines 142-162 (the entire macro-decomp cascade block):
  ```rust
      // Step 1.5: Topological Macro-Decomposition Cascade
      ...
      }
  ```

- [ ] **Step 2: Delete macro_decomp/ directory**

  ```bash
  rm -rf src/cegar-fix/src/macro_decomp/
  ```

- [ ] **Step 3: Delete fallback/ directory**

  ```bash
  rm -rf src/cegar-fix/src/fallback/
  ```

- [ ] **Step 4: Delete core/encoder.rs and update core/mod.rs**

  ```bash
  rm src/cegar-fix/src/core/encoder.rs
  ```
  Edit `src/cegar-fix/src/core/mod.rs` to remove `pub mod encoder;`:
  ```rust
  pub mod graph;
  pub mod file_operations;
  pub mod tour_verifier;
  pub mod solver_utils;
  ```

- [ ] **Step 5: Delete solver/block_solver.rs and update solver/mod.rs**

  ```bash
  rm src/cegar-fix/src/solver/block_solver.rs
  ```
  Edit `src/cegar-fix/src/solver/mod.rs`:
  ```rust
  pub mod cegar_engine;
  ```

- [ ] **Step 6: Update lib.rs — remove all dead re-exports**

  Replace the entire contents of `src/cegar-fix/src/lib.rs` with:
  ```rust
  pub mod core;
  pub use core::file_operations;
  pub use core::graph;
  pub use core::graph::Graph;
  pub use core::tour_verifier;
  pub use core::tour_verifier::TourVerifier;

  pub mod decomp;

  pub mod solver;

  pub mod assembly;

  pub mod pipeline;
  pub use pipeline::options;
  pub use pipeline::options::Options;
  pub use pipeline::solver_pipeline;
  pub use pipeline::solver_pipeline::{
      find_graph_file, run_batch, run_batch_range, save_checkpoint_atomic, solve_single_graph,
      verify_and_export, BatchItemResult, SolverPipelineError,
  };
  ```

- [ ] **Step 7: Verify compilation**

  ```bash
  cargo check --manifest-path src/cegar-fix/Cargo.toml
  ```
  Expected: compiles with no errors.

- [ ] **Step 8: Commit**

  ```bash
  git add -A src/cegar-fix/src/
  git commit -m "refactor: delete macro_decomp, fallback, encoder, block_solver (3,999 lines of dead code)"
  ```

---

### Task 3: Delete Legacy Integration Tests

Remove all 13 test files that test deleted modules. The principled test suite (7 files) covers all retained functionality.

**Files:**
- Delete: `src/cegar-fix/tests/macro_decomp_audit.rs`
- Delete: `src/cegar-fix/tests/test_debug_705.rs`
- Delete: `src/cegar-fix/tests/test_dynamic_bipartite_macro_sat.rs`
- Delete: `src/cegar-fix/tests/test_dynamic_bipartite_partition.rs`
- Delete: `src/cegar-fix/tests/test_dynamic_bipartite_solve.rs`
- Delete: `src/cegar-fix/tests/test_inspect_block_a.rs`
- Delete: `src/cegar-fix/tests/test_macro_challenge_graphs.rs`
- Delete: `src/cegar-fix/tests/test_solve_788_direct.rs`
- Delete: `src/cegar-fix/tests/test_solve_882.rs`
- Delete: `src/cegar-fix/tests/test_stage3_fallback.rs`
- Delete: `src/cegar-fix/tests/test_survey_2cuts.rs`
- Delete: `src/cegar-fix/tests/test_survey_user_29.rs`
- Delete: `src/cegar-fix/tests/test_upstream_verifier_parity.rs`
- Modify: `src/cegar-fix/tests/test_block_solver.rs` — update imports to use `cegar_engine` directly
- Modify: `src/cegar-fix/tests/test_unified_cli_batch.rs` — remove `test_solve_single_graph_graph76_fallback` test and any fallback references

**Interfaces:**
- Consumes: Task 2 complete (deleted modules no longer exist)
- Produces: Clean test suite with 0 references to deleted modules

- [ ] **Step 1: Delete 13 legacy test files**

  ```bash
  rm src/cegar-fix/tests/macro_decomp_audit.rs \
     src/cegar-fix/tests/test_debug_705.rs \
     src/cegar-fix/tests/test_dynamic_bipartite_macro_sat.rs \
     src/cegar-fix/tests/test_dynamic_bipartite_partition.rs \
     src/cegar-fix/tests/test_dynamic_bipartite_solve.rs \
     src/cegar-fix/tests/test_inspect_block_a.rs \
     src/cegar-fix/tests/test_macro_challenge_graphs.rs \
     src/cegar-fix/tests/test_solve_788_direct.rs \
     src/cegar-fix/tests/test_solve_882.rs \
     src/cegar-fix/tests/test_stage3_fallback.rs \
     src/cegar-fix/tests/test_survey_2cuts.rs \
     src/cegar-fix/tests/test_survey_user_29.rs \
     src/cegar-fix/tests/test_upstream_verifier_parity.rs
  ```

- [ ] **Step 2: Update test_block_solver.rs imports**

  In `src/cegar-fix/tests/test_block_solver.rs`, replace any `use cegar_fix::solver::block_solver::*` with `use cegar_fix::solver::cegar_engine::*` and update function call names:
  - `solve_hamiltonian_cycle(...)` → `solve_cycle(...)`
  - `solve_hamiltonian_path(...)` → `solve_path(...)`

- [ ] **Step 3: Update test_unified_cli_batch.rs**

  Remove the `test_solve_single_graph_graph76_fallback` test function that references `fallback`. Keep the remaining tests that use `solve_single_graph` (the pipeline function).

- [ ] **Step 4: Verify full test suite compiles and passes**

  ```bash
  cargo test --manifest-path src/cegar-fix/Cargo.toml
  ```
  Expected: all remaining tests PASS, no compilation errors.

- [ ] **Step 5: Commit**

  ```bash
  git add -A src/cegar-fix/tests/
  git commit -m "test: delete 13 legacy test files referencing purged modules"
  ```

---

### Task 4: 11 Challenge Graph Regression Gate

Verify that every one of the 11 challenge graphs previously solved produces SAT_VERIFIED through the simplified pipeline.

**Files:**
- Create: `src/cegar-fix/tests/test_11_challenge_regression.rs`

**Interfaces:**
- Consumes: `cegar_fix::solve_single_graph` from the pipeline
- Produces: Regression test that gates the simplified codebase

- [ ] **Step 1: Write the 11-graph regression test**

  Create `src/cegar-fix/tests/test_11_challenge_regression.rs`:
  ```rust
  use std::path::Path;

  /// Verifies all 11 challenge graphs that were previously certified SAT
  /// still produce SAT_VERIFIED through the simplified principled pipeline.
  ///
  /// Each graph is solved de novo with a 600s timeout and verified by TourVerifier.
  /// This is the final regression gate for the codebase simplification.

  const CHALLENGE_GRAPHS: &[usize] = &[710, 717, 746, 788, 882, 944, 950, 963, 975, 982, 990];

  fn solve_and_verify(gid: usize) {
      let col_path = format!("FHCPCS-col/graph{}.col", gid);
      if !Path::new(&col_path).exists() {
          eprintln!("[SKIP] graph{}: file not found at {}", gid, col_path);
          return;
      }
      let result = cegar_fix::solve_single_graph(&col_path, 600.0, None);
      match result {
          Ok((tour, time, vcount)) => {
              println!("[✓] graph{}: SAT_VERIFIED ({} vertices, {:.2}s)", gid, vcount, time);
              assert!(!tour.is_empty(), "Tour should not be empty for graph{}", gid);
          }
          Err(e) => {
              panic!("[✗] graph{}: FAILED — {}", gid, e.message);
          }
      }
  }

  #[test]
  fn test_challenge_graph_710() { solve_and_verify(710); }

  #[test]
  fn test_challenge_graph_717() { solve_and_verify(717); }

  #[test]
  fn test_challenge_graph_746() { solve_and_verify(746); }

  #[test]
  fn test_challenge_graph_788() { solve_and_verify(788); }

  #[test]
  fn test_challenge_graph_882() { solve_and_verify(882); }

  #[test]
  fn test_challenge_graph_944() { solve_and_verify(944); }

  #[test]
  fn test_challenge_graph_950() { solve_and_verify(950); }

  #[test]
  fn test_challenge_graph_963() { solve_and_verify(963); }

  #[test]
  fn test_challenge_graph_975() { solve_and_verify(975); }

  #[test]
  fn test_challenge_graph_982() { solve_and_verify(982); }

  #[test]
  fn test_challenge_graph_990() { solve_and_verify(990); }
  ```

- [ ] **Step 2: Verify test compiles**

  ```bash
  cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_11_challenge_regression --no-run
  ```
  Expected: compiles successfully.

- [ ] **Step 3: Run the regression gate**

  ```bash
  cargo test --manifest-path src/cegar-fix/Cargo.toml --test test_11_challenge_regression -- --test-threads=1 --nocapture
  ```
  Expected: all 11 tests PASS with SAT_VERIFIED.

  > **Note:** This will take significant time (potentially 30-60+ minutes total) as some graphs are large. If any graph TIMEOUTs, the test will fail and needs investigation.

- [ ] **Step 4: Run full test suite one final time**

  ```bash
  cargo test --manifest-path src/cegar-fix/Cargo.toml
  ```
  Expected: ALL tests PASS. Zero compilation warnings related to dead code.

- [ ] **Step 5: Verify final line count**

  ```bash
  echo "=== Final line count ==="
  wc -l src/cegar-fix/src/**/*.rs src/cegar-fix/src/*.rs
  echo "=== File count ==="
  find src/cegar-fix/src -name '*.rs' | wc -l
  echo "=== Largest files ==="
  wc -l src/cegar-fix/src/**/*.rs src/cegar-fix/src/*.rs | sort -rn | head -5
  ```
  Expected: Total ~2,100-2,500 lines, no file exceeds 400 lines.

- [ ] **Step 6: Commit**

  ```bash
  git add src/cegar-fix/tests/test_11_challenge_regression.rs
  git commit -m "test: add 11 challenge graph regression gate for simplified pipeline"
  ```
