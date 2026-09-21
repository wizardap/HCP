# Pure Rust Unified Solver Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Unify the entire HCP solver into a single, standalone 100% Rust binary (`target/release/cegar-fix` or `hcp-solver`) with zero Python dependency, zero cache reading (100% de novo), built-in batch runner, Stage 3 fallback (triangle-pruned degree-2 contraction), and macro-decomposition for challenge graphs.

**Architecture:**
- **Single Native Binary**: Compiled via Cargo with all dependencies (`rustsat`, `rustsat-cadical`, `clap`, `rayon`).
- **Pipeline Stages**:
  - Stage 1: Macro-Decomposition for massive challenge instances (|V| >= 4000).
  - Stage 2: Fast Core SAT-CEGAR engine (Sinz encoding + CaDiCaL + DFJ subcycle cuts + port alternating).
  - Stage 3: Automated Degree-2 Chain Contraction + in-memory Rust CaDiCaL CEGAR fallback (with chordless triangle pruning and forced shortcut unit clauses).
  - Stage 4: Built-in `TourVerifier` providing certified 100% sound TSPLIB `.hcp` tours.
- **CLI & Batch Engine**:
  - Single graph mode: `cegar-fix -i <graph.col> -o <tour.hcp> --timeout 10`
  - Batch runner mode: `cegar-fix --batch 1 1001 --workers 2 --timeout 10`

---

### Task 1: Stage 3 Fallback in Rust (Contraction + Forced Shortcuts + Safe 2-Opt)

**Files:**
- Modify: `src/cegar-fix/src/contraction.rs`
- Create: `src/cegar-fix/src/fallback_cegar.rs`
- Modify: `src/cegar-fix/src/lib.rs`
- Test: `src/cegar-fix/tests/test_stage3_fallback.rs`

- [ ] **Step 1: Write integration test for Stage 3 fallback on graph76**
  - Create `src/cegar-fix/tests/test_stage3_fallback.rs`.
  - Load `FHCPCS-col/graph76.col`.
  - Invoke `fallback_cegar::solve_with_contraction(&graph, timeout)`.
  - Verify that produced tour has length 471 and is 100% sound.

- [ ] **Step 2: Update `contraction.rs` with chordless triangle pruning**
  - In `Degree2Contractor::contract`:
  - When degree-2 vertex $v$ with neighbors $u, w$ has direct edge $(u, w)$ in graph with $|V| > 3$:
  - Prune the direct edge $(u, w)$ before introducing shortcut $(u, w)$.
  - Record shortcut edges as `forced_edges`.

- [ ] **Step 3: Implement `fallback_cegar.rs` in Rust**
  - Contract degree-2 chains.
  - Setup 2-degree cardinality constraints for each remaining vertex using `rustsat` or basic pairwise/sequential counters.
  - Force all shortcut edges as unit clauses in CaDiCaL (`solver.add_clause(&[shortcut_lit])`).
  - Implement CEGAR loop with DFJ subcycle cuts.
  - Implement 2-opt cycle merger that refuses to delete forced shortcut edges.
  - Expand contracted cycle back to full vertices.

- [ ] **Step 4: Run tests and verify Task 1 passes**
  - Run: `cargo test --test test_stage3_fallback --release`.
  - Ensure graph 76 solves with 100% sound verification.

- [ ] **Step 5: Commit Task 1**
  - `git commit -m "feat(rust): implement Stage 3 fallback with triangle pruning and forced shortcuts"`

---

### Task 2: Unified CLI & Batch Runner in Rust

**Files:**
- Modify: `src/cegar-fix/Cargo.toml` (add `rayon`, `serde`, `serde_json` if needed)
- Modify: `src/cegar-fix/src/main.rs`
- Modify: `src/cegar-fix/src/options.rs`

- [ ] **Step 1: Add dependencies to `src/cegar-fix/Cargo.toml`**
  - Add `serde = { version = "1.0", features = ["derive"] }`
  - Add `serde_json = "1.0"`
  - Add `rayon = "1.8"`

- [ ] **Step 2: Extend CLI options in `main.rs` and `options.rs`**
  - Add `-o, --output <PATH>` to write certified tour.
  - Add `--batch <START> <END>` to run batch benchmark.
  - Add `--workers <N>` for thread pool concurrency.
  - Add `--checkpoint <PATH>` for saving JSON progress (default: `scratch/batch_1001_results.json`).

- [ ] **Step 3: Implement batch runner execution loop in Rust**
  - Parallel evaluation over range `[start, end]` using thread pool.
  - Capture per-graph status (`SAT_VERIFIED`, `TIMEOUT`, `ERROR`), runtime, and vertex count.
  - Atomic checkpoint flushing every 10 graphs to JSON.

- [ ] **Step 4: Test single-file and batch execution**
  - Run: `cargo run --release -- -i FHCPCS-col/graph1.col -o output_tours/tour_graph1.hcp`
  - Run: `cargo run --release -- --batch 1 5 --workers 2`
  - Verify output tour and JSON checkpoint.

- [ ] **Step 5: Commit Task 2**
  - `git commit -m "feat(rust): add standalone CLI, output tour export, and batch runner"`

---

### Task 3: Macro-Decomposition for Challenge Graphs in Rust

**Files:**
- Create: `src/cegar-fix/src/macro_bipartite.rs`
- Create: `src/cegar-fix/src/macro_corridor.rs`
- Modify: `src/cegar-fix/src/main.rs`

- [ ] **Step 1: Implement `macro_bipartite.rs` in Rust**
  - Super-hub detection & Voronoi BFS clustering (porting logic from `dense_bipartite.py`).
  - Parallel solving of independent cluster paths using Rayon / CaDiCaL.
  - Splicing of solved clusters into full Hamiltonian tour.

- [ ] **Step 2: Implement `macro_corridor.rs` in Rust**
  - 2-cut articulation vertex identification.
  - Block A / Block B corridor splitting.
  - Linking Block A path and Block B path.

- [ ] **Step 3: Integrate 788 solver into main binary**
  - Connect `solve_macro_cegar_788` logic as standard handler when $|V| = 4620$.

- [ ] **Step 4: Test challenge graphs in Rust**
  - Test `graph746.col`: `cargo run --release -- -i FHCPCS-col/graph746.col`
  - Test `graph710.col`: `cargo run --release -- -i FHCPCS-col/graph710.col`
  - Confirm 100% SOUND verification.

- [ ] **Step 5: Commit Task 3**
  - `git commit -m "feat(rust): implement challenge macro-decomposition in pure Rust"`

---

### Task 4: End-to-End Verification & Python-Free Tooling

**Files:**
- Create: `run_solver_rust.sh`
- Update: `README.md`

- [ ] **Step 1: Verify Suite A (`graph1` .. `graph50`) with Rust standalone binary**
  - Run: `./src/cegar-fix/target/release/cegar-fix --batch 1 50 --workers 2 --timeout 10`
  - Confirm 50/50 solved (100.0%) and zero errors.

- [ ] **Step 2: Add wrapper script `run_solver_rust.sh`**
  - Simple bash entry point calling `./src/cegar-fix/target/release/cegar-fix`.

- [ ] **Step 3: Final verification and commit**
  - `git commit -m "docs: document pure Rust standalone solver and usage"`
