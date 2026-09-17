# Multi-Gadget Assumption Absorber & Zero-Stall CEGAR Solver for Graph868

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement, execute, and independently certify a 100% exact, deterministic combinatorial solver for `graph868.col` with zero solver stalls by rotating assumption absorption across all satellite gadgets and bounding core relaxation.

**Architecture:** Replace the single-gadget unbounded core decay in `solve_class1_exact.rs` with an iterative round-robin gadget absorption engine. In each round, each small satellite cycle $S_i$ is systematically targeted; CaDiCaL uses distance-guided UNSAT core relaxation bounded by a strict Giant floor ($\ge 1,200$ edges assumed) to guarantee $\le 1\text{ms}$ solve times per query without unconstrained stalls. Anti-shatter rejection cuts prevent fragment oscillation. When all gadgets are merged into the Giant, the 100% uncontracted tour is written and certified with `TourVerifier`.

**Tech Stack:** Rust 2021 edition, `rustsat 0.6.1`, `rustsat-cadical 0.4.1` (CaDiCaL 1.9.4 `SolveIncremental`), `TourVerifier` (exact cycle and edge certificate verification).

## Global Constraints
- Primary target: `FHCPCS-col/graph868.col` ($N = 5,544, M = 9,072, N_{\text{dir}} = 1,848$ contracted blocks).
- Output file: `scratch/graph868/found_tour_graph868.hcp`.
- Independent 100% certificate verification via `TourVerifier` (`python3 scratch/test_stage4_verification.py FHCPCS-col/graph868.col scratch/graph868/found_tour_graph868.hcp`).
- Total wall-clock time $\le 1,800.0$s pinned strictly to Core 0 (`taskset -c 0 nice -n 19`), single worker (`num_workers = 1`).
- 100% exact combinatorial method: zero heuristic LKH/Concorde, zero reading `.tou` files, zero tour injection.
- Deterministic execution (`seed = 777`).

---

### Task 1: Implement Distance-Guided Round-Robin Gadget Absorber in `solve_class1_exact.rs`

**Files:**
- Modify: `src/cegar-fix/examples/solve_class1_exact.rs:600-788`
- Test: `cargo build --release --manifest-path src/cegar-fix/Cargo.toml --example solve_class1_exact`

**Interfaces:**
- Consumes:
  - `active_cycles: Vec<Vec<usize>>`: current list of cycles sorted by length descending (`active_cycles[0]` is Giant).
  - `dir_adj: Vec<Vec<usize>>`, `in_arcs: Vec<Vec<usize>>`: directed contracted block adjacency.
  - `arc_lit_map: HashMap<(usize, usize), Lit>`: mapping between directed block arcs and CaDiCaL boolean variables.
- Produces:
  - `active_cycles`: updated cycle list with enlarged Giant and decreased cycle count.
  - Checkpoint updates: `scratch/best_edges_graph868.txt`.

- [ ] **Step 1: Write BFS distance function and core literal sorter**
Compute BFS distance `dist` from satellite cycle $S_i$ across `dir_adj` and `in_arcs`. Map each core literal to its contracted arc and sort core literals by `min(dist[u], dist[v])` ascending so that only edges in the immediate vicinity of $S_i$ are relaxed.

- [ ] **Step 2: Implement round-robin candidate rotation with Giant assumption floor**
Iterate through candidate small cycles $S_i \in active\_cycles[1..]$ (smallest first). For each candidate:
1. Initialize assumptions to Giant edges not touching $S_i$.
2. Add subtour cut forbidding $S_i$.
3. Run micro-absorption loop up to 60 iterations with floor `current_assumps.len() >= 1150`.
4. If `Sat`: check if Giant improved. If improved, update `active_cycles` and checkpoint. If regressed, add Anti-Shatter rejection cuts forbidding fragments and re-query.
5. If `Unsat`: remove top $K$ closest literals ($K=2$ if core $\le 20$, $K=4$ if core $> 20$).
6. If floor is breached, stop candidate $S_i$ and move to candidate $S_{i+1}$.

- [ ] **Step 3: Replace 120s unconstrained stall with fast-fail fallback**
Cap any fallback solve at 10.0s rather than 120.0s. If fallback does not find SAT within 10.0s, flush bloated lemmas, reseed with next deterministic seed, and return immediately to assumption-guided absorption.

- [ ] **Step 4: Build and test compilation**
Run: `cargo build --release --manifest-path src/cegar-fix/Cargo.toml --example solve_class1_exact`
Expected: Compile cleanly with exit code 0.

---

### Task 2: Execute Official 1,800.0s Benchmark on Core 0

**Files:**
- Execute: `./src/cegar-fix/target/release/examples/solve_class1_exact FHCPCS-col/graph868.col scratch/graph868/found_tour_graph868.hcp 1800.0`
- Monitor: Core 0 CPU and process logs.

- [ ] **Step 1: Launch background benchmark task strictly pinned to Core 0**
Command: `taskset -c 0 nice -n 19 ./src/cegar-fix/target/release/examples/solve_class1_exact FHCPCS-col/graph868.col scratch/graph868/found_tour_graph868.hcp 1800.0`
Monitor execution via log output without polling loops.

- [ ] **Step 2: Verify tour output generation**
Verify `scratch/graph868/found_tour_graph868.hcp` exists and has standard TSPLIB format (header, 5,544 vertex lines, `-1` EOF).

---

### Task 3: Independent Certificate Verification

**Files:**
- Verify: `scratch/test_stage4_verification.py FHCPCS-col/graph868.col scratch/graph868/found_tour_graph868.hcp`

- [ ] **Step 1: Run independent verification script**
Run: `python3 scratch/test_stage4_verification.py FHCPCS-col/graph868.col scratch/graph868/found_tour_graph868.hcp`
Expected:
- Length: 5,544 / 5,544
- Distinct vertices: 5,544 / 5,544
- Valid edges in G: 5,544 / 5,544
- VERIFICATION PASSED: Valid Hamiltonian Cycle!
