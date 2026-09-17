# Implementation Plan: Exact CEGAR Solver Stall Elimination for Flinders Class 1 (graph868.col)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate premature round timeouts and cold-restart churn in the exact deterministic CEGAR solver, enabling sustained deep search on `FHCPCS-col/graph868.col` to find and certify a 100% exact Hamiltonian tour within 1,800s on Core 0.

**Architecture:** 
1. Maintain true incremental SAT solving across rounds (`solver.add_cnf(cuts)`) rather than flushing and cold-restarting whenever a round exceeds 15.0s.
2. Extend the per-round stall terminator from 60.0s to 240.0s so CaDiCaL can naturally complete deep rounds.
3. Lean gadget cut policy: add boundary cuts ($\delta^+, \delta^-$) exclusively to elementary Flinders gadgets ($|c| \le 8$) in intermediate rounds, preventing exponential clause bloat while strictly forbidding isolated gadgets.
4. Pin strictly to Core 0 (`taskset -c 0 nice -n 19`), single worker (`num_workers = 1`), deterministic seed 777.

**Tech Stack:** Rust (rustsat, rustsat-cadical 0.1.2), CaDiCaL (`chrono=1`, `phase=1`), bash (`taskset`, `nice`).

## Global Constraints
- Primary target: `FHCPCS-col/graph868.col` ($N = 5,544, M = 9,072, N_{\text{dir}} = 1,848$).
- Single worker pinned to **Core 0** (`taskset -c 0 nice -n 19`).
- Exact time limit: **1,800.0s**.
- 100% exact combinatorial method: zero heuristic LKH/Concorde, zero tour injection, zero reading `.tou` files.
- Deterministic execution (`seed = 777`).
- Certified by independent `TourVerifier` (`scratch/test_stage4_verification.py`).

---

## 1. Root Cause Diagnosis of `task-23080` Stall

In `task-23080`, the solver successfully shrank cycle count from 76 to 29 in 7 rounds (71 seconds total):
- Round 1 (0.17s): SAT=76
- Round 2 (0.20s): SAT=63
- Round 3 (0.65s): SAT=55
- Round 4 (2.08s): SAT=49
- Round 5 (7.32s): SAT=52
- Round 6 (20.5s): SAT=36 (largest = 1,316)
- Round 7 (39.8s): SAT=29 (largest = 1,276)

At Round 8, CaDiCaL was aborted at 59.98s and restarted cold 28 times in a row until the 1,800s limit expired.
Two flaws caused this:
1. **The 60.0s Hard Terminator**: Round 8 with 1,093 cuts naturally required ~80–120s. Aborting at 59.98s killed CaDiCaL just before completion.
2. **Cold-Restart Loop (`round_secs > 15.0`)**: Once any round exceeded 15.0s, every subsequent round flushed the solver, losing all learned conflict clauses, activity scores, and watched-literal state. Starting cold with 1,093 cuts guaranteed that every subsequent round would exceed 60s, triggering an infinite abort loop.

---

## 2. Tasks

### Task 1: Update `src/cegar-fix/examples/solve_class1_exact.rs`
**Files:**
- Modify: `src/cegar-fix/examples/solve_class1_exact.rs`

- [ ] **Step 1: Increase round timeout to 240.0s**
  - Set `round_max_secs = 240.0` in `make_solver` calls.
- [ ] **Step 2: Enable true incremental SAT solving**
  - Remove `round_secs > 15.0` flush condition.
  - Retain `solver.add_cnf(cuts)` across rounds so CaDiCaL preserves learned conflict clauses.
  - Reseed/flush only if a round hits the 240.0s terminator (genuine stall recovery) or every 25 rounds.
- [ ] **Step 3: Refine cut generation for elementary gadgets**
  - In intermediate rounds, add dual boundary cuts ($\delta^+, \delta^-$) only for $|c| \le 8$ (elementary Flinders gadgets).
  - For $|c| > 8$, add only pure negative cuts ($\bigvee_{e \in c} \neg x_e$).
  - For checkpoint records, retain dual boundary cuts for all $|c| \le 16$.

### Task 2: Build Release Binary
- [ ] **Step 1: Compile `solve_class1_exact` in release mode**
  - Run `cargo build --release --manifest-path src/cegar-fix/Cargo.toml --example solve_class1_exact`.
  - Ensure zero compilation errors or warnings.

### Task 3: Execute Official 1,800s Benchmark on Core 0
- [ ] **Step 1: Launch solver on Core 0 with 1,800.0s timeout**
  - Run: `taskset -c 0 nice -n 19 ./src/cegar-fix/target/release/examples/solve_class1_exact FHCPCS-col/graph868.col scratch/graph868/found_tour_graph868.hcp 1800.0`.
  - Monitor logs for cycle count reduction below 29 and tour discovery.

### Task 4: Independent Tour Verification
- [ ] **Step 1: Certify found tour with independent Python verifier**
  - Run: `python3 scratch/test_stage4_verification.py FHCPCS-col/graph868.col scratch/graph868/found_tour_graph868.hcp`.
  - Confirm: 5,544 unique vertices, valid simple cycle, 100% edges present in raw graph.
