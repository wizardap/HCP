# Structural Exact Deterministic Solver for graph868 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement, compile, execute, and independently verify a 100% exact, deterministic combinatorial solver on `FHCPCS-col/graph868.col` with a 1,800s time limit pinned to Core 0.

**Architecture:** Single-worker deterministic CEGAR with degree-2 contracted directed representation. Eliminates destructive reseeding stalls by maintaining CaDiCaL incrementally. Fixes checkpoint corruption by strictly loading the 31-cycle 1,686-block Giant (91.23% of graph) and guarding best checkpoints. Throttles boundary cuts to $|C| \le 4$ while applying pure negative subcycle cuts to $|C| \ge 8$ to prevent clause bloat.

**Tech Stack:** Rust (`cegar-fix`, `rustsat`, `rustsat-cadical`), CaDiCaL 1.9.5, Python 3 (`TourVerifier`).

## Global Constraints

- Pinned strictly to **Core 0** (`taskset -c 0 nice -n 19`), single worker (`num_workers = 1`).
- Exact time limit of **1,800s** for `FHCPCS-col/graph868.col`.
- 100% exact combinatorial solver: zero heuristic LKH/Concorde, zero reading `.tou` files, zero tour injection.
- Deterministic (`seed = 777`).
- 100% independent certification via `TourVerifier` (`python3 scratch/test_stage4_verification.py FHCPCS-col/graph868.col <output_tour.hcp>`).

---

### Task 1: Restore and Protect High-Quality Giant Checkpoint

**Files:**
- Restore: `scratch/best_edges_graph868.txt` from `scratch/graph868_giant_1686_full_edges.txt`
- Verify: `scratch/graph868_giant_1686_full_edges.txt` (31 cycles, Giant = 1,686 blocks)

**Interfaces:**
- Input: `scratch/graph868_giant_1686_full_edges.txt` (1,848 directed block-to-block edges)
- Output: `scratch/best_edges_graph868.txt` initialized to the 31-cycle checkpoint

- [ ] **Step 1: Overwrite `scratch/best_edges_graph868.txt` with verified 31-cycle Giant edges**
```bash
cp scratch/graph868_giant_1686_full_edges.txt scratch/best_edges_graph868.txt
```

- [ ] **Step 2: Verify cycle distribution of restored checkpoint**
Run Python validation script to confirm 31 cycles with Giant = 1,686 blocks.

---

### Task 2: Refactor `solve_class1_exact.rs` with Incremental CDCL & Throttled Cuts

**Files:**
- Modify: `src/cegar-fix/examples/solve_class1_exact.rs`

**Interfaces:**
- Consumes: `raw_g` from `FHCPCS-col/graph868.col`, degree-2 contractor, checkpoint files
- Produces: Certified HCP tour file `scratch/graph868/found_tour_graph868.hcp`

- [ ] **Step 1: Update Checkpoint Loader & Guard**
Modify checkpoint discovery to prioritize `scratch/{}_giant_1686_full_edges.txt` and only accept candidates that improve `best_cycle_count`. Initialize `best_cycle_count` to the loaded checkpoint's cycle count (31) instead of `usize::MAX`.

- [ ] **Step 2: Replace Destructive Reseeding with Incremental Solving**
Remove `make_clean_solver` re-instantiation logic during CEGAR rounds. Keep the CaDiCaL instance alive across all iterations. Add new cuts via `solver.add_cnf(cuts)`.

- [ ] **Step 3: Throttle Boundary Cuts & Apply Pure Negative Subcycle Cuts**
For subcycles $C \in \text{sat\_cycles}[1..]$:
- Always add negative cycle cut $\bigvee_{e \in C} \neg x_e$.
- Add dual boundary cuts ONLY when $|C| \le 4$.
- Strictly skip cuts on the Giant cycle $\text{sat\_cycles}[0]$.

- [ ] **Step 4: Compile Release Binary**
```bash
cargo build --release --manifest-path src/cegar-fix/Cargo.toml --example solve_class1_exact
```

---

### Task 3: Execute 1,800s Benchmark on Core 0 for `graph868.col`

**Files:**
- Execute: `src/cegar-fix/target/release/examples/solve_class1_exact`
- Target: `FHCPCS-col/graph868.col`
- Output: `scratch/graph868/found_tour_graph868.hcp`

- [ ] **Step 1: Launch Solver on Core 0 with 1,800s Timeout**
```bash
taskset -c 0 nice -n 19 ./src/cegar-fix/target/release/examples/solve_class1_exact FHCPCS-col/graph868.col scratch/graph868/found_tour_graph868.hcp 1800.0
```

- [ ] **Step 2: Monitor Progress**
Observe per-round cycle count convergence and log outputs.

---

### Task 4: Independent Tour Certification

**Files:**
- Test: `scratch/test_stage4_verification.py`
- Target Tour: `scratch/graph868/found_tour_graph868.hcp`

- [ ] **Step 1: Run Ground-Truth Independent Verifier**
```bash
python3 scratch/test_stage4_verification.py FHCPCS-col/graph868.col scratch/graph868/found_tour_graph868.hcp
```
Confirm:
- Exactly 5,544 vertices
- All 5,544 edges exist in `FHCPCS-col/graph868.col`
- 100% independent verification PASS
