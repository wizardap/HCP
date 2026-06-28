# Design Spec: Solution Path Visualization Tool and Repository Reorganization

**Date:** 2026-06-28  
**Status:** Approved  

---

## 1. Problem Statement

To inspect and verify Hamiltonian Cycle solutions, we need a command-line tool `scripts/visualize.py` that reads DIMACS graph files (`.edge`) and cycle path sequence files (`.path`), renders the graph, highlights the cycle, and saves the result as a static PNG/SVG image.

Additionally, we need to reorganize the workspace directory structure to separate C++ source code, Python automation scripts, and input/output client data.

---

## 2. Directory Reorganization

The repository structure will be reorganized as follows:

```
HCP/
├── graphs/                      # Client input graphs (*.edge)
├── solution_paths/              # Output node cycle sequences (*.path)
├── visualizations/              # Generated visualization PNG/SVG files
├── sol.csv                      # Experiments results CSV
├── scripts/                     # Python scripts
│   ├── run_experiments.py       # Benchmark runner
│   ├── run_CRE_experiments.py   # Chinese Remainder Encoding benchmark
│   ├── visualize.py             # New visualization script
│   └── requirements.txt         # Python dependencies
├── src/                         # C++ source code and binaries
│   ├── AtMostOne/               
│   ├── SymmetryBreaking/        
│   ├── *.cpp                    
│   ├── *.hpp                    
│   ├── Makefile                 
│   ├── hcp-solver               # Compiled binary
│   ├── grid-graph               # Compiled binary
│   └── knight-graph             # Compiled binary
└── refs/                        # External solver references (CaDiCaL, etc.)
```

---

## 3. Visualization Design

### 3.1. Dependencies (`scripts/requirements.txt`)
- `networkx>=3.0`
- `matplotlib>=3.5`

### 3.2. CLI Interface
- **Batch Mode (Default):** Running `python3 scripts/visualize.py` scans `solution_paths/` and generates visualizations in `visualizations/`.
- **Single-File Mode:**
  ```bash
  python3 scripts/visualize.py --graph graphs/graph48.edge --path solution_paths/graph48.path --output graph48.png
  ```

### 3.3. Layout and Rendering Logic
1. **Headless Execution:** Configure `matplotlib.use('Agg')` immediately on startup.
2. **Coordinates Layout:**
   - **Grid detection:** If node count $V = C \times R$ and edge structures match grid neighbors, place node $u$ at:
     - $x = (u - 1) / R$
     - $y = R - ((u - 1) \% R)$
   - **Knight Tour detection:** If node count $V = S^2$, place node $u$ at chessboard coords:
     - $x = (u - 1) \% S$
     - $y = S - ((u - 1) / S)$
   - **Fallback:** Use `networkx.spring_layout`.
3. **Large Graph Optimization:**
   - If node count $V > 1000$, do not draw background edges. Only render the cycle path (nodes and cycle edges) to avoid freezing or crashing.
   - If $V \leq 1000$, draw all background edges in thin light-gray, overlaying the cycle in thick red/orange.

---

## 4. Test & Verification Plan

1. **Reorganization Verification:**
   - Move directories/files and update path resolutions in `scripts/run_experiments.py` and `scripts/run_CRE_experiments.py`.
   - Run `python3 scripts/run_experiments.py` to verify the pipeline compiles, executes, and outputs to the new root-level `sol.csv` and `solution_paths/`.
2. **Dependency Setup:** Run `pip install -r scripts/requirements.txt`.
3. **Visualizer Validation:**
   - Run `python3 scripts/visualize.py` to batch-process all solutions.
   - Verify that `visualizations/graph48.png` correctly plots the 338-node grid graph with no tangles.
