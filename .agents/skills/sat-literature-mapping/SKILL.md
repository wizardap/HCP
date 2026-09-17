---
name: sat-literature-mapping
description: Use when surveying academic literature on SAT-based combinatorial solving, incremental SAT-CEGAR, SMT lazy clause learning, and CDCL theoretical barriers
---

# SAT Literature Mapping

Construct an actionable, gap-oriented literature map specifically focused on SAT-centric algorithms, CDCL theoretical limits, and SAT-heuristic hybrids for combinatorial optimization problems.

<HARD-GATE>
DO NOT write passive paper summaries or superficial bibliographies.
You MUST identify at least one explicit, unverified CDCL or encoding assumption in prior work that creates a failure point on hard benchmark instances, and synthesize it into a formal Research Gap Matrix.
</HARD-GATE>

## Core Principles

1. **SAT-Centric Priority**: Focus exclusively on frameworks where the SAT solver is the primary engine or core decider (SAT-CEGAR, Incremental SAT with Assumptions, Lazy SMT constraint propagation, and SAT-heuristic hybrids like phase seeding or sub-instance decomposition).
2. **Theoretical Limits**: Ground empirical observations in known proof complexity and CDCL bounds (Resolution lower bounds, Tseitin formulas on expanders, Pigeonhole obstructions, Treewidth boundaries, and Chvátal-Erdős conditions).
3. **Actionable Delta**: Every surveyed paper must answer: *What mechanism did they use, what did they assume about CDCL/propagation, where does the solver break down, and what is our concrete exploitation opportunity?*

## Step-by-Step Checklist

- [ ] **Step 1: Build the SAT Taxonomy**
  Categorize the landscape of prior methods:
  - Canonical SAT-CEGAR (e.g. Soh et al. subtour cuts)
  - Incremental SAT with Assumptions (CaDiCaL/Glucose push/pop and selector literals)
  - Lazy SMT / Clausal Propagators (on-the-fly cut generation during BCP)
  - SAT-Heuristic Hybrids (local search phase seeding, DP-bitmask subproblem oracles, macro-cycle contraction)
- [ ] **Step 2: Cross-Reference CDCL Theoretical Boundaries**
  Check whether observed hardness stems from fundamental proof complexity barriers:
  - Exponential tree-like or general resolution lower bounds
  - Symmetry bottlenecks without symmetry-breaking clauses
  - Lack of unit-propagation strength (GAC failure on subtours)
- [ ] **Step 3: Construct the SAT Research Gap Matrix**
  Format the matrix with exact fields:
  | Prior Approach | Core SAT/CDCL Mechanism | Implicit Assumption | Failure Point on Hard Cases | Exploitation Opportunity |
  | :--- | :--- | :--- | :--- | :--- |
- [ ] **Step 4: Identify Baseline Configurations**
  Identify the exact baseline solver versions (e.g. CaDiCaL 1.9.5, Kissat, Takehide Soh original CEGAR) and canonical benchmark instances (e.g. FHCPCS 1,001 graphs).
- [ ] **Step 5: Synthesize and Save Artifact**
  Save to `docs/research/literature/YYYY-MM-DD-<topic>-sat-literature-map.md`.

## Anti-Patterns & Prohibitions

| Anti-Pattern | Reality & Correction |
| :--- | :--- |
| "This paper solves HCP with 100% genetic algorithm" | Irrelevant unless it provides a phase saving hint or cut generation mechanism for a SAT solver. Skip pure heuristics. |
| "Author X showed their solver is fast on random graphs" | Random graphs are trivial for CDCL. Focus on structured, adversarial, or hard benchmark instances (e.g. cubic, bipartite, bridgy graphs). |
| Listing abstracts without critique | Every citation must specify its exact clausal formulation and CDCL interaction failure point. |

## Artifact Template

```markdown
# SAT Literature Map: [Topic / Domain]
**Date:** YYYY-MM-DD  
**Scope:** SAT-based combinatorial solving & CDCL dynamics  

## 1. Algorithmic Taxonomy & SOTA
## 2. CDCL & Proof Complexity Boundaries
## 3. SAT Research Gap Matrix
## 4. Proposed Investigation Vectors
```

## Superpowers Hand-off & Transitions

- **Next State**: Once research gaps are formulated:
  - If investigating an unexplained runtime stall on specific instances ➔ Transition to `sat-root-cause-analysis`.
  - If developing new theoretical explanations for solver behavior ➔ Transition to `sat-hypothesis-generation`.
- **To Engineering**: If the literature review leads directly to a known SAT cut formulation ready for coding, transition to `superpowers:writing-plans`.
