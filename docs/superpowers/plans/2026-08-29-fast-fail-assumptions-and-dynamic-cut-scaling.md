# Fast-Fail Assumptions & Dynamic Cut Scaling Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Accelerate CEGAR solving on non-bipartite and large challenge instances (`graph668.col`) by scaling the cut selection budget to 100 cuts per round for small subcycles ($\le 16$) and applying fast-fail conflict limits to assumptions.

**Architecture:** Enhancements to `CutSelector` and `hcp_solver.rs`.

**Tech Stack:** Rust (2021 edition), CaDiCaL SAT solver (`rustsat`, `rustsat_cadical`).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.

---

### Task 1: Dynamic Cut Budget Scaling in `CutSelector`

**Files:**
- Modify: `src/cegar-fix/src/cut_selector.rs`
- Test: `src/cegar-fix/tests/test_cut_selector.rs`

**Interfaces:**
- Enhance `CutSelectorOptions`:
  ```rust
  #[derive(Debug, Clone)]
  pub struct CutSelectorOptions {
      pub max_cycle_len_threshold: usize, // Default: 64
      pub base_max_cuts: usize,           // Default: 40
      pub high_volume_max_cuts: usize,    // Default: 100
      pub tiny_cycle_boundary_len: usize, // Default: 8
  }
  ```

- [ ] **Step 1: Write unit tests** in `src/cegar-fix/tests/test_cut_selector.rs`
- [ ] **Step 2: Run test to verify it fails** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_cut_selector`)
- [ ] **Step 3: Implement dynamic cut budget scaling in `src/cegar-fix/src/cut_selector.rs`**
- [ ] **Step 4: Run test to verify it passes** (`taskset -c 0,1,2 nice -n 19 cargo test --test test_cut_selector`)
- [ ] **Step 5: Commit changes**

---

### Task 2: Fast-Fail Assumption Conflict Limiting in `hcp_solver.rs`

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_staged_solver.rs`

**Interfaces:**
- In `cegar` solving loop:
  - If `!assumptions.is_empty()`:
    - Set conflict budget limit on solver (e.g. 5,000 conflicts via `solver.limit(...)` or timed fallback).
    - If solver is interrupted or returns UNSAT, immediately clear assumptions and fall back to `solver.solve()`.

- [ ] **Step 1: Implement fast-fail assumption execution in `src/cegar-fix/src/hcp_solver.rs`**
- [ ] **Step 2: Run full workspace test suite** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 3: Commit changes**

---

### Task 3: Benchmark Verification on `graph668.col`

**Files:**
- Verify: `FHCPCS-col/graph668.col`

- [ ] **Step 1: Build release binary** (`taskset -c 0,1,2 nice -n 19 cargo build --release`)
- [ ] **Step 2: Run full workspace test suite** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 3: Run benchmark on `graph668.col` and verify subcycle reduction rate and latency**
- [ ] **Step 4: Document results and commit**
