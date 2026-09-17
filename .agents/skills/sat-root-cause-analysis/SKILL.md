---
name: sat-root-cause-analysis
description: Use when investigating why a SAT solver or CEGAR loop stalls or times out, drilling down through the 4-tier causal stack: Graph Topology → CNF Formulation → CDCL Search Dynamics → Engine Runtime
---

# SAT Root Cause Analysis

Execute an exhaustive, 4-tier causal drill-down to diagnose the fundamental mechanism paralyzing a SAT solver or CEGAR loop on difficult instances.

<HARD-GATE>
STRICT ROOT-CAUSE PROHIBITION OF HEURISTIC GUESSES:
You are STRICTLY FORBIDDEN from proposing any code patches, heuristic tweaks, or new encodings until you have demonstrated the exact failure mechanism with empirical solver metrics at Tier 2 (CNF/BCP) or Tier 3 (CDCL Dynamics).
Guessing "let's try 3-opt" or "let's change the restart limit" without causal proof is grounds for immediate rejection.
</HARD-GATE>

## The 4-Tier Diagnostic Stack

```
Tier 1: Graph Topology (Cut-edges, Bridges, Alternating Cycles, Gadget Symmetries)
           │
           ▼
Tier 2: CNF Formulation & BCP (Variable Semantics, Unit Propagation Strength, Dormant Cuts)
           │
           ▼
Tier 3: CDCL Search Dynamics (1UIP Depth, LBD Distribution, Phase Thrashing, Starvation)
           │
           ▼
Tier 4: Engine Runtime & Memory (Watchlist Cache Misses, Allocation Stalls, Loop Overhead)
```

## Step-by-Step Diagnostic Protocol

### Tier 1: Graph Topology Analysis
- [ ] Map structural bottlenecks:
  - Bridges and cut-edges: Edges that MUST be traversed or disconnect the tour.
  - Alternating cycles: 2-factor cycle decompositions that share edges with the target cycle.
  - Vertex-sharing dense gadgets: Subgraphs where local connectivity creates deceptive Hamiltonian paths.
  - Extract the **minimal stubborn subgraph** (e.g. 20-component decomposition on graph868).

### Tier 2: CNF Formulation & BCP Propagation Power
- [ ] Audit variable semantics:
  - Directed arc variables $x_{u,v}$ vs undirected edge variables vs positional stage variables.
- [ ] Measure Boolean Constraint Propagation (BCP) strength:
  - Do subtour elimination cuts trigger immediate unit propagations during BCP?
  - Or are the cut clauses **dormant** (containing $\ge 3$ unassigned literals until late in the decision branch)?
  - Check clause length: Are cuts too long ($>20$ literals) to provide propagation leverage?

### Tier 3: CDCL Search Dynamics (The Heart of Diagnosis)
- [ ] Inspect Conflict Graph & 1UIP (First Unique Implication Point):
  - At what decision level do conflicts occur? (Early at level $<10$, or thrashing at level $>500$?).
  - Are learned clauses derived from structural backbone variables or peripheral variables?
- [ ] Profile Literal Block Distance (LBD) Distribution:
  - High-quality clauses have $LBD \le 3$ (kept permanently by CaDiCaL).
  - Low-quality clauses have $LBD > 10$. If average LBD is high, CaDiCaL's clause reduction will purge them rapidly, causing the solver to relearn identical sub-conflicts endlessly.
- [ ] Check for Phase Saving Thrashing:
  - Is the solver oscillating polarities across component boundaries without making global progress?
- [ ] Check for VSIDS / CHB Variable Starvation:
  - Are cut-edge variables receiving branching score bumps, or are they starved because conflicts occur entirely inside peripheral gadgets?

### Tier 4: Engine Runtime & CEGAR Mechanics
- [ ] Profile low-level solver execution:
  - Ratio of time in watchlist traversal vs conflict analysis.
  - Memory stalls or vector reallocation costs during incremental assumption push/pop.
  - Frequency of CEGAR callback calls.

## Anti-Patterns & Prohibitions

| Anti-Pattern | Reality & Correction |
| :--- | :--- |
| "Let's try a different solver" | Changing from CaDiCaL to Kissat without knowing why CaDiCaL stalled is blind trial-and-error. Diagnose first. |
| "Maybe we should add more cuts" | Adding cuts without analyzing BCP often backfires: long cuts pollute watchlists and degrade BCP throughput. |
| "The solver just needs more time" | An exponential resolution tree on a 500-node graph will not finish in a billion years. The representation is flawed. |

## Artifact Template

```markdown
# SAT Root Cause Analysis: [Instance Name]
**Date:** YYYY-MM-DD  
**Target:** [e.g. graph868.col, graph950.col]  

## 1. Tier 1 — Graph Topology Obstructions
## 2. Tier 2 — CNF Formulation & BCP Propagation Audit
## 3. Tier 3 — CDCL Conflict & LBD Search Dynamics
## 4. Tier 4 — Engine Runtime Profiling
## 5. Formal Causal Conclusion
```

## Superpowers Hand-off & Transitions

- **To Software Bug Fix**: If Tier 2 or 4 reveals an implementation defect (e.g. inverted literal sign, off-by-one in node indexing, memory leak) ➔ Transition immediately to `superpowers:systematic-debugging`.
- **To Scientific Theory**: If confirmed as an intrinsic mathematical/representation hardness ➔ Transition to `sat-hypothesis-generation`.
