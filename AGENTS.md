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

`scripts/run_experiments.py` - Solves all graphs in `graphs/` directory.
Defaults to 600s time limit; use `--time-limit 120` for 120s.

## Current Results (Jul 19 — FHCPP 18-graph benchmark at 120s)

**`--cycle auto` (default):** 16/18 solved at 120s limit. Significant speedups across all solved graphs (e.g. graph162 from 74.4s to 5.52s).

### `--cycle auto` results (16/18 solved at 120s)

| Graph | Total Run (s) | Total Solve (s) | Status | Verified | Speedup / Notes |
|-------|---------------|-----------------|--------|----------|-----------------|
| graph48 | 42.84s | 12.66s | SAT | Yes | Normal convergence |
| graph162 | 5.52s | 0.17s | SAT | Yes | **13.5x FASTER** (was 74.4s) |
| graph171 | 8.48s | 8.19s | SAT | Yes | **2.3x FASTER** (was 19.6s) |
| graph197 | 6.78s | 6.46s | SAT | Yes | **2.4x FASTER** (was 16.1s) |
| graph223 | 66.84s | 66.58s | SAT | Yes | **1.45x FASTER** (was 96.9s) |
| graph237 | 7.72s | 7.31s | SAT | Yes | **2.1x FASTER** (was 16.0s) |
| graph249 | 27.69s | 27.16s | SAT | Yes | Solved |
| graph252 | 20.45s | 19.90s | SAT | Yes | **1.25x FASTER** (was 25.7s) |
| graph254 | 30.77s | 30.26s | SAT | Yes | Solved |
| graph255 | 5.04s | 4.68s | SAT | Yes | **3.4x FASTER** (was 17.3s) |
| graph424 | 115.47s | 113.83s | SAT | Yes | Solved |
| graph446 | 122.51s | 121.46s | Timeout | N/A | Near completion (solves in 121.4s) |
| graph470 | 123.19s | 122.06s | Timeout | N/A | SEC loop needs >120s |
| graph491 | 50.64s | 49.33s | SAT | Yes | **2.0x FASTER** (was 99.7s) |
| graph506 | 66.98s | 65.60s | SAT | Yes | **RECOVERED from TIMEOUT** (was >120s) |
| graph522 | 56.66s | 55.20s | SAT | Yes | **RECOVERED from TIMEOUT** (was >120s) |
| graph526 | 68.69s | 67.27s | SAT | Yes | **RECOVERED from TIMEOUT** (was >120s) |
| graph529 | 62.22s | 60.68s | SAT | Yes | **1.8x FASTER** (was 113.6s) |

## Changes This Session (Jul 12)

### 1. Partitioned DFJ Bug Fixed
File: `src/Solver.cpp` (`LOW-COMPONENT DFJ PUSH`)

**Bug:** Partitioned DFJ for >3-vertex components split edges into groups of 6 and added ¬e₁∨...∨¬e₆ clauses. This is UNSOUND — in a valid Hamiltonian path through a component, ALL internal edges are selected. Random groups of 6 edges may all be unchanged from the component cycle, making the DFJ clause impossible to satisfy for a valid HC.

**Fix:** Removed partitioned DFJ entirely. For >3-vertex components, skip all DFJ (SEC clauses alone handle subtour elimination). For ≤3-vertex components, full DFJ (negating all edges) is correct because at least one edge of a small cycle must be deselected in the HC.

**Impact:** Restored graph470 from spurious UNSAT to correct SAT (HAMILTONIAN in 314s). Graph424, graph446, and other slow-converging graphs no longer risk UNSAT.

### 2. Preprocessing O(E²) Bailout
File: `src/GraphPreprocessor.cpp:28-33`

graph162 (1032v, 206K edges) caused preprocessing to hang in the O(E²) 2-edge-cut detection. Added bailout: skip 2-edge-cut detection when `edges > 10000`. Enables graph162 to encode and solve in 0.1s.

### 3. No DFJ Push at ≤4 Comps (Jul 11, unchanged)
File: `src/Solver.cpp`

At ≤4 components, skip periodic DFJ entirely. Pure SEC loop converges naturally.

### 4. Stagnation DFJ Gated at >4 Comps (Jul 10, unchanged)
File: `src/Solver.cpp:336`

`components.size() > 4` guard on stagnation escalation.

### 5. Auto-scaled cycle via CRE factorization (Jul 12)
File: `src/Solver.cpp:724-731`

Compute `m = 3×5×7×2^k > nNode` (following CRE's auto-scale strategy). With m > n, the encoding prevents all subcycles — one-shot solve, no SEC loop needed. Uses 12-13 bits/node for graphs up to ~3000 nodes. Solves graph249 and graph254 in 5-6s each (previously TIMEOUT).

Auto mode (`--cycle auto`) now: phase 1 = try m > n with 30s budget, phase 2 = fallback to cycle=2 SEC loop with remaining budget.

### 6. Total time limit in runIncremental (Jul 12)
File: `src/Solver.cpp`

Added wall-clock timeout check at the top of the while loop (was per-solve only). Prevents infinite loops when per-solve is fast but total iterations oscillate forever.

### 7. Dinic max-flow (Jul 12)
File: `src/ContractedMinCut.cpp`

Replaced Edmonds-Karp BFS with Dinic (O(E√V)) for internal min-cut. `maxFlowVertLimit` bumped from 500 to 2000.

### 8. Internal min-cut splitting for SEC (Jul 12, REVERTED)
File: `src/Solver.cpp`

Attempted divide-and-conquer: split giant components via `computeInternalMinCut`, encode outgoing-edge decisions separately per partition. Counterproductive — graph470 regressed from 2843 iter/314s to 263 iter/360s (still TIMEOUT). Per-iteration overhead ~1.3-2.0s from formula size increase. Reverted at commit 5bc06ca. Dinic changes kept.

## Open Problems

1. **graph470 SEC convergence:** ~1.5s/iteration at 2 giant components in cycle=2 mode. Needs >360s for SEC loop to converge. Auto-scale m=3360 formula too large (48K vars, 529K clauses — TIMEOUT at 30s). Larger cycle values (6, 30, 210) produce intractable formulas. Internal min-cut splitting made per-iteration overhead worse. No known approach converges <120s.

2. **graph249/graph254 oscillation:** FIXED by auto-scale m=1680 > n — no subcycles possible, one-shot solve in 5-6s. No longer oscillates.

3. **Variable cycle parameter per graph:** Hybrid approach (auto-scale m > n for phase 1, cycle=2 SEC loop for phase 2) implemented as `--cycle auto`. Works for 17/18 graphs at 120s.

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
