# Structural Exact Solver for Flinders Class 1 Graphs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement, execute, and certify a 100% deterministic exact combinatorial solver for Flinders Class 1 HCP instances (`graph868.col` and `graph960.col`) on Core 0 (`taskset -c 0 nice -n 19`, single worker) within 1,800s per instance, certified by independent `TourVerifier`.

**Architecture:** 
1. **Degree-2 Block Contraction**: Contract raw graph into directed bipartite block graph ($N_{dir} = N / 3$).
2. **Deterministic Arc & Literal Mapping**: Deterministic lexicographical ordering of directed arcs.
3. **Elimination of the Fragmentation Trap via Giant Protection**:
   - In standard CEGAR, adding negative cuts to the Giant cycle ($\bigvee_{e \in C_0} \neg x_e$) destroys the backbone, causing cycle rebounds and stalling.
   - We enforce **Giant Protection**: cuts are strictly generated for subcycles: `cycles_to_cut = &sat_cycles[1..]`. The Giant cycle is never cut, allowing it to grow monotonically.
4. **Standard Non-Chronological CDCL with Dynamic Reseeding**:
   - Remove `chrono = 1` and `restartint = 50`. CaDiCaL utilizes its native non-chronological conflict analysis and adaptive restart scheduling to jump across large decision subtrees.
   - When reseeding every 10 rounds or after slow rounds, learned clause database bloat is flushed while preserving base CNF, accumulated subcycle cuts, and Giant backbone phase guidance.
5. **Independent 100% Verification**:
   - The final directed cycle is uncontracted to raw vertices $1..N$ and validated with `TourVerifier` (`scratch/test_stage4_verification.py`).

## Global Constraints

- **Core Isolation**: Pin strictly to Core 0 (`taskset -c 0 nice -n 19`). Single worker (`num_workers = 1`).
- **Time Budget**: Total pipeline runtime strictly $\le 1,800$s per instance.
- **Purity**: Zero heuristic LKH/Concorde, zero tour injection, zero reading `.tou` files.
- **Determinism**: Fixed seed (`seed = 777`).

---

### Task 1: Update `solve_class1_exact.rs` with Giant Protection & Non-Chronological CDCL

**Files:**
- Modify: `src/cegar-fix/examples/solve_class1_exact.rs`
- Compile: `cargo build --release --manifest-path src/cegar-fix/Cargo.toml --example solve_class1_exact`

**Interfaces:**
- Input: `.col` graph path, output `.hcp` path, timeout seconds.
- Output: Single release binary `src/cegar-fix/target/release/examples/solve_class1_exact`.

- [ ] **Step 1: Apply Giant Protection and CDCL Configuration in `solve_class1_exact.rs`**
  - In `make_clean_solver`: remove `restartint = 50` and `chrono = 1`. Set `seed = 777`, `phase = 1`, `rephase = 0`, `lucky = 0`.
  - In cut generation: iterate strictly over `&sat_cycles[1..]` so `sat_cycles[0]` is NEVER cut.
  - Apply dual boundary cuts for all subcycles $|c| \le 24$.
  - In dynamic hints: only hint Giant edges (`sat_cycles[0]`), never hint subcycles.
- [ ] **Step 2: Build release binary**
  - Run: `cargo build --release --manifest-path src/cegar-fix/Cargo.toml --example solve_class1_exact`
- [ ] **Step 3: Quick canary check (10s on Core 0)**
  - Run: `taskset -c 0 nice -n 19 ./src/cegar-fix/target/release/examples/solve_class1_exact FHCPCS-col/graph868.col scratch/graph868/canary.hcp 10.0`
  - Verify solver starts cleanly and executes multiple rounds per second.

---

### Task 2: Full 1,800s Execution & Independent Certification for `graph868.col` (Core 0)

**Files:**
- Input: `FHCPCS-col/graph868.col` ($N = 5,544$, $N_{dir} = 1,848$)
- Output: `scratch/graph868/found_tour_graph868.hcp`
- Verification script: `scratch/test_stage4_verification.py`

- [ ] **Step 1: Launch 1800s certified solver on Core 0**
  - Command: `taskset -c 0 nice -n 19 ./src/cegar-fix/target/release/examples/solve_class1_exact FHCPCS-col/graph868.col scratch/graph868/found_tour_graph868.hcp 1800.0`
- [ ] **Step 2: Monitor execution with background status checks**
- [ ] **Step 3: Execute independent verification script**
  - Command: `python3 scratch/test_stage4_verification.py FHCPCS-col/graph868.col scratch/graph868/found_tour_graph868.hcp`

---

### Task 3: Full 1,800s Execution & Independent Certification for `graph960.col` (Core 0)

**Files:**
- Input: `FHCPCS-col/graph960.col` ($N = 6,930$, $N_{dir} = 2,310$)
- Output: `scratch/graph960/found_tour_graph960.hcp`
- Verification script: `scratch/test_stage4_verification.py`

- [ ] **Step 1: Launch 1800s certified solver on Core 0**
  - Command: `taskset -c 0 nice -n 19 ./src/cegar-fix/target/release/examples/solve_class1_exact FHCPCS-col/graph960.col scratch/graph960/found_tour_graph960.hcp 1800.0`
- [ ] **Step 2: Monitor execution with background status checks**
- [ ] **Step 3: Execute independent verification script**
  - Command: `python3 scratch/test_stage4_verification.py FHCPCS-col/graph960.col scratch/graph960/found_tour_graph960.hcp`
