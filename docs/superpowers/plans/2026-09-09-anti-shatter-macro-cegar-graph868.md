# Anti-Shatter Macro-CEGAR Implementation Plan for Graph868

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement, execute, and independently certify a 100% exact, deterministic combinatorial solver for Flinders Class 1 HCP instances (`FHCPCS-col/graph868.col`), strictly within 1,800.0s on Core 0.

**Architecture:** A single-worker deterministic Macro-CEGAR solver in Rust using incremental CaDiCaL. It loads the best certified 2-factor checkpoint, enforces monotonic Giant preservation through anti-shatter rejection cuts that immediately forbid any fragmented cycles if CaDiCaL attempts to break the Giant, integrates 2-opt and 3-opt cycle splicing, and uses 2ms incremental assumption-guided core relaxation to absorb remaining 8-block gadgets without Giant disruption.

**Tech Stack:** Rust (edition 2021), `rustsat 0.6.6`, `rustsat-cadical 0.4.6` (`SolveIncremental`), `TourVerifier` (TSPLIB HCP format).

## Global Constraints
- Target: Flinders Class 1 HCP benchmark `FHCPCS-col/graph868.col` ($N = 5,544, M = 9,072, N_{dir} = 1,848$ contracted blocks).
- Output: TSPLIB certified tour at `scratch/graph868/found_tour_graph868.hcp`.
- Verification: 100% independent certificate verification via `TourVerifier` (`python3 scratch/test_stage4_verification.py FHCPCS-col/graph868.col scratch/graph868/found_tour_graph868.hcp`).
- Resource limit: strictly $\le 1,800.0$ seconds wall-clock time pinned strictly to Core 0 (`taskset -c 0 nice -n 19`), single worker (`num_workers = 1`).
- Determinism: `seed = 777`. Zero heuristics (no LKH/Concorde), zero reading `.tou` files, zero tour injection.

---

### Task 1: Checkpoint Evaluation & Monotonic Anti-Shatter Filter

**Files:**
- Modify: `src/cegar-fix/examples/solve_class1_exact.rs:340-445`
- Modify: `src/cegar-fix/examples/solve_class1_exact.rs:560-610`

**Interfaces:**
- Consumes: `possible_paths` array containing candidate checkpoint files.
- Produces: `best_giant_len: usize`, `best_cycle_count: usize`, `best_hints: Vec<Lit>`, and monotonic acceptance logic.

- [ ] **Step 1: Write candidate comparison logic**
In `solve_class1_exact.rs`, scan all paths in `possible_paths`, parse each candidate 2-factor using directed block coloring and partner mapping, and select the checkpoint maximizing `giant.len()` (or minimizing cycle count).

- [ ] **Step 2: Add Monotonic Acceptance Filter**
When CaDiCaL solves a round, evaluate the resulting 2-factor:
```rust
let is_better = effective_cycles[0].len() > best_giant_len
    || (effective_cycles[0].len() == best_giant_len && effective_cycles.len() < best_cycle_count);
```
If `is_better`: update `best_giant_len`, `best_cycle_count`, `best_hints`, and persist `scratch/best_edges_graph868.txt`.
If not: flag the solution as a regression to trigger anti-shatter rejection cuts.

- [ ] **Step 3: Compile and verify checkpoint selection**
Run: `cargo run --release --manifest-path src/cegar-fix/Cargo.toml --example solve_class1_exact -- FHCPCS-col/graph868.col scratch/test.hcp 0.001`
Expected: Selects candidate with Giant $\ge 1,608$ (or $1,686$).

---

### Task 2: Anti-Shatter Rejection Cuts on Fragmented Cycles

**Files:**
- Modify: `src/cegar-fix/examples/solve_class1_exact.rs:650-715`

**Interfaces:**
- Consumes: Regressed `effective_cycles` where `effective_cycles[0].len() < best_giant_len`.
- Produces: Negative subtour cuts on all newly created non-giant fragments, injected directly into `solver.add_cnf(cuts)`.

- [ ] **Step 1: Implement rejection cut generation**
When a round produces a regressed Giant ($|G_{new}| < |G_{best}|$), for every non-giant cycle $C \in \text{effective\_cycles}[1..]$:
Generate a negative subtour cut:
$$\bigvee_{e \in C} \neg e$$
Inject these cuts into both `accumulated_cuts` and the live `solver`.

- [ ] **Step 2: Reinforce Phase Hints on Best Trajectory**
When a regression occurs, re-apply `best_hints` via `solver.phase_lit(lit)` to force CaDiCaL's default decision variable polarities back toward the un-shattered Giant.

- [ ] **Step 3: Test anti-shatter behavior**
Run: `cargo run --release --manifest-path src/cegar-fix/Cargo.toml --example solve_class1_exact -- FHCPCS-col/graph868.col scratch/test.hcp 5.0`
Expected: Giant never decreases across accepted rounds; regressed rounds generate immediate rejection cuts.

---

### Task 3: Incremental Assumption Core Relaxation for Target Gadgets

**Files:**
- Modify: `src/cegar-fix/examples/solve_class1_exact.rs:600-670`

**Interfaces:**
- Consumes: Target 8-block cycle $C_{target}$ from `effective_cycles.last()`, `best_hints`, and `SolveIncremental::solve_assumps`.
- Produces: Fast (< 5ms) bounded-neighborhood absorption of $C_{target}$ into the Giant.

- [ ] **Step 1: Construct Giant Frozen Assumptions**
Identify Giant edges not adjacent to $C_{target}$. Form `frozen_assumps = { arc_lit(u, v) : (u, v) \in Giant, \text{not touching } C_{target} }`.

- [ ] **Step 2: Implement Iterative Assumption Core Relaxation Loop**
In each round, prior to unconstrained solving, query `solver.solve_assumps(&assumps_vec)`:
- If `Ok(SolverResult::Sat)`: Extract solution, test for Giant growth; if improved, adopt immediately.
- If `Ok(SolverResult::Unsat)`: Query `solver.core().unwrap()`, remove conflicting literals from `assumps_vec`, and repeat for up to 10 micro-iterations.
- If assumption budget exhausted: fallback to unconstrained CEGAR with anti-shatter rejection cuts.

- [ ] **Step 3: Verify core relaxation speed**
Run: `cargo test` or test run on `graph868.col`.
Expected: Assumption micro-iterations execute in 1.0 - 2.5ms per iteration.

---

### Task 4: Release Build and Official 1,800.0s Execution on Core 0

**Files:**
- Output: `scratch/graph868/found_tour_graph868.hcp`
- Verifier: `scratch/test_stage4_verification.py`

**Interfaces:**
- Consumes: `FHCPCS-col/graph868.col`, timeout = 1800.0s, CPU core = 0.
- Produces: Verified Hamiltonian tour file.

- [ ] **Step 1: Clean build release binary**
Run: `cargo build --release --manifest-path src/cegar-fix/Cargo.toml --example solve_class1_exact`
Expected: Exit code 0, binary at `src/cegar-fix/target/release/examples/solve_class1_exact`.

- [ ] **Step 2: Launch official 1,800.0s run on Core 0**
Command:
```bash
taskset -c 0 nice -n 19 ./src/cegar-fix/target/release/examples/solve_class1_exact FHCPCS-col/graph868.col scratch/graph868/found_tour_graph868.hcp 1800.0
```
Monitor log and CPU usage to confirm strict Core 0 single-worker binding.

- [ ] **Step 3: Independent Certificate Verification**
Run: `python3 scratch/test_stage4_verification.py FHCPCS-col/graph868.col scratch/graph868/found_tour_graph868.hcp`
Expected:
`SUCCESS: Valid Hamiltonian Tour!`
`Unique vertices: 5544 / 5544`
`Simple cycle: TRUE`
`All edges valid in original graph: TRUE`
