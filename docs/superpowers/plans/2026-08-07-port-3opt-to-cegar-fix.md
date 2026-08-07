# Plan: Port Candidate-Graph 3-Opt to cegar-fix

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Port Candidate-Graph Restricted 3-opt subcycle merging heuristic into `cegar-fix`.

## Tasks

- [ ] **Task 1: Add `--three-opt` CLI Flag in `cegar-fix`**
  - Files: `src/cegar-fix/src/options.rs`, `src/cegar-fix/src/main.rs`
  - Action: Add `three-opt` (short `-x`) to CLI options in `options.rs`, parse `three_opt` in `main.rs` with default 0.

- [ ] **Task 2: Thread `three_opt` Parameter in `cegar-fix`**
  - Files: `src/cegar-fix/src/hcp_solver.rs`, `src/cegar-fix/src/main.rs`
  - Action: Update `solve_hamilton`, `cegar`, and `two_opt` signatures in `hcp_solver.rs` to accept `three_opt: i32`. Update call in `main.rs`.

- [ ] **Task 3: Implement 3-Opt Core & Candidate Graph in `cegar-fix`**
  - Files: `src/cegar-fix/src/hcp_solver.rs`
  - Action: Implement `swap_three_nodes`, `cycle_join_three`, `merge_three_cycles` with candidate graph filtering.

- [ ] **Task 4: Integrate 3-Opt Fallback into `two_opt` Loop in `cegar-fix`**
  - Files: `src/cegar-fix/src/hcp_solver.rs`
  - Action: Update `while merged` loop in `two_opt` to invoke `merge_three_cycles` when `!merged && three_opt == 1 && active_cycles_number.len() >= 3`.

- [ ] **Task 5: Build and Verify `cegar-fix`**
  - Files: N/A
  - Action: Run `cargo check` and `cargo build --release` in `src/cegar-fix/`. Run benchmark on `graph12.col` and `graph470.col`.
