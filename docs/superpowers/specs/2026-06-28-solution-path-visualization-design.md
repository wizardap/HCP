# Design Spec: Solution Path Visualization Tool

**Date:** 2026-06-28  
**Status:** Approved  

---

## 1. Problem Statement

To inspect and verify Hamiltonian Cycle solutions, we need a command-line tool `src/visualize.py` that reads DIMACS graph files (`.edge`) and cycle path sequence files (`.path`), renders the graph, highlights the cycle, and saves the result as a static PNG/SVG image.

The tool must handle:
1. **Large Scale Graphs:** Prevent memory exhaustion and visual clutter by only rendering the cycle path (and skipping background edges) for graphs with more than 1000 nodes.
2. **Structural Layouts:** Position nodes correctly on a 2D plane for grid graphs and knight-move graphs to make verification visually clear.
3. **Execution Modes:** Support both batch-processing all solutions and visualizing a single graph.

---

## 2. Architecture & Design

### 2.1. Dependencies
Dependencies are declared in `src/requirements.txt`:
- `networkx>=3.0`
- `matplotlib>=3.5`

### 2.2. Inputs and Output Directory
- Graph files: `src/graphs/<graph_name>.edge`
- Path files: `src/solution_paths/<graph_name>.path`
- Visualizations: `src/visualizations/<graph_name>.png`

### 2.3. Layout Logic
To determine node coordinates:
1. **Grid detection:** Check if the node count $V$ matches a grid $C \times R$ where edges match grid adjacency. If detected, position node $u$ at:
   - $x = (u - 1) / R$
   - $y = R - ((u - 1) \% R)$
2. **Knight Tour detection:** Check if the node count $V$ is a perfect square $S^2$ matching chessboard coordinate layout:
   - $x = (u - 1) \% S$
   - $y = S - ((u - 1) / S)$
3. **Fallback:** Use `networkx.spring_layout` (force-directed layout).

### 2.4. Drawing Logic
- Set Matplotlib to non-interactive backend `matplotlib.use('Agg')` on startup.
- Nodes count $V \leq 1000$: Render all edges in thin light-gray, overlay the cycle in thick red/orange.
- Nodes count $V > 1000$: Render only the cycle path (nodes and cycle edges) to prevent memory crashes.

---

## 3. Test & Verification Plan

1. **Setup:** Install dependencies via `pip install -r src/requirements.txt`.
2. **Single-Run Test:** Run on `graph48.edge` (338 nodes grid graph) and verify:
   - Node positioning aligns perfectly as a grid.
   - Output `graph48.png` is generated and clearly displays the Hamiltonian Cycle.
3. **Batch-Run Test:** Run `python3 src/visualize.py` to batch-process all 18 solved graphs.
