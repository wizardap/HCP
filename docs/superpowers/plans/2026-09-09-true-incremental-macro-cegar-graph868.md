# Implementation Plan: True Incremental Macro-CEGAR for Flinders Class 1 (graph868.col)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Drive the cycle count from the current record of 28 cycles down to 1 single certified Hamiltonian tour on `FHCPCS-col/graph868.col` within 1,800s on Core 0 by maintaining true incremental CDCL solving across rounds and eliminating cold-restart churn.

**Architecture:** 
1. **Start from Record 28 Cycles**: Initialize solver from `scratch/best_edges_graph868.txt` (28 cycles, Giant = 1,327, 25 8-block gadgets).
2. **True Incremental Solving**: Eliminate the eager `round_secs > 15.0` flush condition that repeatedly forced cold starts. Keep CaDiCaL alive across rounds (`solver.add_cnf(cuts)`), preserving trained VSIDS variable activities, saved phases, and learned conflict lemmas.
3. **Dynamic All-Cycle Phase Hints**: At the end of every round, update phase hints for all 1,848 active edges of the current 2-factor, guiding CaDiCaL to search in the immediate neighborhood of the latest solution.
4. **Subtour Cuts for All Cycles (Including Giant)**: In every round, add negative cuts for ALL cycles $< N_{\text{dir}}$ and dual boundary cuts for all small cycles $\le 16$.
5. **Generous Stall-Only Timeout (300.0s)**: Per-round terminator set to 300.0s, ensuring deep rounds are never killed prematurely while still recovering from true stalls.
6. **Execution Constraints**: Single worker strictly pinned to **Core 0** (`taskset -c 0 nice -n 19`), deterministic seed 777, global timeout 1,800.0s.

**Tech Stack:** Rust (rustsat, rustsat-cadical 0.1.2), CaDiCaL (`chrono=1`, `phase=1`, `restartint=100`), bash (`taskset`, `nice`).

## Global Constraints
- Primary target: `FHCPCS-col/graph868.col` ($N = 5,544, M = 9,072, N_{\text{dir}} = 1,848$).
- Single worker pinned to **Core 0** (`taskset -c 0 nice -n 19`).
- Exact time limit: **1,800.0s**.
- 100% exact combinatorial method: zero heuristic LKH/Concorde, zero tour injection, zero reading `.tou` files.
- Deterministic execution (`seed = 777`).
- Certified by independent `TourVerifier` (`scratch/test_stage4_verification.py`).

---

## 1. Root Cause Analysis of `task-23381`

In `task-23381`, the solver broke the world record down to **28 cycles** at Round 6 in just 11.65 seconds:
- Round 1: 27ms (SAT=72)
- Round 2: 47ms (SAT=47, Giant=1,067)
- Round 3: 2.74s (SAT=66)
- Round 4: 2.44s (SAT=40)
- Round 5: 1.65s (SAT=45)
- Round 6: 11.65s (**SAT=28, largest=1,327** - New Checkpoint!)
All 6 rounds took a combined 18.5 seconds because CaDiCaL was solving **incrementally** (`solver.add_cnf(cuts)`).

However, at Round 7, solve time was 25.5s, which triggered the condition `round_secs > 15.0`.
This destroyed CaDiCaL and triggered a **cold restart from scratch** with 1,012 cuts.
A cold restart with > 1,000 cuts on Core 0 takes 100s–250s because all learned conflict clauses and VSIDS activities are lost.
Every subsequent round then took > 15.0s, permanently locking CaDiCaL into a cold-restart loop where 5 seeds timed out right around the 240s mark (wasting 1,200 seconds of budget).

By maintaining **true incremental solving** and only flushing on genuine stalls (> 300s), CaDiCaL can execute 50–100 rapid rounds within 1,800s, driving the cycle count from 28 down to 1!

---

## 2. Tasks

### Task 1: Update `src/cegar-fix/examples/solve_class1_exact.rs`
**Files:**
- Modify: `src/cegar-fix/examples/solve_class1_exact.rs`

- [ ] **Step 1: Set round terminator timeout to 300.0s**
  - Update `round_max_secs = 300.0` in `make_solver` calls.
- [ ] **Step 2: Maintain true incremental solving**
  - Remove `round_secs > 15.0` flush condition.
  - Always call `solver.phase_lit(lit)` and `solver.add_cnf(cuts)` incrementally.
  - Only reseed if `round % 20 == 0 && round > 0` (periodic housekeeping) or on genuine stall (> 300s).
- [ ] **Step 3: Checkpoint and cut invariants**
  - Load `scratch/best_edges_graph868.txt` (28 cycles).
  - Pre-phase all 1,848 active edges.
  - Pre-inject cuts for all cycles $< N_{\text{dir}}$ (including giant).

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
