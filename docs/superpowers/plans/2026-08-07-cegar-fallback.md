# Plan: CEGAR Hard Blocking Fallback in cegar-fix

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement `--cegar-fallback` CLI flag and logic in `cegar-fix` to break infinite SAT solver subcycle loops.

## Tasks

- [ ] **Task 1: Add `--cegar-fallback` CLI Flag and Thread Parameter**
  - Files: `src/cegar-fix/src/options.rs`, `src/cegar-fix/src/main.rs`, `src/cegar-fix/src/hcp_solver.rs`
  - Action: Add `cegar-fallback` flag in `options.rs`, parse in `main.rs`, and thread `cegar_fallback: i32` into `solve_hamilton`, `cegar`, and `two_opt` in `hcp_solver.rs`.

- [ ] **Task 2: Inject Hard Blocking Clauses in `two_opt`**
  - Files: `src/cegar-fix/src/hcp_solver.rs`
  - Action: In `two_opt` return section when `opt == 3`, append `get_blocking_clauses(&active_cycles, encoder, g, 0, balanced)` if `cegar_fallback == 1`.

- [ ] **Task 3: Build, Benchmark and Validate**
  - Files: N/A
  - Action: Run `cargo build --release` in `src/cegar-fix/`. Benchmark `graph339.col` with official flags plus `--three-opt 1 --cegar-fallback 1`.
