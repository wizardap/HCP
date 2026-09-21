# Pure Rust Modular Refactoring & Upstream Verifier Certification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor the native Rust HCP solver (`src/cegar-fix`) into a clean domain-driven architecture (`core/`, `fallback/`, `macro_decomp/`, `pipeline/`, `engine/`), eliminate duplication between fallback and corridor solvers, remove all compiler warnings, and certify tour verification directly against Takehide Soh's upstream reference at `/home/ubuntu/SAT-based-CEGAR/parse/is_hamiltonian.py`.

**Architecture:**
- `core/`: Fundamental graph, SAT encoder, file parser, and `TourVerifier` (implementing exact logic of `is_hamiltonian.py` plus permutation uniqueness, and supporting direct Python subprocess execution of `is_hamiltonian.py`).
- `fallback/`: Canonical `Degree2Contractor` (with protected ports support and triangle pruning), canonical `safe_2opt_merge` (with forbidden edge protection), and Stage 3 fallback CEGAR solver.
- `macro_decomp/`: Domain modules for challenge graphs (`bipartite.rs` for graph746/graph950, `corridor.rs` for graph710 using canonical fallback contraction, and `portfolio_788.rs` for graph788).
- `pipeline/`: Options parser, single graph solver, batch runner with atomic JSON checkpoints.
- `engine/`: CEGAR hybrid orchestrator, DFJ cuts, and heuristic patchers with all warnings cleaned.
- `lib.rs` & `main.rs`: Clean re-exports and lean entry point ensuring 100% backward compatibility with all tests and scripts.

**Tech Stack:** Rust 2021 edition, `rustsat-cadical`, `rayon`, `serde`, `serde_json`, Python 3 (upstream verification reference).

**Spec:** `docs/superpowers/specs/2026-09-21-pure-rust-modular-refactoring-design.md`

## Global Constraints
- 100% Rust runtime for solving: No external Python process is required to find solutions.
- Upstream Verifier Certification: All verification must strictly conform to `/home/ubuntu/SAT-based-CEGAR/parse/is_hamiltonian.py` and produce identical truth verdicts.
- Zero Compiler Warnings: `cargo check` must pass with 0 warnings.
- Zero Regression: All existing integration tests (`test_stage3_fallback`, `test_macro_challenge_graphs`, `test_unified_cli_batch`, and Suite A 1..50) must pass.

---

### Task 1: Core Domain Module & Upstream Verifier Integration

**Files:**
- Create: `src/cegar-fix/src/core/mod.rs`
- Create: `src/cegar-fix/src/core/graph.rs`
- Create: `src/cegar-fix/src/core/encoder.rs`
- Create: `src/cegar-fix/src/core/file_operations.rs`
- Create: `src/cegar-fix/src/core/tour_verifier.rs`
- Test: `src/cegar-fix/tests/test_upstream_verifier_parity.rs`
- Modify: `src/cegar-fix/src/lib.rs`

**Interfaces:**
- Produces:
  - `core::graph::Graph`
  - `core::encoder::Encoder`
  - `core::file_operations::parse_graph_from_file`
  - `core::tour_verifier::TourVerifier::verify(raw_g: &Graph, tour: &[i32]) -> (bool, String)`
  - `core::tour_verifier::TourVerifier::verify_upstream_python(graph_path: &str, tour: &[i32]) -> Result<bool, String>`

- [ ] **Step 1: Write integration test for upstream verifier parity**
Create `src/cegar-fix/tests/test_upstream_verifier_parity.rs`:
```rust
use cegar_fix::core::file_operations;
use cegar_fix::core::tour_verifier::TourVerifier;
use cegar_fix::fallback_cegar;

#[test]
fn test_graph1_upstream_verifier_parity() {
    let graph_path = "../../FHCPCS-col/graph1.col";
    let g = file_operations::parse_graph_from_file(graph_path).expect("Failed to parse graph1");
    let tour = fallback_cegar::solve_with_contraction(&g, 30.0).expect("Failed to solve graph1");
    
    // Internal verifier parity check
    let (valid, err) = TourVerifier::verify(&g, &tour);
    assert!(valid, "Internal TourVerifier failed: {}", err);

    // Direct upstream check via ~/SAT-based-CEGAR/parse/is_hamiltonian.py
    let upstream_valid = TourVerifier::verify_upstream_python(graph_path, &tour)
        .expect("Failed to run upstream is_hamiltonian.py");
    assert!(upstream_valid, "Upstream is_hamiltonian.py returned False");
}
```

- [ ] **Step 2: Create core module files and implement `verify_upstream_python` in `tour_verifier.rs`**
In `src/cegar-fix/src/core/tour_verifier.rs`, add:
```rust
use std::process::Command;
use std::fs::File;
use std::io::Write;

impl TourVerifier {
    // ... existing verify and verify_raw_tour ...

    /// Runs Takehide Soh's upstream verifier at ~/SAT-based-CEGAR/parse/is_hamiltonian.py
    pub fn verify_upstream_python(graph_path: &str, tour: &[i32]) -> Result<bool, String> {
        let tmp_path = format!("/tmp/tour_verify_{}.txt", std::process::id());
        {
            let mut f = File::create(&tmp_path).map_err(|e| e.to_string())?;
            writeln!(f, "solution: ").map_err(|e| e.to_string())?;
            for (idx, &v) in tour.iter().enumerate() {
                if idx > 0 { write!(f, " ").map_err(|e| e.to_string())?; }
                write!(f, "{}", v).map_err(|e| e.to_string())?;
            }
            writeln!(f).map_err(|e| e.to_string())?;
            writeln!(f, "s SATISFIABLE").map_err(|e| e.to_string())?;
        }

        let script_path = "/home/ubuntu/SAT-based-CEGAR/parse/is_hamiltonian.py";
        let output = Command::new("python3")
            .arg(script_path)
            .arg(graph_path)
            .arg(&tmp_path)
            .output()
            .map_err(|e| format!("Failed to execute python3 {}: {}", script_path, e))?;

        let _ = std::fs::remove_file(&tmp_path);

        let stdout = String::from_utf8_lossy(&output.stdout);
        let trimmed = stdout.trim();
        if trimmed.ends_with("True") {
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
```
Set up `core/mod.rs` and re-export `graph`, `encoder`, `file_operations`, `tour_verifier`.
Update `src/cegar-fix/src/lib.rs` to declare `pub mod core;` and re-export `pub use core::graph::Graph;` etc.

- [ ] **Step 3: Run parity test to verify compilation and execution**
Run: `cargo test --test test_upstream_verifier_parity --manifest-path src/cegar-fix/Cargo.toml --release`
Expected: PASS with `upstream is_hamiltonian.py returned True`.

- [ ] **Step 4: Commit Task 1**
```bash
git add src/cegar-fix/src/core src/cegar-fix/src/lib.rs src/cegar-fix/tests/test_upstream_verifier_parity.rs
git commit -m "refactor(core): organize core module and add upstream verifier cross-validation"
```

---

### Task 2: Fallback Deduplication & Canonical 2-Opt Merger

**Files:**
- Create: `src/cegar-fix/src/fallback/mod.rs`
- Create: `src/cegar-fix/src/fallback/contraction.rs`
- Create: `src/cegar-fix/src/fallback/cycle_merge.rs`
- Create: `src/cegar-fix/src/fallback/fallback_cegar.rs`
- Modify: `src/cegar-fix/src/lib.rs`
- Test: `src/cegar-fix/tests/test_stage3_fallback.rs`

**Interfaces:**
- Consumes: `core::graph::Graph`, `core::tour_verifier::TourVerifier`
- Produces:
  - `fallback::contraction::Degree2Contractor::contract(g: &Graph) -> Self`
  - `fallback::contraction::Degree2Contractor::contract_with_protected(g: &Graph, protected: &HashSet<i32>) -> Self`
  - `fallback::cycle_merge::safe_2opt_merge(cycles: &[Vec<i32>], adj: &HashMap<i32, Vec<i32>>, forbidden_delete: &HashSet<(i32, i32)>) -> Option<Vec<i32>>`
  - `fallback::fallback_cegar::solve_with_contraction(g: &Graph, timeout_secs: f64) -> Result<Vec<i32>, String>`

- [ ] **Step 1: Unify Degree-2 Contraction in `fallback/contraction.rs`**
Implement `Degree2Contractor::contract_with_protected` that accepts optional protected port vertices (so neither port is contracted nor treated as an interior degree-2 vertex), along with chordless triangle pruning and forced shortcut edge collection.

- [ ] **Step 2: Unify Safe 2-Opt Cycle Merge in `fallback/cycle_merge.rs`**
Extract `safe_2opt_merge` as a standalone clean function that takes cycles, graph adjacency, and `forbidden_delete` edge set.

- [ ] **Step 3: Connect `fallback/fallback_cegar.rs` to use unified modules**
Port `fallback_cegar.rs` into `fallback/fallback_cegar.rs` referencing `crate::fallback::contraction::Degree2Contractor` and `crate::fallback::cycle_merge::safe_2opt_merge`.
Update `src/cegar-fix/src/lib.rs` to export `pub mod fallback;` and re-export `pub use fallback::fallback_cegar; pub use fallback::contraction::Degree2Contractor;`.

- [ ] **Step 4: Run existing fallback tests**
Run: `cargo test --test test_stage3_fallback --manifest-path src/cegar-fix/Cargo.toml --release`
Expected: PASS (solving graph76 in <1s and verifying 471 vertices).

- [ ] **Step 5: Commit Task 2**
```bash
git add src/cegar-fix/src/fallback src/cegar-fix/src/lib.rs
git commit -m "refactor(fallback): deduplicate degree-2 contraction and 2-opt cycle merger"
```

---

### Task 3: Macro-Decomposition Domain Restructuring

**Files:**
- Create: `src/cegar-fix/src/macro_decomp/mod.rs`
- Create: `src/cegar-fix/src/macro_decomp/bipartite.rs` (port from `macro_bipartite.rs`)
- Create: `src/cegar-fix/src/macro_decomp/corridor.rs` (port from `macro_corridor.rs`, replacing inline contractor with `fallback::contraction::Degree2Contractor` and `fallback::cycle_merge::safe_2opt_merge`)
- Create: `src/cegar-fix/src/macro_decomp/portfolio_788.rs` (port from `macro_788.rs`)
- Modify: `src/cegar-fix/src/lib.rs`
- Test: `src/cegar-fix/tests/test_macro_challenge_graphs.rs`

**Interfaces:**
- Consumes:
  - `core::graph::Graph`
  - `fallback::contraction::Degree2Contractor`
  - `fallback::cycle_merge::safe_2opt_merge`
- Produces:
  - `macro_decomp::bipartite::solve_746(g: &Graph, timeout_secs: f64) -> Option<Vec<i32>>`
  - `macro_decomp::corridor::solve_710(g: &Graph, timeout_secs: f64) -> Option<Vec<i32>>`
  - `macro_decomp::portfolio_788::solve_788(g: &Graph, timeout_secs: f64) -> Option<Vec<i32>>`

- [ ] **Step 1: Port `bipartite.rs` and `portfolio_788.rs` under `macro_decomp/`**
Move logic from `macro_bipartite.rs` and `macro_788.rs` into `macro_decomp/bipartite.rs` and `macro_decomp/portfolio_788.rs`.

- [ ] **Step 2: Refactor `corridor.rs` to use canonical `fallback` modules**
Replace the inline degree-2 contractor and inline 2-opt merge in `corridor.rs` with calls to `Degree2Contractor::contract_with_protected` and `safe_2opt_merge`. Verify that port vertices (1876, 2491) remain uncontracted.

- [ ] **Step 3: Wire `macro_decomp/mod.rs` and update `lib.rs` re-exports**
Re-export modules in `lib.rs` under `macro_bipartite`, `macro_corridor`, and `macro_788` for test compatibility.

- [ ] **Step 4: Run challenge macro integration test**
Run: `cargo test --test test_macro_challenge_graphs --manifest-path src/cegar-fix/Cargo.toml --release`
Expected: PASS (both graph710 and graph746 solved and verified).

- [ ] **Step 5: Commit Task 3**
```bash
git add src/cegar-fix/src/macro_decomp src/cegar-fix/src/lib.rs
git commit -m "refactor(macro_decomp): modularize challenge solvers and link canonical contraction"
```

---

### Task 4: Pipeline, CLI & Engine Organization

**Files:**
- Create: `src/cegar-fix/src/pipeline/mod.rs`
- Create: `src/cegar-fix/src/pipeline/options.rs`
- Create: `src/cegar-fix/src/pipeline/solver_pipeline.rs`
- Create: `src/cegar-fix/src/engine/mod.rs`
- Modify: `src/cegar-fix/src/main.rs`
- Modify: `src/cegar-fix/src/lib.rs`
- Test: `src/cegar-fix/tests/test_unified_cli_batch.rs`

**Interfaces:**
- Consumes: `core`, `fallback`, `macro_decomp`, `engine`
- Produces:
  - `pipeline::options::Options`
  - `pipeline::solver_pipeline::solve_single_graph`
  - `pipeline::solver_pipeline::run_batch`

- [ ] **Step 1: Move options and solver_pipeline into `pipeline/`**
Organize `pipeline/mod.rs`, `pipeline/options.rs`, and `pipeline/solver_pipeline.rs`. Update internal imports to use `crate::core`, `crate::fallback`, `crate::macro_decomp`, `crate::engine`.

- [ ] **Step 2: Organize `engine/mod.rs`**
Group `hybrid_orchestrator.rs`, `hcp_solver.rs`, and core heuristics under `engine/mod.rs`.

- [ ] **Step 3: Simplify `main.rs`**
Streamline `main.rs` to < 80 lines: parse options, dispatch to `pipeline::solver_pipeline::run_batch` if batch mode, or `pipeline::solver_pipeline::solve_single_graph` if single graph mode.

- [ ] **Step 4: Verify unified CLI test**
Run: `cargo test --test test_unified_cli_batch --manifest-path src/cegar-fix/Cargo.toml --release`
Expected: PASS.

- [ ] **Step 5: Commit Task 4**
```bash
git add src/cegar-fix/src/pipeline src/cegar-fix/src/engine src/cegar-fix/src/main.rs src/cegar-fix/src/lib.rs
git commit -m "refactor(pipeline): organize pipeline, options, engine, and lean main entry point"
```

---

### Task 5: Warning Elimination & Full Test Suite Pass

**Files:**
- Modify: `src/cegar-fix/src/**/*.rs`
- Clean up unused legacy flat files in `src/cegar-fix/src/` that have been migrated to subdirectories.
- Test: All test suites and Suite A benchmark.

- [ ] **Step 1: Remove redundant migrated legacy flat files**
Safely remove the redundant top-level flat files (`contraction.rs`, `fallback_cegar.rs`, `macro_bipartite.rs`, `macro_corridor.rs`, `macro_788.rs`, `options.rs`, `solver_pipeline.rs`, `graph.rs`, `encoder.rs`, `file_operations.rs`, `tour_verifier.rs`) now that they cleanly reside in submodules.

- [ ] **Step 2: Fix all compiler warnings**
Run `RUSTFLAGS="-D warnings" cargo check --manifest-path src/cegar-fix/Cargo.toml`
Fix any remaining `unused_variables`, `unused_mut`, `unused_imports`, or `dead_code` warnings.

- [ ] **Step 3: Run all cargo tests**
Run: `cargo test --manifest-path src/cegar-fix/Cargo.toml --release`
Expected: All tests pass with 0 errors and 0 warnings.

- [ ] **Step 4: Run Suite A batch benchmark and cross-verify with upstream verifier**
Run: `./run_solver_rust.sh --batch 1 50 --workers 2 --timeout 10`
Verify: 50/50 solved (100.0%), 0 errors, 0 timeouts.
Cross-verify generated tour files against `python3 /home/ubuntu/SAT-based-CEGAR/parse/is_hamiltonian.py`.

- [ ] **Step 5: Commit Task 5**
```bash
git add -A src/cegar-fix/
git commit -m "refactor(cleanup): eliminate all compiler warnings and remove redundant legacy flat files"
```
