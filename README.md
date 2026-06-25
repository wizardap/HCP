# HCP Solver — SAT-Based Hamiltonian Cycle Problem Solver

A C++ solver for the **Hamiltonian Cycle Problem (HCP)** using SAT encoding with support for both classic one-shot solving and an **incremental mode** with lazy Subtour Elimination Constraints (SECs).

---

## Features

- **One-shot mode**: Encodes the full HCP to DIMACS CNF and outputs it for an external SAT solver.
- **Incremental mode** (`--incremental`): Uses CaDiCaL's C API directly, detecting and eliminating subtours iteratively with lazy SECs — significantly reducing the number of clauses needed for sparse/small-moduli graphs.
- Pluggable **AtMostOne** modules: `default`, `pblib`.
- Pluggable **symmetry breaking** modules: `default`, `none`.
- Configurable start node heuristics: `min` (min-degree), `max` (max-degree), `first`, or a specific node index.
- Decoder to verify solutions against the original graph.

---

## Repository Structure

```
HCP/
├── src/                        # Source code
│   ├── Solver.cpp / .hpp       # Main solver & CLI
│   ├── HcpEncoder.hpp          # CNF encoder for HCP
│   ├── HcpDecoder.hpp          # Solution verifier
│   ├── Graph.hpp               # Graph representation (DIMACS edge format)
│   ├── IncrementalSolver.*     # CaDiCaL incremental wrapper
│   ├── SubtourDetector.*       # Union-Find subtour detection
│   ├── SecEncoder.*            # Subtour Elimination Clause generator
│   ├── VariableManager.*       # SAT variable allocation
│   ├── AtMostOne/              # AtMostOne encoding modules
│   ├── SymmetryBreaking/       # Symmetry breaking modules
│   ├── GridGraphGenerator.cpp  # Utility: generate grid graphs
│   ├── KnightGraphGenerator.cpp# Utility: generate knight-move graphs
│   ├── Makefile
│   ├── run_experiments.py      # Experiment automation
│   └── test_incremental_solver.cpp  # Unit test suite
├── refs/                       # Git submodules (external dependencies)
│   ├── cadical/                # CaDiCaL SAT solver
│   ├── pblib/                  # PBLib pseudo-boolean library
│   ├── ChineseRemainderEncoding/  # Reference CRE implementation
│   └── painless/               # Painless parallel solver (reference)
└── docs/                       # Design documents & specs
```

---

## Prerequisites

- **g++** with C++17 support (`g++ --version` ≥ 7)
- **make**
- **cmake** (for building CaDiCaL and PBLib)
- **Python 3** (for `run_experiments.py`)

---

## Setup

### 1. Clone with submodules

```bash
git clone --recurse-submodules https://github.com/wizardap/HCP.git
cd HCP
```

Or if already cloned:

```bash
git submodule update --init --recursive
```

### 2. Build CaDiCaL

```bash
cd refs/cadical
./configure && make
cd ../..
```

This produces `refs/cadical/build/libcadical.a` which the solver links against.

### 3. Build PBLib

```bash
cd refs/pblib
cmake -B build -S . && cmake --build build
cd ../..
```

This produces `refs/pblib/build/libpb.a`.

### 4. Build the solver

```bash
make -C src
```

Produces: `src/hcp-solver`, `src/grid-graph`, `src/knight-graph`.

### 5. Run unit tests

```bash
make -C src test
```

Expected output:
```
Testing VariableManager... VariableManager passed!
Testing IncrementalSolver Basic... IncrementalSolver Basic passed!
Testing IncrementalSolver Timeout... IncrementalSolver Timeout passed!
Testing SubtourDetector and SecEncoder... SubtourDetector and SecEncoder passed!
All unit tests passed successfully!
```

---

## Usage

```
./src/hcp-solver <graph.dimacs> [options]

Options:
  -c, --cycle <int>       Cycle multiplier (default: 2)
  -a, --amo <opt>         AtMostOne module: default, pblib
  -s, --start <opt>       Start node: min, max, first, or node index
  -b, --sym-break <opt>   Symmetry breaking module: default, none
  --incremental           Use incremental SAT solving with subtour detection
  --time-limit <sec>      Solver time limit in seconds (default: 600)
  -d, --decode <file>     Decode and verify a solution file
  -h, --help              Show this help
```

### Examples

**Generate and solve a 4×4 grid (one-shot CNF output):**
```bash
./src/grid-graph 4 4 > grid4x4.edge
./src/hcp-solver grid4x4.edge
```

**Incremental mode with subtour elimination:**
```bash
./src/hcp-solver grid4x4.edge --incremental
# c Iteration: found 5 components, added 10 SEC clauses
# c Iteration: found 2 components, added 4 SEC clauses
# c HAMILTONIAN found
```

**Verify a solution:**
```bash
./src/hcp-solver grid4x4.edge --decode solution.sat
# c VERIFIED HCP of size 16
```

**Run full benchmark suite:**
```bash
python3 src/run_experiments.py
```

---

## Input Format

Standard **DIMACS edge format**:
```
p edge <nodes> <edges>
e <u> <v>
...
```

---

## Branch: `optimized-cre`

This branch adds the **incremental SEC solver** on top of the base Chinese Remainder Encoding (CRE). See [`docs/superpowers/specs/`](docs/superpowers/specs/) for the design spec and [`docs/superpowers/plans/`](docs/superpowers/plans/) for the implementation plan.
