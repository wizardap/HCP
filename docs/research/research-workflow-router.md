# SAT Research Workflow Playbook & Router

This document serves as the operational playbook and state router for the **SAT-Centric Research Skills Suite**, guiding pair programmers and autonomous agents through the scientific discovery lifecycle and connecting seamlessly with the **Superpowers** software engineering suite.

---

## 1. Quick Navigation Router

Use this decision table to immediately identify which skill to invoke based on your current research situation:

| If you are experiencing... | Trigger this Skill | Expected Next State / Hand-off |
| :--- | :--- | :--- |
| An unexpected solver timeout, stall, or memory cliff on a benchmark | `sat-phenomenon-analysis` | ➔ `sat-root-cause-analysis` |
| Surveying prior algorithms, SOTA encodings, or CDCL theoretical limits | `sat-literature-mapping` | ➔ `sat-hypothesis-generation` |
| A solver paralyzed on a hard graph; need to find the exact causal layer | `sat-root-cause-analysis` | ➔ `sat-hypothesis-generation` (or `systematic-debugging` if software bug) |
| Diagnosed the root cause; ready to formulate competing causal theories | `sat-hypothesis-generation` | ➔ `sat-research-falsification` (Mandatory gate) |
| Formulated a hypothesis; need to actively stress-test before coding | `sat-research-falsification` | ➔ `sat-experiment-design` (if survives) or `sat-hypothesis-generation` (if refuted) |
| Hypothesis verified against traps; need controlled evaluation matrix | `sat-experiment-design` | ➔ `superpowers:writing-plans` (to code benchmark scripts) |
| Ready to formulate formal CNF clauses, assumptions, or CEGAR cuts | `sat-encoding-design` | ➔ `superpowers:writing-plans` + `superpowers:test-driven-development` |
| Benchmark run completed; raw logs, times, and conflict counts ready | `sat-benchmark-analysis` | ➔ Scientific Loop Closure (re-enter `sat-phenomenon-analysis` or milestone complete) |

---

## 2. The 3 Operational Clusters

```
                     [ Benchmark Anomaly / Search Stalemate ]
                                        │
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ CLUSTER 1: SAT DISCOVERY & DIAGNOSTICS                                      │
│                                                                             │
│   • sat-literature-mapping     (Survey SAT-CEGAR, SMT lazy cuts, CDCL gaps) │
│   • sat-phenomenon-analysis    (Detect CDCL runtime cliffs, cycle shatter)  │
│   • sat-root-cause-analysis    (🔴 4-Tier Stack: Graph → CNF → CDCL → Run)  │
└───────────────────────────────────────┬─────────────────────────────────────┘
                                        │
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ CLUSTER 2: SAT THEORY & ADVERSARIAL FALSIFICATION                           │
│                                                                             │
│   • sat-hypothesis-generation  (Mechanistic SAT & SAT-heuristic hypotheses) │
│   • sat-research-falsification (🔴 Trap graphs, clause bloat, seed noise)   │
│         │ (If falsified)                                                    │
│         └──► Return to sat-hypothesis-generation                            │
│   • sat-experiment-design      (🔴 Multi-seed, CDCL metrics, SAT baselines) │
└───────────────────────────────────────┬─────────────────────────────────────┘
                                        │
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ CLUSTER 3: SAT SYNTHESIS & EVALUATION                                       │
│                                                                             │
│   • sat-encoding-design        (CNF variables, incremental assumptions)     │
│   • sat-benchmark-analysis     (SAT Competition metrics: PAR-2, Cactus, VBS)│
└───────────────────────────────────────┬─────────────────────────────────────┘
                                        │
                                        ▼
                 [ Loop Closure: New Discovery or Milestone Complete ]
```

---

## 3. Superpowers Hand-off Gates

The research skills suite is strictly decoupled from ad-hoc coding. When transitioning from theory to software, enforce these mandatory gates:

### Gate A: To Engineering Implementation
* **From**: `sat-encoding-design` or `sat-experiment-design`.
* **Action**: Do NOT implement code inside the research skill.
* **Handoff**: Invoke `superpowers:writing-plans` to generate an implementation plan.
* **Discipline**: Follow `superpowers:test-driven-development` (TDD) to write small-graph invariant tests (RED), implement the minimal encoder/cut logic in Rust (`src/cegar-fix`) or Python (GREEN), and verify before deploying into the main solver loop.

### Gate B: To Software Bug Debugging
* **From**: `sat-root-cause-analysis` (Tier 2 or Tier 4).
* **Action**: When diagnostic data reveals an implementation defect (e.g. inverted literal sign, off-by-one indexing in DIMACS mapping, memory leak in CEGAR loop, unclosed file descriptors).
* **Handoff**: Invoke `superpowers:systematic-debugging` to trace and patch the software defect.

### Gate C: To Distributed Batch Execution
* **From**: `sat-experiment-design`.
* **Action**: When executing multi-seed benchmarks across large graph suites (e.g. 1,001 FHCP graphs).
* **Handoff**: Invoke `superpowers:dispatching-parallel-agents` to parallelize execution cleanly across worker instances.

---

## 4. Standardized Research Artifacts Structure

All research deliverables must be archived in reproducible markdown files under `docs/research/`:

```
docs/research/
├── literature/        # sat-literature-mapping outputs
│   └── YYYY-MM-DD-<topic>-sat-literature-map.md
├── phenomena/         # sat-phenomenon-analysis reports
│   └── YYYY-MM-DD-<phenomenon-name>.md
├── root-cause/        # sat-root-cause-analysis 4-tier dossiers
│   └── YYYY-MM-DD-<instance>-sat-root-cause.md
├── hypotheses/        # sat-hypothesis-generation registries
│   └── YYYY-MM-DD-<topic>-sat-hypotheses.md
├── falsifications/   # sat-research-falsification attack reports
│   └── YYYY-MM-DD-<hypothesis-id>-sat-falsification.md
├── experiments/       # sat-experiment-design controlled protocols
│   └── YYYY-MM-DD-<experiment-id>-sat-protocol.md
├── encodings/         # sat-encoding-design formal specifications & proofs
│   └── YYYY-MM-DD-<encoding-name>-sat-spec.md
└── benchmarks/        # sat-benchmark-analysis PAR-2 reports & Cactus data
    └── YYYY-MM-DD-<benchmark-id>-sat-report.md
```
