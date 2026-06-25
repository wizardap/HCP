<!-- CODEGRAPH_START -->
## CodeGraph

In repositories indexed by CodeGraph (a `.codegraph/` directory exists at the repo root), reach for it BEFORE grep/find or reading files when you need to understand or locate code:

- **MCP tool** (when available): `codegraph_explore` answers most code questions in one call — the relevant symbols' verbatim source plus the call paths between them, including dynamic-dispatch hops grep can't follow. Name a file or symbol in the query to read its current line-numbered source. If it's listed but deferred, load it by name via tool search.
- **Shell** (always works): `codegraph explore "<symbol names or question>"` prints the same output.

If there is no `.codegraph/` directory, skip CodeGraph entirely — indexing is the user's decision.
<!-- CODEGRAPH_END -->

## Project Overview

This is a C++ Hamiltonian Cycle Problem (HCP) solver project with SAT encoding.

## Building & Running

- Primary solver: `src/hcp-solver <graph.dimacs> [options]` - Encode and solve graphs
- Grid generator: `src/grid-graph <cols> <rows>` - Generate grid graphs
- Knight graph: `src/knight-graph` - Generate knight-move graphs

Options for hcp-solver:
- `-c, --cycle <int>`: Cycle multiplier (default: 2)
- `-a, --amo <opt>`: AtMostOne module: default, pblib
- `-s, --start <opt>`: Start node: min (min degree), max (max degree), first (node 0), or node index
- `-b, --sym-break <opt>`: Symmetry breaking module: default, none
- `-d, --decode <file>`: Decode solution from file to verify

## Directories

- `src/`: Executable binaries and source code
- `graphs/`: Generated DIMACS graph files (`.edge` format) - client data
- `refs/cadical/build/`: Required SAT solver dependency

## Experiments

Run comprehensive tests: `src/run_experiments.py` - Solves all 36 test graphs and benchmarks

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
