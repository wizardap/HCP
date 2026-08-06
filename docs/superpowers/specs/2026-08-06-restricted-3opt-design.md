# Design Document: Restricted 3-Opt Subcycle Merging in CEGAR HCP Solver

**Date**: 2026-08-06  
**Status**: Approved (v2 — updated after review)

---

## 1. Overview & Objective

The current Hamiltonian Cycle Problem (HCP) solver in `cegar-ffi` uses a `2-opt` heuristic to merge subcycles returned by the SAT solver during the CEGAR loop. While `2-opt` merges pairs of subcycles when two valid reconnecting edges exist, it gets stuck when reconnecting three distinct subcycles requires a 3-edge swap.

This document specifies the design for **Restricted 3-opt**, a targeted fallback heuristic that triggers **inside the `two_opt()` loop** whenever `merge_cycles()` cannot merge any 2 subcycles further, and attempts to merge triplets of subcycles via a 3-edge swap.

---

## 2. CLI Options (`options.rs` & `main.rs`)

A new CLI option `--three-opt` (short `-x`) will be added to `options.rs`.

- **Flag**: `--three-opt <n>` / `-x <n>`
- **Values**:
  - `0`: Disabled (default).
  - `1`: Enabled — runs Restricted 3-opt as inline fallback inside `two_opt()` when `merge_cycles()` fails.

### Updated Function Signatures

All functions that previously received `opt: i32` (for 2-opt) will also receive a new `three_opt: i32` parameter:

| Function | Current Signature | Updated Signature |
|---|---|---|
| `solve_hamilton` | `(..., opt: i32, ...)` | `(..., opt: i32, three_opt: i32, ...)` |
| `cegar` | `(..., opt: i32, ...)` | `(..., opt: i32, three_opt: i32, ...)` |
| `two_opt` | `(..., opt: i32)` | `(..., opt: i32, three_opt: i32)` |

`main.rs` reads `three_opt` from the CLI and passes it through the call chain.

---

## 3. Algorithm & Architecture (`hcp_solver.rs`)

### 3.1 Execution Flow

3-opt is triggered **inline inside `two_opt()`**, not as a separate post-processing step. When `merge_cycles()` fails (returns `merged = false`) and `three_opt == 1` and there are ≥ 3 active cycles, the loop attempts `merge_three_cycles()` before giving up and moving to block clauses.

```
[SAT Solver Returns Subcycles C1..Ck]
            │
            ▼
    ┌────────────────────────────────────────┐
    │      while merged (loop in two_opt)    │
    │                                        │
    │  merge_cycles() ──► Success?           │
    │       │YES: add new merged cycle,      │
    │       │     update active list,        │
    │       │     continue loop              │
    │       │                                │
    │       │NO: (2-opt stuck)               │
    │       │                                │
    │       └─► three_opt==1 && active≥3 ?  │
    │                │YES                    │
    │                ▼                       │
    │       merge_three_cycles()             │
    │                │                       │
    │          Success? ─► add new cycle,    │
    │                │     reset loop        │
    │                │                       │
    │          Fail? ──► break loop          │
    └────────────────────────────────────────┘
            │
            ├─► active_cycles == 1 ? ──► [SATISFIABLE Output]
            │
            └─► active_cycles > 1  ? ──► Proceed to Block Clauses
```

### 3.2 New Functions

#### `merge_three_cycles()` — Orchestrator (mirrors `merge_cycles()`)

```rust
fn merge_three_cycles(
    cycles: &Vec<Vec<i32>>,
    g: &Graph,
    active_cycles_number: &Vec<usize>,
) -> (bool, (usize, usize, usize), Vec<i32>)
```

- Iterates over active triplets $(C_i, C_j, C_k)$.
- For each triplet, calls `swap_three_nodes()`.
- Returns: `(merged, (i, j, k), new_cycle)`.

#### `swap_three_nodes()` — Edge Matching for Directed Graphs

The graph is **directed**. For triplet $(C_1, C_2, C_3)$, three edges are removed — one per cycle — and three new edges are added in one of the valid **directed reconnection configurations** below. All edge checks use `adjacency_list`.

For cut positions $i \in C_1$, $j \in C_2$, $k \in C_3$, denote:
- $u_1 = C_1[i]$, $v_1 = C_1[(i+1) \% |C_1|]$
- $u_2 = C_2[j]$, $v_2 = C_2[(j+1) \% |C_2|]$
- $u_3 = C_3[k]$, $v_3 = C_3[(k+1) \% |C_3|]$

**Valid reconnection configurations to try** (all must use edges that exist in $G$):

| Config | New edges added | Description |
|---|---|---|
| A | $(u_1 \to v_2),\,(u_2 \to v_3),\,(u_3 \to v_1)$ | Cyclic forward: $C_1 \to C_2 \to C_3 \to C_1$ |
| B | $(u_1 \to v_3),\,(u_3 \to v_2),\,(u_2 \to v_1)$ | Cyclic forward: $C_1 \to C_3 \to C_2 \to C_1$ |

> Each configuration is tried for all $(i, j, k)$ positions. On the first match found, return the configuration and positions for joining. No exhaustive search needed — first-fit suffices.

#### `cycle_join_three()` — Cycle Reconstruction

Merges three cycle vectors into one contiguous cycle given the cut positions $(i, j, k)$ and the configuration (A or B). Returns `Option<Vec<i32>>`.

---

## 4. Verification & Benchmarking Plan

1. **Build Verification**: `cargo check && cargo build` in `src/cegar-ffi`.
2. **Functional Test**: Run on benchmark instances from `data/` with:
   - Baseline: `-t 1 --three-opt 0`
   - With 3-opt: `-t 1 --three-opt 1`
3. **Metrics to compare**:
   - `incremented number` (CEGAR loop count)
   - `number of connected cycles` / `number of merged cycles`
   - Total solving time (`overall time`)
