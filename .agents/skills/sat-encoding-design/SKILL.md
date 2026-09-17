---
name: sat-encoding-design
description: Use when formalizing verified hypotheses into sound, complete, and propagation-efficient SAT encodings, incremental assumption interfaces, and CEGAR cut mechanisms
---

# SAT Encoding Design

Formalize verified mathematical insights and algorithmic hypotheses into rigorous, sound, complete, and propagation-efficient SAT formulations, incremental assumption protocols, and CEGAR cuts.

<HARD-GATE>
SOUNDNESS/COMPLETENESS PROOFS & TDD TEST SUITE:
1. You MUST formally prove two invariants before coding:
   - Invariant 1 (Soundness): Every satisfying truth assignment decodes to a valid Hamiltonian cycle.
   - Invariant 2 (Completeness): No valid Hamiltonian cycle is rendered unsatisfiable (no false UNSAT).
2. You MUST define minimal graph invariant test suites ($N=3, 4, 5, 6$) and hand off directly to `superpowers:writing-plans` and `superpowers:test-driven-development` to implement code in Rust or Python. DO NOT implement code directly inside this skill.
</HARD-GATE>

## Core Principles

1. **BCP Propagation Strength**: An encoding that enforces constraints only when all variables are assigned is vastly inferior to an encoding that achieves Generalized Arc Consistency (GAC) or Unit-Refutation Completeness during early BCP.
2. **Watchlist & Cache Efficiency**: Adding redundant implied clauses can boost BCP, but excessive clause length degrades CPU cache locality during 2-watched-literal traversal. Balance clausal strength against memory footprint.
3. **Incremental Scoping via Assumptions**: Use assumption selector literals ($a_k \implies \text{clause}$) so that structural cuts can be activated or retracted in CaDiCaL without destroying internal learned clauses or resetting variable scores.

## Step-by-Step Specification Protocol

- [ ] **Step 1: Propositional Variable Semantics**
  Define every variable class with its exact domain:
  - Directed Arc Variables: $x_{u,v} \in \{0, 1\}$ (edge $(u, v)$ is selected in the tour).
  - Positional / Stage Variables: $y_{v,k} \in \{0, 1\}$ (vertex $v$ is visited at step $k$).
  - Incremental Assumption Selectors: $a_k \in \{0, 1\}$ (enables lazy or component-specific cuts).
  - Component Indicators: $z_c \in \{0, 1\}$.
- [ ] **Step 2: Constraint Formulation & Cardinality Encodings**
  - **In/Out Degree-2 Constraints**:
    Select appropriate At-Least-One (ALO) and At-Most-One (AMO) encodings:
    - Pairwise encoding ($O(d^2)$ clauses, simple, fast for small degrees $d \le 4$).
    - Commander or Bimultaneous encoding (for dense graphs where $d > 5$).
  - **Subtour Elimination**:
    - Lazy DFJ cuts: Generated on-the-fly when CEGAR detects a disconnected subcycle $C$: $\sum_{u \in C, v \notin C} x_{u,v} \ge 1$.
    - Static or Hybrid MTZ constraints: Linear order variables preventing small cycles up to size $k$.
  - **Symmetry Breaking**:
    Fix orientation ($x_{v_0, v_1} = 1$) or lexicographical order to prevent exploring mirrored identical cycles.
- [ ] **Step 3: Complexity & Propagation Strength Analysis**
  - Calculate Big-O asymptotic scaling for variables and clauses ($O(|V|), O(|E|), O(|V|^2)$).
  - Evaluate propagation power: Does the formulation deduce forced edges across bridge cuts during initial unit propagation before decisions?
  - Verify 2-watched-literal watchlist footprint: Ensure average clause length is minimized.
- [ ] **Step 4: Formal Invariant Proofs**
  - **Proof of Soundness**: Demonstrate that any model satisfying the CNF satisfies all Hamiltonian cycle criteria.
  - **Proof of Completeness**: Demonstrate that any Hamiltonian cycle in $G$ corresponds to a satisfying model.
- [ ] **Step 5: Define Invariant Test Cases on Small Graphs**
  Define explicit truth expectations on small graphs:
  - Triangle $K_3$: Exactly 1 cycle (SAT).
  - Bowtie graph (two triangles sharing 1 cut-vertex): Guaranteed non-Hamiltonian (UNSAT).
  - Petersen graph: Non-Hamiltonian (UNSAT).
  - Small cubic graphs ($N=4, 6$).
- [ ] **Step 6: Synthesize and Save Artifact**
  Save specification to `docs/research/encodings/YYYY-MM-DD-<encoding-name>-sat-spec.md`.

## Anti-Patterns & Prohibitions

| Anti-Pattern | Reality & Correction |
| :--- | :--- |
| Writing code before completing proofs | Implementing unproven encodings leads to subtle false UNSAT bugs that waste weeks. Prove first. |
| Using quadratic encodings on large graphs | A naive $O(|V|^2)$ subtour encoding produces $10^6$ clauses for $N=1000$, exhausting RAM. Use lazy CEGAR cuts. |
| Neglecting assumption selector semantics | Hardcoding temporary constraints permanently mutates solver state. Use assumption literals for revocable cuts. |

## Artifact Template

```markdown
# SAT Encoding Specification: [Encoding Name]
**Date:** YYYY-MM-DD  
**Status:** Approved for Implementation  

## 1. Variable Semantics & Index Mapping
## 2. Constraint Classes & DIMACS Formulation
## 3. Asymptotic Complexity & BCP Propagation Power
## 4. Formal Proofs: Soundness & Completeness
## 5. Invariant Test Suite on Small Graphs (N=3, 4, 5, 6)
## 6. Implementation Contract & Hand-off
```

## Superpowers Hand-off & Transitions

- **Next State**: Once the specification is saved:
  - Transition immediately to `superpowers:writing-plans` to plan out the Rust/Python implementation tasks.
  - Enforce `superpowers:test-driven-development`: Implement the minimal small-graph invariant tests first (RED), write the encoder logic (GREEN), and verify before deploying into the main solver loop.
