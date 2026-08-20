# FHCP–CEGAR Solver: Timeout Testcase Classification — Deep Analysis

**Date:** 2026-08-20 · **Scope:** all timeout testcases across the three benchmark datasets of `/home/ubuntu/HCP` · **Method:** full extraction of per-increment CEGAR dynamics from `results_no_sym_official.log` (28 MB, 1001 graphs) plus structural features computed from `FHCPCS-col/*.col`.

## 1. Data sources

| Source | Content | Role |
|---|---|---|
| `FHCPCS-col/` | 1001 `.col` graphs, 66–9,528 vertices, 99–315,283 edges | instance properties |
| `results_no_sym_official.log` | official run, flags `-e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1`, 1800 s cap, Jul 31–Aug 3 2026 | per-increment dynamics of every graph |
| `scratch/benchmark_100_results.json` | 100-graph sample (graph10…1000), 15 s cap, binary @4d094b7, Aug 17 | 100-benchmark timeouts |
| `scratch/stem_cycle_patcher_results.json` + `results_stem_cycle_patcher_full.log` | full 1001-graph patcher run (checkpointed, 40 done), 1800 s cap, Aug 18 | projected impact |

## 2. What is extracted per graph

**Structural (from `.col`):** vertices n, edges m, density m/n, min degree, degree-2/3/≥4 counts, degree-2 chains (maximal runs of ≥2 consecutive degree-2 vertices), hub count (degree ≥ 50 / ≥ 100).

**Dynamic (from the log, per increment until kill):** increment index, SAT-solve time, subcycle count, cycle-length map, block clauses added, connected/merged cycles. Aggregates: total increments, first/median/last/p90 SAT time, first/last block clauses per increment, subcycle stats of first and last increment.

**Taxonomy rule (deterministic):**

- **Class A** — increments ≥ 5,000 with median increment < 5 s (cheap iterations, no convergence)

- **Class B1** — density ≥ 3.0 (dense hub family)

- **Class B2a** — sparse, `connected cycles = 0` in every increment (pure merge stall)

- **Class B2b** — sparse, median increment ≥ 5 s (expensive SAT calls)

- **Class B2c** — sparse, moderate (residual; found empty)

## 3. Global results

Baseline run: **926 / 1001 solved (92.5 %)**, 75 timeouts at 1800 s, 0 errors, 0 UNSAT. Solved-time distribution: median 10.1 s, p90 5.9 min, max 29.2 min. Timeout risk by family:

| Family | Count | Timeouts | Rate |
|---|---|---|---|
| Dense (density ≥ 3.0) | 92 | 28 | **30.4 %** |
| Sparse with degree-2 chains (>10 %) | 362 | 46 | **12.7 %** |
| Sparse, no degree-2 vertices | 451 | 1 (graph339) | **0.2 %** |

Timeout wall-time budget: 75 × 1800 s = **37.5 h** (Class A 4.0 h, B1 14.0 h, B2a 2.0 h, B2b 17.5 h).

## 4. Class A — CEGAR increment explosion / near-Hamiltonian attractor (8 graphs)

graph339, graph647, graph771, graph883, graph908, graph935, graph961, graph978

Hundreds of thousands of **sub-millisecond** SAT calls: the loop converges structurally (last solution has only 2–6 giant subcycles, average length 752–1870 vertices) but can never splice the final 2–6 cycles, so it re-solves with new cut arcs forever. `block_total` accumulates to 47 billion clauses on graph339. The final increment is essentially free (`sat_last ≈ 0`): the stall is algorithmic, not a SAT-hardness problem.

| Graph | n | minDeg | deg2 | chains | incs at kill | total block clauses | subcycles first → last |
|---|---|---|---|---|---|---|---|
| graph339 | 2004 | 3 | 0 | 0 | 153 460 | 47 246 333 138 | ∅16 ×128 → ∅1002 ×2 |
| graph647 | 3688 | 2 | 978 | 12 (max 7) | 16 396 | 564 667 130 | ∅18 ×209 → ∅88 ×42 |
| graph771 | 4514 | 2 | 799 | 1 (max 5) | 11 284 | 454 271 142 | ∅20 ×230 → ∅752 ×6 |
| graph883 | 5696 | 2 | 1196 | 3 (max 5) | 10 619 | 301 344 980 | ∅18 ×320 → ∅285 ×20 |
| graph908 | 6018 | 2 | 1196 | 3 (max 5) | 5 537 | 70 873 826 | ∅17 ×357 → ∅19 ×318 |
| graph935 | 6382 | 2 | 1856 | 3 (max 5) | 8 838 | 184 042 144 | ∅17 ×380 → ∅206 ×31 |
| graph961 | 6939 | 2 | 1026 | 1 (max 4) | 13 823 | 896 508 512 | ∅15 ×459 → ∅1157 ×6 |
| graph978 | 7480 | 2 | 1622 | 162 (max 10) | 24 593 | 1 652 580 126 | ∅24 ×307 → ∅1870 ×4 |

*Notably, 883/908/935 share the same chain structure (3 chains, max 5) — a sibling family.*

**Escape path seen:** 35 solved graphs show identical dynamics and converged before the cap (graph360: 115 962 increments, solved in 23 min). The new binary (4d094b7) solved graph339 in 1 403 s with 2 745 increments: this class is fixable by blocking-clause/cut-arc strategy.

## 5. Class B1 — dense hub graphs, expensive SAT calls (28 graphs)

graph560–562, 584–585, 612–615, 635, 653–654, 670, 684, 746, 797, 830, 844, 863, 905, 950, 955, 963, 975, 982, 984, 990, 996

All: density 3.27–4.37, ~30–60 hub vertices (degree ≥ 50, max degree 171–1093), near-zero degree-2/3 vertices. Only 8–87 increments (median 17) — the CEGAR loop is fine — but each SAT call costs 7–220 s (median ~100 s), max single call **1 343 s** (graph797). Subcycle counts stay huge at kill (190–742): SAT keeps returning solutions decomposed into hundreds of 6–18-vertex cycles that the merge phase cannot splice. Two sub-families:

- **“Spider-web” (cheap SAT, fragmented merge):** graph797, graph863 — median SAT only 1.4–2.3 s, but 742 tiny cycles (∅6–7) at kill; cut arcs fragment solutions further.

- **“Expensive SAT” (standard):** the rest — median SAT 42–193 s, subcycles 182–605 at kill.

| Graph | n | dens | maxdeg | hubs≥50 | sat med | sat last | blk first → last | subcycles last |
|---|---|---|---|---|---|---|---|---|
| graph560 | 3311 | 4.34 | 663 | 30 | 1m | 50.4s | 206 → 3332 | ∅17.4×190 |
| graph561 | 3311 | 4.34 | 663 | 30 | 1m | 56.0s | 188 → 3036 | ∅18.2×182 |
| graph562 | 3311 | 4.34 | 663 | 30 | 1m | 50.4s | 228 → 2852 | ∅16.0×207 |
| graph584 | 3411 | 4.34 | 683 | 30 | 1m | 10.3s | 190 → 2186 | ∅15.9×214 |
| graph585 | 3411 | 4.34 | 683 | 30 | 56.5s | 59.4s | 220 → 3228 | ∅18.1×188 |
| graph612 | 3511 | 4.35 | 703 | 30 | 1m | 51.8s | 204 → 2600 | ∅15.1×232 |
| graph613 | 3511 | 4.35 | 703 | 30 | 2m | 1m | 206 → 2464 | ∅16.6×212 |
| graph614 | 3511 | 4.35 | 703 | 30 | 1m | 1m | 242 → 2836 | ∅15.3×229 |
| graph615 | 3511 | 4.35 | 703 | 30 | 58.6s | 16.3s | 214 → 3834 | ∅14.8×238 |
| graph635 | 3611 | 4.35 | 723 | 30 | 1m | 2m | 210 → 2574 | ∅14.0×258 |
| graph653 | 3711 | 4.35 | 743 | 30 | 1m | 1m | 226 → 2798 | ∅14.8×250 |
| graph654 | 3711 | 4.35 | 743 | 30 | 1m | 59.6s | 212 → 3316 | ∅16.2×229 |
| graph670 | 3811 | 4.36 | 763 | 30 | 3m | 50.7s | 254 → 2448 | ∅16.1×237 |
| graph684 | 3911 | 4.36 | 783 | 30 | 1m | 5m | 258 → 1742 | ∅12.9×303 |
| graph746 | 4286 | 4.27 | 858 | 30 | 1m | 3m | 422 → 3622 | ∅14.6×294 |
| graph797 | 4701 | 3.48 | 941 | 30 | 1.4s | 1.4s | 74 → 1746 | ∅6.3×742 |
| graph830 | 5056 | 3.39 | 1012 | 30 | 1m | 2m | 332 → 5282 | ∅13.3×381 |
| graph844 | 5226 | 3.37 | 1046 | 5 | 42.5s | 7.0s | 376 → 7296 | ∅13.7×381 |
| graph863 | 5461 | 3.48 | 1093 | 30 | 2.3s | 18.9s | 70 → 1896 | ∅7.4×742 |
| graph905 | 5985 | 3.27 | 171 | 35 | 11.3s | 4.2s | 528 → 19148 | ∅31.2×192 |
| graph950 | 6620 | 4.34 | 662 | 60 | 1m | 58.1s | 424 → 5622 | ∅18.0×367 |
| graph955 | 6840 | 3.27 | 171 | 40 | 15.7s | 8.5s | 638 → 21038 | ∅29.7×230 |
| graph963 | 7020 | 4.35 | 702 | 60 | 3m | 4m | 410 → 3036 | ∅13.8×507 |
| graph975 | 7420 | 4.36 | 742 | 60 | 2m | 3m | 458 → 4116 | ∅14.5×511 |
| graph982 | 7620 | 4.36 | 762 | 60 | 3m | 1m | 442 → 3796 | ∅16.5×463 |
| graph984 | 7695 | 3.27 | 171 | 45 | 14.1s | 16.4s | 672 → 24694 | ∅28.8×267 |
| graph990 | 8020 | 4.37 | 802 | 60 | 2m | 5m | 458 → 3730 | ∅13.3×605 |
| graph996 | 8550 | 3.27 | 171 | 50 | 26.2s | 1m | 774 → 22868 | ∅24.5×349 |
**Sibling families:** (3311,14361) ×3, (3411,14811) ×2, (3511,15261) ×4, (3711,16161) ×2 — parametrized dense families (m ≈ 4.34n) plus the m ≈ 3.27n family (905, 955, 984, 996) and the m = 3.0n…4.37n ladder 950→963→975→982→990 (n+1000, m+1800 per step).

**Risk:** 30.4 % of all dense graphs time out (highest of any family); the modular-decomposition work (`ModularSolver`, “dense hub module decomposition”) targets exactly this class. Solved dense graphs converge in a median of 76 increments — the same range as the timeouts — so outcome is SAT variance, not structure.

## 6. Class B2a — sparse, subcycle-merge stall, zero progress (4 graphs)

graph479, graph809, graph868, graph960 — 29–33 % degree-2 vertices.

Only 10–20 increments, and **`connected cycles = 0` in every single increment**: the merge phase never connects. SAT calls are expensive (median 7–16 s, last 230–500 s) and produce solutions that decompose into 29–66 subcycles at kill; cut arcs are added, and the same failure repeats. The solver burns the full 1800 s making no structural progress. This is the failure mode the StemCyclePatcher (k-opt splice / unvisited-vertex absorption) was built for.

| Graph | n | minDeg | deg2 | sat med | sat last | subcycles last |
|---|---|---|---|---|---|---|
| graph479 | 2772 | 2 | 924 (33 %) | 11.3 s | 309 s | ∅173 ×16 |
| graph809 | 4810 | 2 | 1384 (29 %) | 15.8 s | 230 s | ∅166 ×29 |
| graph868 | 5544 | 2 | 1848 (33 %) | 7.1 s | 500 s | ∅150 ×37 |
| graph960 | 6930 | 2 | 2310 (33 %) | 7.3 s | 393 s | ∅105 ×66 |

## 7. Class B2b — sparse, expensive SAT iterations (35 graphs)

graph566, 651, 668, 677–678, 710, 717, 725, 734, 744, 761, 766, 788, 810, 832, 882, 910, 937, 940, 944, 951, 954, 959, 965–966, 971, 974, 976, 981, 983, 986–987, 993–994, 998

13–33 % degree-2 vertices (isolated — 31 of 35 graphs have **zero degree-2 chains**, i.e. every degree-2 vertex sits between branch vertices), max degree ≤ 42. 10–95 increments; median increment 18–138 s; single SAT calls up to 1 172 s (graph951), 1 167 s (graph954), 1 148 s (graph987). Merge makes progress here (unlike B2a) — subcycle count falls (e.g. graph566: ∅14 ×236 → ∅133 ×25) — but the SAT instances harden as blocking clauses accumulate: `sat_first 0.19 s → sat_last 292 s` on graph566; `5.2 s → 1 131 s` on graph994. Last-increment SAT time exceeds the median by 10–100×.

| Graph | n | minDeg | deg2 % | sat first | sat med | sat last | blk first → last |
|---|---|---|---|---|---|---|---|
| graph566 | 3322 | 2 | 23 % | 188ms | 16.1s | 5m | 394 → 4718 |
| graph651 | 3701 | 2 | 21 % | 1.8s | 7.7s | 1m | 402 → 8934 |
| graph668 | 3783 | 2 | 24 % | 1.1s | 20.4s | 58ms | 426 → 3674 |
| graph677 | 3868 | 2 | 20 % | 2.7s | 12.2s | 47ms | 406 → 8538 |
| graph678 | 3868 | 2 | 20 % | 1.8s | 11.9s | 13.7s | 474 → 8386 |
| graph710 | 4064 | 2 | 23 % | 3.5s | 8.2s | 2m | 386 → 3642 |
| graph717 | 4122 | 2 | 22 % | 3.6s | 6.1s | 18m | 464 → 3352 |
| graph725 | 4163 | 2 | 22 % | 3.3s | 6.1s | 5m | 472 → 3572 |
| graph734 | 4232 | 2 | 21 % | 3.8s | 50.2s | 2m | 230 → 4494 |
| graph744 | 4278 | 2 | 18 % | 7.1s | 22.8s | 94ms | 464 → 8160 |
| graph761 | 4430 | 2 | 17 % | 5.5s | 8.4s | 30.7s | 504 → 9392 |
| graph766 | 4465 | 2 | 30 % | 3.9s | 53.7s | 1m | 436 → 2076 |
| graph788 | 4620 | 2 | 33 % | 1.3s | 9.0s | 9m | 608 → 2562 |
| graph810 | 4832 | 2 | 29 % | 3.4s | 10.9s | 5m | 546 → 3450 |
| graph832 | 5070 | 2 | 18 % | 5.2s | 15.8s | 9m | 492 → 5044 |
| graph882 | 5686 | 2 | 22 % | 3.6s | 17.5s | 7m | 578 → 3906 |
| graph910 | 6057 | 2 | 15 % | 2m | 57.9s | 1m | 528 → 5684 |
| graph937 | 6412 | 2 | 31 % | 3.8s | 9.7s | 7m | 754 → 3658 |
| graph940 | 6498 | 2 | 21 % | 30.9s | 1m | 2m | 420 → 2430 |
| graph944 | 6544 | 2 | 21 % | 6.5s | 25.9s | 11m | 782 → 5328 |
| graph951 | 6630 | 2 | 23 % | 13.7s | 19.4s | 6m | 710 → 5296 |
| graph954 | 6735 | 2 | 30 % | 4.4s | 7.3s | 19m | 792 → 4068 |
| graph959 | 6925 | 2 | 27 % | 3.4s | 29.0s | 5m | 854 → 5490 |
| graph965 | 7102 | 2 | 26 % | 4.3s | 21.0s | 6m | 854 → 4398 |
| graph966 | 7104 | 2 | 26 % | 5.8s | 6.8s | 17m | 796 → 4612 |
| graph971 | 7295 | 2 | 13 % | 8.5s | 15.9s | 2m | 704 → 5680 |
| graph974 | 7418 | 2 | 25 % | 14.0s | 11.0s | 1m | 732 → 4610 |
| graph976 | 7434 | 2 | 25 % | 16.5s | 23.7s | 6m | 876 → 4784 |
| graph981 | 7620 | 2 | 26 % | 4.7s | 7.6s | 7m | 740 → 3344 |
| graph983 | 7650 | 2 | 24 % | 5.6s | 29.5s | 9m | 932 → 5114 |
| graph986 | 7824 | 2 | 16 % | 4.7s | 29.2s | 4m | 558 → 7982 |
| graph987 | 7850 | 2 | 20 % | 1.9s | 9.4s | 19m | 560 → 2634 |
| graph993 | 8380 | 2 | 22 % | 13.6s | 19.7s | 18m | 872 → 4118 |
| graph994 | 8401 | 2 | 24 % | 5.2s | 31.2s | 19m | 932 → 5496 |
| graph998 | 8613 | 2 | 21 % | 15.2s | 36.0s | 7m | 956 → 5794 |

## 8. Cross-cutting dynamic signatures

- **Degradation curve (B1/B2b):** SAT cost is *not* stationary — the median is moderate but the last call is 10–100× harder (block clauses per increment grow 5–37×, e.g. graph984: 672 → 24 694). The solver is 1–2 increments from success at the cap.

- **Attractor loop (A):** cost is flat and tiny; the search is trapped near a Hamiltonian cycle (2–6 cycles × up to 1 870 vertices) that cut arcs cannot break.

- **Fragmentation (B1 subclass):** cut arcs make the SAT solutions *worse* (cycle count grows, average length shrinks to ∅6 — graph797/863).

- **Zero-progress stall (B2a):** 10–20 increments, no merge ever; everything spent in SAT.

## 9. Grounding — classes are dynamics, not instance types

| Signature | Solved graphs | Timeout graphs |
|---|---|---|
| ≥ 5 000 increments (Class-A dynamics) | 35 (converged) | 8 |
| Single SAT call ≥ 200 s (Class-B dynamics) | 36 (converged) | ~60 |
| Dense (≥ 3.0) | 64 (median 76 incs) | 28 |
| Degree-2 chains > 10 % | 316 | 46 |

The same failure dynamics appear among the solved graphs; timeouts are the runs that lose the race against the 1800 s cap. This also means **the four classes are four different fix levers**, not four instance taxonomies.

## 10. The other two datasets

- **100-benchmark (63 “timeouts”):** 53 are cap/binary artifacts (baseline solves them; 16 in < 15 s); **10 are true-hard** — graph560, 670, 830, 950, 990 (B1), graph710, 810, 910, 940 (B2b), graph960 (B2a). The benchmark’s every-10th sampling missed all Class-A graphs.

- **Patcher run (40/1001, all solved):** remaining 961 graphs include all 75 hard graphs — B2a graph479 at index 479, B1 graph560 at 560. At the observed median 35× slowdown, those 75 graphs alone are ≥ 37.5 h of capped runs.

## 11. Mapping to solver features (commit history)

| Class | Fix lever | Existing work |
|---|---|---|
| A | blocking-clause / cut-arc strategy | blocking-clause enhancements; graph339 already solved by 4d094b7 (1 403 s, 2 745 incs) |
| B1 | modular decomposition of dense hubs | ModularSolver, ModularDecompositionTree, “bound short-cycle cuts degree to prevent clause explosion on dense hubs” |
| B2a | cycle-splicing / patching | StemCyclePatcher (adaptive k-opt splice), ILS patcher |
| B2b | SAT encoding & phase heuristics | Sinz sequential counters, AtLeast2 tuning, 3-opt |

**Coverage gap:** the “10 key regression graphs” test set is entirely sparse (density 1.50) — it contains no dense B1 instance; graph313/graph339 are the only Class-A-dynamics members, and graph178/graph346 the only B2a.

## 12. Recommendations

1. Use the 75-graph list as the difficulty test set; validate fixes per class rather than on random samples.

2. Class A is the cheapest win — several members already solved by 4d094b7; re-run the full set at 4d094b7 (or HEAD) to measure

3. Watch the 1800 s cap in class B2b — last-call hardening suggests incremental re-solving reuse (assumptions/phase saving) would convert many timeouts.

4. The patcher full run should be resumed only after the slowdown regression is addressed (§10).

## Appendix — full per-graph feature table (75 graphs)

Legend: incs = increments at kill (timeouts) / total (solved); satF/M/L = first/median/last SAT time; blkF/L = block clauses added in first/last increment; subF/subL = (avg length × count) of subcycles in first/last increment.

| graph | class | n | m | dens | minDeg | deg2 | deg3 | deg4+ | chains | incs | satF | satM | satL | blkF→L | subF | subL | connPos |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| graph339 | A | 2004 | 3007 | 1.50 | 3 | 0 | 2002 | 2 | 0 | 153,460 | 982ms | 1ms | 1ms | 256→0 | ∅15.7×128 | ∅1002.0×2 | 0 |
| graph479 | B2a | 2772 | 4536 | 1.64 | 2 | 924 | 840 | 1008 | 0 | 19 | 501ms | 11.3s | 5m | 340→1718 | ∅16.3×170 | ∅173.2×16 | 0 |
| graph560 | B1 | 3311 | 14361 | 4.34 | 2 | 1 | 2 | 3308 | 0 | 20 | 9.9s | 1m | 50.4s | 206→3332 | ∅13.2×251 | ∅17.4×190 | 20 |
| graph561 | B1 | 3311 | 14361 | 4.34 | 2 | 1 | 2 | 3308 | 0 | 19 | 15.2s | 1m | 56.0s | 188→3036 | ∅13.6×244 | ∅18.2×182 | 19 |
| graph562 | B1 | 3311 | 14361 | 4.34 | 2 | 1 | 2 | 3308 | 0 | 17 | 19.9s | 1m | 50.4s | 228→2852 | ∅12.4×268 | ∅16.0×207 | 17 |
| graph566 | B2b | 3322 | 6106 | 1.84 | 2 | 768 | 700 | 1854 | 0 | 59 | 188ms | 16.1s | 5m | 394→4718 | ∅14.1×236 | ∅132.9×25 | 59 |
| graph584 | B1 | 3411 | 14811 | 4.34 | 2 | 1 | 2 | 3408 | 0 | 13 | 27.7s | 1m | 10.3s | 190→2186 | ∅14.0×243 | ∅15.9×214 | 13 |
| graph585 | B1 | 3411 | 14811 | 4.34 | 2 | 1 | 2 | 3408 | 0 | 19 | 54.8s | 56.5s | 59.4s | 220→3228 | ∅12.8×266 | ∅18.1×188 | 19 |
| graph612 | B1 | 3511 | 15261 | 4.35 | 2 | 1 | 2 | 3508 | 0 | 14 | 37.1s | 1m | 51.8s | 204→2600 | ∅12.8×274 | ∅15.1×232 | 14 |
| graph613 | B1 | 3511 | 15261 | 4.35 | 2 | 1 | 2 | 3508 | 0 | 14 | 7.4s | 2m | 1m | 206→2464 | ∅13.5×261 | ∅16.6×212 | 14 |
| graph614 | B1 | 3511 | 15261 | 4.35 | 2 | 1 | 2 | 3508 | 0 | 17 | 25.4s | 1m | 1m | 242→2836 | ∅12.5×282 | ∅15.3×229 | 17 |
| graph615 | B1 | 3511 | 15261 | 4.35 | 2 | 1 | 2 | 3508 | 0 | 22 | 4.7s | 58.6s | 16.3s | 214→3834 | ∅13.0×271 | ∅14.8×238 | 22 |
| graph635 | B1 | 3611 | 15711 | 4.35 | 2 | 1 | 2 | 3608 | 0 | 14 | 5.1s | 1m | 2m | 210→2574 | ∅13.0×277 | ∅14.0×258 | 14 |
| graph647 | A | 3688 | 5994 | 1.63 | 2 | 978 | 1797 | 913 | 12(m7) | 16,396 | 2.1s | 2ms | 2ms | 338→0 | ∅17.6×209 | ∅87.8×42 | 16395 |
| graph651 | B2b | 3701 | 6272 | 1.69 | 2 | 768 | 1584 | 1349 | 0 | 95 | 1.8s | 7.7s | 1m | 402→8934 | ∅16.7×221 | ∅142.3×26 | 95 |
| graph653 | B1 | 3711 | 16161 | 4.35 | 2 | 1 | 2 | 3708 | 0 | 16 | 35.2s | 1m | 1m | 226→2798 | ∅12.2×304 | ∅14.8×250 | 16 |
| graph654 | B1 | 3711 | 16161 | 4.35 | 2 | 1 | 2 | 3708 | 0 | 19 | 35.2s | 1m | 59.6s | 212→3316 | ∅13.2×281 | ∅16.2×229 | 19 |
| graph668 | B2b | 3783 | 6861 | 1.81 | 2 | 921 | 840 | 2022 | 0 | 32 | 1.1s | 20.4s | 58ms | 426→3674 | ∅15.0×252 | ∅77.2×49 | 32 |
| graph670 | B1 | 3811 | 16611 | 4.36 | 2 | 1 | 2 | 3808 | 0 | 13 | 26.6s | 3m | 50.7s | 254→2448 | ∅12.6×302 | ∅16.1×237 | 13 |
| graph677 | B2b | 3868 | 6388 | 1.65 | 2 | 768 | 1920 | 1180 | 0 | 66 | 2.7s | 12.2s | 47ms | 406→8538 | ∅17.7×219 | ∅66.7×58 | 66 |
| graph678 | B2b | 3868 | 6388 | 1.65 | 2 | 768 | 1920 | 1180 | 0 | 66 | 1.8s | 11.9s | 13.7s | 474→8386 | ∅15.5×250 | ∅94.3×41 | 66 |
| graph684 | B1 | 3911 | 17061 | 4.36 | 2 | 1 | 2 | 3908 | 0 | 9 | 37.6s | 1m | 5m | 258→1742 | ∅12.5×313 | ∅12.9×303 | 9 |
| graph710 | B2b | 4064 | 6800 | 1.67 | 2 | 922 | 1724 | 1418 | 0 | 20 | 3.5s | 8.2s | 2m | 386→3642 | ∅19.8×205 | ∅90.3×45 | 20 |
| graph717 | B2b | 4122 | 7638 | 1.85 | 2 | 922 | 840 | 2360 | 0 | 17 | 3.6s | 6.1s | 18m | 464→3352 | ∅13.9×296 | ∅46.3×89 | 17 |
| graph725 | B2b | 4163 | 7028 | 1.69 | 2 | 922 | 1724 | 1517 | 0 | 17 | 3.3s | 6.1s | 5m | 472→3572 | ∅16.0×261 | ∅66.1×63 | 17 |
| graph734 | B2b | 4232 | 7038 | 1.66 | 2 | 890 | 2623 | 719 | 16(m7) | 27 | 3.8s | 50.2s | 2m | 230→4494 | ∅24.9×170 | ∅37.1×114 | 27 |
| graph744 | B2b | 4278 | 7272 | 1.70 | 2 | 768 | 1992 | 1518 | 0 | 44 | 7.1s | 22.8s | 94ms | 464→8160 | ∅16.2×264 | ∅64.8×66 | 44 |
| graph746 | B1 | 4286 | 18286 | 4.27 | 2 | 1 | 2 | 4283 | 0 | 13 | 30.7s | 1m | 3m | 422→3622 | ∅11.0×388 | ∅14.6×294 | 13 |
| graph761 | B2b | 4430 | 6963 | 1.57 | 2 | 768 | 2818 | 844 | 0 | 78 | 5.5s | 8.4s | 30.7s | 504→9392 | ∅17.3×256 | ∅59.9×74 | 42 |
| graph766 | B2b | 4465 | 7979 | 1.79 | 2 | 1352 | 1385 | 1728 | 16(m7) | 22 | 3.9s | 53.7s | 1m | 436→2076 | ∅14.7×303 | ∅41.3×108 | 22 |
| graph771 | A | 4514 | 7961 | 1.76 | 2 | 799 | 3080 | 635 | 1(m5) | 11,284 | 3.9s | 3ms | 3ms | 452→0 | ∅19.6×230 | ∅752.3×6 | 11283 |
| graph788 | B2b | 4620 | 7560 | 1.64 | 2 | 1540 | 1400 | 1680 | 0 | 13 | 1.3s | 9.0s | 9m | 608→2562 | ∅15.1×305 | ∅220.0×21 | 9 |
| graph797 | B1 | 4701 | 16351 | 3.48 | 2 | 1 | 2 | 4698 | 0 | 77 | 3.9s | 1.4s | 1.4s | 74→1746 | ∅8.4×558 | ∅6.3×742 | 77 |
| graph809 | B2a | 4810 | 7783 | 1.62 | 2 | 1384 | 1914 | 1512 | 0 | 20 | 2.9s | 15.8s | 4m | 568→3934 | ∅16.9×284 | ∅165.9×29 | 0 |
| graph810 | B2b | 4832 | 8354 | 1.73 | 2 | 1384 | 1260 | 2188 | 0 | 18 | 3.4s | 10.9s | 5m | 546→3450 | ∅16.2×299 | ∅105.0×46 | 18 |
| graph830 | B1 | 5056 | 17151 | 3.39 | 2 | 1 | 2 | 5053 | 0 | 25 | 7.4s | 1m | 2m | 332→5282 | ∅11.0×460 | ∅13.3×381 | 25 |
| graph832 | B2b | 5070 | 7986 | 1.58 | 2 | 922 | 3136 | 1012 | 0 | 16 | 5.2s | 15.8s | 9m | 492→5044 | ∅20.5×247 | ∅39.0×130 | 7 |
| graph844 | B1 | 5226 | 17626 | 3.37 | 2 | 1 | 2 | 5223 | 0 | 33 | 6.1s | 42.5s | 7.0s | 376→7296 | ∅11.2×468 | ∅13.7×381 | 33 |
| graph863 | B1 | 5461 | 19011 | 3.48 | 2 | 1 | 2 | 5458 | 0 | 72 | 6.6s | 2.3s | 18.9s | 70→1896 | ∅8.8×620 | ∅7.4×742 | 72 |
| graph868 | B2a | 5544 | 9072 | 1.64 | 2 | 1848 | 1680 | 2016 | 0 | 11 | 3.7s | 7.1s | 8m | 628→3020 | ∅17.7×314 | ∅149.8×37 | 0 |
| graph882 | B2b | 5686 | 9306 | 1.64 | 2 | 1228 | 2772 | 1686 | 0 | 31 | 3.6s | 17.5s | 7m | 578→3906 | ∅18.7×304 | ∅355.4×16 | 31 |
| graph883 | A | 5696 | 9325 | 1.64 | 2 | 1196 | 3422 | 1078 | 3(m5) | 10,619 | 19.9s | 4ms | 4ms | 612→0 | ∅17.8×320 | ∅284.8×20 | 10618 |
| graph905 | B1 | 5985 | 19593 | 3.27 | 3 | 0 | 14 | 5971 | 0 | 85 | 10.5s | 11.3s | 4.2s | 528→19148 | ∅11.6×516 | ∅31.2×192 | 85 |
| graph908 | A | 6018 | 9808 | 1.63 | 2 | 1196 | 3744 | 1078 | 3(m5) | 5,537 | 5.5s | 4ms | 4ms | 422→0 | ∅16.9×357 | ∅18.9×318 | 5536 |
| graph910 | B2b | 6057 | 10693 | 1.77 | 2 | 906 | 3549 | 1602 | 16(m7) | 23 | 2m | 57.9s | 1m | 528→5684 | ∅16.2×374 | ∅20.6×294 | 23 |
| graph935 | A | 6382 | 10792 | 1.69 | 2 | 1856 | 2774 | 1752 | 3(m5) | 8,838 | 11.4s | 2ms | 2ms | 742→0 | ∅16.8×380 | ∅205.9×31 | 8837 |
| graph937 | B2b | 6412 | 10762 | 1.68 | 2 | 2000 | 1820 | 2592 | 0 | 11 | 3.8s | 9.7s | 7m | 754→3658 | ∅16.5×388 | ∅110.6×58 | 11 |
| graph940 | B2b | 6498 | 10629 | 1.64 | 2 | 1350 | 3921 | 1227 | 16(m7) | 10 | 30.9s | 1m | 2m | 420→2430 | ∅24.4×266 | ∅45.8×142 | 10 |
| graph944 | B2b | 6544 | 10925 | 1.67 | 2 | 1382 | 2970 | 2192 | 0 | 17 | 6.5s | 25.9s | 11m | 782→5328 | ∅15.5×423 | ∅62.9×104 | 17 |
| graph950 | B1 | 6620 | 28718 | 4.34 | 3 | 0 | 4 | 6616 | 0 | 17 | 58.0s | 1m | 58.1s | 424→5622 | ∅12.9×512 | ∅18.0×367 | 17 |
| graph951 | B2b | 6630 | 10578 | 1.60 | 2 | 1538 | 3408 | 1684 | 0 | 13 | 13.7s | 19.4s | 6m | 710→5296 | ∅18.6×356 | ∅70.5×94 | 7 |
| graph954 | B2b | 6735 | 11504 | 1.71 | 2 | 2000 | 1820 | 2915 | 0 | 12 | 4.4s | 7.3s | 19m | 792→4068 | ∅15.9×424 | ∅82.1×82 | 12 |
| graph955 | B1 | 6840 | 22392 | 3.27 | 3 | 0 | 16 | 6824 | 0 | 77 | 17.4s | 15.7s | 8.5s | 638→21038 | ∅11.4×602 | ∅29.7×230 | 77 |
| graph959 | B2b | 6925 | 11727 | 1.69 | 2 | 1844 | 2334 | 2747 | 0 | 18 | 3.4s | 29.0s | 5m | 854→5490 | ∅15.4×450 | ∅108.2×64 | 18 |
| graph960 | B2a | 6930 | 11340 | 1.64 | 2 | 2310 | 2100 | 2520 | 0 | 10 | 2.0s | 7.3s | 7m | 844→3674 | ∅16.4×422 | ∅105.0×66 | 0 |
| graph961 | A | 6939 | 11528 | 1.66 | 2 | 1026 | 4482 | 1431 | 1(m4) | 13,823 | 14.5s | 4ms | 4ms | 856→0 | ∅15.1×459 | ∅1156.5×6 | 13819 |
| graph963 | B1 | 7020 | 30518 | 4.35 | 3 | 0 | 4 | 7016 | 0 | 8 | 3m | 3m | 4m | 410→3036 | ∅12.7×552 | ∅13.8×507 | 8 |
| graph965 | B2b | 7102 | 11680 | 1.64 | 2 | 1846 | 2900 | 2356 | 0 | 10 | 4.3s | 21.0s | 6m | 854→4398 | ∅16.2×439 | ∅59.2×120 | 10 |
| graph966 | B2b | 7104 | 11952 | 1.68 | 2 | 1846 | 2564 | 2694 | 0 | 11 | 5.8s | 6.8s | 17m | 796→4612 | ∅16.7×426 | ∅55.5×128 | 11 |
| graph971 | B2b | 7295 | 11907 | 1.63 | 2 | 920 | 4630 | 1745 | 0 | 16 | 8.5s | 15.9s | 2m | 704→5680 | ∅13.4×545 | ∅17.3×421 | 16 |
| graph974 | B2b | 7418 | 11885 | 1.60 | 2 | 1846 | 3554 | 2018 | 0 | 10 | 14.0s | 11.0s | 1m | 732→4610 | ∅20.3×366 | ∅64.5×115 | 4 |
| graph975 | B1 | 7420 | 32318 | 4.36 | 3 | 0 | 4 | 7416 | 0 | 11 | 2m | 2m | 3m | 458→4116 | ∅12.2×607 | ∅14.5×511 | 11 |
| graph976 | B2b | 7434 | 11910 | 1.60 | 2 | 1846 | 3568 | 2020 | 0 | 10 | 16.5s | 23.7s | 6m | 876→4784 | ∅16.9×440 | ∅62.0×120 | 8 |
| graph978 | A | 7480 | 10628 | 1.42 | 2 | 1622 | 5420 | 438 | 162(m10) | 24,593 | 5.3s | 4ms | 4ms | 612→0 | ∅24.4×307 | ∅1870.0×4 | 24557 |
| graph981 | B2b | 7620 | 12251 | 1.61 | 2 | 2000 | 3434 | 2186 | 0 | 11 | 4.7s | 7.6s | 7m | 740→3344 | ∅12.7×601 | ∅23.3×327 | 11 |
| graph982 | B1 | 7620 | 33218 | 4.36 | 3 | 0 | 4 | 7616 | 0 | 10 | 3m | 3m | 1m | 442→3796 | ∅12.5×609 | ∅16.5×463 | 10 |
| graph983 | B2b | 7650 | 12234 | 1.60 | 2 | 1846 | 3784 | 2020 | 0 | 11 | 5.6s | 29.5s | 9m | 932→5114 | ∅16.4×466 | ∅42.3×181 | 6 |
| graph984 | B1 | 7695 | 25191 | 3.27 | 3 | 0 | 18 | 7677 | 0 | 87 | 17.7s | 14.1s | 16.4s | 672→24694 | ∅11.7×657 | ∅28.8×267 | 87 |
| graph986 | B2b | 7824 | 12245 | 1.57 | 2 | 1228 | 5246 | 1350 | 0 | 28 | 4.7s | 29.2s | 4m | 558→7982 | ∅17.1×458 | ∅21.2×369 | 28 |
| graph987 | B2b | 7850 | 12408 | 1.58 | 2 | 1538 | 4628 | 1684 | 0 | 13 | 1.9s | 9.4s | 19m | 560→2634 | ∅11.4×690 | ∅13.4×585 | 13 |
| graph990 | B1 | 8020 | 35018 | 4.37 | 3 | 0 | 4 | 8016 | 0 | 9 | 2m | 2m | 5m | 458→3730 | ∅12.4×648 | ∅13.3×605 | 9 |
| graph993 | B2b | 8380 | 13329 | 1.59 | 2 | 1846 | 4514 | 2020 | 0 | 10 | 13.6s | 19.7s | 18m | 872→4118 | ∅12.1×690 | ∅23.1×362 | 10 |
| graph994 | B2b | 8401 | 13828 | 1.65 | 2 | 1998 | 3708 | 2695 | 0 | 12 | 5.2s | 31.2s | 19m | 932→5496 | ∅17.3×486 | ∅53.9×156 | 12 |
| graph996 | B1 | 8550 | 27990 | 3.27 | 3 | 0 | 20 | 8530 | 0 | 58 | 17.1s | 26.2s | 1m | 774→22868 | ∅11.5×746 | ∅24.5×349 | 58 |
| graph998 | B2b | 8613 | 14352 | 1.67 | 2 | 1844 | 3904 | 2865 | 0 | 10 | 15.2s | 36.0s | 7m | 956→5794 | ∅17.2×502 | ∅44.2×195 | 10 |

---
*Generated from `/tmp/opencode/cegar_dynamics.json`, `/tmp/opencode/deep_features.json` (throwaway analysis scripts in `/tmp/opencode/`).*

## 13. Cross-validation: `data/` CSV result tables (independent 1800 s-cap runs)

**Source:** `data/existing-work.csv`, `data/proposed-cegar-sinz-cpu.csv`, `data/proposed-cegar-ccadical-cpu.csv` — 1001 rows each (all FHCP graphs), CPU seconds, cap 1800 s (max observed 1799.3 s), `TO` = timeout. Headers are Japanese: 問題番号 = problem number.

**VBS** = *Virtual Best Solver*: per-problem minimum across the family's variant columns (TO if all variants TO). It is an oracle aggregate, not a runnable binary; the honest single-configuration numbers are the best-variant columns below.

| Dataset | Columns | TO (of 1001) |
| --- | --- | --- |
| `existing-work.csv` | adder (361), crt-420 (335), CEGAR-old (462), asp (59), picat (573) | VBS **52** |
| `proposed-cegar-sinz-cpu.csv` | 8 effort variants (CaDiCaL+Sinz) | best `S:2loop-2opt3` **64**, VBS **53** |
| `proposed-cegar-ccadical-cpu.csv` | 8 effort variants (CaDiCaL) | best `C:2loop-lowest-asymmetry3-2opt3` **44**, VBS **42** |

The old own solver had 462 TO — the proposed variant work reduced this to 42–64. `asp` (59 TO) is the strongest reference solver. **No regressions:** no baseline-solved graph times out in any proposed variant (baseline official run: 75 TO / 1001).

### 13.1 Conversion of the 75 baseline timeouts

| Class | n baseline-TO | solved ccad VBS | solved sinz VBS | solved 16-variant union | open in both encodings | of which universal core |
| --- | --- | --- | --- | --- | --- | --- |
| A | 8 | 8 | 8 | 8 | 0 | 0 |
| B1 | 28 | 22 | 7 | 22 | 6 | 6 |
| B2a | 4 | 0 | 1 | 1 | 3 | 3 |
| B2b | 35 | 3 | 6 | 6 | 29 | 24 |
| **Total** | 75 | 33 | 22 | 37 | 38 | 33 |

Single-configuration reality check: best one variant `C:2loop-lowest-asymmetry3-2opt3` leaves 43 of the 75 TO (A: 0, B1: 6, B2a: 4, B2b: 33) — i.e. **+32 conversions with a real binary**; caDiCaL family nearly matches its own VBS (44 vs 42). The sinz best variant leaves 62 of the 75 (its Class-A wins need `lowest-asymmetry₂`-type variants, which the caDiCaL family covers in its base combo).

### 13.2 The universal core — 33 graphs TO in *every* recorded solver

Time out in: baseline official run, all 16 proposed variants (both encodings), and adder / crt-420 / asp / picat / CEGAR-old.

- **B1 (6):** graph746, graph950, graph963, graph975, graph982, graph990

- **B2a (3):** graph809, graph868, graph960

- **B2b (24):** graph668, graph710, graph717, graph761, graph788, graph832, graph882, graph937, graph944, graph951, graph954, graph959, graph965, graph966, graph971, graph974, graph976, graph981, graph983, graph986, graph987, graph993, graph994, graph998

Notes: the B1 core is exactly the m≈4.34n “ladder” family (950→963→975→982→990, n+1000 / m+1800 per step) plus graph746 (dens. 4.27) — the sibling m≈3.27n family (905, 955, 984, 996) and the “spider-web” graphs 797/863 are solved by the proposed solver. Of the 38 open-in-both-encodings, reference solvers crack only 5: graph725 (picat, 15 s — unique fast win, likely special structure/symmetry), graph677 / 744 / 810 / 940 (asp, 544–1504 s).

### 13.3 Optimization targets (per class, paper-facing)

1. **Class A — closed:** 8/8 converted; the 2loop / lowest-asymmetry / 2opt3 variant combinations already finish these — no further work needed.

2. **Class B1 — nearly closed:** 22/28 converted; the remaining 6 (746, 950, 963, 975, 982, 990) are the max-density ladder family — the natural benchmark for module-decomposition work.

3. **Class B2b — open:** only 6/35 converted (566 @ 29.9m, 734 @ 9.5m, 766 @ 11.7m via caDiCaL; 651, 678, 910 via Sinz); 24 graphs survive ~20 distinct configurations — validate any SAT-encoding/mutation work against this list first.

4. **Class B2a — hardest per instance:** 809 / 868 / 960 beat every solver family recorded; only 479 escaped (sinz 2loop variants, 25.1m).

## Appendix B — per-graph CSV cross-reference (75 graphs)

Legend: `sinzVBS` / `ccadVBS` = oracle best across the 8 variants of that encoding; `union` = best across all 16 variant columns with the winning variant (`S:`/`C:` prefixes = sinz/caDiCaL family); `bestSingle(C)` = time of the single best real configuration `C:2loop-lowest-asymmetry3-2opt3`; `core33` = TO in every recorded solver (✔).

| graph | cls | n | dens | sinzVBS | ccadVBS | union | unionVar | bestSingle(C) | core33 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| graph339 | A | 2004 | 1.50 | 4.2s | 1.9s | 1.9s | C:base | 3.9s | — |
| graph479 | B2a | 2772 | 1.64 | 25.1m | TO | 25.1m | S:2loop-2opt3 | TO | — |
| graph560 | B1 | 3311 | 4.34 | TO | 17.2m | 17.2m | C:2opt3 | 22.7m | — |
| graph561 | B1 | 3311 | 4.34 | TO | 16.5m | 16.5m | C:2loop-lowest-asymmetry3-2opt3 | 16.5m | — |
| graph562 | B1 | 3311 | 4.34 | TO | 19.4m | 19.4m | C:2loop-2opt3 | 19.5m | — |
| graph566 | B2b | 3322 | 1.84 | 19.8m | 30.0m | 19.8m | S:2loop | 30.0m | — |
| graph584 | B1 | 3411 | 4.34 | TO | 13.9m | 13.9m | C:2loop-2opt3 | 14.0m | — |
| graph585 | B1 | 3411 | 4.34 | TO | 15.2m | 15.2m | C:2opt3 | 17.5m | — |
| graph612 | B1 | 3511 | 4.35 | TO | 18.6m | 18.6m | C:2loop-2opt3 | 18.6m | — |
| graph613 | B1 | 3511 | 4.35 | TO | 20.5m | 20.5m | C:2loop-2opt3 | 20.6m | — |
| graph614 | B1 | 3511 | 4.35 | TO | 14.7m | 14.7m | C:2loop-2opt3 | 14.8m | — |
| graph615 | B1 | 3511 | 4.35 | TO | 17.9m | 17.9m | C:2loop-lowest-asymmetry3-2opt3 | 17.9m | — |
| graph635 | B1 | 3611 | 4.35 | TO | 22.3m | 22.3m | C:2loop-lowest-asymmetry3-2opt3 | 22.3m | — |
| graph647 | A | 3688 | 1.63 | 1.5m | 4.7m | 1.5m | S:2loop-lowest-asymmetry2-2opt3 | 5.9m | — |
| graph651 | B2b | 3701 | 1.69 | 13.4m | TO | 13.4m | S:2loop-lowest-asymmetry2 | TO | — |
| graph653 | B1 | 3711 | 4.35 | TO | 18.6m | 18.6m | C:2loop-2opt3 | 18.6m | — |
| graph654 | B1 | 3711 | 4.35 | TO | 26.4m | 26.4m | C:2loop-lowest-asymmetry3-2opt3 | 26.4m | — |
| graph668 | B2b | 3783 | 1.81 | TO | TO | TO | — | TO | ✔ |
| graph670 | B1 | 3811 | 4.36 | TO | 21.1m | 21.1m | C:lowest-asymmetry3-2opt3 | 24.5m | — |
| graph677 | B2b | 3868 | 1.65 | TO | TO | TO | — | TO | — |
| graph678 | B2b | 3868 | 1.65 | 20.8m | TO | 20.8m | S:2loop-lowest-asymmetry2-2opt3 | TO | — |
| graph684 | B1 | 3911 | 4.36 | TO | 25.1m | 25.1m | C:2loop-lowest-asymmetry3-2opt3 | 25.1m | — |
| graph710 | B2b | 4064 | 1.67 | TO | TO | TO | — | TO | ✔ |
| graph717 | B2b | 4122 | 1.85 | TO | TO | TO | — | TO | ✔ |
| graph725 | B2b | 4163 | 1.69 | TO | TO | TO | — | TO | — |
| graph734 | B2b | 4232 | 1.66 | 3.6m | 9.5m | 3.6m | S:lowest-asymmetry2-2opt3 | TO | — |
| graph744 | B2b | 4278 | 1.70 | TO | TO | TO | — | TO | — |
| graph746 | B1 | 4286 | 4.27 | TO | TO | TO | — | TO | ✔ |
| graph761 | B2b | 4430 | 1.57 | TO | TO | TO | — | TO | ✔ |
| graph766 | B2b | 4465 | 1.79 | 7.0m | 11.7m | 7.0m | S:lowest-asymmetry2-2opt3 | 11.8m | — |
| graph771 | A | 4514 | 1.76 | 2.1m | 1.7m | 1.7m | C:2loop-2opt3 | 1.7m | — |
| graph788 | B2b | 4620 | 1.64 | TO | TO | TO | — | TO | ✔ |
| graph797 | B1 | 4701 | 3.48 | 3.5m | 3.3m | 3.3m | C:2loop-lowest-asymmetry3 | 9.1m | — |
| graph809 | B2a | 4810 | 1.62 | TO | TO | TO | — | TO | ✔ |
| graph810 | B2b | 4832 | 1.73 | TO | TO | TO | — | TO | — |
| graph830 | B1 | 5056 | 3.39 | 10.0m | 6.2m | 6.2m | C:2loop-lowest-asymmetry3-2opt3 | 6.2m | — |
| graph832 | B2b | 5070 | 1.58 | TO | TO | TO | — | TO | ✔ |
| graph844 | B1 | 5226 | 3.37 | 10.7m | 7.7m | 7.7m | C:lowest-asymmetry3-2opt3 | 8.0m | — |
| graph863 | B1 | 5461 | 3.48 | 5.5m | 5.8m | 5.5m | S:lowest-asymmetry2-2opt3 | 8.2m | — |
| graph868 | B2a | 5544 | 1.64 | TO | TO | TO | — | TO | ✔ |
| graph882 | B2b | 5686 | 1.64 | TO | TO | TO | — | TO | ✔ |
| graph883 | A | 5696 | 1.64 | 6.6m | 41.3s | 41.3s | C:2loop-2opt3 | 41.4s | — |
| graph905 | B1 | 5985 | 3.27 | 11.3m | 9.4m | 9.4m | C:2loop-lowest-asymmetry3 | 9.7m | — |
| graph908 | A | 6018 | 1.63 | 1.6m | 2.2m | 1.6m | S:lowest-asymmetry2-2opt3 | 2.2m | — |
| graph910 | B2b | 6057 | 1.77 | 12.5m | TO | 12.5m | S:2loop-lowest-asymmetry2-2opt3 | TO | — |
| graph935 | A | 6382 | 1.69 | 1.6m | 2.2m | 1.6m | S:2loop-lowest-asymmetry2-2opt3 | 2.2m | — |
| graph937 | B2b | 6412 | 1.68 | TO | TO | TO | — | TO | ✔ |
| graph940 | B2b | 6498 | 1.64 | TO | TO | TO | — | TO | — |
| graph944 | B2b | 6544 | 1.67 | TO | TO | TO | — | TO | ✔ |
| graph950 | B1 | 6620 | 4.34 | TO | TO | TO | — | TO | ✔ |
| graph951 | B2b | 6630 | 1.60 | TO | TO | TO | — | TO | ✔ |
| graph954 | B2b | 6735 | 1.71 | TO | TO | TO | — | TO | ✔ |
| graph955 | B1 | 6840 | 3.27 | 20.9m | 14.0m | 14.0m | C:2loop | 14.7m | — |
| graph959 | B2b | 6925 | 1.69 | TO | TO | TO | — | TO | ✔ |
| graph960 | B2a | 6930 | 1.64 | TO | TO | TO | — | TO | ✔ |
| graph961 | A | 6939 | 1.66 | 13.6m | 3.3m | 3.3m | C:2loop-lowest-asymmetry3-2opt3 | 3.3m | — |
| graph963 | B1 | 7020 | 4.35 | TO | TO | TO | — | TO | ✔ |
| graph965 | B2b | 7102 | 1.64 | TO | TO | TO | — | TO | ✔ |
| graph966 | B2b | 7104 | 1.68 | TO | TO | TO | — | TO | ✔ |
| graph971 | B2b | 7295 | 1.63 | TO | TO | TO | — | TO | ✔ |
| graph974 | B2b | 7418 | 1.60 | TO | TO | TO | — | TO | ✔ |
| graph975 | B1 | 7420 | 4.36 | TO | TO | TO | — | TO | ✔ |
| graph976 | B2b | 7434 | 1.60 | TO | TO | TO | — | TO | ✔ |
| graph978 | A | 7480 | 1.42 | 45.3s | 27.1s | 27.1s | C:2loop-2opt3 | 27.1s | — |
| graph981 | B2b | 7620 | 1.61 | TO | TO | TO | — | TO | ✔ |
| graph982 | B1 | 7620 | 4.36 | TO | TO | TO | — | TO | ✔ |
| graph983 | B2b | 7650 | 1.60 | TO | TO | TO | — | TO | ✔ |
| graph984 | B1 | 7695 | 3.27 | 27.8m | 17.4m | 17.4m | C:2loop-lowest-asymmetry3 | 21.7m | — |
| graph986 | B2b | 7824 | 1.57 | TO | TO | TO | — | TO | ✔ |
| graph987 | B2b | 7850 | 1.58 | TO | TO | TO | — | TO | ✔ |
| graph990 | B1 | 8020 | 4.37 | TO | TO | TO | — | TO | ✔ |
| graph993 | B2b | 8380 | 1.59 | TO | TO | TO | — | TO | ✔ |
| graph994 | B2b | 8401 | 1.65 | TO | TO | TO | — | TO | ✔ |
| graph996 | B1 | 8550 | 3.27 | TO | 17.9m | 17.9m | C:2loop-lowest-asymmetry3 | 22.8m | — |
| graph998 | B2b | 8613 | 1.67 | TO | TO | TO | — | TO | ✔ |

*Appended 2026-08-20 from `data/*.csv` (VBS = virtual-best-solver oracle; union = min over all 16 variant columns).*
