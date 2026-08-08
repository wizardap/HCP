# Spec: Adaptive Stall-based Escalation Strategy for CEGAR HCP Solver

**Date**: 2026-08-08  
**Status**: Approved  

---

## 1. Goal
Implement an adaptive multi-tier escalation strategy (`--adaptive-escalation`) in `cegar-fix` to maintain ultra-fast solving times (< 0.2s) on easy/medium graphs while dynamically escalating to Restricted 3-opt, CEGAR Hard Blocking Fallback, and Partial MTZ only when local search stalls on hard/timeout instances.

## 2. Architecture & Escalation Levels

| Level | Mode Name | Trigger Condition | Active Components |
|---|---|---|---|
| **Level 0** | **Baseline Standard** | Iterations 0+ (Initial state) | 2-opt + ASP Cut-Set (`-b 3`). Clause addition is lightweight. |
| **Level 1** | **Restricted 3-Opt** | Stalled for $K_1$ iterations (default: 3) | 2-opt + Restricted 3-opt (Candidate Graph Filtering). |
| **Level 2** | **Hard Fallback & MTZ** | Stalled for additional $K_2$ iterations (default: 3) | 2-opt + 3-opt + Hard Blocking Fallback (`block_method = 0`) + Partial MTZ (for subcycles $\le 100$). |

### Transition Rules
- If `remaining_cycle_count` decreases: reset `stall_count = 0` (or maintain current effective level if progressing).
- If `remaining_cycle_count` does not decrease: `stall_count += 1`.
- When `stall_count >= 3` at Level 0: escalate to Level 1.
- When `stall_count >= 6` at Level 1: escalate to Level 2.

## 3. CLI & Options Interface

- `--adaptive-escalation` / `-A`:
  - `0`: Disabled (manual flag control).
  - `1`: Enabled (default adaptive mode).

## 4. Expected Impact
- Easy instances (`graph12`, `graph13`, etc.): Finish at Level 0 in **< 0.2 seconds** without clause bloat.
- Hard/Timeout instances (`graph339`, `graph470`, etc.): Automatically escalate to Level 1 & 2 to break local optima traps.
