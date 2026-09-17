---
name: sat-benchmark-analysis
description: Use when evaluating benchmark outputs using SAT Competition standards, calculating PAR-2 scores, Cactus plots, statistical significance, and closing the scientific discovery loop
---

# SAT Benchmark Analysis

Analyze empirical benchmark outputs, compute standard SAT Competition metrics, perform statistical significance testing, diagnose regressions, and close the scientific discovery loop.

<HARD-GATE>
NO CLAIMS OF SUPERIORITY WITHOUT STATISTICAL RIGOR:
1. You are STRICTLY PROHIBITED from claiming that a new method is superior based solely on arithmetic mean runtime. You MUST compute PAR-2 scores and perform a paired Wilcoxon signed-rank test ($p < 0.05$).
2. If total solved instances decreased in ANY difficulty tier (even if average runtime on solved instances improved), the method CANNOT be claimed as an unqualified success.
</HARD-GATE>

## Core Principles

1. **SAT Competition Standards**: Evaluate all solver runs using **PAR-2** (Penalized Average Runtime with a $2\times$ penalty for timeouts) and cumulative **Cactus Plots** (solved instances ordered by runtime).
2. **Virtual Best Solver (VBS) Contribution**: Determine whether the proposed solver solves unique instances that no baseline solver can solve, establishing complementary algorithmic value in a portfolio.
3. **Loop Closure**: Benchmarking is not an end point; it is a discovery tool. Every new timeout or regression is an opportunity to trigger a new cycle of phenomenon analysis and root cause investigation.

## Step-by-Step Analysis Protocol

- [ ] **Step 1: Compute Macro Performance Metrics**
  - **Solved Count ($N_{solved}$)**: Total instances solved within cutoff $T_{cutoff}$ (e.g. 1800s).
  - **PAR-2 Score**:
    $$\text{PAR-2} = \frac{1}{|I|} \left( \sum_{i \in \text{Solved}} t_i + \sum_{i \in \text{Timeout}} 2 \cdot T_{cutoff} \right)$$
  - **Median & Interquartile Range (IQR)**: Report non-parametric central tendency.
- [ ] **Step 2: Generate Cactus Plot Data**
  - Sort runtime arrays ascendingly for each solver configuration: $t_{(1)} \le t_{(2)} \le \dots \le t_{(k)}$.
  - Plot cumulative solved count vs wall-clock runtime.
  - Identify intersection points (crossover points where one solver overtakes another).
- [ ] **Step 3: Analyze Virtual Best Solver (VBS) Impact**
  - Compute $VBS(I) = \min_{s \in Solvers} t(s, I)$.
  - Measure marginal contribution: How many instances are solved uniquely by the proposed method that baseline CaDiCaL/Takehide Soh failed?
- [ ] **Step 4: Paired Statistical Significance Testing**
  - Execute a paired **Wilcoxon Signed-Rank Test** on runtime differences: $\Delta t_i = t_{\text{baseline}}(i) - t_{\text{proposed}}(i)$.
  - Verify $p < 0.05$ to reject the null hypothesis of equal performance.
- [ ] **Step 5: Perform Regression Root-Cause Breakdown**
  - Extract all instances where $t_{\text{proposed}} > 1.5 \times t_{\text{baseline}}$ or where baseline solved but proposed timed out.
  - Categorize regression cause:
    - Watchlist bloat (excessive clauses slowing down BCP props/sec).
    - Phase saving corruption (heuristic hints steering CDCL into dead subtrees).
    - Unproductive CEGAR iterations (cuts too weak to prevent identical subcycles).
- [ ] **Step 6: Scientific Loop Closure**
  - If new unexplained timeouts or hardness cliffs emerge ➔ Trigger `sat-phenomenon-analysis` or `sat-root-cause-analysis` with the offending graphs.
  - Save report to `docs/research/benchmarks/YYYY-MM-DD-<benchmark-id>-sat-report.md`.

## Anti-Patterns & Prohibitions

| Anti-Pattern | Reality & Correction |
| :--- | :--- |
| "Average runtime dropped from 10s to 8s, so we won" | If timeout count increased from 2 to 5, PAR-2 worsened dramatically. Look at PAR-2, not solved-only mean. |
| Ignoring regressions as "random noise" | Every regression has a mathematical or implementation cause in CDCL. Investigate why it happened. |
| Plotting scatter plots on linear scale | SAT runtimes span 4 orders of magnitude ($0.01$s to $1800$s). Always use log-log scatter plots. |

## Artifact Template

```markdown
# SAT Benchmark Report: [Benchmark / Experiment ID]
**Date:** YYYY-MM-DD  
**Dataset:** [e.g. FHCPCS 1,001 benchmark graphs]  
**Cutoff:** [e.g. 1800s, PAR-2 penalty 3600s]  

## 1. Summary Performance Matrix (Solved, PAR-2, Median)
## 2. Cactus Plot Data & Crossover Dynamics
## 3. Virtual Best Solver (VBS) Contribution
## 4. Statistical Significance Testing (Wilcoxon p-value)
## 5. Regression Analysis & Degraded Instances
## 6. Scientific Insights & Next Iteration Triggers
```

## Superpowers Hand-off & Transitions

- **Next State**:
  - If all benchmarks pass criteria and no regressions remain ➔ Milestone complete! Transition to `superpowers:finishing-a-development-branch` or prepare research publication/report.
  - If regressions or new hardness cliffs are discovered ➔ Close the loop by transitioning back to `sat-phenomenon-analysis` or `sat-root-cause-analysis`.
