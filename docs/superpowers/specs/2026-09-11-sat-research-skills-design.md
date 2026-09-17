# Design Spec: SAT-Centric Research Skills Suite for Superpowers

**Date:** 2026-09-11  
**Status:** Approved by User (No-Commit Mode)  
**Topic:** SAT-Centric & SAT-Hybrid Algorithmic Research Skills Suite Synchronized with Superpowers  

---

## 1. Overview & Motivation

In algorithmic research for NP-hard combinatorial optimization problems—specifically the Hamiltonian Cycle Problem (HCP) solved via SAT-based CEGAR and CDCL engines—pure heuristic approaches (e.g. standalone local search or genetic algorithms) often struggle with scalability and completeness on pathological instances. Conversely, modern SAT solvers (e.g., CaDiCaL, Kissat) provide powerful conflict-driven clause learning (CDCL), inprocessing, and incremental solving capabilities.

This specification establishes a suite of **8 specialized SAT-centric research skills** where **SAT is the dominant driver and core engine** of problem solving. Heuristics and metaheuristics (such as 2-opt/3-opt cycle patching, DP bitmask component solvers, graph contraction, and local search) are integrated strictly as auxiliary satellites—providing phase saving initialization hints, generating lazy CEGAR cuts, or solving localized sub-instances.

The suite is structured as a **State-Aware Research Mesh** that synchronizes seamlessly with the **Superpowers** ecosystem (`brainstorming`, `writing-plans`, `subagent-driven-development`, `systematic-debugging`, `test-driven-development`, `verification-before-completion`).

---

## 2. Core Philosophy: SAT as the Primary Backbone

1. **SAT-Centric Focus**: Every hypothesis, diagnosis, and encoding must reason about propositional semantics, Boolean Constraint Propagation (BCP), conflict graphs, learned clause database management, and resolution complexity.
2. **Auxiliary Heuristics**: Heuristics are allowed and encouraged, but they must interface with the SAT solver through formal channels (incremental assumptions, phase-saving hints, lazy propagator callbacks, or external CEGAR cuts).
3. **Falsification-First**: Unvalidated heuristic guesses are strictly barred. Any proposed algorithmic modification must be formulated as a causal claim about SAT dynamics and survive adversarial counterexample probes before implementation.

---

## 3. Architecture & State Machine Flow

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
                 [ Loop Closure: New Discovery or Next Iteration ]
```

### 3.1 Superpowers Hand-off Gates

1. **Gate A — To Engineering Implementation (`sat-encoding-design` / `sat-experiment-design` ➔ `writing-plans` + `test-driven-development`)**:
   - Once a SAT encoding specification or experimental protocol is complete, the agent MUST NOT write production code within the research skill.
   - It triggers `superpowers:writing-plans` to generate an implementation plan.
   - Code development in Rust (`src/cegar-fix`) or Python must follow `superpowers:test-driven-development` to verify clause invariants, soundness, and completeness on small graphs.
2. **Gate B — To Software Bug Debugging (`sat-root-cause-analysis` ➔ `systematic-debugging`)**:
   - If an investigation reveals an implementation flaw (e.g. corrupted literal mapping, memory leak in CEGAR loop, invalid DIMACS formatting), the agent transfers control to `superpowers:systematic-debugging`.
3. **Gate C — To Distributed Batch Experimentation (`sat-experiment-design` ➔ `dispatching-parallel-agents`)**:
   - Batch executions across large benchmark suites (e.g. 1,001 graphs in FHCPCS) are executed via `superpowers:dispatching-parallel-agents` or `subagent-driven-development`.

---

## 4. Detailed Specifications for the 8 Skills

### Skill 1: `sat-literature-mapping`
- **YAML Frontmatter**:
  ```yaml
  ---
  name: sat-literature-mapping
  description: Use when surveying academic literature on SAT-based combinatorial solving, incremental SAT-CEGAR, SMT lazy clause learning, and CDCL theoretical barriers.
  ---
  ```
- **Core Objectives**:
  1. Survey SAT-based solving architectures: SAT-CEGAR, incremental SAT with assumptions, lazy clausal propagators, and SAT + heuristic hybrids (phase seeding from local search, DP-bitmask cut oracles).
  2. Map theoretical CDCL barriers: Resolution Lower Bounds (Tseitin formulas on expanders), Pigeonhole obstructions, Treewidth bounds, and Chvátal-Erdős conditions.
  3. Synthesize a **SAT Research Gap Matrix**: `[SAT Approach | Underlying CDCL Assumption | Failure Point on Hard Instances | Novel Opportunity]`.
- **Hard Gate**: Strictly prohibit passive bibliographies. Must identify at least one concrete CDCL or encoding bottleneck that remains unaddressed in prior literature.
- **Output Artifact**: `docs/research/literature/YYYY-MM-DD-<topic>-sat-literature-map.md`.

---

### Skill 2: `sat-phenomenon-analysis`
- **YAML Frontmatter**:
  ```yaml
  ---
  name: sat-phenomenon-analysis
  description: Use when analyzing benchmark logs and runtime traces to isolate SAT-specific performance anomalies, CDCL hardness cliffs, and CEGAR iteration bottlenecks.
  ---
  ```
- **Core Objectives**:
  1. Time Budget Decomposition: Measure $T_{total} = T_{CDCL} + T_{cut\_generation} + T_{heuristic\_patching}$. Establish whether the bottleneck resides inside CDCL or in auxiliary modules.
  2. Detect SAT search anomalies: Sudden hardness cliffs (orders of magnitude runtime divergence on graphs of identical size/density), cycle shattering (CDCL generating hundreds of disjoint small cycles instead of merging into a giant cycle).
  3. Noise Elimination: Test across $\ge 3$ random solver seeds in CaDiCaL to filter out lucky decision orderings from true algorithmic properties.
- **Hard Gate**: Never draw conclusions without quantifying the proportion of time spent within the CDCL core versus auxiliary heuristics.
- **Output Artifact**: `docs/research/phenomena/YYYY-MM-DD-<phenomenon-name>.md`.

---

### Skill 3: `sat-root-cause-analysis` (🔴 Critical Diagnostic Core)
- **YAML Frontmatter**:
  ```yaml
  ---
  name: sat-root-cause-analysis
  description: Use when diagnosing why a SAT solver or CEGAR loop stalls or times out, drilling down through the 4-tier causal stack: Graph Topology → CNF Formulation → CDCL Search Dynamics → Engine Runtime.
  ---
  ```
- **The 4-Tier SAT Diagnostic Stack**:
  1. **Tier 1 (Graph Topology ➔ SAT Impact)**:
     - Identify topological obstacles: Cut-edges, bottleneck bridges, high-overlap alternating cycles, tight vertex-sharing gadgets.
     - Identify candidate backbone edges that must be true in every valid assignment.
  2. **Tier 2 (CNF Formulation & BCP Propagation)**:
     - Variable semantics: Directed arcs vs undirected edges vs positional stage variables.
     - Unit Propagation power: Do subtour elimination cuts trigger BCP early to prune the decision tree, or remain inactive until full assignment?
  3. **Tier 3 (CDCL Search Dynamics)**:
     - Conflict Graph analysis: 1UIP depth and decision levels of conflicts.
     - Learned clause Literal Block Distance (LBD) distribution: Are learned clauses high quality (LBD $\le 3$) or bloated (LBD $> 10$) causing premature purge during garbage collection?
     - Phase Saving Thrashing: Is CaDiCaL rapidly alternating polarities across gadget boundaries without deriving global conflict clauses?
     - VSIDS / CHB Starvation: Is branching heuristic starved on critical cut-edges?
  4. **Tier 4 (Solver Engine Runtime & CEGAR Mechanics)**:
     - Watchlist traversal overhead, cache locality during BCP, assumption reset costs in incremental loops.
- **Hard Gate**: **Strict Root-Cause Prohibition of Heuristic Guesses**. Strictly forbid proposing any patch, heuristic, or encoding tweak until the failure mechanism is demonstrated at Tier 2 (CNF/BCP) or Tier 3 (CDCL Dynamics) with empirical solver data.
- **Output Artifact**: `docs/research/root-cause/YYYY-MM-DD-<instance>-sat-root-cause.md`.

---

### Skill 4: `sat-hypothesis-generation`
- **YAML Frontmatter**:
  ```yaml
  ---
  name: sat-hypothesis-generation
  description: Use when formulating competing, scientifically rigorous, and falsifiable hypotheses centered on SAT mechanics, CDCL search dynamics, and SAT-heuristic interactions.
  ---
  ```
- **Core Objectives**:
  1. Enforce Chamberlin's Multiple Working Hypotheses: Formulate at least 2–3 competing hypotheses explaining the causal mechanism.
  2. Each hypothesis must explicitly link to SAT solver internals:
     - Mechanistic claim (how it modifies BCP, conflict depth, LBD, or phase saving).
     - Quantitative prediction of SAT metrics (e.g. "reduces conflict count by $\ge 40\%$", "shifts average LBD from $12.4$ to $\le 3.5$").
     - Quantitative falsification threshold (e.g. "if conflict count does not decrease by at least $25\%$ across 5 seeds, the hypothesis is refuted").
  3. Parsimony: Prioritize hypotheses that introduce minimal additional constraints while maximizing BCP deductions.
- **Hard Gate**: Reject any hypothesis lacking an explicit, quantitative falsification threshold tied to SAT solver metrics.
- **Output Artifact**: `docs/research/hypotheses/YYYY-MM-DD-<topic>-sat-hypotheses.md`.

---

### Skill 5: `sat-research-falsification` (🔴 Mandatory Adversarial Gate)
- **YAML Frontmatter**:
  ```yaml
  ---
  name: sat-research-falsification
  description: Use when stress-testing and attempting to disprove SAT-centric hypotheses using adversarial counterexample graphs, clause pollution checks, and confounder analysis before coding.
  ---
  ```
- **Core Objectives**:
  1. **Adversarial Graph Mining**: Construct or identify minimal graphs ($N \le 20$) with deceptive topologies (e.g. bipartite traps without Hamiltonian cycles, highly symmetric Petersen-like subgraphs) designed to defeat the proposed mechanism.
  2. **Clause Pollution & Watchlist Bloat Verification**: Verify whether proposed auxiliary clauses or cuts pollute the solver's 2-watched-literal lists with redundant, high-LBD constraints that degrade BCP throughput.
  3. **Confounder Isolation**: Verify whether observed runtime differences stem merely from variable numbering permutations, compiler optimization flags, or random seed luck.
  4. **Verdict Classification**: Refuted (return to Skill 4) | Refined (constrain graph class) | Survives (approved for experiment design).
- **Hard Gate**: No hypothesis may proceed to encoding design or implementation without surviving at least 3 distinct adversarial tests (minimal trap graph, clause bloat check, seed jitter test).
- **Output Artifact**: `docs/research/falsifications/YYYY-MM-DD-<hypothesis-id>-sat-falsification.md`.

---

### Skill 6: `sat-experiment-design` (🔴 Critical Empirical Gate)
- **YAML Frontmatter**:
  ```yaml
  ---
  name: sat-experiment-design
  description: Use when designing controlled, statistically valid experiments measuring SAT-internal dynamics, CDCL metrics, and evaluating against SOTA SAT baselines.
  ---
  ```
- **Core Objectives**:
  1. Standard Baseline Comparisons: Compare against pure CaDiCaL 1.9.5, Kissat, Glucose, and Takehide Soh's canonical SAT-based CEGAR solver.
  2. Ablation Matrix: Isolate the contribution of each component:
     $$\text{Pure SAT Baseline} \quad \text{vs} \quad \text{SAT + Heuristic Phase Seed} \quad \text{vs} \quad \text{SAT + New Cut} \quad \text{vs} \quad \text{Full Proposed System}$$
  3. Comprehensive Metric Suite:
     - External: Solved instance count, PAR-2 score (Penalized Average Runtime with $2\times$ timeout penalty), Cactus plot curve.
     - Internal CDCL: BCP propagations/sec, total conflicts, decisions, learned clause LBD distribution, purge rate, percentage of time inside CDCL.
  4. Stratified Benchmark Tiers: Easy ($<1$s), Medium ($1$s–$60$s), Hard ($60$s–$1,800$s), Timeout ($>1,800$s).
  5. Pre-registered Failure Threshold: A decrease in solved instances on Easy/Medium sets ($>5\%$) triggers immediate disqualification.
- **Hard Gate & Superpowers Handoff**: Hand off directly to `superpowers:writing-plans` to script runners and `superpowers:dispatching-parallel-agents` to execute.
- **Output Artifact**: `docs/research/experiments/YYYY-MM-DD-<experiment-id>-sat-protocol.md`.

---

### Skill 7: `sat-encoding-design`
- **YAML Frontmatter**:
  ```yaml
  ---
  name: sat-encoding-design
  description: Use when formalizing verified hypotheses into sound, complete, and propagation-efficient SAT encodings, incremental assumption interfaces, and CEGAR cut mechanisms.
  ---
  ```
- **Core Objectives**:
  1. Formal Variable Specification:
     - Arc/edge variables $x_{u,v}$, order/stage variables $y_{v,k}$, assumption selector literals $a_k$.
  2. Constraint Classes:
     - Degree-2 constraints (Pairwise vs Commander vs Bimultaneous encoding).
     - Subtour Elimination: Lazy DFJ cuts vs MTZ ordering constraints vs Lazy SMT clauses.
     - Incremental Assumption Interface: Efficient activation/deactivation of structural assumptions across solver calls.
     - SAT-Heuristic Interface: Feeding heuristic seeds (phase saving, backbone locks) into solver state.
  3. Complexity & Propagation Strength Analysis:
     - Big-O clause and variable scaling ($O(|V|), O(|E|)$).
     - Unit propagation strength: Does the formulation achieve Arc Consistency (GAC) or Unit-Refutation Completeness?
     - Minimization of 2-watched-literal watchlist cache misses.
  4. Soundness & Completeness Proof:
     - Invariant 1 (Soundness): Every satisfying assignment decodes to a valid Hamiltonian cycle.
     - Invariant 2 (Completeness): No valid Hamiltonian cycle is rendered UNSAT.
  5. Invariant Test Cases: Minimal validation graphs ($N=3, 4, 5, 6$).
- **Hard Gate & Superpowers Handoff**: Hand off to `superpowers:writing-plans` and `superpowers:test-driven-development` to implement in Rust (`src/cegar-fix`) or Python.
- **Output Artifact**: `docs/research/encodings/YYYY-MM-DD-<encoding-name>-sat-spec.md`.

---

### Skill 8: `sat-benchmark-analysis`
- **YAML Frontmatter**:
  ```yaml
  ---
  name: sat-benchmark-analysis
  description: Use when evaluating benchmark outputs using SAT Competition standards, calculating PAR-2 scores, Cactus plots, statistical significance, and closing the scientific discovery loop.
  ---
  ```
- **Core Objectives**:
  1. SAT Competition Standard Metrics:
     - PAR-2 scoring (penalizing timeouts at $2\times$ cutoff).
     - Cumulative Cactus plot generation.
     - Virtual Best Solver (VBS) contribution: Unique instances solved exclusively by this method.
  2. Statistical Rigor: Paired Wilcoxon signed-rank test ($p < 0.05$) to confirm improvements over baseline are statistically significant.
  3. Regression Analysis: Detailed inspection of every instance that ran slower than baseline (diagnosing whether degradation was caused by watchlist bloat or misguided phase choices).
  4. Scientific Loop Closure: Uncovered anomalies feed directly into `sat-phenomenon-analysis` or `sat-root-cause-analysis` for the subsequent research iteration.
- **Hard Gate**: Strictly prohibit claiming superiority based on mean runtime if total solved instances decreased in any difficulty stratum.
- **Output Artifact**: `docs/research/benchmarks/YYYY-MM-DD-<benchmark-id>-sat-report.md`.

---

## 5. Directory Structure & Dual Deployment

### 5.1 Installation Targets
1. **Global Superpowers Plugin** (`~/.gemini/config/plugins/superpowers/skills/`):
   ```
   ~/.gemini/config/plugins/superpowers/skills/
   ├── sat-literature-mapping/SKILL.md
   ├── sat-phenomenon-analysis/SKILL.md
   ├── sat-root-cause-analysis/SKILL.md
   ├── sat-hypothesis-generation/SKILL.md
   ├── sat-experiment-design/SKILL.md
   ├── sat-research-falsification/SKILL.md
   ├── sat-encoding-design/SKILL.md
   └── sat-benchmark-analysis/SKILL.md
   ```
2. **Project Workspace Mirror** (`/home/ubuntu/HCP/.agents/skills/`):
   Identical copy maintained in the workspace repository.

### 5.2 Research Artifacts Structure
```
/home/ubuntu/HCP/docs/research/
├── literature/        # SAT literature maps & research gap matrices
├── phenomena/         # Anomaly dossiers & hardness cliff traces
├── root-cause/        # 4-tier diagnostic reports (Graph/CNF/CDCL/Runtime)
├── hypotheses/        # Competing mechanistic SAT hypotheses
├── falsifications/   # Counterexample trap graphs & clause bloat reports
├── experiments/       # Ablation matrices & empirical CDCL protocols
├── encodings/         # Formal CNF/CEGAR specifications & proofs
└── benchmarks/        # SAT Competition PAR-2 reports & Cactus data
```

---

## 6. Research Playbook & Router (`research-workflow-router.md`)

A centralized router guide maps situations to specific skills:

| Current Research State | Triggered Skill | Next Expected State / Hand-off |
| :--- | :--- | :--- |
| Solver timeout, stall, or memory explosion on benchmark | `sat-phenomenon-analysis` | ➔ `sat-root-cause-analysis` |
| Exploring theoretical bounds, prior SOTA, or baseline encodings | `sat-literature-mapping` | ➔ `sat-hypothesis-generation` |
| Solver paralyzed; need to determine exact failure tier | `sat-root-cause-analysis` | ➔ `sat-hypothesis-generation` (or `systematic-debugging` if software bug) |
| Multiple competing ideas for solver improvement | `sat-hypothesis-generation` | ➔ `sat-research-falsification` (Mandatory) |
| Hypothesis formulated; need to stress-test before code | `sat-research-falsification` | ➔ `sat-experiment-design` (if passes) or `sat-hypothesis-generation` (if refuted) |
| Hypothesis verified; need controlled evaluation protocol | `sat-experiment-design` | ➔ `writing-plans` (to code benchmark harnesses) |
| Formulating new CNF clauses, assumptions, or CEGAR cuts | `sat-encoding-design` | ➔ `writing-plans` + `test-driven-development` |
| Benchmark run completed; raw logs and CSVs available | `sat-benchmark-analysis` | ➔ Close loop or re-enter `sat-phenomenon-analysis` |

---

## 7. Self-Review & Quality Assurance Checklist

- [x] **Placeholder Scan**: Zero "TBD", "TODO", or incomplete sections.
- [x] **SAT-Centric Verification**: Every skill explicitly centers on SAT mechanics (BCP, CDCL, LBD, 1UIP, assumptions) and treats heuristics as auxiliary satellites.
- [x] **Internal Consistency**: All 8 skills follow identical Agentskills YAML frontmatter conventions (`Use when...`), Hard Gates, and artifact naming standards.
- [x] **Scope Demarcation**: Explicit boundaries between the scientific discovery layer (the 8 SAT skills) and the engineering implementation layer (Superpowers core skills).
- [x] **No-Commit Directive Honored**: File generated in workspace without triggering git commit.
