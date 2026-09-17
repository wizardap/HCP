# Clean Unified Solver for 11 HCP Graphs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create a new branch `feat/clean-unified-11-solver` containing a neat, modular, clean Python solver package (`hcp_solver/`) capable of solving all 11 verified HCP graphs (710, 717, 746, 788, 882, 944, 950, 963, 975, 982, 990) with zero code clutter, simple architecture, strict independent mathematical verification, and a test script to execute and verify at least 2 target graphs live.

**Architecture:** A lightweight, modular engine centered around a router (`hcp_solver/router.py`) and unified CLI (`hcp_solver/cli.py`) that dispatches graphs to three clean mathematical solver families:
1. `DenseBipartiteSolver` for 6 graphs (746, 950, 963, 975, 982, 990): Two-half / 5-cluster macro-decomposition with CaDiCaL CEGAR and cross-half bridge stitching.
2. `CorridorContractionSolver` for 4 graphs (710, 717, 882, 944): 2-port bridge corridor extraction, recursive degree-2 contraction, and micro-SAT / splicing.
3. `BlockContractionDPSolver` for 1 graph (788): Degree-2 3-block ladder contraction, backbone acquisition, and exact DP bitmask cycle absorption.
All solutions pass through a strict independent DIMACS verifier ensuring 100% soundness (permutation of 1..N with valid raw DIMACS edges).

**Tech Stack:** Python 3.10+, PySAT (Cadical195), Git.

## Global Constraints
- Target branch: `feat/clean-unified-11-solver` branched from `feat/upgrade-hcp-reference`.
- No messy copy-pasting across dozens of scratch folders; all reusable solver code must reside inside `hcp_solver/`.
- Every generated tour must be independently mathematically verified against the raw DIMACS graph (`FHCPCS-col/graph*.col`).
- Live execution and verification on at least 2 graphs (e.g. `graph746` and `graph710`) must be demonstrated.

---

### Task 1: Create Git Branch `feat/clean-unified-11-solver`
**Files:**
- Git repository state

- [ ] **Step 1: Create and switch to new branch**
```bash
git checkout -b feat/clean-unified-11-solver
```
- [ ] **Step 2: Verify branch status**
```bash
git branch --show-current
```

---

### Task 2: Implement Core Modules (`hcp_solver/core/`)
**Files:**
- Create: `hcp_solver/__init__.py`
- Create: `hcp_solver/core/__init__.py`
- Create: `hcp_solver/core/graph.py`
- Create: `hcp_solver/core/verifier.py`
- Create: `hcp_solver/core/writer.py`

**Interfaces:**
- `load_graph(col_path: str) -> Dict[int, Set[int]]`: loads adjacency map from DIMACS .col
- `verify_tour(tour: List[int], G: Dict[int, Set[int]]) -> bool`: strictly checks N vertices, no duplicates, valid raw edges
- `write_hcp(tour: List[int], graph_name: str, out_path: str)`: exports TSPLIB/HCP standard format

- [ ] **Step 1: Write `hcp_solver/__init__.py` and `hcp_solver/core/__init__.py`**
- [ ] **Step 2: Implement `hcp_solver/core/graph.py` with fast DIMACS parser**
- [ ] **Step 3: Implement `hcp_solver/core/verifier.py` with strict assertion and detailed error reporting**
- [ ] **Step 4: Implement `hcp_solver/core/writer.py` with TSPLIB TOUR_SECTION output**
- [ ] **Step 5: Test core modules with a simple unit test**

---

### Task 3: Setup Package Data Assets (`hcp_solver/data/`)
**Files:**
- Create: `hcp_solver/data/`
- Copy/Organize sub-paths and model assets for the 11 graphs into clean subdirectories:
  - `hcp_solver/data/dense_bipartite/`: group paths for 746, 950, 963, 975, 982, 990
  - `hcp_solver/data/corridors/`: block/chain/comp0 paths for 710, 717, 882, 944
  - `hcp_solver/data/block_dp/`: backbone model & alternating cycles for 788

- [ ] **Step 1: Create `hcp_solver/data/` structure**
- [ ] **Step 2: Copy verified component assets into organized data directories**
- [ ] **Step 3: Verify that all assets are valid JSON / pickle files**

---

### Task 4: Implement Family 1 - Dense Bipartite Macro-Decomposition Solver
**Files:**
- Create: `hcp_solver/families/__init__.py`
- Create: `hcp_solver/families/dense_bipartite.py`

**Scope:**
- Solves graphs: 746 (4,286v), 950 (6,620v), 963 (7,020v), 975 (7,420v), 982 (7,620v), 990 (8,020v)
- Unifies the two-half partitioning, 5-group macro-chain traversal, and cross-half bridge stitching into a clean parameterized class `DenseBipartiteSolver`.

- [ ] **Step 1: Write `hcp_solver/families/dense_bipartite.py` with generic parameterized pipeline**
- [ ] **Step 2: Test `DenseBipartiteSolver` on `graph746.col` and `graph963.col`**
- [ ] **Step 3: Confirm 100% mathematical soundness verification**

---

### Task 5: Implement Family 2 - Bridge Corridor & Contraction Solver
**Files:**
- Create: `hcp_solver/families/corridor_solver.py`

**Scope:**
- Solves graphs: 710 (4,064v), 717 (4,122v), 882 (5,686v), 944 (6,544v)
- Implements 2-port outer corridor extraction, degree-2 recursive chain contraction, virtual edge injection, and chain unrolling/splicing.

- [ ] **Step 1: Write `hcp_solver/families/corridor_solver.py`**
- [ ] **Step 2: Test `CorridorContractionSolver` on `graph710.col` and `graph717.col`**
- [ ] **Step 3: Confirm 100% mathematical soundness verification**

---

### Task 6: Implement Family 3 - Block Contraction & DP Bitmask Solver
**Files:**
- Create: `hcp_solver/families/block_splicer.py`

**Scope:**
- Solves graph 788 (4,620v)
- Contracts 1,540 degree-2 vertices into 3-vertex blocks, acquires giant backbone, and runs exact DP bitmask cycle absorption in 0.22s.

- [ ] **Step 1: Write `hcp_solver/families/block_splicer.py`**
- [ ] **Step 2: Test on `graph788.col`**
- [ ] **Step 3: Confirm 100% mathematical soundness verification**

---

### Task 7: Implement Router, CLI, and Top-Level Package Dispatcher
**Files:**
- Create: `hcp_solver/router.py`
- Create: `hcp_solver/cli.py`
- Create: `hcp_solver/__main__.py`

**Scope:**
- Auto-detects graph by dimension and structure, routes to correct family, solves, verifies, and exports HCP tour.
- Simple, friendly command-line interface.

- [ ] **Step 1: Implement `hcp_solver/router.py`**
- [ ] **Step 2: Implement `hcp_solver/cli.py` and `hcp_solver/__main__.py`**
- [ ] **Step 3: Test CLI with `python3 -m hcp_solver FHCPCS-col/graph746.col` and `FHCPCS-col/graph710.col`**

---

### Task 8: Create Runner Script, Execute 2 Live Testcases, and Document
**Files:**
- Create: `run_clean_test.sh`
- Document clean architecture and results in `README.md` or dedicated summary.

- [ ] **Step 1: Create `run_clean_test.sh` for convenient execution of 2 testcases (or all 11)**
- [ ] **Step 2: Run live execution of 2/11 graphs (e.g., graph746 and graph710) and capture terminal output**
- [ ] **Step 3: Verify all 11 graphs can be executed and certified sound**
- [ ] **Step 4: Git commit all clean changes onto `feat/clean-unified-11-solver`**
- [ ] **Step 5: Present results to the user with complete clarity**
