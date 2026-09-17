# Implementation Plan: Adaptive Batch Macro-CEGAR with Giant Preservation for Flinders Class 1 (graph868.col)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Achieve 100% exact convergence to a certified Hamiltonian tour on `FHCPCS-col/graph868.col` within 1,800s on Core 0 by preserving the Giant cycle, applying adaptive batch subtour cuts, anchoring phase preferences to the best checkpoint, and recovering from stalls within 30s.

---

## 1. Root Cause Diagnosis & Experimental Verification

Rigorous experimental profiling on Core 0 revealed two fundamental causes of solver stalls:
1. **The Giant Cutting Trap (Mode 1)**:
   Adding subtour cuts to the Giant cycle ($\bigvee_{e \in \text{Giant}} \neg x_e$) for giants of size 1,327, 1,492, 1,511 explicitly forbad the solver from keeping the giant backbone. In Round 7, the giant shattered to 616 blocks, and CaDiCaL became hopelessly over-constrained, causing 5 consecutive seeds to stall at 300s.
2. **The Simultaneous Multi-Gadget Rewiring Trap (Mode 2)**:
   When Giant reached 1,576 blocks (85.3%), there were 34 isolated 8-block gadgets. Adding subtour cuts for all 34 gadgets simultaneously demanded that CaDiCaL coordinate $4^{34} \approx 2.8 \times 10^{20}$ local configuration combinations at once, causing Round 7 to exceed 300s.
3. **The Breakthrough Discovery (Adaptive Batch Cuts)**:
   In `test_adaptive_batch.rs`:
   - Cutting a batch of 2-3 gadgets per round when Giant $\ge 1400$ executed **44 rounds in 105.1 seconds** (average 1.3s/round).
   - At Round 44, it shattered the world record to **Giant = 1,600 blocks (86.6%)** and **32 cycles** (`[1600, 8, 8, 8, ...]`).
   - All 31 remaining cycles were purely isolated 8-block gadgets ($1600 + 31 \times 8 = 1848$).

---

## 2. Architecture & Design Principles

1. **Giant Preservation (Never Cut the Giant)**:
   The Giant cycle $C_0$ is NEVER cut. Subtour elimination cuts are applied ONLY to small non-giant cycles $C_1, C_2, \dots, C_k$.
2. **Adaptive Batch Sizing**:
   - Giant $\ge 1400$: cut $\min(3, k)$ non-giant cycles per round.
   - Giant $\ge 1000$: cut $\min(6, k)$ non-giant cycles per round.
   - Giant $< 1000$: cut $\min(10, k)$ non-giant cycles per round.
   This guarantees CaDiCaL only solves for a small local perturbation in each round, keeping solve times $< 2$ seconds.
3. **Monotonic Best Phase Anchoring**:
   Maintain `best_hints` corresponding to the best solution ever found (Giant $\ge$ best_giant). Update `current_hints` only when a strictly superior 2-factor is found. If CaDiCaL wanders in an intermediate round, CaDiCaL's variable polarity remains anchored to the best Giant.
4. **Agile 30s Stall Recovery**:
   If any round exceeds 30.0s, terminate the stalled solver, flush bloated learned lemmas, reload from `scratch/best_edges_graph868.txt`, reseed (`777 + restart_count * 19`), and resume. This prevents wasting 300s on dead-end search branches.
5. **Independent Certification**:
   Once $k = 1$ (or Giant = $N_{\text{dir}}$), uncontract the tour and verify via `TourVerifier` (5,544 unique vertices, valid simple cycle, 100% edges present in raw benchmark).

---

## 3. Tasks

### Task 1: Update `src/cegar-fix/examples/solve_class1_exact.rs`
**Files:**
- Modify: `src/cegar-fix/examples/solve_class1_exact.rs`

- [ ] **Step 1: Implement Giant preservation in cut generation**
  - Never cut `sat_cycles[0]`. Iterate over `sat_cycles[1..=num_to_cut]`.
- [ ] **Step 2: Implement Adaptive Batching**
  - Set `batch = if g_len >= 1400 { 3 } else if g_len >= 1000 { 6 } else { 10 }`.
  - Set `num_to_cut = (sat_cycles.len() - 1).min(batch)`.
- [ ] **Step 3: Implement Monotonic Best Phase Anchoring**
  - Save `best_hints` and only update them when `sat_cycles[0].len() >= best_giant_len`.
  - Apply `best_hints` to `solver.phase_lit`.
- [ ] **Step 4: Configure 30s Agile Stall Recovery**
  - Set round timeout terminator to 30.0s.
  - On stall: flush solver, increment seed (`777 + restart_count * 19`), re-inject clean cuts from `best_edges_graph868.txt`.

### Task 2: Compile Release Binary
- [ ] **Step 1: Compile `solve_class1_exact`**
  - Run `cargo build --release --manifest-path src/cegar-fix/Cargo.toml --example solve_class1_exact`.
  - Confirm 0 errors.

### Task 3: Execute Official 1,800s Benchmark on Core 0
- [ ] **Step 1: Run benchmark pinned to Core 0**
  - Run `taskset -c 0 nice -n 19 ./src/cegar-fix/target/release/examples/solve_class1_exact FHCPCS-col/graph868.col scratch/graph868/found_tour_graph868.hcp 1800.0`.
  - Monitor logs for cycle count reduction below 32 down to 1.

### Task 4: Independent Tour Certification
- [ ] **Step 1: Run independent Python verification**
  - Run `python3 scratch/test_stage4_verification.py FHCPCS-col/graph868.col scratch/graph868/found_tour_graph868.hcp`.
  - Confirm 5,544 vertices, simple cycle, 100% valid edges.
