# Multi-Macro Gadget Interface Engine & Full-Spectrum Late-Stage Cut Selector Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend `GadgetInterfaceParityEngine` invocation to all macro-cycles ($|C| \ge |V| / 10$) and enforce 100% full-spectrum cut coverage when $\le 50$ subcycles remain.

**Architecture:** Modify `src/cegar-fix/src/hcp_solver.rs` and `src/cegar-fix/tests/test_staged_solver.rs`.

**Tech Stack:** Rust (2021 edition).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.
- Empirical Rigor: No overpromising. Maintain strict verification.

---

### Task 1: Multi-Macro Gadget & Full-Spectrum Cut Selector in `hcp_solver.rs`

**Files:**
- Modify: `src/cegar-fix/src/hcp_solver.rs`

- [ ] **Step 1: Iterate over all macro-cycles ($|C| \ge |V| / 10$) in Gadget Interface Parity stage**
- [ ] **Step 2: When `_active_cycles.len() <= 50`, select 100% of non-giant subcycles for SEC clauses and boundary cuts**
- [ ] **Step 3: Commit changes**

---

### Task 2: Integration Tests & Workspace Verification

**Files:**
- Modify: `src/cegar-fix/tests/test_staged_solver.rs`

- [ ] **Step 1: Add integration test `test_cegar_multi_macro_gadget_integration` in `test_staged_solver.rs`**
- [ ] **Step 2: Run full workspace test suite** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 3: Commit changes**

---

### Task 3: Benchmark Verification on `graph479.col` & `graph668.col`

**Files:**
- Verify: `FHCPCS-col/graph479.col` and `FHCPCS-col/graph668.col`

- [ ] **Step 1: Build release binary** (`taskset -c 0,1,2 nice -n 19 cargo build --release`)
- [ ] **Step 2: Run full workspace test suite** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 3: Run benchmark on `graph479.col` and `graph668.col`**
- [ ] **Step 4: Document results and commit**
