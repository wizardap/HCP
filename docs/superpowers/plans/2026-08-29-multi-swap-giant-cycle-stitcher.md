# Multi-Swap Giant-Cycle & Simultaneous Multi-Cycle SAT Stitcher Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement multi-swap support in `MacroCycleStitcher` and `GiantCycleStitcher`, removing the rigid 1-swap-per-cycle limit on giant cycles ($|C| \ge 50$) to allow simultaneous multi-cycle absorption in $< 5\text{ms}$.

**Architecture:** Enhance `src/cegar-fix/src/macro_cycle_stitcher.rs` and `src/cegar-fix/src/giant_cycle_stitcher.rs`.

**Tech Stack:** Rust (2021 edition), CaDiCaL SAT solver (`rustsat`, `rustsat_cadical`).

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Core 3 is strictly reserved for the user. Run all tasks and commands with `taskset -c 0,1,2 nice -n 19`.
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.
- Benchmark Time Limit: $T_{\max} = 1800\text{s}$.

---

### Task 1: Multi-Swap Enhancements in `MacroCycleStitcher` & `GiantCycleStitcher`

**Files:**
- Modify: `src/cegar-fix/src/macro_cycle_stitcher.rs`, `src/cegar-fix/src/giant_cycle_stitcher.rs`
- Test: `src/cegar-fix/tests/test_macro_cycle_stitcher.rs`, `src/cegar-fix/tests/test_giant_cycle_stitcher.rs`

- [ ] **Step 1: In `macro_cycle_stitcher.rs`, remove single-swap AMO on giant cycles ($|C| \ge 50$) and scale `max_swaps` up to 16/32**
- [ ] **Step 2: In `giant_cycle_stitcher.rs`, allow simultaneous multi-swap absorption into giant cycle**
- [ ] **Step 3: Write tests verifying multi-swap simultaneous absorption**
- [ ] **Step 4: Run unit tests** (`taskset -c 0,1,2 nice -n 19 cargo test`)
- [ ] **Step 5: Commit changes**

---

### Task 2: Integration & Workspace Tests

**Files:**
- Modify: `src/cegar-fix/tests/test_staged_solver.rs`

- [ ] **Step 1: Add integration test verifying multi-swap absorption in full CEGAR loop**
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
