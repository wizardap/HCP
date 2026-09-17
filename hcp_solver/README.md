# HCP Lean Unified Solver Package (`hcp_solver`)

A lightweight, clean, and modular Python package providing unified solvers for the **11 solved Hamiltonian Cycle Problem (HCP) challenge graphs**:
`710, 717, 746, 788, 882, 944, 950, 963, 975, 982, 990`.

---

## 1. Architectural Overview

The solver architecture is structured into 3 distinct mathematical graph families, orchestrated by an automatic router and verified by a strict mathematical verifier:

```
hcp_solver/
├── __init__.py           # Package metadata & version
├── cli.py                # Command-line interface
├── router.py             # Automatic graph family classifier & dispatcher
├── core/
│   ├── graph.py          # Fast DIMACS .col reader & graph adjacency abstraction
│   ├── verifier.py       # Strict mathematical validator (soundness, 0 dupes, 100% valid edges)
│   └── writer.py         # Standard TSPLIB / HCP tour exporter
├── families/
│   ├── dense_bipartite.py    # Family 1: graphs 746, 950, 963, 975, 982, 990
│   ├── corridor_solver.py    # Family 2: graphs 710, 717, 882, 944
│   └── block_splicer.py      # Family 3: graph 788
└── data/                 # Clean, compact precomputed components & certificates (~400KB)
```

---

## 2. Supported Graph Families

### Family 1: Dense Bipartite Macro-Decomposition
- **Graphs**: `746` ($N=4,286$), `950` ($N=6,620$), `963` ($N=7,020$), `975` ($N=7,420$), `982` ($N=7,620$), `990` ($N=8,020$).
- **Mechanism**:
  1. Decomposes graph into 2 halves or 5 symmetric macro-clusters.
  2. Solves group bulk paths via SAT CEGAR.
  3. Stitches macro-chains across cross-half bridges.

### Family 2: Bridge Corridor & Degree-2 Contraction
- **Graphs**: `710` ($N=4,064$), `717` ($N=4,122$), `882` ($N=5,686$), `944` ($N=6,544$).
- **Mechanism**:
  1. Extracts 2-port outer corridors and replaces them with virtual shortcut edges.
  2. Recursively contracts degree-2 chains on the central core (`Comp 0`).
  3. Reconstructs and unrolls chains/corridors into full Hamiltonian cycles.

### Family 3: Directed Block Contraction & DP Bitmask Splicing
- **Graphs**: `788` ($N=4,620$).
- **Mechanism**:
  1. Contracts 1,540 degree-2 vertices into 3-vertex blocks.
  2. Acquires Giant Backbone covering >90% of blocks.
  3. Executes exact DP bitmask cycle absorption to merge remaining subcycles into a single Hamiltonian tour in $<0.25$s.

---

## 3. Quickstart & Usage

### Running via CLI
```bash
# Solve and verify a single graph:
python3 -m hcp_solver FHCPCS-col/graph746.col -o output_tours/tour_746.hcp

# Solve without writing output:
python3 -m hcp_solver FHCPCS-col/graph710.col
```

### Running Test Suite
```bash
# Run 2 sample testcases (graph746 & graph710):
./run_clean_11.sh

# Run and verify all 11 graphs:
./run_clean_11.sh --all

# Run a specific graph:
./run_clean_11.sh 788
```

### Python API
```python
from hcp_solver.router import HCPRouter

tour = HCPRouter.solve_file("FHCPCS-col/graph746.col", "tour_746.hcp", verify=True)
print(f"Solved tour with {len(tour)} vertices!")
```

---

## 4. Soundness Guarantees

Every solved tour is strictly and independently validated against the raw DIMACS `.col` graph:
1. **Dimension check**: Length of tour equals $|V|$ of the raw graph.
2. **Injectivity check**: Zero duplicate vertices visited (permutation of $V$).
3. **Adjacency check**: Every consecutive pair $(u, v)$ and $(v_{last}, v_0)$ is an explicit edge in $E(G)$.
