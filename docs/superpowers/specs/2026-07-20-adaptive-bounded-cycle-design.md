# Adaptive Bounded Cycle Escalation (c = 1 -> 2 -> 3) Design Specification

## Goal
Design and implement a single-threaded, progressive cycle-multiplier escalation strategy ($c = 1 \to 2 \to 3$) for the Hamiltonian Cycle Problem (HCP) SAT solver. This combines the ultra-fast per-iteration solve speed and minimal formula size of $c = 1$ with the stronger subtour elimination capacity of $c = 2$ and $c = 3$. Accumulated Subtour Elimination Constraint (SEC) clauses are transferred across phase transitions as warm-start clauses.

---

## 1. Motivation & Mathematical Invariants

### 1.1 Compact Bounded Space $c \in [1, 2, 3]$
- $c = 1$: Smallest base formula (~12 bits/node). Solves easy/medium graphs in 2–10 seconds.
- $c = 2$: Modulo capacity $m_2 = 2 m_1$. Prunes ~75% of subtour configurations.
- $c = 3$: Modulo capacity $m_3 = 3 m_1$. Prunes ~90%+ of subtour configurations.

All cycle multipliers $c \in \{1, 2, 3\}$ maintain compact CNF clause sizes without causing memory or variable explosion.

### 1.2 Invariant: Edge Variable Transferability
In `HcpEncoder`, directed edge variables $x_{u \to v}$ are mapped to fixed variable indices $1, 2, \dots, 2E$ based solely on the graph adjacency structure:
$$\text{EdgeVar}(u, v) \in [1, 2E]$$

Because edge variable indexing is identical across all cycle multipliers $c$, any valid Subtour Elimination Constraint ($\sum_{e \in \delta(S)} x_e \ge 1$) or DFJ clause ($\neg e_1 \lor \dots \lor \neg e_k$) learned during Phase 1 ($c = 1$) is **100% valid** when transferred to Phase 2 ($c = 2$) or Phase 3 ($c = 3$).

---

## 2. Multiphase Architecture & Escalation Triggers

```
[Phase 1: c = 1] ──(Solves in <20s or <300 iters?)──► [HAMILTONIAN Found!]
       │
       ▼ (Stagnated / Budget Exceeded)
[Extract Accumulated SEC Clauses]
       │
       ▼
[Phase 2: c = 2] ──(Inject Transfer SECs & Solve)──► [HAMILTONIAN Found!]
       │
       ▼ (Stagnated / Budget Exceeded)
[Extract Accumulated SEC Clauses]
       │
       ▼
[Phase 3: c = 3] ──(Inject Transfer SECs & Solve)──► [HAMILTONIAN Found / TIMEOUT]
```

### 2.1 Escalation Triggers

| Phase | Cycle Multiplier | Iteration Limit | Time Budget | Escalation Trigger |
| :--- | :--- | :--- | :--- | :--- |
| **Phase 1** | $c = 1$ | 300 iterations | 20 seconds | Exceeds 300 iterations OR $\le 4$ components for 30 consecutive iterations |
| **Phase 2** | $c = 2$ | 500 iterations | 40 seconds | Exceeds 500 iterations OR $\le 4$ components for 50 consecutive iterations |
| **Phase 3** | $c = 3$ | Unlimited | Remaining time limit | Solves until `SAT` or global time limit expires |

---

## 3. Class Interface Updates

### 3.1 `Solver` Header Updates (`src/Solver.hpp`)

```cpp
#ifndef SOLVER_HPP
#define SOLVER_HPP

#include <vector>
#include <cstdint>

class Solver {
public:
    enum class CycleMode {
        FIXED,              // Fixed cycle multiplier (-c N)
        ADAPTIVE_BOUNDED    // Progressive escalation c = 1 -> 2 -> 3
    };

    void setCycleMode(CycleMode mode) { cycleMode_ = mode; }
    CycleMode getCycleMode() const { return cycleMode_; }

    // Adaptive escalation solver loop
    bool runIncrementalAdaptive123(int64_t totalTimeLimitMs);

private:
    CycleMode cycleMode_ = CycleMode::FIXED;
    int phase1MaxIters_ = 300;
    int phase2MaxIters_ = 500;
    int64_t phase1TimeLimitMs_ = 20000;
    int64_t phase2TimeLimitMs_ = 40000;

    std::vector<std::vector<int>> accumulatedSecClauses_;
};

#endif // SOLVER_HPP
```

---

## 4. Verification & Testing Plan

1. **Unit Testing**:
   - Verify `accumulatedSecClauses_` clause collection and injection into subsequent solver instances.
   - Verify `--cycle bounded-adaptive` CLI option handling.
2. **Correctness Verification**:
   - Verify all decoded solutions via `HcpDecoder` (ensure 1-factor and single Hamiltonian cycle).
3. **FHCPP Benchmark Suite Evaluation**:
   - Benchmark across all 18 FHCPP graphs at 120s limit.
   - Confirm fast graphs (`graph48`, `graph162`, `graph171`) solve in Phase 1 ($c = 1$) within $<10$s.
   - Confirm hard graphs (`graph470`, `graph506`, `graph526`) escalate to Phase 2/3 and solve within 120s limit.
