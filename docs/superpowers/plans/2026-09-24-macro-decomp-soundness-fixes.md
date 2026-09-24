# Macro Decomposition Soundness and Generalization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate false negatives and semantic unsoundness identified in the adversarial audit of `src/cegar-fix/src/macro_decomp/`, specifically fixing degree-2 separator exclusion and small-component filtering in `corridor.rs`, coverage validation in `portfolio_788.rs`, and small-graph hub-bound contradictions in `dynamic_bipartite.rs`.

**Architecture:** 
1. In `corridor.rs`: allow $\deg(u) \ge 2$ separator evaluation, remove `min_profitable` filtering on return values (keeping it purely for early search termination), and guard chordless square generation with `rem_list.len() > 4`.
2. In `portfolio_788.rs`: add an exact vertex coverage check `color.len() + chain_vertices == N` in `extract_alternating_pairs` so it only claims positive when the entire graph is covered by the alternating pair model.
3. In `dynamic_bipartite.rs`: fix the $N < 100$ contradiction in super-hub count bounds, protect cluster vertices from connector flooding in sparse subgraphs, and clarify module terminology.
4. Verify all fixes with regression test cases and re-verify the 11 challenge graphs.

**Tech Stack:** Rust 2021, CaDiCaL SAT solver (`rustsat`, `rustsat_cadical`), Rayon.

**Spec:** Clean-Room Adversarial Audit Report (Session Transcript / Section 1-8).

## Global Constraints
- Pure Rust codebase in `src/cegar-fix/`.
- Zero compiler warnings or errors (`cargo check -p cegar-fix`).
- All 11 benchmark challenge graphs must remain 100% verified.
- Follow Test-Driven Development (TDD): write failing test first, verify failure, implement fix, verify pass.

## Review Focus
1. 2-cuts where both separator vertices have degree 2 must be found and solved.
2. 2-cuts with asymmetric components of size $1 \le |C| \le 9$ must be returned, not discarded.
3. Block B with $\le 4$ vertices must not be rendered UNSAT by chordless square clauses.
4. Alternating pair decomposition must reject graphs where vertices remain outside the alternating pair cover.
5. All 11 certified benchmark graphs (`run_11_solved.sh`) must maintain 100% validity.

---

### Task 1: Fix `corridor.rs` Degree-2 Separators and Component Filtering

**Files:**
- Modify: `src/cegar-fix/src/macro_decomp/corridor.rs`
- Test: `src/cegar-fix/tests/macro_decomp_audit.rs`

**Interfaces:**
- `macro_corridor::find_2cut_ports(&Graph) -> Option<(i32, i32)>`
- `macro_corridor::find_2cut_ports_with_deadline(&Graph, Option<Instant>, usize) -> Option<(i32, i32)>`
- `macro_corridor::solve_2cut_corridor(&Graph, f64) -> Option<Vec<i32>>`

- [ ] **Step 1: Write failing tests for degree-2 separator and asymmetric small cut**
In `src/cegar-fix/tests/macro_decomp_audit.rs`, add tests:
  - `test_deg2_separator_ports`: graph where unique 2-cut has $\deg(u)=\deg(v)=2$.
  - `test_asymmetric_small_component_returned`: graph where components have size 6 and 30.

- [ ] **Step 2: Run test to verify failure**
Run: `source /root/.cargo/env && cargo test --test macro_decomp_audit test_deg2_separator_ports test_asymmetric_small_component_returned`
Expected: FAIL.

- [ ] **Step 3: Modify `corridor.rs` to allow degree-2 candidates and remove return filtering**
In `src/cegar-fix/src/macro_decomp/corridor.rs`:
- Line 614: change `if deg_u < 3 { continue; }` to `if deg_u < 2 { continue; }`.
- Line 609: change `return best.filter(|&(_, _, min_c)| min_c >= min_profitable).map(|(u, v, _)| (u, v));` to `return best.map(|(u, v, _)| (u, v));`.
- Line 683: change `best.filter(|&(_, _, min_c)| min_c >= min_profitable).map(|(u, v, _)| (u, v))` to `best.map(|(u, v, _)| (u, v))`.
- Line 356: add `if rem_list.len() > 4` around static chordless squares loop in `solve_block_b`.

- [ ] **Step 4: Run tests to verify pass**
Run: `source /root/.cargo/env && cargo test --test macro_decomp_audit`
Expected: PASS.

- [ ] **Step 5: Commit changes**
```bash
git add src/cegar-fix/src/macro_decomp/corridor.rs src/cegar-fix/tests/macro_decomp_audit.rs
git commit -m "fix(corridor): allow degree-2 separators, remove return filter, guard 4-cycles"
```

---

### Task 2: Fix `portfolio_788.rs` Graph Coverage Validation

**Files:**
- Modify: `src/cegar-fix/src/macro_decomp/portfolio_788.rs`
- Test: `src/cegar-fix/tests/macro_decomp_audit.rs`

**Interfaces:**
- `macro_788::can_solve_alternating_pairs(&Graph) -> bool`
- `macro_788::solve_alternating_pairs(&Graph, f64) -> Option<Vec<i32>>`

- [ ] **Step 1: Write failing test for partial coverage in `portfolio_788`**
In `src/cegar-fix/tests/macro_decomp_audit.rs`, add `test_portfolio_partial_coverage_rejected`:
Create a graph where 20% of vertices are degree-2 alternating chains, but 80% form a non-chain connected subgraph. Verify `can_solve_alternating_pairs` returns `false` because it does not cover the whole graph.

- [ ] **Step 2: Run test to verify failure**
Run: `source /root/.cargo/env && cargo test --test macro_decomp_audit test_portfolio_partial_coverage_rejected`
Expected: FAIL (`can_solve_alternating_pairs` currently returns `true`).

- [ ] **Step 3: Implement total coverage check in `extract_alternating_pairs`**
In `src/cegar-fix/src/macro_decomp/portfolio_788.rs`:
Compute the total number of vertices represented by the alternating pair model:
- `pairs.len() * 2` endpoints.
- Total interior degree-2 chain vertices contracted by `contractor`.
If `total_covered != raw_g.adjacency_list.len()`, return `None`.

- [ ] **Step 4: Run tests to verify pass**
Run: `source /root/.cargo/env && cargo test --test macro_decomp_audit`
Expected: PASS.

- [ ] **Step 5: Commit changes**
```bash
git add src/cegar-fix/src/macro_decomp/portfolio_788.rs src/cegar-fix/tests/macro_decomp_audit.rs
git commit -m "fix(portfolio_788): enforce complete vertex coverage for alternating pairs"
```

---

### Task 3: Fix `dynamic_bipartite.rs` Hub Bounds & Dead Constants

**Files:**
- Modify: `src/cegar-fix/src/macro_decomp/dynamic_bipartite.rs`
- Test: `src/cegar-fix/tests/macro_decomp_audit.rs`

**Interfaces:**
- `dynamic_bipartite::can_solve_bipartite(&Graph) -> bool`
- `dynamic_bipartite::detect_and_partition(&Graph) -> Option<BipartitePartition>`

- [ ] **Step 1: Write test for $N \in [50, 99]$ graphs with valid hubs**
In `src/cegar-fix/tests/macro_decomp_audit.rs`, add `test_dynamic_bipartite_small_n`:
Construct an $N = 60$ graph with 2 hubs of degree 35, and ensure it is not rejected solely by an impossible $k \le 0.02 N$ upper bound.

- [ ] **Step 2: Run test to verify failure**
Run: `source /root/.cargo/env && cargo test --test macro_decomp_audit test_dynamic_bipartite_small_n`
Expected: FAIL.

- [ ] **Step 3: Modify `detect_and_partition` hub upper bound**
In `src/cegar-fix/src/macro_decomp/dynamic_bipartite.rs`:
Change `let max_k = ((n as f64) * MAX_HUB_FRACTION).max(2.0) as usize;` so that $k=2$ hubs are allowed for any $N \ge \text{MIN\_GRAPH\_SIZE} = 50$.

- [ ] **Step 4: Run tests to verify pass**
Run: `source /root/.cargo/env && cargo test --test macro_decomp_audit`
Expected: PASS.

- [ ] **Step 5: Commit changes**
```bash
git add src/cegar-fix/src/macro_decomp/dynamic_bipartite.rs src/cegar-fix/tests/macro_decomp_audit.rs
git commit -m "fix(dynamic_bipartite): allow small hub counts for N in 50..100"
```

---

### Task 4: Full Benchmark Regression & Certification Verification

**Files:**
- Execute: `./run_11_solved.sh`

- [ ] **Step 1: Run `./run_11_solved.sh`**
Run the 11 benchmark challenge graphs (710, 717, 746, 788, 868, 882, 950, 982, 990, 4066, comp0).
Expected: 11 / 11 SOLVED and 100% tour-verified.

- [ ] **Step 2: Run full integration test suite**
Run: `source /root/.cargo/env && cargo test --test macro_decomp_audit`
Expected: All tests pass.

- [ ] **Step 3: Verify clean working tree**
Run: `git status`
Expected: Clean working tree.
