# Subtour Trajectory Analysis: Experimental Design

**Date:** 2026-07-07
**Branch:** `subtour-experiment`
**Status:** Design spec

## Objective

Rigorously determine whether observed subtour similarity in Incremental SAT solving of HCP represents (A) a CDCL artifact, (B) a consequence of graph topology, or (C) a genuinely interesting search behavior worth further study.

## 1. Architecture & Data Format

### Two-component system

- **C++ solver** (`hcp-solver`): Add `--trajectory <file.ndjson>` flag. Per iteration, serialize raw solver state to NDJSON trace file. Normal solving continues uninterrupted.
- **Python analysis pipeline**: Read NDJSON traces offline, compute all metrics, statistics, visualizations.

### Data flow

```
hcp-solver --trajectory trace.ndjson graph.edge
  → writes one NDJSON row per iteration
  → normal output still written (solution.sat, stderr stats)

analysis-pipeline/
├── run_study.py           # orchestrate solver runs, collect traces
├── compute_metrics.py     # load NDJSON, compute all 14 analyses
├── visualize.py           # generate figures
└── report.py              # statistical summaries, hypothesis tests
```

### NDJSON schema

One row per iteration, batch-written at end of each iteration:

```
{"iteration":1,"action":1,"solve_time_s":0.042,"total_time_s":0.042,
 "solver_conflicts":12,"solver_decisions":45,"solver_propagations":1023,
 "components":[{"id":0,"size":24,"vertices":[3,7,11,...],
                "edges":[104,205,...]},...],
 "blocked_component_ids":[0],
 "model_edge_vars":[5,8,12,...]}

{"iteration":2,...}
...

{"iteration":N,"hamiltonian":true,"cycle":[v0,v1,...,vn-1]}
```

Each row includes the component set that was blocked in the **previous** iteration (via `blocked_component_ids`), closing the chain: blocked constraint → solver response → new subtours.

### C++ changes required

- `Solver.cpp` `runIncremental()`: serialize component sets to NDJSON per iteration
- `SubtourDetector`: return edge variable IDs per component (currently returns only vertices)
- Add `TrajectoryLogger` class: wraps `ofstream`, handles NDJSON serialization, RAII open/close

### Rollback safety

All changes on `subtour-experiment` branch; `optimized-cre` unmodified.

## 2. Experiments

### Tier 1: Per-iteration trace (single run)

| # | Experiment | Computation |
|---|-----------|-------------|
| 1 | Vertex frequency | Count iterations each vertex appears in any subtour component. Histogram. |
| 2 | Edge frequency | Same for edge variables selected in SAT model. |
| 3 | Consecutive Jaccard | For each (i, i+1), compute J(S_i, S_i+1) between component sets and between largest components. Time series. |
| 4 | Jaccard matrix | Full N×N pairwise Jaccard matrix. Hierarchical clustering heatmap. |
| 5 | Persistent core | Core_k = vertices in ≥ k iterations. Size vs. threshold. |
| 6 | Core lifetime | For each core vertex (various k), first and last iteration. Survival curves. |
| 7 | Core evolution | Core size and composition over iterations. |
| 8 | Edge transition | Per consecutive pair: |E_i △ E_i+1|. Added vs. removed edges. |
| 9 | Solver trajectory | Conflicts, decisions, propagations, time per iteration. Overlay component size. |
| 11 | Frequent patterns | Prefix-span or apriori on vertex sets for frequent co-occurring subsets. |

### Tier 2: Multi-run aggregation (3 seeds per graph)

| # | Experiment | Computation |
|---|-----------|-------------|
| 13 | Seed sensitivity | Core set overlap, Jaccard similarity, iteration counts across seeds. |

### Tier 3: Cross-graph aggregation

| # | Experiment | Computation |
|---|-----------|-------------|
| 10 | Similarity clustering | Mean consecutive Jaccard per graph. Cluster graphs. Family separation. |
| 14 | Graph family comparison | Aggregate metrics per family: iterations, core persistence, Jaccard. Box plots. |

### Tier 4: Baseline

| # | Experiment | Computation |
|---|-----------|-------------|
| 12 | CRE baseline | Solve same graphs with CRE (auto cycle). Compare solve time, solution consistency. |

## 3. Hypothesis Testing

### Exp 1-2: Vertex/Edge Frequencies

- **H₀**: Frequencies uniform (all vertices equally likely in subtours).
- **H₁**: Non-uniform — subset dominates.
- **Support H₁**: χ² goodness-of-fit rejects uniformity (p < 0.01) or Gini > 0.3.
- **Reject H₁**: Approx uniform (p > 0.05, Gini < 0.15).

### Exp 3-4: Jaccard Similarity

- **H₀**: Consecutive subtours independent (mean Jaccard ≤ hypergeometric baseline ~0.1).
- **H₁**: Systematically similar (mean Jaccard > 0.1).
- **Support H₁**: One-sample t-test vs μ₀=0.1, p < 0.01, Cohen's d > 0.8.
- **Reject H₁**: Mean Jaccard not significantly > 0.1.

### Exp 5-7: Persistent Cores

- **H₀**: Core size follows binomial expectation: n × P(vertex in component)^k.
- **H₁**: Core exceeds binomial.
- **Support H₁**: Observed core > 95th percentile of binomial for ≥ 3 consecutive k.
- **Reject H₁**: Core within binomial range.

### Exp 8: Edge Transitions

- **H₀**: Edits symmetric (|added| ≈ |removed| per step).
- **H₁**: Asymmetric bias toward removal (contraction toward HC).
- **Support H₁**: Sign test (added−removed), p < 0.01, negative median.
- **Reject H₁**: No significant asymmetry.

### Exp 9: Solver Trajectory

- **H₀**: No correlation component size ↔ solver cost.
- **H₁**: Larger components → harder SAT calls.
- **Support H₁**: Spearman ρ > 0.5, p < 0.01.
- **Reject H₁**: ρ < 0.3.

### Exp 13: Seed Sensitivity

- **H₀**: Core sets seed-dependent.
- **H₁**: Cores robust across seeds.
- **Support H₁**: Jaccard of core sets across seeds > 0.7, ≥ 80% vertex overlap.
- **Reject H₁**: < 30% overlap.

### Exp 10, 14: Cross-graph

- **H₀**: Within-family variance ≈ between-family variance.
- **H₁**: Families cluster by trajectory properties.
- **Support H₁**: ANOVA/Kruskal-Wallis p < 0.01, silhouette > 0.5.
- **Reject H₁**: No significant grouping.

### Exp 11: Frequent Pattern Mining

- **H₀**: No frequent patterns beyond random co-occurrence (support of any pattern matches hypergeometric expectation).
- **H₁**: Non-random frequent vertex subsets exist.
- **Support H₁**: Pattern support exceeds 95th percentile of randomized baseline (shuffled vertex labels per component, 1000 permutations).
- **Reject H₁**: Support within permutation range.

### Exp 12: CRE Baseline

- **H₀**: CRE and incremental solving find Hamilton cycle with equal probability.
- **H₁**: One method succeeds more often or faster.
- **Support H₁**: McNemar's test on paired outcomes (both SAT, both timeout, one wins) p < 0.05, or Wilcoxon signed-rank on solve times p < 0.05.
- **Reject H₁**: No significant difference.

### Statistical methods

- Normality: Shapiro-Wilk.
- Non-parametric: Mann-Whitney U, Kruskal-Wallis H, Spearman ρ.
- Confidence intervals: 95% bootstrap CI (1000 resamples).
- Effect size: Cohen's d, ε², ρ.
- Multiple comparison: Bonferroni-Holm across 14 experiments per graph.

## 4. Expected Outcomes

### Scenario A: CDCL artifact
- Uniform/slightly skewed frequencies (Gini < 0.2)
- Jaccard barely above baseline (0.1–0.2)
- No cores beyond binomial
- Cores seed-dependent
- Conflicts decrease steadily
- No family clustering

### Scenario B: Graph topology driven
- Non-uniform frequencies — core vertices are structurally important (high-degree, articulation points)
- Cores deterministic across seeds
- Jaccard correlates with graph invariants
- Strong family clustering
- Core size correlates with graph size, not iteration count

### Scenario C: New search behavior
- Cores exceed binomial expectation AND graph-structural expectation
- Jaccard non-monotonic (oscillates)
- Cyclic core re-emergence (same vertices exit/re-enter)
- Structured edge asymmetry
- Seeds produce similar (70%) but not identical cores
- Solver trajectory shows regimes/plateaus

### Key discriminators

| Evidence | A | B | C |
|----------|---|---|---|
| Core > binomial | Weak | Strong (if structural) | Strong (if NOT structural) |
| Seed-invariant core | Weak | Strong | Moderate |
| Family clustering | None | Strong | Moderate |
| Non-structural core | N/A | Contradicts | Strong |
| Cyclic re-emergence | Weak | Weak | Strong |
| Jaccard oscillates | Weak | Weak | Strong |

## 5. Evidence Quality

- **Weak**: Mean Jaccard > 0.1, p < 0.05 but d < 0.5. Single family. No seed check.
- **Moderate**: Jaccard > 0.3, d > 0.8, multiple families. Cores exceed binomial at k ≥ 5. Seeds agree ≥ 70%. Multiple-comparison correction applied.
- **Strong**: Jaccard > 0.5, bootstrapped CI lower bound > 0.3. Cores non-structural. Cyclic re-emergence significant. Effect sizes large across ALL families. Reproducible across seeds AND solvers (Glucose, Kissat). Rigorous vs. hypergeometric model.
- **Publication (journal)**: All strong + controls for graph size/density/degree. Null model via random edge-assignment Markov chain. Open-source reproducible pipeline. Quantitative related-work comparison.

## 6. Future Research Directions (ranked)

1. **Persistent vertex cores**: Graph's "Hamiltonian backbone"? Fixing core vertices to reduce search. Direct solver impact.
2. **Overlap/transition graphs**: Subtour set as nodes, transitions as edges. Small-world? Reveals search basins.
3. **Backbone structures**: Compare core with intersection of all HCs (classical concept). Novel if core ⊈ backbone.
4. **Recurring search states**: Hash subtour sets to detect cycles. Practical inefficiency if cycles exist.
5. **Search basins**: Cluster iterations by subtour similarity. Inform restart/rephase heuristics.
6. **Frequent substructures**: Subgraphs appearing in many subtours. Pre-block as "poison" subgraphs.
7. **Edge cores**: Edges persisting across iterations but never in HC. If impossible in any HC → pruning.

## 7. Experimental Roadmap

### Phase 1: Pilot (implement first)
- **Instrumentation**: Add `--trajectory` flag, NDJSON output, `TrajectoryLogger`
- **Exp 1, 2, 3, 5**: Vertex/edge frequency, consecutive Jaccard, core detection
- **Single graph**: Pick smallest fhcppp graph (graph1, ~64 nodes). Fast iteration.
- **Goal**: Verify instrumentation works, phenomenon visible.

### Phase 2: Core analysis
- **Exp 4, 6, 7, 8, 9**: Full per-graph analysis suite
- **All fhcppp graphs** (smallest family, fastest)
- **Goal**: Characterize phenomenon within a family.

### Phase 3: Generalization
- **Exp 10, 14, 13**: Cross-family comparison, seed sensitivity
- **All families** + generated graphs
- **Goal**: Determine if phenomenon generalizes. Distinguish A vs B vs C.

### Phase 4: Baseline and rigor
- **Exp 11, 12**: Frequent patterns, CRE baseline
- **Statistical rigor**: Bootstrapping, effect sizes, multiple comparison correction
- **Goal**: Reach moderate-to-strong evidence level.

### Phase 5 (if warranted)
- Multi-solver reproducibility (Glucose, Kissat)
- Null model validation
- Controls for graph invariants
- **Goal**: Publication-quality evidence.
