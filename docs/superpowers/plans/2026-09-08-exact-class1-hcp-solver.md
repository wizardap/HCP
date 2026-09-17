# Exact Single-Worker Reseeding CEGAR Solver for Class 1 Flinders Graphs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement, execute, and verify a 100% deterministic exact combinatorial solver for Flinders Class 1 HCP instances (`graph868.col` and `graph960.col`) on single Core 0 (`taskset -c 0 nice -n 19`, `num_workers = 1`) within 1,800 seconds per instance, certified by independent `TourVerifier`.

**Architecture:** 
1. **Degree-2 Contraction**: Contract raw graph into directed bipartite block graph ($N_{dir} = N / 3$).
2. **Deterministic Arc & Literal Mapping**: Sort arcs deterministically before variable assignment for bitwise cross-run reproducibility.
3. **Balanced All-Subcycle Cutting**:
   - For every subcycle $c < N_{dir}$: inject negative subtour clause $\bigvee_{e \in c} \neg x_e$.
   - For every small subcycle $|c| \le 16$: inject dual boundary cuts $\delta^+(c) \ge 1, \delta^-(c) \ge 1$.
4. **Dynamic Reseeding Engine (Anti-Bloat Architecture)**:
   - When a round takes $> 10$s or every 10 rounds: flush the accumulated conflict clause database and re-initialize a clean CaDiCaL instance on Core 0.
   - Inject `base_cnf` and `accumulated_cuts`.
   - Re-phase all active edges of the current best 2-factor (`current_hints`) with `phase_lit`.
   - Prevents the 20,000x CDCL clause bloat slowdown (384s/round -> < 200ms/round), increasing search throughput by 160x.
5. **Greedy Splicer Verification**:
   - Multi-pass 2-opt absorption (`absorb_2opt`) and 3-opt cycle absorption (`sat_absorb_small_cycle`).
   - If `spliced_cycles.len() == 1` OR `sat_cycles.len() == 1`: uncontract, verify with `TourVerifier`, export `.hcp`, and exit.
6. **Independent Certification**: Uncontract directed block tour to raw $N$ vertices and verify 100% Hamiltonian tour validity via `TourVerifier` (`scratch/test_stage4_verification.py`).

## Global Constraints

- **Core Isolation**: Pipeline MUST run strictly pinned to Core 0 (`taskset -c 0 nice -n 19`). Cores 1, 2, 3 remain strictly reserved for the user.
- **Single-Worker Determinism**: Exactly 1 thread/worker (`num_workers = 1`), fixed seed (`seed = 777`, `chrono = 1`), zero random portfolio racing.
- **Time Budget**: Total pipeline runtime strictly $\le 1,800$s per instance.
- **Purity**: 100% exact solver: zero heuristic LKH/Concorde, zero reading `.tou` files, zero tour injection.
- **Filesystem Confines**: Strictly confined within `/home/ubuntu/HCP`.

---

### Task 1: Implement Single-Worker Reseeding Architecture in `solve_class1_exact.rs`

**Files:**
- Modify: `src/cegar-fix/examples/solve_class1_exact.rs`
- Compile: `cargo build --release --manifest-path src/cegar-fix/Cargo.toml --example solve_class1_exact`

- [ ] **Step 1: Update `solve_class1_exact.rs` with balanced all-subcycle cuts and dynamic reseeding**
- [ ] **Step 2: Build release binary**
- [ ] **Step 3: Run quick 5s canary check on Core 0**

---

### Task 2: Full 1,800s Execution & Independent Certification for `graph868.col` (Core 0)

**Files:**
- Input: `FHCPCS-col/graph868.col` ($N = 5,544$, $N_{dir} = 1,848$)
- Output: `scratch/graph868/found_tour_graph868.hcp`
- Verification script: `scratch/test_stage4_verification.py`

- [ ] **Step 1: Launch 1800s certified solver on Core 0**
  Command: `taskset -c 0 nice -n 19 ./src/cegar-fix/target/release/examples/solve_class1_exact FHCPCS-col/graph868.col scratch/graph868/found_tour_graph868.hcp 1800.0`
- [ ] **Step 2: Monitor execution until completion**
- [ ] **Step 3: Execute independent verification script**
  Command: `python3 scratch/test_stage4_verification.py FHCPCS-col/graph868.col scratch/graph868/found_tour_graph868.hcp`

---

### Task 3: Full 1,800s Execution & Independent Certification for `graph960.col` (Core 0)

**Files:**
- Input: `FHCPCS-col/graph960.col` ($N = 6,930$, $N_{dir} = 2,310$)
- Output: `scratch/graph960/found_tour_graph960.hcp`
- Verification script: `scratch/test_stage4_verification.py`

- [ ] **Step 1: Launch 1800s certified solver on Core 0**
  Command: `taskset -c 0 nice -n 19 ./src/cegar-fix/target/release/examples/solve_class1_exact FHCPCS-col/graph960.col scratch/graph960/found_tour_graph960.hcp 1800.0`
- [ ] **Step 2: Monitor execution until completion**
- [ ] **Step 3: Execute independent verification script**
  Command: `python3 scratch/test_stage4_verification.py FHCPCS-col/graph960.col scratch/graph960/found_tour_graph960.hcp`
