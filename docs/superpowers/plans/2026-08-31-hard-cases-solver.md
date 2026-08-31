# Hard Instances Multi-Engine Solver Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement and integrate the 3-Engine architecture (`HubAwareStaticCutter`, `MacroCrossoverSplicer`, and `QuotientBlockCutter`) to solve universal timeout benchmark instances (`graph710`, `graph717`, `graph788`, `graph479`).

**Architecture:**
- **Engine 1 (`HubAwareStaticCutter`):** Throttles static 6/7/8-cycle subtour cuts near high-degree Hubs ($\deg \ge 10$), slashing encoding time on `graph717` from 10.3s to <0.3s and preventing SAT clause pollution.
- **Engine 2 (`MacroCrossoverSplicer`):** Formulates and solves an auxiliary local SAT subproblem over boundary cross-edges between $2 \le k \le 6$ macro-cycles in 1–5ms, breaking through bipartite parity barriers where 2-opt and 3-opt fail.
- **Engine 3 (`QuotientBlockCutter`):** Identifies modular 44-vertex block partitions and generates algebraic quotient cut clauses to eliminate $2^{42}$ / $2^{70}$ subcycle oscillation loops.

**Tech Stack:** Rust, `rustsat`, `rustsat_cadical` (CaDiCaL SAT solver), Cargo, bash (`taskset -c 0,1,2 nice -n 19`).

## Global Constraints
- Core Reservation: Core 3 is strictly reserved for the user. All compute-intensive commands MUST run under `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: NEVER read, import, or parse answer tour files (`.hcp.tou`) into the solver pipeline.
- Directory Constraint: NEVER touch system configurations, OS configs, or anything outside `/home/ubuntu/HCP`.
- TDD Enforcement: Strict Red-Green-Refactor. No production code without a failing test first.

---

### Task 1: Hub-Aware Selective Static Cutter (Engine 1)

**Files:**
- Modify: `src/cegar-fix/src/static_cycle_cutter.rs`
- Modify: `src/cegar-fix/src/hcp_solver.rs:260-280`
- Test: `src/cegar-fix/tests/test_hub_aware_cutter.rs`

**Interfaces:**
- Consumes: `Graph`, `Encoder`
- Produces: `StaticCycleCutter::generate_selective_static_cycle_cuts(g: &Graph, encoder: &Encoder, hub_deg_threshold: usize) -> Cnf`

- [ ] **Step 1: Write the failing test**
  Create `src/cegar-fix/tests/test_hub_aware_cutter.rs` containing a synthetic graph with a Hub vertex of degree 12 and peripheral vertices forming multiple 6-cycles through the Hub as well as peripheral-only 4-cycles.
  Verify that when `hub_deg_threshold = 10`, 3-cycles and 4-cycles are retained, but 6-cycles passing through the Hub are throttled.

- [ ] **Step 2: Run test to confirm it fails to compile (RED)**
  ```bash
  cargo test --test test_hub_aware_cutter
  ```

- [ ] **Step 3: Implement `generate_selective_static_cycle_cuts` in `static_cycle_cutter.rs` (GREEN)**
  Add `generate_selective_static_cycle_cuts(g: &Graph, encoder: &Encoder, hub_deg_threshold: usize) -> Cnf`:
  - Detect vertices with degree $\ge \text{hub\_deg\_threshold}$.
  - Globally generate 3-cycles and 4-cycles.
  - For 6, 7, 8-cycles: skip any candidate cycle if it contains a vertex with degree $\ge \text{hub\_deg\_threshold}$.
  - Keep existing `generate_static_small_cycle_cuts` calling the selective version with `hub_deg_threshold = usize::MAX` for full backwards compatibility.

- [ ] **Step 4: Verify test passes (GREEN)**
  ```bash
  cargo test --test test_hub_aware_cutter
  ```

- [ ] **Step 5: Integrate into `hcp_solver.rs` and verify on `graph717.col`**
  In `hcp_solver.rs` Round 0 initialization:
  - If `hub_registry` has hubs or graph has vertices with degree $\ge 10$, invoke `generate_selective_static_cycle_cuts(&g, &encoder, 10)` instead of generating 27,776 unconstrained clauses.
  - Run benchmark test on `graph717.col`:
    ```bash
    taskset -c 0,1,2 nice -n 19 ./src/cegar-fix/target/release/cegar-fix --input FHCPCS-col/graph717.col --auto 1 --timeout 15
    ```
  - Confirm encoding time drops from 10.3s to <0.5s and clause count drops below 60,000.

- [ ] **Step 6: Commit changes**
  ```bash
  git add src/cegar-fix/src/static_cycle_cutter.rs src/cegar-fix/src/hcp_solver.rs src/cegar-fix/tests/test_hub_aware_cutter.rs
  git commit -m "feat(cutter): implement HubAwareStaticCutter to throttle cycle cuts near hubs"
  ```

---

### Task 2: Macro-Crossover Local SAT Splicer (Engine 2)

**Files:**
- Create: `src/cegar-fix/src/macro_crossover_splicer.rs`
- Modify: `src/cegar-fix/src/lib.rs`
- Modify: `src/cegar-fix/src/hcp_solver.rs:745-770`
- Test: `src/cegar-fix/tests/test_macro_crossover_splicer.rs`

**Interfaces:**
- Consumes: `cycles: &[Vec<i32>]`, `g: &Graph`, `contractor: &Degree2Contractor`
- Produces: `MacroCrossoverSplicer::try_crossover_splice(cycles: &[Vec<i32>], g: &Graph, contractor: &Degree2Contractor) -> Option<Vec<i32>>`

- [ ] **Step 1: Write the failing unit test**
  Create `src/cegar-fix/tests/test_macro_crossover_splicer.rs`:
  - Construct a synthetic bipartite graph with two macro-cycles $C_1, C_2$ where alternating protected edges prevent any 2-opt or 3-opt merge (2-opt count = 0), but an alternating 4-opt crossover between cross-edges merges $C_1$ and $C_2$ into a single cycle.
  - Call `MacroCrossoverSplicer::try_crossover_splice(&cycles, &g, &contractor)`.
  - Assert result is `Some(tour)` of length $|V|$, valid in $g$, and preserving all `contractor.chain_map` protected edges.

- [ ] **Step 2: Run test to confirm failure (RED)**
  ```bash
  cargo test --test test_macro_crossover_splicer
  ```

- [ ] **Step 3: Implement `MacroCrossoverSplicer` (GREEN)**
  In `src/cegar-fix/src/macro_crossover_splicer.rs`:
  - Extract boundary vertices $B$ (vertices in each macro-cycle with neighbors in other macro-cycles).
  - Collect candidate cross-edges between cycles and candidate unprotected cycle edges incident to $B$.
  - Formulate a local `CaDiCaL` SAT problem:
    - Directed or undirected edge choice variables $x_e \in \{0, 1\}$.
    - Degree constraints: for each boundary vertex $v \in B$, exactly 2 incident edges are active. If $v$ has an incident protected edge, that protected edge is fixed active (value 1), leaving 1 free incident edge.
    - Connectivity constraints: enforce cycle-merging cuts so subcycles cannot remain disconnected.
  - Solve with CaDiCaL (with conflict limit 5,000 / timeout 500ms).
  - If SAT, trace the unique 2-factor cycle, verify it forms a single cycle of full length, and return `Some(tour)`.
  - Register module in `src/cegar-fix/src/lib.rs`.

- [ ] **Step 4: Verify test passes (GREEN)**
  ```bash
  cargo test --test test_macro_crossover_splicer
  ```

- [ ] **Step 5: Integrate into `hcp_solver.rs`**
  In `hcp_solver.rs` inside the patching cascade (around line 750):
  - When $2 \le \_active\_cycles.len() \le 6$:
    ```rust
    if let Some(spliced_tour) = MacroCrossoverSplicer::try_crossover_splice(&_active_cycles, &g, contractor) {
        println!("MacroCrossoverSplicer: successfully spliced {} macro-cycles into single tour", _active_cycles.len());
        let final_tour = contractor.uncontract_cycle(&spliced_tour);
        // print SATISFIABLE and return solution
        return (count, clause_count, Some(final_tour));
    }
    ```

- [ ] **Step 6: Verify `cargo test --lib` passes**
  ```bash
  cargo test --lib
  ```

- [ ] **Step 7: Commit changes**
  ```bash
  git add src/cegar-fix/src/macro_crossover_splicer.rs src/cegar-fix/src/lib.rs src/cegar-fix/src/hcp_solver.rs src/cegar-fix/tests/test_macro_crossover_splicer.rs
  git commit -m "feat(splicer): implement MacroCrossoverSplicer for parity-breaking k-opt"
  ```

---

### Task 3: Quotient Block Cutter (Engine 3)

**Files:**
- Create: `src/cegar-fix/src/quotient_block_cutter.rs`
- Modify: `src/cegar-fix/src/lib.rs`
- Modify: `src/cegar-fix/src/hcp_solver.rs:880-920`
- Test: `src/cegar-fix/tests/test_quotient_block_cutter.rs`

**Interfaces:**
- Consumes: `Graph`, `Degree2Contractor`, `cycles: &[Vec<i32>]`, `Encoder`
- Produces:
  - `QuotientBlockCutter::detect_modular_blocks(g: &Graph, contractor: &Degree2Contractor) -> Vec<HashSet<i32>>`
  - `QuotientBlockCutter::generate_quotient_sec_clauses(cycles: &[Vec<i32>], blocks: &[HashSet<i32>], g: &Graph, encoder: &Encoder) -> Vec<Clause>`

- [ ] **Step 1: Write the failing test**
  Create `src/cegar-fix/tests/test_quotient_block_cutter.rs`:
  - Build a synthetic graph with 4 distinct modular blocks where each block is internally connected by protected edges.
  - Provide a 2-factor cycle configuration that partitions the blocks into $\{B_0, B_1\}$ and $\{B_2, B_3\}$.
  - Assert that `generate_quotient_sec_clauses` detects that the cycle covers whole blocks and produces a unified cut clause containing all cross-edges exiting $\{B_0, B_1\}$.

- [ ] **Step 2: Run test to confirm failure (RED)**
  ```bash
  cargo test --test test_quotient_block_cutter
  ```

- [ ] **Step 3: Implement `QuotientBlockCutter` (GREEN)**
  In `src/cegar-fix/src/quotient_block_cutter.rs`:
  - `detect_modular_blocks`: Groups vertices by components formed by protected chains and dense internal clustering.
  - `generate_quotient_sec_clauses`:
    - For each subcycle $C$ in `cycles`:
    - Check if $C$ aligns with a union of modular blocks $\mathcal{S}$.
    - If so, collect all directed boundary arcs $\delta^+(\mathcal{S}) = \{(u, v) \in E(G) \mid u \in \mathcal{S}, v \notin \mathcal{S}\}$.
    - Generate clause: $\bigvee_{(u, v) \in \delta^+(\mathcal{S})} x_{uv}$.
    - Generate reverse clause: $\bigvee_{(v, u) \in \delta^-(\mathcal{S})} x_{vu}$.
  - Register module in `src/cegar-fix/src/lib.rs`.

- [ ] **Step 4: Verify test passes (GREEN)**
  ```bash
  cargo test --test test_quotient_block_cutter
  ```

- [ ] **Step 5: Integrate into `hcp_solver.rs`**
  In `hcp_solver.rs` blocking clause generation:
  - Cache detected modular blocks at Round 0.
  - When subcycle count $\le 20$, invoke `generate_quotient_sec_clauses` and append generated clauses to `round_cuts`.

- [ ] **Step 6: Commit changes**
  ```bash
  git add src/cegar-fix/src/quotient_block_cutter.rs src/cegar-fix/src/lib.rs src/cegar-fix/src/hcp_solver.rs src/cegar-fix/tests/test_quotient_block_cutter.rs
  git commit -m "feat(cutter): implement QuotientBlockCutter for block-level SECs"
  ```

---

### Task 4: End-to-End Verification & Benchmarking

**Files:**
- Test all: `cargo test --tests`
- Benchmark: `FHCPCS-col/graph479.col`, `FHCPCS-col/graph788.col`, `FHCPCS-col/graph710.col`, `FHCPCS-col/graph717.col`

- [ ] **Step 1: Run full unit & integration test suite**
  ```bash
  cargo test --lib
  cargo test --tests
  ```
  Ensure all 50+ unit tests and all integration tests pass with 0 failures.

- [ ] **Step 2: Build release binary**
  ```bash
  cargo build --release
  ```

- [ ] **Step 3: Benchmark on target graphs (Core 0,1,2, nice -n 19)**
  Run evaluation on the target hard graphs with timeout monitoring:
  ```bash
  taskset -c 0,1,2 nice -n 19 timeout 300 ./src/cegar-fix/target/release/cegar-fix --input FHCPCS-col/graph479.col --auto 1 --timeout 250 --output-tour scratch/found_tour_479_verify.hcp
  taskset -c 0,1,2 nice -n 19 timeout 300 ./src/cegar-fix/target/release/cegar-fix --input FHCPCS-col/graph788.col --auto 1 --timeout 250 --output-tour scratch/found_tour_788_verify.hcp
  taskset -c 0,1,2 nice -n 19 timeout 300 ./src/cegar-fix/target/release/cegar-fix --input FHCPCS-col/graph710.col --auto 1 --timeout 250 --output-tour scratch/found_tour_710_verify.hcp
  taskset -c 0,1,2 nice -n 19 timeout 300 ./src/cegar-fix/target/release/cegar-fix --input FHCPCS-col/graph717.col --auto 1 --timeout 250 --output-tour scratch/found_tour_717_verify.hcp
  ```

- [ ] **Step 4: Independent Tour Verification**
  Run Python verifier to assert 100% correctness:
  - Tour vertex count matches $N$.
  - All vertices distinct (no missing or duplicated vertices).
  - Every adjacent pair is a valid edge in `.col`.
  - Closing edge $(v_N, v_1)$ exists in `.col`.

- [ ] **Step 5: Final Documentation & Git Summary**
  Log benchmark timings and tour verification results in `docs/superpowers/specs/2026-08-31-hard-cases-solver-design.md`.
