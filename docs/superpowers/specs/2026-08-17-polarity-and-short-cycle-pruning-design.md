# Design Document: Polarity Phase Hints & Global Short-Cycle Pruning

**Document ID**: `2026-08-17-polarity-and-short-cycle-pruning-design`  
**Target Systems**: `src/cegar-fix`  
**Status**: APPROVED DESIGN SPEC

---

## 1. Executive Summary & Objective

Dense Hub graphs (`graph560` – `graph684`) stall in CEGAR because:
1. **Micro-Subcycle Proliferation:** Over 50% of initial 2-factor subcycles are trivial 3-node and 4-node cycles.
2. **Cold-Start SAT Resets:** In each CEGAR iteration, CaDiCaL spends 40–150+ seconds blindly searching across 166k clauses, repeatedly destroying 400+ node aggregate cycles constructed by RAM patchers.

This specification designs **Global Short-Cycle Pruning** and **Polarity Phase Hints (via RustSAT `PhaseLit`)** to:
- Pre-emptively forbid all 3-cycles and 4-cycles in the initial CNF encoding.
- Warm-start CaDiCaL by injecting positive polarity phase hints for edges in large aggregate cycles ($|C| \ge 20$), reducing SAT solving time per iteration from ~150s down to < 5s.

---

## 2. Architectural Design

```
                    [ Input Graph G (e.g. 3,311 vertices) ]
                                      │
                                      ▼
                 [ Step 1: Global Short-Cycle Pruning ]
                 Extract all triangles & 4-cycles in G
                 Add static blocking clauses to initial CNF
                                      │
                                      ▼
                 [ Step 2: Initial 2-Factor Solve (Iter 0) ]
                 CaDiCaL solves with 0 short 3/4-cycles
                 Subcycle count drops from ~260 to < 60
                                      │
                                      ▼
                 [ Step 3: Fast RAM Patching Cascade ]
                 Hub + Matching + Chained LK + ILS + Macro
                 Produces large aggregate cycles (e.g. 500+ vertices)
                                      │
                                      ▼
                 [ Step 4: Polarity Phase Hint Injection ]
                 For all edges (u, v) in aggregate cycles:
                     solver.phase_lit(encoder.get_var(u, v))
                                      │
                                      ▼
                 [ Step 5: Accelerated Next CEGAR Solve ]
                 CaDiCaL branches on preserved edges
                 Solving time per iteration: 150s ──► < 5s !
```

---

## 3. Implementation Details

### 3.1 Global Short-Cycle Pruning in `src/cegar-fix/src/hcp_solver.rs`
- Add function `add_global_short_cycle_cuts(g: &Graph, encoder: &Encoder, cnf: &mut Cnf)`:
  - Enumerates all triangles $(u, v, w)$ in $G$ and adds `(!x_uv | !x_vw | !x_wu)` and `(!x_uw | !x_wv | !x_vu)`.
  - Enumerates all 4-cycles $(u, v, w, z)$ in $G$ (for vertices with degree $\le 100$ or chordless 4-cycles) and adds 4-cycle prohibition clauses.
- Enabled by default when graph has $> 500$ vertices or via `--loop 3`.

### 3.2 Polarity Phase Hints in `cegar()` Loop
- Import `use rustsat::solvers::PhaseLit;`.
- In `cegar()` in `src/cegar-fix/src/hcp_solver.rs`, after subcycle merging:
  ```rust
  // Inject polarity phase hints for edges in large cycles
  for cycle in &sol_cycles {
      if cycle.len() >= 10 {
          for i in 0..cycle.len() {
              let u = cycle[i];
              let v = cycle[(i + 1) % cycle.len()];
              if let Some(lit) = encoder.get_var(u, v) {
                  let _ = solver.phase_lit(lit);
              }
          }
      }
  }
  ```

---

## 4. Invariants & Safety Constraints

1. **Strict 100% Mathematical Soundness**:
   - Short-cycle cuts are mathematically sound: no Hamiltonian cycle in $|V| > 4$ can contain a 3-cycle or 4-cycle.
   - `phase_lit()` only sets branching polarity order; CaDiCaL backtracks normally if conflicts arise. Never emits false `s UNSATISFIABLE`.
2. **Degree-2 Contraction Safety**:
   - Respects `contractor.chain_map` and uncontracts cleanly.

---

## 5. Verification & Benchmark Plan

1. **Unit Tests**:
   - `test_global_short_cycle_pruning`: Verifies correct triangle and 4-cycle clause generation.
   - `test_polarity_phase_hints_trait`: Verifies `PhaseLit` invocation on `CaDiCaL`.
2. **Regression Benchmark**:
   - 10 Key Regression Graphs (`graph45` – `graph346`) $\implies 10/10$ `s SATISFIABLE`.
3. **Dense Hub Profiling**:
   - Benchmark on `graph560.col`, `graph562.col`, `graph584.col` and measure SAT solving acceleration.
