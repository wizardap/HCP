---
name: sat-hypothesis-generation
description: Use when formulating competing, scientifically rigorous, and falsifiable hypotheses centered on SAT mechanics, CDCL search dynamics, and SAT-heuristic interactions
---

# SAT Hypothesis Generation

Formulate rigorous, competing, and falsifiable scientific hypotheses that explain observed root causes and predict quantitative solver dynamics under proposed SAT modifications.

<HARD-GATE>
1. ENFORCE MULTIPLE WORKING HYPOTHESES: You MUST formulate at least 2–3 competing hypotheses ($H_1, H_2, H_3$). Never propose a single monolithic idea.
2. QUANTITATIVE FALSIFIABILITY: Every hypothesis MUST contain an explicit, quantitative falsification threshold tied directly to SAT solver metrics (conflicts, decisions, LBD, BCP throughput). Qualitative claims (e.g. "it will run faster") are strictly rejected.
</HARD-GATE>

## Core Principles

1. **Mechanistic Depth**: Hypotheses must specify *how* the modification interacts with the solver's internal state: Boolean Constraint Propagation (BCP), learned clause database management (LBD scoring), branching heuristics (VSIDS/CHB), or phase saving.
2. **SAT-Heuristic Interface**: When auxiliary heuristics are involved, the hypothesis must explicitly state the communication protocol with the SAT engine (e.g. phase hints, incremental assumption selectors, or lazy cut generation).
3. **Parsimony (Occam's Razor)**: Hypotheses requiring fewer auxiliary variables and simpler clause structures are prioritized.

## Step-by-Step Checklist

- [ ] **Step 1: Ingest Root Cause Diagnosis**
  Extract verified findings from `sat-root-cause-analysis` (e.g. "CaDiCaL thrashing on component bridges due to high-LBD cut clauses being purged").
- [ ] **Step 2: Formulate Competing Hypotheses ($H_1, H_2, H_3$)**
  Generate distinct causal theories:
  - **Topology/Representation ($H_1$)**: Modifying CNF encoding or edge directionality alters unit propagation reach.
  - **Search Guidance ($H_2$)**: Seeding CDCL phase saving from auxiliary heuristic cycles prevents phase thrashing.
  - **Search Localization ($H_3$)**: Using incremental assumptions to isolate component search prevents resolution tree explosion.
- [ ] **Step 3: Define the Mandatory 3-Part Schema for Each Hypothesis**
  For each $H_i$, define:
  1. **Mechanism**: The exact causal sequence inside the SAT solver.
  2. **Quantitative Prediction**: The specific solver metrics that will change (e.g. "reduces conflict count by $\ge 40\%$ across 5 seeds").
  3. **Falsification Threshold**: The exact observation that will definitively disprove the hypothesis (e.g. "if average learned clause LBD does not drop below 4.0, $H_i$ is refuted").
- [ ] **Step 4: Rank by Testability & Parsimony**
  Evaluate which hypothesis can be tested most decisively with minimal engineering effort.
- [ ] **Step 5: Synthesize and Save Artifact**
  Save to `docs/research/hypotheses/YYYY-MM-DD-<topic>-sat-hypotheses.md`.

## Anti-Patterns & Prohibitions

| Anti-Pattern | Reality & Correction |
| :--- | :--- |
| "My idea is obviously right, no need for alternative hypotheses" | Confirmation bias trap. You must actively invent competing explanations that challenge your preferred idea. |
| "Hypothesis: Adding a heuristic makes the solver better" | Vague and unfalsifiable. What heuristic? Better on what metric? Under what conditions does it fail? |
| Ignoring CDCL internal trade-offs | Any constraint added to help one component may pollute the global watchlist. State the negative trade-off explicitly. |

## Artifact Template

```markdown
# SAT Hypotheses Registry: [Topic / Domain]
**Date:** YYYY-MM-DD  
**Diagnosis Reference:** docs/research/root-cause/YYYY-MM-DD-<instance>-sat-root-cause.md  

## 1. Primary Problem Statement & Mechanistic Gap
## 2. Competing Hypotheses
### Hypothesis 1: [Name]
- **Mechanism:** ...
- **Quantitative Prediction:** ...
- **Falsification Condition:** ...
### Hypothesis 2: [Name]
- **Mechanism:** ...
- **Quantitative Prediction:** ...
- **Falsification Condition:** ...
## 3. Comparative Evaluation & Testability Ranking
```

## Superpowers Hand-off & Transitions

- **Next State**: Once competing hypotheses are registered ➔ Transition directly to `sat-research-falsification` to actively attempt to destroy each hypothesis before writing code.
- **NEVER** transition directly from `sat-hypothesis-generation` to code or implementation.
