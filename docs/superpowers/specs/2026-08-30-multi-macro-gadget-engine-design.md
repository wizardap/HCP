# Design Specification: Multi-Macro Gadget Interface Engine & Full-Spectrum Late-Stage Cut Selector (`MultiMacroGadgetEngine`)

- **Date:** 2026-08-30
- **Target Repository:** `wizardap/HCP` (`src/cegar-fix`)
- **System Constraints:** Core 3 must ALWAYS be left free for the user. Single/Multi-core commands use `taskset -c 0,1,2 nice -n 19`. Time limit $T_{\max} = 1800\text{s}$.
- **Commitment to Scientific Rigor:** Zero Tour Injection policy (never read `.hcp.tou` files). Exact mathematical gadget parity and 100% full-spectrum cut coverage in late rounds.

---

## 1. Executive Summary & Problem Context

### 1.1 Multi-Macro Cycles in Late CEGAR Rounds
In late CEGAR rounds on `graph668.col` (e.g. Round 10), the solution consists of multiple large macro-cycles (e.g. 776, 733, 364 vertices) plus small satellite cycles.
- **The Bottleneck**:
  1. `GadgetInterfaceParityEngine` was only invoked if a single giant cycle had $> |V| / 2$ vertices. When the top cycles are 776 and 733 (each $\sim 27\%$ of the graph), it was completely skipped.
  2. `CutSelector` was restricted to selecting a subset of subcycles (e.g. 20/29 subcycles) due to conservative budget limits, leaving some small subcycles unconstrained.
- **The Solution — `MultiMacroGadgetEngine`**:
  1. **Multi-Macro Target Splicing**:
     - Allow `GadgetInterfaceParityEngine` to analyze and splice small subcycles against ALL macro-cycles with length $\ge |V| / 10$.
  2. **100% Full-Spectrum Cut Coverage When $\le 50$ Subcycles Remain**:
     - When total remaining subcycles $\le 50$, select 100% of all non-giant subcycles for SEC clause generation and boundary cut injection, guaranteeing no small cycle can be repeated in the next round.

---

## 2. Architecture & Algorithmic Design

### 2.1 Changes in `src/cegar-fix/src/hcp_solver.rs`
1. In `GadgetInterfaceParityEngine` loop:
   - Identify all macro-cycles with $|C| \ge \text{total\_nodes} / 10$.
   - Iterate through every small subcycle ($|C_s| \le 32$) and test direct splicing against each macro-cycle.
   - Inject port pruning and cut parity clauses into `working_cnf` and `accumulated_cut_cnfs`.
2. In `CutSelector` / SEC generation loop:
   - If `_active_cycles.len() <= 50`, bypass budget limits and select 100% of subcycles ($|C| < \text{total\_nodes} / 2$).

---

## 3. Verification Strategy

1. **Unit & Integration Tests (`tests/test_staged_solver.rs`):**
   - Test multi-macro gadget analysis and full-spectrum cut coverage.
2. **Benchmark Verification:**
   - Run benchmark on `graph479.col` and `graph668.col` with `taskset -c 0,1,2 nice -n 19`.
