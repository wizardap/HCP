---
name: sat-phenomenon-analysis
description: Use when analyzing benchmark logs and runtime traces to isolate SAT-specific performance anomalies, CDCL hardness cliffs, and CEGAR iteration bottlenecks
---

# SAT Phenomenon Analysis

Systematically detect, isolate, and quantify behavioral anomalies in SAT solvers and SAT-CEGAR loops, distinguishing true algorithmic hardness cliffs from environmental noise.

<HARD-GATE>
DO NOT declare an algorithmic anomaly without:
1. Decomposing the time budget ($T_{total} = T_{CDCL} + T_{cut} + T_{auxiliary}$) to prove whether the bottleneck is in CDCL or auxiliary heuristics.
2. Ruling out system noise across at least 3 random solver seeds in CaDiCaL/Kissat.
</HARD-GATE>

## Core Principles

1. **SAT-Centric Diagnostics**: Frame all observations in terms of SAT solver dynamics: conflicts per second, decision tree depth, learned clause generation rate, restart frequency, and CEGAR iteration trajectory.
2. **Hardness Cliffs**: Look for sudden orders-of-magnitude runtime divergence on structurally comparable instances (e.g. graph868 vs graph1 with identical vertex count $N=1000$).
3. **Cycle Dynamics**: Track whether the solver is converging toward a single giant cycle or suffering from **cycle shattering** (repeatedly outputting hundreds of disjoint 2-cycles or 3-cycles that fail to merge).

## Step-by-Step Checklist

- [ ] **Step 1: Time Budget Decomposition**
  Parse runtime logs to compute the exact breakdown:
  - $T_{CDCL}$: Time spent inside the CDCL solver core.
  - $T_{cut}$: Time spent finding disconnected components and generating cuts.
  - $T_{auxiliary}$: Time spent in local search / cycle patching heuristics.
  *If $T_{CDCL} < 20\%$, the bottleneck is in auxiliary code, not the SAT solver.*
- [ ] **Step 2: Hardness Cliff & Outlier Detection**
  Identify instances exhibiting:
  - Disproportionate conflict counts ($>10^6$ conflicts on $N < 1000$)
  - Extreme CEGAR iterations ($>500$ iterations without cycle consolidation)
  - Severe timeout cliffs (e.g. 99% of benchmark solves in $<2$s, but 5 graphs stall at $>1800$s)
- [ ] **Step 3: Graph Invariant Correlation**
  Correlate the anomaly with graph-theoretic properties:
  - Bridge density and cut-vertices
  - Alternating cycle configurations
  - Diameter, girth, degree-2 chain lengths, spectral gap
- [ ] **Step 4: Seed Jitter & Noise Filtration**
  Run the anomalous instance across 3 distinct seeds:
  - If runtime varies wildly ($0.1$s on seed 1, $1800$s on seed 2), this is **Phase/Branching Jitter**.
  - If solver stalls consistently across all seeds, this is a **Structural CDCL Obstruction**.
- [ ] **Step 5: Synthesize and Save Artifact**
  Save report to `docs/research/phenomena/YYYY-MM-DD-<phenomenon-name>.md`.

## Anti-Patterns & Prohibitions

| Anti-Pattern | Reality & Correction |
| :--- | :--- |
| "The solver is slow because graph is big" | False. $N=5000$ graphs can solve in 0.1s while $N=200$ graphs can timeout. Identify the topological trap. |
| "A single timeout run is enough to prove hardness" | False. CDCL phase saving can get trapped by lucky/unlucky seed orders. Multi-seed verification is mandatory. |
| Blaming SAT when cut generation takes 90% of time | Always measure $T_{CDCL}$ vs $T_{cut}$. If cut generation is slow, that's a graph algorithm bug, not a SAT solver issue. |

## Artifact Template

```markdown
# SAT Phenomenon Report: [Anomaly Name]
**Date:** YYYY-MM-DD  
**Target Graph / Instances:** [e.g. graph868.col, graph950.col]  

## 1. Empirical Observation & Runtime Cliff
## 2. Time Budget Decomposition (CDCL vs Auxiliary)
## 3. Solver Internal Metrics (Conflicts, Restarts, Cycles)
## 4. Multi-Seed Jitter Validation
## 5. Topological Correlates
```

## Superpowers Hand-off & Transitions

- **Next State**: 
  - Once a genuine CDCL anomaly is isolated ➔ Transition immediately to `sat-root-cause-analysis`.
  - If the anomaly is purely due to external script overhead ➔ Transition to `superpowers:systematic-debugging`.
