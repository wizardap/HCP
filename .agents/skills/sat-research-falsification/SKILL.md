---
name: sat-research-falsification
description: Use when stress-testing and attempting to disprove SAT-centric hypotheses using adversarial counterexample graphs, clause pollution checks, and confounder analysis before coding
---

# SAT Research Falsification

Actively and aggressively attack proposed SAT hypotheses to disprove them before investing costly engineering resources.

<HARD-GATE>
A hypothesis CANNOT proceed to encoding design, algorithm prototyping, or implementation planning without surviving at least THREE distinct falsification trials:
1. An adversarial minimal trap graph probe ($N \le 20$).
2. A clause database bloat & BCP degradation analysis.
3. A multi-seed / variable-ordering confounder test.
If any test exposes a fatal flaw, the hypothesis is REFUTED and must be returned to `sat-hypothesis-generation`.
</HARD-GATE>

## Core Principles

1. **Popperian Asymmetry**: No amount of successful benchmark runs can prove a hypothesis true, but a single reproducible counterexample proves it false.
2. **Adversarial Mindset**: Assume the proposed SAT heuristic or constraint is actively detrimental until rigorous adversarial probing fails to break it.
3. **Clause Pollution Defense**: Many SAT enhancements look good on paper but degrade solver performance by filling 2-watched-literal lists with redundant, high-LBD clauses that slow down BCP.

## Step-by-Step Falsification Protocol

- [ ] **Step 1: Adversarial Minimal Trap Graph Mining**
  Construct or search for small pathological instances ($N \le 20$):
  - Highly symmetric graphs where heuristic phase hints cause infinite search ping-pong.
  - Bipartite graphs with unequal partition sizes $|V_1| \neq |V_2|$ (guaranteed non-Hamiltonian): Does the SAT formulation detect UNSAT immediately via BCP, or does it explore an exponential tree?
  - Bridge-heavy graphs where local patching creates dead ends.
- [ ] **Step 2: Clause Database Bloat & BCP Throughput Audit**
  Analyze the clause dynamics:
  - What is the expected LBD of clauses added by the proposed technique?
  - Will these clauses survive CaDiCaL's periodic clause database reduction (`reduce`)?
  - If clauses have $LBD > 10$, they will be purged quickly, turning the technique into wasted CPU cycles.
  - Does the extra clause volume reduce raw BCP propagation rate (props/sec)?
- [ ] **Step 3: Confounder Isolation**
  Verify whether observed improvements are illusory:
  - **Random Seed Variance**: Test across 5 distinct seeds. If the speedup only appears on 1 seed, it is noise.
  - **Variable Index Ordering**: Permute DIMACS variable IDs. If the speedup evaporates, it was an accidental artifact of CaDiCaL's initial variable array order.
- [ ] **Step 4: Boundary Edge Case Stressing**
  Examine boundary limits:
  - Extremely sparse graphs (cubic, 3-regular).
  - Dense graphs with high clique density.
  - Long degree-2 chains (testing if contraction is bypassed).
- [ ] **Step 5: Verdict Classification & Registry Update**
  Formulate the explicit verdict:
  - **REFUTED**: Counterexample found. Record minimal graph and feedback to `sat-hypothesis-generation`.
  - **REFINED**: Hypothesis fails globally but holds unconditionally on a well-defined graph sub-class (e.g. 3-regular bridgeless graphs). Narrow the scope.
  - **SURVIVED**: Passed all 3 adversarial gates. Qualified for empirical experiment design.
  Save report to `docs/research/falsifications/YYYY-MM-DD-<hypothesis-id>-sat-falsification.md`.

## Anti-Patterns & Prohibitions

| Anti-Pattern | Reality & Correction |
| :--- | :--- |
| "It worked on graph868 so it must be valid" | Single-instance success is anecdotal. You must actively search for the graph that breaks it. |
| Explaining away a counterexample as an "outlier" | In mathematics and SAT, an outlier counterexample is a refutation. Embrace it and revise the theory. |
| Testing only on easy benchmark graphs | Easy graphs solve regardless of heuristics. Falsification requires hard, adversarial test cases. |

## Artifact Template

```markdown
# SAT Falsification Report: [Hypothesis ID]
**Date:** YYYY-MM-DD  
**Hypothesis Under Attack:** [Quote exact mechanism and prediction]  

## 1. Adversarial Trap Graph Probe
- Graph topology & DIMACS: ...
- Observed behavior vs Predicted outcome: ...
## 2. Clause Database Bloat & BCP Audit
- Expected LBD & Watchlist impact: ...
## 3. Confounder & Seed Permutation Tests
## 4. Final Verdict: [REFUTED / REFINED / SURVIVED]
## 5. Required Actions & Feedback
```

## Superpowers Hand-off & Transitions

- **If Refuted**: Return immediately to `sat-hypothesis-generation` with the counterexample graph added to the constraint list.
- **If Survived**: Advance to `sat-experiment-design` to build the formal benchmark evaluation protocol.
