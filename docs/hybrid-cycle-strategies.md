# Hybrid Cycle Length Strategies for HCP SAT Solving

## Overview
This document outlines three hybrid strategies for combining different cycle multipliers ($c = 1, c = 2$, and $m > n$ CRE factorization) in the Hamiltonian Cycle Problem (HCP) SAT solver.

---

## 1. Strategy Comparison Summary

| Strategy | Primary Advantage | Typical Overhead | Recommended Use Case |
|---|---|---|---|
| **Strategy 1: Stagnation-Triggered Escalation ($c=1 \to m>n$)** | Fast $c=1$ solve on easy/medium graphs; zero-subcycle guarantee fallback for hard graphs | Small restart overhead if Phase 1 escalates | General purpose single-threaded solver |
| **Strategy 2: Dual-Thread Portfolio Solver ($c=1$ + $m>n$)** | Best possible runtime on all graphs (whichever worker finishes first wins) | Requires 2 CPU threads | Multi-core / high-performance benchmarks |
| **Strategy 3: Graph Feature-Based Pre-Selection** | Zero phase-switch overhead; single-shot encoding decision | Requires pre-tuning heuristic thresholds | Fast static solver configuration |

---

## 2. Detailed Strategy Descriptions

### Strategy 1: Stagnation-Triggered Adaptive Escalation ($c=1 \to m>n$)

```
+------------------------------+
| Start Phase 1: cycle = 1     |
| (Minimal CNF formula size)   |
+------------------------------+
               |
               v
    Solves in <15s or <500 SEC iterations?
              / \
        Yes  /   \  No (Stagnated)
            v     v
+-----------------------+     +-----------------------------------------+
| Return HAMILTONIAN    |     | Switch to Phase 2: m > n (auto-scale)   |
| (Fastest solve time)  |     | (Guarantees zero subcycles upfront)     |
+-----------------------+     +-----------------------------------------+
```

- **Phase 1 ($c = 1$)**: Runs incremental SEC loop with $c = 1$. For easy/medium graphs (e.g. `graph48`, `graph162`, `graph171`), it finishes in **2–10 seconds** with minimal memory.
- **Phase 2 ($m > n$)**: If Phase 1 hits stagnation (e.g., component count remains $\le 4$ for $>500$ SEC iterations), automatically switch to $m > n$ auto-scaled CRE mode.

---

### Strategy 2: Dual-Thread Portfolio Solver ($c=1$ and $m>n$ Concurrently)

- **Worker 1 (Incremental SEC with $c=1$)**: Solves under small formula size. Superior for graphs where subcycles break quickly.
- **Worker 2 (One-shot $m>n$ CRE)**: Solves under full cycle capacity where subcycles are mathematically impossible. Superior for dense/complex graphs (`graph470`, `graph506`).
- **Execution**: Both workers run concurrently. Whichever thread reports `SAT` first returns the valid Hamiltonian cycle and terminates the sibling worker.

---

### Strategy 3: Graph Feature-Based Pre-Selection

- **Feature Extraction**: Before encoding, analyze graph metrics:
  - Node count ($N$)
  - Edge density / Average degree ($D$)
- **Heuristic Rule**:
  - If $N \le 300$ or average degree $D > 6$: Select $c = 1$.
  - If $N > 300$ and complex topology: Auto-scale $m > n$.
- **Advantage**: Zero phase-switch or multi-threading overhead; chooses the single optimal encoding upfront.
