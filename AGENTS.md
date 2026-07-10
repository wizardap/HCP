<!-- CODEGRAPH_START -->
## CodeGraph

In repositories indexed by CodeGraph (a `.codegraph/` directory exists at the repo root), reach for it BEFORE grep/find or reading files when you need to understand or locate code:

- **MCP tool** (when available): `codegraph_explore` answers most code questions in one call — the relevant symbols' verbatim source plus the call paths between them, including dynamic-dispatch hops grep can't follow. Name a file or symbol in the query to read its current line-numbered source. If it's listed but deferred, load it by name via tool search.
- **Shell** (always works): `codegraph explore "<symbol names or question>"` prints the same output.

If there is no `.codegraph/` directory, skip CodeGraph entirely — indexing is the user's decision.
<!-- CODEGRAPH_END -->

## Project Overview

C++ Hamiltonian Cycle Problem (HCP) solver with SAT encoding (incremental).

## Building & Running

- Solver: `src/hcp-solver <graph.dimacs> [options]`
- Grid graph gen: `src/grid-graph <cols> <rows>`
- Knight graph gen: `src/knight-graph`

Solver options:
- `-c, --cycle <int>`: Cycle multiplier (default: 2)
- `-a, --amo <opt>`: AtMostOne: default, pblib
- `-s, --start <opt>`: Start node: min/max/first/index
- `-b, --sym-break <opt>`: Symmetry breaking: default, none
- `-d, --decode <file>`: Decode solution from file
- `--incremental`: Use incremental SAT solving
- `--time-limit <ms>`: Per-solve time limit (default: 120000)
- `--stagnation-strategy <greedy|dfj>`: How to break stagnation

## Directories

- `src/`: Source code
- `graphs/`: DIMACS edge format graph files
- `experiments/`: Run results and automation
- `refs/cadical/build/`: CaDiCaL SAT solver (pre-built)

## Experiments

`src/run_experiments.py` - Solves all 136 FHCP graph benchmarks.

## Scoring History

**Jul 7 (commit 7242fe8):** 134/136 solved at 600s. II_3972, II_7932 timeout.
**Jul 10 (new encoding):** 126/136 at 120s. 10 timeouts: GPN_244, GPN_482, GPN_998, FLS_1014, FLS_845, FLS3_408, FLS3_731, FLS3_1054 + II_3972, II_7932.

### Fix: Stagnation Escalation Threshold

**Root cause:** New encoding enabled SAT search, finding diverse solutions. Stagnation detection (~3 consecutive high-Jaccard solutions) fired at 2-4 components, triggering greedy escalation that blocked half the graph. This destroyed the productive SEC-driven convergence loop, creating oscillation between 2-4 and 10+ components.

**Fix at `src/Solver.cpp:301`:** Added `components.size() > 4` guard on stagnation escalation. Below 5 components, base SEC loop converges naturally without greedy disruption.

**Fix at `src/Solver.cpp:224`:** Reduced `maxSkipVars` from `g.getNodes() * 15` to `g.getNodes()`. Skip pool = 1x vertices avoids inflating CaDiCaL formula 5x+, slowing per-solve calls.

**Jul 10 post-fix:** 125/136 solved at 120s wall clock.
- 7/8 non-II timeouts now SOLVED within 120s: GPN_244 (0.002s), GPN_482 (5.2s), FLS_1014 (91.8s), FLS_845 (85.7s), FLS3_408 (13.6s), FLS3_731 (46.4s), FLS3_1054 (95.8s)
- GPN_998 still TIMEOUT (converges to 2 comps but needs 3105 iters; 120s insufficient)
- Remaining timeouts at 120s: GPN_998, graph162, graph424, graph446, graph470, graph180, II_3972, II_7932, v-800-5, v-900-5, v-1000-5
- Limitation: new encoding's SAT search per solve() takes 0.01-10s vs old code's sub-ms 0-conflict solving, so large graphs need more wall time

### Fix: Convergence Fix — Preprocessing Defaults + DFJ Push

**Jul 10 convergence fix:** Enable preprocess/vertex-sep/dfj defaults + DFJ push at low comps.

**Phase A:** Flip `preprocess_`, `useVertexSep_`, `stagnationStrategy` defaults. Restore forced-clause generation for deg-2 vertices and 2-edge-cuts in preprocessing block.

**Phase B:** Add component-count tracking and periodic DFJ clause injection every 10th iteration when components ≤ 4. Breaks local-minimum stall at 2 components by forcing partition changes.

**Results:**
- GPN_998: SOLVED 93.6s (was TIMEOUT at 120s) — DFJ push breaks the 2-comp stall
- FLS_845: SOLVED 45.5s (was 85.7s) — vertex-sep + dfj strategies converge faster
- GPN_482: 16.5s (was 5.2s) — still well within limits
- FLS_1014: 112.6s (was 91.8s) — slight overhead from new defaults
- v-1000-5: still TIMEOUT — oscillates at 17-57 components, never reaches ≤4 for DFJ push

**Limitation:** DFJ push only fires at ≤4 components. Some graphs oscillate at higher component counts and need combined min-cut + DFJ approach.

## Key Features

- SAT encoding with pluggable AtMostOne and SymmetryBreaking modules
- Command-line argument parsing for solver options
- Encoding, solving, and decoding pipeline
- Experiment automation script for benchmarking

## Special Notes

- Build with: `make -C src` or individual g++ commands
- Generated graph files (`.edge`) in graphs/ are not tracked in git
- cadical SAT solver must be pre-built in refs/cadical/build/
- Input format: Standard DIMACS edge format
- Output format: DIMACS CNF for solver, plain text for decoding
