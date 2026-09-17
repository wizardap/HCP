# Implementation Plan: Lean Subtour Exact Macro-CEGAR for Flinders Class 1 (graph868.col)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate redundant boundary clause bloat and restart thrashing, enabling the solver to execute hundreds of rapid incremental rounds ($< 2$s per round) starting from the 28-cycle world record to find and certify a 100% exact Hamiltonian tour on `FHCPCS-col/graph868.col` within 1,800s on Core 0.

**Architecture:** 
1. **Mathematical Redundancy of Boundary Cuts**:
   In bipartite 8-block gadgets, degree-1 constraints + static 2-, 3-, 4-cycle cuts logically entail that no internal 2-factor can exist other than the 8-cycle itself. Therefore, a pure negative subtour cut ($\bigvee_{e \in c} \neg x_e$) *mathematically guarantees* external connectivity. Adding dual boundary cuts adds 2 massive clauses (30+ literals each) per gadget, bloating the CNF by 30,000 literals and causing exponential BCP slowdown. By retaining exclusively pure negative cuts in intermediate rounds, clause addition drops from ~2,500 literals/round to ~200 literals/round ($> 10\times$ leaner).
2. **Remove `restartint = 100` Thrashing**:
   Setting `restartint = 100` forced CaDiCaL to restart every 100 conflicts ($> 100$ times per second), trapping the solver in shallow root decisions and preventing deep branch exploration. Restoring CaDiCaL's native adaptive restart heuristic allows both fast propagation and deep branch resolution.
3. **True Incremental CDCL**:
   Maintain incremental solving (`solver.phase_lit` + `solver.add_cnf(cuts)`) without premature flushes. Refresh solver only every 25 rounds or upon a genuine 300s stall.
4. **All-Cycle Dynamic Phase Guidance**:
   Update `current_hints` to all 1,848 edges of the current 2-factor every round, keeping CaDiCaL searching in the immediate neighborhood of the latest best solution.
5. **Execution Constraints**:
   Single worker pinned strictly to **Core 0** (`taskset -c 0 nice -n 19`), deterministic seed 777, global timeout 1,800.0s.

**Tech Stack:** Rust (rustsat, rustsat-cadical 0.1.2), CaDiCaL (`chrono=1`, `phase=1`, adaptive restarts), bash (`taskset`, `nice`).

## Global Constraints
- Primary target: `FHCPCS-col/graph868.col` ($N = 5,544, M = 9,072, N_{\text{dir}} = 1,848$).
- Single worker pinned to **Core 0** (`taskset -c 0 nice -n 19`).
- Exact time limit: **1,800.0s**.
- 100% exact combinatorial method: zero heuristic LKH/Concorde, zero tour injection, zero reading `.tou` files.
- Deterministic execution (`seed = 777`).
- Certified by independent `TourVerifier` (`scratch/test_stage4_verification.py`).

---

## 1. Tasks

### Task 1: Update `src/cegar-fix/examples/solve_class1_exact.rs`
**Files:**
- Modify: `src/cegar-fix/examples/solve_class1_exact.rs`

- [ ] **Step 1: Remove `restartint = 100` from `make_solver`**
  - Use native CaDiCaL adaptive restarts with `chrono = 1` and `phase = 1`.
- [ ] **Step 2: Streamline cut generation to pure negative subtour cuts**
  - For each $c \in \text{sat\_cycles}$ with $|c| < N_{\text{dir}}$ (including giant): add pure negative clause $\bigvee_{e \in c} \neg x_e$.
  - Remove redundant dual boundary cuts from intermediate rounds.
- [ ] **Step 3: Keep true incremental solving**
  - Dynamic phase hints on all active edges.
  - Refresh solver only every 25 rounds or on genuine stall (> 300s).

### Task 2: Build Release Binary
- [ ] **Step 1: Compile `solve_class1_exact` in release mode**
  - Run `cargo build --release --manifest-path src/cegar-fix/Cargo.toml --example solve_class1_exact`.
  - Ensure zero compilation errors or warnings.

### Task 3: Execute Official 1,800s Benchmark on Core 0
- [ ] **Step 1: Launch solver on Core 0 with 1,800.0s timeout**
  - Run: `taskset -c 0 nice -n 19 ./src/cegar-fix/target/release/examples/solve_class1_exact FHCPCS-col/graph868.col scratch/graph868/found_tour_graph868.hcp 1800.0`.
  - Monitor logs for cycle count reduction below 28 down to 1.

### Task 4: Independent Tour Verification
- [ ] **Step 1: Certify found tour with independent Python verifier**
  - Run: `python3 scratch/test_stage4_verification.py FHCPCS-col/graph868.col scratch/graph868/found_tour_graph868.hcp`.
  - Confirm: 5,544 unique vertices, valid simple cycle, 100% edges present in raw graph.
