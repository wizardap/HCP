# Spec: CEGAR Hard Blocking Fallback for Eliminating Timeouts in cegar-fix

**Date**: 2026-08-07  
**Status**: Approved  

---

## 1. Goal
Implement CEGAR Hard Blocking Fallback in `cegar-fix` to prevent SAT Solver infinite assignment loops on stuck local optima, specifically targeting the 75 timeout testcases in the `FHCPCS-col` benchmark set.

## 2. Technical Design

1. **CLI Flag (`options.rs` & `main.rs`)**:
   - Add CLI parameter `--cegar-fallback` (short `-f`, long `"cegar-fallback"`, value_name `"n"`, default `0`).
   - `0`: Disabled (default).
   - `1`: Enabled (forces `block_method = 0` CEGAR hard blocking clauses when local search stalls).

2. **Parameter Threading (`hcp_solver.rs` & `main.rs`)**:
   - Pass `cegar_fallback: i32` through `solve_hamilton -> cegar -> two_opt`.

3. **Fallback Logic in `two_opt` (`hcp_solver.rs`)**:
   - When `two_opt` finishes merging and returns active cycles:
   ```rust
   if opt == 3 {
       let active_cycles = get_active_cycles(&cycles, &active_cycles_number);
       block_clauses.extend(get_blocking_clauses(&active_cycles, encoder, g, block_method, balanced));
       if cegar_fallback == 1 {
           block_clauses.extend(get_blocking_clauses(&active_cycles, encoder, g, 0, balanced));
       }
   }
   ```

## 3. Verification
- Build release binary in `src/cegar-fix/`.
- Run benchmark on `FHCPCS-col/graph339.col` with `--three-opt 1 --cegar-fallback 1` to verify elimination of the 30-minute timeout.
