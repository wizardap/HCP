# Task 2 Brief: Wire `MetagraphRouter` into Base Encoding in `hcp_solver.rs`

## Overview
Wire `MetagraphRouter` into `solve_hamilton` in `src/cegar-fix/src/hcp_solver.rs` to detect gadget supernodes and inject supernode MTZ order constraints directly into `cnf` at Round 0.

## Global Constraints
- Target directory: `/home/ubuntu/HCP/src/cegar-fix`
- Core Reservation: Commands use `taskset -c 0,1,2 nice -n 19` (Core 3 reserved for user).
- Zero Tour Injection: Absolutely NO importing, reading, or referencing `.hcp.tou` files during solving.

## Requirements & Interfaces

### 1. File Structure
- Modify: `src/cegar-fix/src/hcp_solver.rs`
- Test: `src/cegar-fix/tests/test_staged_solver.rs`

### 2. Implementation Details
In `src/cegar-fix/src/hcp_solver.rs`:
- Import `use crate::metagraph_router::MetagraphRouter;`
- In `solve_hamilton` (around line 282, after `StaticCycleCutter`):
  ```rust
  // Metagraph Router: detect gadget supernodes and inject Supernode MTZ constraints
  let modules = MetagraphRouter::detect_gadget_modules(&g);
  if modules.len() >= 3 && modules.len() <= 120 {
      let pre_clauses = cnf.len();
      MetagraphRouter::encode_supernode_mtz(&modules, &g, &mut encoder, &mut cnf);
      let mtz_clauses = cnf.len() - pre_clauses;
      println!("MetagraphRouter: detected {} supernode modules, injected {} supernode MTZ clauses at Round 0", modules.len(), mtz_clauses);
  }
  ```

### 3. Integration Test
- Add `test_cegar_metagraph_router_integration` in `src/cegar-fix/tests/test_staged_solver.rs`.
