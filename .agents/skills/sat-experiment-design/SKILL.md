---
name: sat-experiment-design
description: Use when designing controlled, statistically valid experiments measuring SAT-internal dynamics, CDCL metrics, and evaluating against SOTA SAT baselines
---

# SAT Experiment Design

Design rigorous, reproducible, and controlled empirical experiments that measure both external solver speedups and internal CDCL dynamics, adhering to SAT Competition standards.

<HARD-GATE>
PRE-REGISTERED FAILURE THRESHOLD & CONTROL RIGOR:
1. You MUST pre-register an explicit failure threshold before running tests (e.g. "If solved instance count drops by >5% on easy/medium graphs, or PAR-2 score regresses on the full suite, the proposed method is rejected").
2. You MUST define comparisons against standard SAT baselines (CaDiCaL 1.9.5, Kissat, Takehide Soh original CEGAR) across at least 5 random seeds.
</HARD-GATE>

## Core Principles

1. **Ablation-First**: Never test a bundle of heuristics together. Every cut, phase hint, and assumption mechanism must have an isolated ablation condition to prove its individual contribution.
2. **Internal CDCL Telemetry**: Beyond wall-clock time, experiments must measure internal solver physics: conflicts, decisions, BCP propagations/second, learned clause LBD distribution, purge rates, and time spent inside CDCL vs external modules.
3. **Statistical Power**: Always compute sample variance, confidence intervals, and prepare data for non-parametric paired significance testing (Wilcoxon signed-rank).

## Step-by-Step Experiment Protocol

- [ ] **Step 1: Define the SOTA Baselines**
  Establish canonical reference points:
  - Baseline 1: Pure CaDiCaL 1.9.5 (or Kissat) with default inprocessing.
  - Baseline 2: Canonical Takehide Soh SAT-based CEGAR solver.
  - Baseline 3: Current best in-tree configuration (`src/cegar-fix`).
- [ ] **Step 2: Construct the Ablation Matrix**
  Define the condition matrix:
  - $C_0$: Pure Baseline.
  - $C_1$: Baseline + Auxiliary Heuristic Phase Seeding only.
  - $C_2$: Baseline + New SAT Cut / Clause formulation only.
  - $C_3$: Full Proposed Hybrid ($C_1 + C_2$).
- [ ] **Step 3: Define Environmental Controls**
  Standardize execution conditions:
  - Fixed CPU affinity (pin to single physical core to eliminate scheduling jitter).
  - Explicit timeouts: Tiered at $60$s (rapid screen), $300$s (medium), $1800$s (full competition standard).
  - Multi-seed evaluation: Test across $\ge 5$ predetermined seeds.
- [ ] **Step 4: Select Comprehensive Telemetry Metrics**
  - **Macro-level**: Solved count ($N_{solved}$), PAR-2 score (Penalized Average Runtime: timeout instances penalized at $2 \times T_{cutoff}$), Cactus curve data points.
  - **Micro-level (CDCL)**: Conflicts, decisions, BCP propagations/sec, average LBD of kept clauses, CEGAR iteration count, and $T_{CDCL} / T_{total}$ ratio.
- [ ] **Step 5: Stratify the Benchmark Instances**
  Partition graphs into difficulty tiers:
  - Tier A (Trivial): Solved in $<1$s by baseline.
  - Tier B (Moderate): Solved in $1$s–$60$s.
  - Tier C (Hard): Solved in $60$s–$1800$s.
  - Tier D (Pathological / Timeout): Unsolved at $1800$s.
- [ ] **Step 6: Pre-Register Failure Criteria**
  Record explicit disqualification rules:
  - "Any regression on Tier A/B instances $>5\%$ terminates candidate."
  - "If $T_{CDCL}$ increases due to watchlist cache misses without reducing conflict count by $\ge 20\%$, reject."
- [ ] **Step 7: Synthesize and Save Artifact**
  Save protocol to `docs/research/experiments/YYYY-MM-DD-<experiment-id>-sat-protocol.md`.

## Anti-Patterns & Prohibitions

| Anti-Pattern | Reality & Correction |
| :--- | :--- |
| Reporting only arithmetic mean runtime | Arithmetic mean is dominated by timeouts and easily distorted. PAR-2 and Cactus curves are mandatory in SAT research. |
| Testing only on instances where your idea wins | Cherry-picking invalidates scientific claims. You must evaluate against the entire standardized benchmark suite. |
| Changing compiler flags between baseline and experiment | Both must be compiled with identical optimizations (`cargo build --release`). |

## Artifact Template

```markdown
# SAT Experiment Protocol: [Experiment ID / Title]
**Date:** YYYY-MM-DD  
**Hypothesis Tested:** [Link to hypothesis registry]  

## 1. Baselines & Benchmark Suite
## 2. Ablation Matrix
## 3. Environmental Controls & Hardware Specs
## 4. Telemetry Metrics Suite
## 5. Pre-Registered Failure Thresholds
## 6. Execution Plan & Script Specs
```

## Superpowers Hand-off & Transitions

- **Next State**: Once the experiment protocol is finalized:
  - Transition immediately to `superpowers:writing-plans` to write the benchmark execution harness scripts and runner configurations.
  - When executing large batch runs across multi-core systems ➔ Trigger `superpowers:dispatching-parallel-agents`.
