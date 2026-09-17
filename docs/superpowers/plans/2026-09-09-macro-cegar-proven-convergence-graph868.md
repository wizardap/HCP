# Implementation Plan: Proven Macro-CEGAR Convergence Architecture for Flinders Class 1 (graph868.col)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Apply the exact macro-CEGAR convergence architecture that solved `FHCPCS-col/graph788.col` to `FHCPCS-col/graph868.col`, driving 2-factor cycle count from 29 down to 1 single Hamiltonian tour within 1,800s on Core 0.

**Architecture:** 
1. **Cut Every Cycle (Including Giant)**: In every round, inject pure negative subtour cuts for ALL cycles $c$ with $|c| < N_{\text{dir}}$, including the giant. This forces the solver to reconfigure the giant backbone to absorb small gadgets instead of leaving the giant frozen.
2. **Dual Boundary Cuts for Gadgets**: Enforce $\delta^+(c) \ge 1$ and $\delta^-(c) \ge 1$ for all small cycles ($|c| \le 16$), guaranteeing that isolated gadgets must connect to the outside.
3. **Dynamic All-Cycle Phase Hints**: At the end of every round, update `current_hints` to all 1,848 active edges of the current 2-factor solution. This ensures CaDiCaL always explores neighboring configurations close to the current best 2-factor.
4. **Dynamic Reseeder & Lemma Housekeeping**: If a round takes $> 15.0$s or every 10 rounds, flush bloated lemmas and re-instantiate CaDiCaL with accumulated clean cuts and latest phase hints.
5. **Generous Round Deadline (240.0s)**: Prevent premature aborts during deep search.
6. **Execution Constraints**: Single worker pinned to **Core 0** (`taskset -c 0 nice -n 19`), deterministic seed 777, timeout 1,800s.

**Tech Stack:** Rust (rustsat, rustsat-cadical 0.1.2), CaDiCaL (`chrono=1`, `phase=1`, `restartint=100`), bash (`taskset`, `nice`).

## Global Constraints
- Primary target: `FHCPCS-col/graph868.col` ($N = 5,544, M = 9,072, N_{\text{dir}} = 1,848$).
- Single worker pinned strictly to **Core 0** (`taskset -c 0 nice -n 19`).
- Exact time limit: **1,800.0s**.
- 100% exact combinatorial method: zero heuristic LKH/Concorde, zero tour injection, zero reading `.tou` files.
- Deterministic execution (`seed = 777`).
- Certified by independent `TourVerifier` (`scratch/test_stage4_verification.py`).

---

## 1. Proven Architecture from `graph788` Success

In commit `fbc9f96`, `solve_macro_cegar_788.rs` solved `graph788` ($N=4,620$) in 23 rounds (1,120s):
- Round 1: 63 cycles
- Round 6: 28 cycles (Giant = 1,008)
- Round 13: 8 cycles (Giant = 550)
- Round 14: 4 cycles
- Round 21: 2 cycles ([770, 770])
- Round 23: **1 single Hamiltonian cycle (1,540 blocks, 4,620 vertices)** in 555ms!

Analysis of the log identified the 4 crucial mechanisms that enabled convergence:
1. **Cutting the Giant**: By adding $\bigvee_{e \in c} \neg x_e$ for ALL cycles $< N_{\text{dir}}$ (including the giant), the giant cannot remain frozen. CaDiCaL is forced to rewire the giant's ports.
2. **Phasing All Active Edges**: Setting `phase_lit` on all 1,848 edges of the current 2-factor preserves 99% of the tour structure while letting CaDiCaL swap just the few edges needed to satisfy the cuts.
3. **Reseeding upon Round > 15s**: When a round takes > 15s, clearing bloated lemmas and restarting with clean accumulated cuts and updated hints lets CaDiCaL solve the next round in < 4s (as seen in Round 14 and 16 on graph788).
4. **Chrono Backtracking (`chrono=1`)**: Preserves assignments near phase hints.

---

## 2. Tasks

### Task 1: Update `src/cegar-fix/examples/solve_class1_exact.rs`
**Files:**
- Modify: `src/cegar-fix/examples/solve_class1_exact.rs`

- [ ] **Step 1: Set CaDiCaL options**
  - `solver.set_option("chrono", 1)`
  - `solver.set_option("phase", 1)`
  - `solver.set_option("restartint", 100)`
- [ ] **Step 2: Add negative cuts for ALL cycles $< N_{\text{dir}}$ (including giant)**
  - For every $c \in \text{sat\_cycles}$:
    - If $|c| < N_{\text{dir}}$: add pure negative cut $\bigvee_{e \in c} \neg x_e$.
    - If $|c| \le 16$: add dual boundary cuts $\delta^+(c) \ge 1, \delta^-(c) \ge 1$.
- [ ] **Step 3: Update phase hints to ALL cycles every round**
  - Update `current_hints` to include all edges of `effective_cycles` in every round.
- [ ] **Step 4: Align reseeder policy with `solve_macro_cegar_788.rs`**
  - If round took $> 15.0$s or `round % 10 == 0 && round > 0`: reseed solver with clean `accumulated_cuts` and updated `current_hints`.
  - Otherwise: `solver.add_cnf(cuts)` incrementally.
  - Per-round terminator deadline: 240.0s.

### Task 2: Build Release Binary
- [ ] **Step 1: Compile `solve_class1_exact` in release mode**
  - Run `cargo build --release --manifest-path src/cegar-fix/Cargo.toml --example solve_class1_exact`.
  - Ensure zero compilation errors or warnings.

### Task 3: Execute Official 1,800s Benchmark on Core 0
- [ ] **Step 1: Launch solver on Core 0 with 1,800.0s timeout**
  - Run: `taskset -c 0 nice -n 19 ./src/cegar-fix/target/release/examples/solve_class1_exact FHCPCS-col/graph868.col scratch/graph868/found_tour_graph868.hcp 1800.0`.

### Task 4: Independent Tour Verification
- [ ] **Step 1: Certify found tour with independent Python verifier**
  - Run: `python3 scratch/test_stage4_verification.py FHCPCS-col/graph868.col scratch/graph868/found_tour_graph868.hcp`.
  - Confirm: 5,544 unique vertices, valid simple cycle, 100% edges present in raw graph.
