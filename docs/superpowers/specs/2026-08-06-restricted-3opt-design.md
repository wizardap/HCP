# Design Document: Restricted 3-Opt Subcycle Merging in CEGAR HCP Solver

**Date**: 2026-08-06  
**Status**: Approved  

---

## 1. Overview & Objective

The current Hamiltonian Cycle Problem (HCP) solver in `cegar-ffi` uses a `2-opt` heuristic to merge subcycles returned by the SAT solver during the CEGAR loop. While `2-opt` merges pairs of subcycles when two valid reconnecting edges exist, it gets stuck when reconnecting three distinct subcycles requires a 3-edge swap.

This document specifies the design for **Restricted 3-opt**, a targeted heuristic that attempts to merge triplets of subcycles when `2-opt` reaches a local optimum (i.e. can no longer merge any 2 subcycles).

---

## 2. CLI Options (`options.rs` & `main.rs`)

A new CLI option `--three-opt` (short `-x`) will be added to `options.rs`.

- **Flag**: `--three-opt <n>` / `-x <n>`
- **Values**:
  - `0`: Disabled (default).
  - `1`: Enabled (runs Restricted 3-opt fallback whenever 2-opt cannot make further merges).

`main.rs` and `hcp_solver.rs` signatures will be updated to pass `three_opt: i32` into `solve_hamilton` and `cegar`.

---

## 3. Algorithm & Architecture (`hcp_solver.rs`)

### 3.1 Execution Flow

```
[SAT Solver Returns Subcycles]
            │
            ▼
    ┌───────────────┐
    │  Run 2-Opt    │◄──── Loop while 2-cycles can be merged
    └───────┬───────┘
            │
            ├─► Merged into 1 Cycle? ──► [SATISFIABLE Output]
            │
    (2-opt Stuck)
            │
            ▼
   Is three_opt == 1 && active_cycles >= 3 ?
       ├── YES ──► [Run Restricted 3-Opt]
       │                 │
       │                 ├─► Successfully Merged 3 cycles? ──► Re-try 2-Opt loop
       │                 └─► Cannot merge further ───────────► Proceed to Block Clauses
       │
       └── NO ───► Proceed to Block Clauses
```

### 3.2 3-Opt Swap Mechanism (`swap_three_nodes` & `cycle_join_three`)

1. **Selection**: Iterate over active subcycle triplets $(C_1, C_2, C_3)$.
2. **Edge Matching**: For edges $(u_1, v_1) \in C_1$, $(u_2, v_2) \in C_2$, $(u_3, v_3) \in C_3$:
   - Check if reconnecting edges exist in graph $G$'s adjacency list for cyclic order $C_1 \to C_2 \to C_3 \to C_1$:
     - E.g., $(u_1, v_2) \in G$, $(u_2, v_3) \in G$, $(u_3, v_1) \in G$.
3. **Joining**: Reconstruct the merged single vector representation of the combined cycle.

---

## 4. Verification & Benchmarking Plan

1. **Build Verification**: Run `cargo check` and `cargo build` in `src/cegar-ffi`.
2. **Functional Test**: Execute `cegar-ffi` on benchmark graph instances with `-t 1 --three-opt 1` vs `-t 1 --three-opt 0`.
3. **Metrics**: Compare number of subcycles merged, CEGAR increment iterations, and total solving time.
