# Spec: Port Candidate-Graph Restricted 3-Opt to cegar-fix

**Date**: 2026-08-07  
**Status**: Approved  

---

## 1. Goal
Port the candidate-graph optimized Restricted 3-opt subcycle merging heuristic from `cegar-ffi` into the official project solver `cegar-fix`.

## 2. Requirements

1. **CLI Flag**:
   - Add `--three-opt` / `-x` option in `src/cegar-fix/src/options.rs`.
   - Default value: 0.

2. **Parameter Threading**:
   - Update `solve_hamilton`, `cegar`, and `two_opt` in `src/cegar-fix/src/hcp_solver.rs` and `main.rs` to accept `three_opt: i32`.

3. **Core Heuristic Functions (`hcp_solver.rs`)**:
   - `swap_three_nodes(c1, c2, c3, g)`: Check Config A ($u_1 \to v_2, u_2 \to v_3, u_3 \to v_1$) and Config B ($u_1 \to v_3, u_3 \to v_2, u_2 \to v_1$) using `adjacency_list`.
   - `cycle_join_three(c1, c2, c3, i, j, k, config)`: Reconstruct cycle with safe array boundary indexing (`c2[j+1..]`, `c2[..=j]`, etc.).
   - `merge_three_cycles(cycles, encoder, g, block_method, balanced, active_cycles_number)`: Candidate graph filtering using `vertex_to_active` and `cycle_neighbors`.

4. **Integration in `two_opt`**:
   - Inside `while merged` in `two_opt`, when `!merged && three_opt == 1 && active_cycles_number.len() >= 3`, trigger `merge_three_cycles`.
   - On success:
     - Push new cycle.
     - `swap_remove` 3 merged active cycle indices in descending order.
     - Push new active cycle index.
     - Reset `cache_vertex.clear()` and set `merged = true`.

## 3. Verification
- `cargo check` and `cargo build --release` in `src/cegar-fix/`.
- Benchmark test runs on `graph12.col` and `graph470.col`.
