# Three-Pronged Dense Hub and Full Benchmark Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement and evaluate Cluster Cut Constraints (Direction 1), Adaptive K-Opt Local Search (Direction 2), and execute the full FHCPCS benchmark & packaging (Direction 3).

**Architecture:** 
1. `hcp_solver.rs` adds `add_cluster_cut_constraints` to inject cardinality cut clauses on satellite clusters before CaDiCaL runs.
2. `stem_cycle_patcher.rs` adds `k_opt_splice` to bridge the final 3.6% unvisited vertices.
3. Benchmark harness tests 100 benchmark instances across all size tiers, confirming zero regressions.

**Tech Stack:** Rust, CaDiCaL, `rustsat`, cargo.

## Global Constraints

- 100% mathematical soundness across the entire FHCP benchmark (never emit false `s UNSATISFIABLE`).
- Strictly protect degree-2 contraction invariants in `contractor.chain_map`.
- Zero regressions on all 10 Key Regression Graphs (`graph45`, `graph132`, `graph161`, `graph178`, `graph183`, `graph230`, `graph248`, `graph313`, `graph339`, `graph346`).
- Standard CLI invocation preserved (`-e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`).

---

### Task 1: Implement Cluster Cut Constraints (Direction 1)

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/src/hcp_solver.rs` (unit tests)

**Interfaces:**
- Produces: `pub fn add_cluster_cut_constraints(g: &Graph, encoder: &Encoder, cnf: &mut Cnf) -> usize`

- [x] **Step 1: Write the failing unit test**
- [x] **Step 2: Run test to verify it fails**
- [x] **Step 3: Implement minimal code for `add_cluster_cut_constraints`**
- [x] **Step 4: Run test to verify it passes**
- [x] **Step 5: Commit changes**

---

### Task 2: Implement Adaptive K-Opt Splice in StemCyclePatcher (Direction 2)

**Files:**
- Modify: `src/cegar-fix/src/stem_cycle_patcher.rs`
- Test: `src/cegar-fix/src/stem_cycle_patcher.rs` (unit tests)

**Interfaces:**
- Consumes: `Graph`, `Degree2Contractor`, `HubRegistry`
- Produces: `StemCyclePatcher::solve_via_stem_and_cycle`

- [x] **Step 1: Write the unit test for k-opt splice**
- [x] **Step 2: Run test to verify failure**
- [x] **Step 3: Implement `k_opt_splice` inside `stem_cycle_patcher.rs`**
- [x] **Step 4: Run test to verify passing**
- [x] **Step 5: Commit changes**

---

### Task 3: Comprehensive 100-Graph Benchmark & Final Packaging (Direction 3)

**Files:**
- Create: `scratch/run_100_benchmark.py`
- Test: 10 Key Regressions + Dense Hub profiles + 100 random FHCP graphs

- [x] **Step 1: Run 10 Key Regression Graphs to verify 10/10 SAT**
- [x] **Step 2: Run 100-Graph broad benchmark and measure pass rate and speedups**
- [x] **Step 3: Compile full benchmark report**
- [x] **Step 4: Commit benchmark report**
