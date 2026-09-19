# Router-Free General HCP Solver Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace all graph-ID-based routing with a single, pure algorithmic pipeline that takes any arbitrary graph $G = (V, E)$ and solves both general benchmark graphs (e.g. graphs 1..400) and the 11 massive challenge graphs ($N \ge 4000$) with 100% mathematical soundness.

**Architecture:** A 5-stage automated structural reduction and CEGAR pipeline:
1. Automated Degree-2 Chain Contraction (shrinks chains $u-v-w \to u-w$).
2. Automated 2-Cut / Bridge Corridor Detection (isolates outer corridors into virtual shortcut edges).
3. Automated Bipartite Strip Partitioning (detects 2-coloring symmetry for dense bipartite graphs).
4. Sound Core CDCL SAT-CEGAR Engine (CaDiCaL with strict DFJ subcycle cuts, zero false UNSAT).
5. Recursive Tour Assembly & Verification (unrolls virtual edges and certifies $|V|$ sound edges).

**Tech Stack:** Rust 1.83+ (`cegar-fix`), Python 3.12, PySAT (`pysat.solvers.Cadical195`), DIMACS `.col` standard.

---

### Task 1: Fix Soundness Bug in `cegar-fix` (Rust Engine)

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs:770-865`
- Test: `tests/test_soundness_graph1.rs` or CLI test on `FHCPCS-col/graph1.col`

**Interfaces:**
- Consumes: Raw SAT subcycles `sol_cycles`
- Produces: `raw_sol_cycles` preserved strictly for `get_blocking_clauses`; heuristic patches used ONLY for early termination when `len() == 1`.

- [ ] **Step 1: Write reproducing test command showing graph1 is soundly SAT**
```bash
python3 -c "
import subprocess
out = subprocess.run(['src/cegar-fix/target/release/cegar-fix', '-i', 'FHCPCS-col/graph1.col', '--auto', '0', '-e', '1', '-b', '3', '-y', '0', '-t', '3', '-l', '1', '--timeout', '10'], capture_output=True, text=True).stdout
assert 's UNSATISFIABLE' not in out, 'Bug still present: graph1 claimed UNSAT!'
"
```

- [ ] **Step 2: Run test to confirm it currently triggers false UNSAT or unstability**
Run: the above python command
Expected: may fail if random seed triggers false UNSAT.

- [ ] **Step 3: Modify `src/cegar-fix/src/hcp_solver.rs` to preserve `raw_sol_cycles`**
In `src/cegar-fix/src/hcp_solver.rs`:
Save `let raw_sol_cycles = sol_cycles.clone();`.
For each patcher (`BoundaryAlternatingPatcher`, `GiantCycleStitcher`, `HemisphereSplicer`), only check if result has `len() == 1 && len == total_v` to return SAT.
Do NOT assign `sol_cycles = patched;` when `patched.len() > 1`.
Pass `&raw_sol_cycles` to `get_blocking_clauses`.

- [ ] **Step 4: Recompile Rust solver in release mode**
```bash
cd src/cegar-fix && cargo build --release
```

- [ ] **Step 5: Verify `graph1.col` and `graph48.col` produce SATISFIABLE or TIMEOUT, never UNSAT**
```bash
./src/cegar-fix/target/release/cegar-fix -i FHCPCS-col/graph1.col -e 1 -b 3 -y 0 -t 3 -l 1 --timeout 10
```
Expected: `s SATISFIABLE` with valid tour.

- [ ] **Step 6: Commit fix**
```bash
git add src/cegar-fix/src/hcp_solver.rs
git commit -m "fix(cegar-fix): prevent heuristic patchers from mutating raw subcycles for cut generation"
```

---

### Task 2: Implement Pure Algorithmic Pipeline (`GeneralHcpEngine`)

**Files:**
- Create: `hcp_solver/core/pipeline.py`
- Modify: `hcp_solver/__init__.py`
- Test: `scratch/test_general_pipeline.py`

**Interfaces:**
- `solve_general(adj: Dict[int, Set[int]], timeout: float = 60.0) -> List[int]`
- Accepts ANY graph adjacency dictionary `Dict[int, Set[int]]`. ZERO graph IDs or filename assumptions.
- Returns certified tour as `List[int]`.

- [ ] **Step 1: Write unit test for `solve_general` on a 6-node cycle and a 10-node Petersen-like graph**
Create `scratch/test_general_pipeline.py`:
Test that `solve_general` correctly solves small graphs without knowing any name or ID.

- [ ] **Step 2: Run test to verify it fails (module not created yet)**
```bash
python3 scratch/test_general_pipeline.py
```
Expected: FAIL (ModuleNotFoundError)

- [ ] **Step 3: Implement `hcp_solver/core/pipeline.py`**
Implement the 5-stage reduction:
1. `contract_degree2_chains`: collapse paths of degree-2 vertices.
2. `detect_bridge_corridors`: detect 2-vertex cuts / outer corridors.
3. `detect_bipartite_halves`: 2-coloring check; if large bipartite strip detected, partition groups.
4. `solve_core_cegar`: call `cegar-fix` (or PySAT Cadical fallback) on the reduced core graph.
5. `reconstruct_tour`: unroll contracted chains and corridor paths.
6. Verify sound against input graph.

- [ ] **Step 4: Run unit test to verify it passes**
```bash
python3 scratch/test_general_pipeline.py
```
Expected: PASS

- [ ] **Step 5: Commit `pipeline.py`**
```bash
git add hcp_solver/core/pipeline.py scratch/test_general_pipeline.py
git commit -m "feat(pipeline): implement 100% router-free general graph reduction and CEGAR pipeline"
```

---

### Task 3: Unify CLI and Delete Graph-ID Router

**Files:**
- Modify: `hcp_solver/cli.py`
- Modify: `hcp_solver/router.py` (deprecate or redirect directly to `GeneralHcpEngine`)
- Modify: `run_clean_11.sh` $\to$ update to universal `run_solver.sh`

**Interfaces:**
- CLI command: `python3 -m hcp_solver <file.col>`
- Positional argument: any `.col` file path.
- Flags: `-o <output_tour>`, `--timeout <seconds>`.
- ZERO checks on graph ID or filename.

- [ ] **Step 1: Update `hcp_solver/cli.py` to use `GeneralHcpEngine`**
Point `cli.py` directly to `solve_general(adj)`.
Print stage timings (Contraction, Corridors, Core SAT, Assembly).
Validate output tour using `verify_tour`.

- [ ] **Step 2: Test CLI on multiple different graph archetypes**
```bash
python3 -m hcp_solver FHCPCS-col/graph1.col
python3 -m hcp_solver FHCPCS-col/graph2.col
python3 -m hcp_solver FHCPCS-col/graph710.col
python3 -m hcp_solver FHCPCS-col/graph746.col
```
Expected: ALL pass with `[✓] Tour Verified 100% SOUND`.

- [ ] **Step 3: Create universal runner script `run_solver.sh`**
Support single file: `./run_solver.sh FHCPCS-col/graph1.col`
Support batch range: `./run_solver.sh 1 50`
Support all challenge graphs: `./run_solver.sh --challenge`

- [ ] **Step 4: Commit CLI and runner**
```bash
git add hcp_solver/cli.py run_solver.sh
git commit -m "feat(cli): wire universal router-free solver pipeline to CLI and runner"
```

---

### Task 4: Comprehensive Benchmark & Regression Verification

**Files:**
- Test script: `scratch/verify_universal_pipeline.py`

- [ ] **Step 1: Test 50 general graphs (graph1 .. graph50)**
Run `GeneralHcpEngine` on graphs 1..50.
Record solve rate and time. Ensure 0 crashes, 0 false UNSAT.

- [ ] **Step 2: Test the 11 Challenge graphs**
Run `GeneralHcpEngine` on graph710, 717, 746, 788, 882, 944, 950, 963, 975, 982, 990.
Confirm all 11 graphs solve and verify 100% SOUND.

- [ ] **Step 3: Document results and performance in `docs/UNIVERSAL_SOLVER_BENCHMARK.md`**
Summarize timings and verified soundness across both suites.

- [ ] **Step 4: Final commit**
```bash
git add docs/UNIVERSAL_SOLVER_BENCHMARK.md scratch/verify_universal_pipeline.py
git commit -m "docs(benchmark): document router-free universal solver validation across general & challenge suites"
```
