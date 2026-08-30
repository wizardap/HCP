# Continuous Incremental CNF Subsumer Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement continuous every-round and fast-trigger CNF subsumption in `hcp_solver.rs` to keep SAT solving times bounded under 30s across all CEGAR rounds.

**Architecture:** Modify `src/cegar-fix/src/hcp_solver.rs` and `src/cegar-fix/tests/test_staged_solver.rs`.

**Tech Stack:** Rust (2021 edition).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.
- Empirical Rigor: No overpromising. Maintain strict verification.

---

### Task 1: Continuous Reseeding & Subsumption in `hcp_solver.rs`

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs`

- [ ] **Step 1: Set reseeder threshold to `sat_solving_time >= 15.0s || count % 3 == 0 || accumulated_cut_cnfs.len() >= 10`**
- [ ] **Step 2: Commit changes**

---

### Task 2: Integration Tests & Workspace Verification

**Files:**
- Modify: `src/cegar-fix/tests/test_staged_solver.rs`

- [ ] **Step 1: Verify `test_cegar_solver_reseeder_integration` and full workspace test suite** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 2: Commit changes**

---

### Task 3: Benchmark Verification on `graph479.col` & `graph668.col`

**Files:**
- Verify: `FHCPCS-col/graph479.col` and `FHCPCS-col/graph668.col`

- [ ] **Step 1: Build release binary** (`taskset -c 0,1,2 nice -n 19 cargo build --release`)
- [ ] **Step 2: Run full workspace test suite** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 3: Run benchmark on `graph479.col` and `graph668.col`**
- [ ] **Step 4: Document results and commit**
