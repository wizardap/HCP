# Plan: Adaptive Stall-based Escalation Strategy in cegar-fix

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement adaptive multi-tier escalation strategy (`--adaptive-escalation` / `-A`) in `cegar-fix` to keep solving times ultra-fast (< 0.2s) on easy/medium graphs while dynamically escalating to 3-opt, Hard Fallback, and Partial MTZ on stalled instances.

## Tasks

- [ ] **Task 1: Add `--adaptive-escalation` CLI Flag and Thread Parameter**
  - Files: `src/cegar-fix/src/options.rs`, `src/cegar-fix/src/main.rs`, `src/cegar-fix/src/hcp_solver.rs`
  - Action: Add `adaptive-escalation` flag in `options.rs` (short `-A`), parse in `main.rs`, and thread `adaptive_escalation: i32` into `solve_hamilton`, `cegar`, and `two_opt`.

- [ ] **Task 2: Implement Dynamic Escalation Logic in `two_opt` & `cegar`**
  - Files: `src/cegar-fix/src/hcp_solver.rs`
  - Action:
    - Track effective escalation level (Level 0, Level 1, Level 2) based on `stall_count`.
    - If `adaptive_escalation == 1`:
      - Level 0: Disable 3-opt (`three_opt = 0`), disable Hard Fallback (`cegar_fallback = 0`), disable MTZ (`mtz_stall = 0`).
      - Level 1 (`stall_count >= 3`): Enable 3-opt (`three_opt = 1`).
      - Level 2 (`stall_count >= 6`): Enable Hard Fallback (`cegar_fallback = 1`) and Partial MTZ (`mtz_stall = 1`).

- [ ] **Task 3: Build, Benchmark and Validate**
  - Files: N/A
  - Action:
    - Run `cargo build --release` in `src/cegar-fix/`.
    - Verify `graph12.col` finishes in **< 0.2s** (Level 0 speed).
    - Verify `graph339.col` automatically escalates to Level 1 & Level 2 when stalled.
