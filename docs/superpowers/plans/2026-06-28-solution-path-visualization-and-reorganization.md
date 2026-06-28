# Solution Path Visualization and Repository Reorganization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reorganize the repository directory structure to separate source code, scripts, and data, and create a python-based solution path visualization tool using `networkx` and `matplotlib`.

**Architecture:**
1. Move the scripts to `scripts/` and input/output directories to the repository root.
2. Update paths in `run_experiments.py` and `run_CRE_experiments.py` to match the new reorganization structure.
3. Add `scripts/requirements.txt` for Python dependencies.
4. Implement `scripts/visualize.py` with automatic coordinate layout mapping for grid/knight graphs, fallback spring-layout, and large graph optimization to prevent rendering freezes.

**Tech Stack:** Python 3, NetworkX, Matplotlib, Git

## Global Constraints
- Do not modify any files in the `HCP/refs/` directory (specifically `refs/cadical/`).
- Standard DIMACS and CaDiCaL statistics output formats must be preserved.
- All temporary CNF and SAT files must be cleaned up properly after verification.
- Output path sequence must be space-separated in `solution.path`.

---

### Task 1: Reorganize Repository Structure

**Files:**
- Create: `scripts/` (directory)
- Move: `src/graphs/` -> `graphs/`
- Move: `src/solution_paths/` -> `solution_paths/`
- Move: `src/run_experiments.py` -> `scripts/run_experiments.py`
- Move: `src/run_CRE_experiments.py` -> `scripts/run_CRE_experiments.py`
- Delete: `src/sol.log`, `src/sol.csv` (if present)

**Interfaces:**
- Consumes: C++ solver binaries and data files from updated locations.
- Produces: Correctly configured path routes for benchmark executors.

- [ ] **Step 1: Create directories and move files via git**

  Run:
  ```bash
  # Ensure CWD is repository root
  cd /home/ubuntu/HCP
  
  # Create scripts directory
  mkdir -p scripts
  
  # Move graphs and solution paths to root
  git mv src/graphs graphs
  git mv src/solution_paths solution_paths
  
  # Move scripts to scripts/
  git mv src/run_experiments.py scripts/run_experiments.py
  git mv src/run_CRE_experiments.py scripts/run_CRE_experiments.py
  
  # Remove old log/csv files in src/ if any
  rm -f src/sol.log src/sol.csv
  ```

- [ ] **Step 2: Update path resolutions in `scripts/run_experiments.py`**

  Modify `scripts/run_experiments.py` to point to the new directory structure:
  - Replace lines 7-65 with:
    ```python
    def main():
        script_dir = os.path.dirname(os.path.abspath(__file__))
        non_incremental = "--non-incremental" in sys.argv
        incremental = not non_incremental
        # Resolve to root-level graphs/
        graphs_dir = os.path.join(script_dir, "../graphs")
        
        # Build original decoder
        print("c Compiling original hcp-decode...")
        subprocess.run(
            ["make", "-C", os.path.join(script_dir, "../refs/ChineseRemainderEncoding"), "hcp-decode"],
            check=True
        )
        
        # Ensure root-level solution_paths directory exists
        solution_paths_dir = os.path.join(script_dir, "../solution_paths")
        if os.path.exists(solution_paths_dir):
            shutil.rmtree(solution_paths_dir)
        os.makedirs(solution_paths_dir)
        
        # Find all .edge files in graphs/
        files = [f for f in os.listdir(graphs_dir) if f.endswith(".edge")]
        # Sort files numerically if possible
        def get_num(filename):
            match = re.search(r'\d+', filename)
            return int(match.group()) if match else filename
        files.sort(key=get_num)
        
        # CSV Header
        header = "Graph,Total Variables,Total Clauses,Total Runtime (s),Total Solver Time (s),Final Solve Time (s),Status,Verified,Actions,Conflicts,Decisions,Propagations"
        
        # Resolve to root-level sol.csv
        log_file = os.path.join(script_dir, "../sol.csv")
        with open(log_file, "w") as log:
            log.write(header + "\n")
            
            # Print visual table header on console
            print(f"{'Graph':<15} | {'Variables':<10} | {'Clauses':<10} | {'Total Run (s)':<15} | {'Total Solve (s)':<15} | {'Final Solve (s)':<15} | {'Status':<12} | {'Verified':<10}")
            print("-" * 115)
            
            for file in files:
                graph_path = os.path.join(graphs_dir, file)
                graph_name = os.path.splitext(file)[0]
    ```
  - And update subprocess `hcp-solver` binary resolution and cleanups:
    - Replace `os.path.join(script_dir, "hcp-solver")` with `os.path.join(script_dir, "../src/hcp-solver")`
    - Replace `os.path.join(script_dir, "solution.sat")` with `os.path.join(script_dir, "../src/solution.sat")`
    - Replace `os.path.join(script_dir, "temp_clean.sat")` with `os.path.join(script_dir, "../src/temp_clean.sat")`
    - Replace `os.path.join(script_dir, "temp_run.cnf")` with `os.path.join(script_dir, "../src/temp_run.cnf")`
    - Replace `os.path.join(script_dir, "temp_run.sat")` with `os.path.join(script_dir, "../src/temp_run.sat")`
    - Replace `os.path.join(script_dir, "temp_run_stdout.sat")` with `os.path.join(script_dir, "../src/temp_run_stdout.sat")`
    - Replace `solution.path` source_path to: `os.path.join(script_dir, "../src/solution.path")`
    - Replace `cwd=script_dir` with `cwd=os.path.join(script_dir, "../src")`

- [ ] **Step 3: Update path resolutions in `scripts/run_CRE_experiments.py`**

  Modify `scripts/run_CRE_experiments.py`:
  - Replace lines 7-11 with:
    ```python
    CRE_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "refs", "ChineseRemainderEncoding")
    CADICAL = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "refs", "cadical", "build", "cadical")
    HCP_ENCODE = os.path.join(CRE_DIR, "hcp-encode")
    HCP_DECODE = os.path.join(CRE_DIR, "hcp-decode")
    GRAPHS_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "graphs")
    ```
  - Replace line 42 with:
    ```python
    log_file = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "CRE_sol.log")
    ```

- [ ] **Step 4: Verify the reorganization works**

  Run:
  ```bash
  python3 scripts/run_experiments.py
  ```
  Expected:
  - Compiles hcp-decode successfully.
  - Successfully solves, verifies, and outputs results.
  - Outputs are created at the root level: `sol.csv` and files under `solution_paths/`.

- [ ] **Step 5: Commit reorganization changes**

  Run:
  ```bash
  git add scripts/run_experiments.py scripts/run_CRE_experiments.py
  git commit -m "refactor: reorganize project structure and update benchmark script paths"
  ```

---

### Task 2: Create Python Visualization Script with Reorganization Paths

**Files:**
- Create: `scripts/requirements.txt`
- Create: `scripts/visualize.py`

**Interfaces:**
- Consumes: `.edge` files from `graphs/`, `.path` files from `solution_paths/`
- Produces: `.png` files under `visualizations/`

- [ ] **Step 1: Create `scripts/requirements.txt`**

  Write to `scripts/requirements.txt`:
  ```text
  networkx>=3.0
  matplotlib>=3.5
  ```

- [ ] **Step 2: Create `scripts/visualize.py`**

  Write to `scripts/visualize.py`:
  ```python
  import os
  import sys
  import argparse
  import re
  import matplotlib
  matplotlib.use('Agg') # Headless backend
  import matplotlib.pyplot as plt
  import networkx as nx

  def parse_args():
      parser = argparse.ArgumentParser(description="Visualize Hamiltonian Cycle solution paths")
      parser.add_argument("--graph", type=str, default=None, help="Path to DIMACS .edge file")
      parser.add_argument("--path", type=str, default=None, help="Path to space-separated solution .path file")
      parser.add_argument("--output", type=str, default=None, help="Output image file path (.png)")
      return parser.parse_args()

  def read_graph(graph_path):
      G = nx.Graph()
      num_nodes = 0
      with open(graph_path, "r") as f:
          for line in f:
              line = line.strip()
              if not line:
                  continue
              if line.startswith("p edge"):
                  parts = line.split()
                  num_nodes = int(parts[2])
                  G.add_nodes_from(range(1, num_nodes + 1))
              elif line.startswith("e"):
                  parts = line.split()
                  u, v = int(parts[1]), int(parts[2])
                  G.add_edge(u, v)
      return G, num_nodes

  def read_path(path_path):
      with open(path_path, "r") as f:
          content = f.read().strip()
      if not content:
          return []
      return [int(x) for x in content.split()]

  def detect_layout(G, num_nodes):
      # 1. Grid Check
      # Try to factor num_nodes = C * R
      factors = []
      for r in range(1, int(num_nodes**0.5) + 1):
          if num_nodes % r == 0:
              factors.append((r, num_nodes // r))
              factors.append((num_nodes // r, r))
              
      for rows, cols in factors:
          # Check if edges match grid structure (u is connected to u+1 and u+rows)
          matching_edges = 0
          for u, v in G.edges():
              diff = abs(u - v)
              if diff == 1 or diff == rows:
                  matching_edges += 1
          # If a significant majority of edges match a grid, assume it's a grid
          if matching_edges > 0.8 * len(G.edges()):
              pos = {}
              for u in range(1, num_nodes + 1):
                  col = (u - 1) // rows
                  row = (u - 1) % rows
                  pos[u] = (col, rows - 1 - row)
              return pos, "Grid"
              
      # 2. Knight's Chessboard Check (perfect square layout)
      s_float = num_nodes**0.5
      s = int(round(s_float))
      if s * s == num_nodes:
          pos = {}
          for u in range(1, num_nodes + 1):
              i = u - 1
              col = i % s
              row = i // s
              pos[u] = (col, s - 1 - row)
          return pos, "Knight Chessboard"

      # 3. Fallback: Spring Layout
      return nx.spring_layout(G), "Spring"

  def visualize(graph_path, path_path, output_path):
      print(f"c Loading graph {graph_path}...")
      G, num_nodes = read_graph(graph_path)
      
      print(f"c Loading solution path {path_path}...")
      path_nodes = read_path(path_path)
      
      # Build directed cycle edges
      cycle_edges = []
      if len(path_nodes) > 1:
          for i in range(len(path_nodes) - 1):
              cycle_edges.append((path_nodes[i], path_nodes[i+1]))
          # Close cycle
          cycle_edges.append((path_nodes[-1], path_nodes[0]))
          
      pos, layout_type = detect_layout(G, num_nodes)
      print(f"c Detected layout style: {layout_type}")
      
      plt.figure(figsize=(10, 10))
      plt.title(f"Hamiltonian Cycle on {os.path.basename(graph_path)}\nNodes: {num_nodes}, Layout: {layout_type}")
      
      # Rendering threshold for large graphs to prevent CPU/memory exhaust
      if num_nodes > 1000:
          print("c Large graph detected. Rendering ONLY the cycle path to prevent memory exhaustion.")
          # Draw only cycle nodes and cycle edges
          nx.draw_networkx_nodes(G, pos, nodelist=path_nodes, node_size=10, node_color='blue')
          nx.draw_networkx_edges(G, pos, edgelist=cycle_edges, edge_color='red', width=2.0, arrows=True, arrowsize=8)
      else:
          # Draw full graph in background (nodes + edges)
          nx.draw_networkx_nodes(G, pos, node_size=15, node_color='lightblue')
          nx.draw_networkx_edges(G, pos, edge_color='lightgray', width=0.5)
          
          # Draw cycle overlay
          nx.draw_networkx_edges(G, pos, edgelist=cycle_edges, edge_color='red', width=2.5, arrows=True, arrowsize=10)
          
          # Highlight start node in green
          if path_nodes:
              nx.draw_networkx_nodes(G, pos, nodelist=[path_nodes[0]], node_size=40, node_color='green')

      plt.axis('off')
      plt.tight_layout()
      plt.savefig(output_path, dpi=150)
      plt.close()
      print(f"c Visualization saved successfully to {output_path}")

  def main():
      script_dir = os.path.dirname(os.path.abspath(__file__))
      args = parse_args()
      
      if args.graph and args.path and args.output:
          # Single run
          visualize(args.graph, args.path, args.output)
      else:
          # Batch run
          solution_paths_dir = os.path.join(script_dir, "../solution_paths")
          graphs_dir = os.path.join(script_dir, "../graphs")
          visualizations_dir = os.path.join(script_dir, "../visualizations")
          
          if not os.path.exists(visualizations_dir):
              os.makedirs(visualizations_dir)
              
          if not os.path.exists(solution_paths_dir):
              print(f"Error: Solution paths directory {solution_paths_dir} does not exist.")
              sys.exit(1)
              
          path_files = [f for f in os.listdir(solution_paths_dir) if f.endswith(".path")]
          if not path_files:
              print("No solution path files found.")
              sys.exit(0)
              
          print(f"c Found {len(path_files)} solution paths. Generating visualizations...")
          for path_file in path_files:
              graph_name = os.path.splitext(path_file)[0]
              graph_path = os.path.join(graphs_dir, f"{graph_name}.edge")
              path_path = os.path.join(solution_paths_dir, path_file)
              output_path = os.path.join(visualizations_dir, f"{graph_name}.png")
              
              if os.path.exists(graph_path):
                  try:
                      visualize(graph_path, path_path, output_path)
                  except Exception as e:
                      print(f"Error visualizing {graph_name}: {e}")
              else:
                  print(f"Warning: Graph file {graph_path} not found for path {path_file}")

  if __name__ == "__main__":
      main()
  ```

- [ ] **Step 3: Setup requirements and verify visualizer on small graph**

  Run:
  ```bash
  # Install dependencies
  pip install -r scripts/requirements.txt
  
  # Run visualizer batch mode
  python3 scripts/visualize.py
  ```
  Expected:
  - `requirements.txt` installs cleanly.
  - Script detects the layouts (e.g. Grid for `graph48`) and renders them cleanly.
  - Outputs `visualizations/*.png` successfully.
  - `graph162` renders quickly by skipping background edges.

- [ ] **Step 4: Commit visualizer changes**

  Run:
  ```bash
  git add scripts/requirements.txt scripts/visualize.py
  git commit -m "feat: add solution path visualization script with grid layout support"
  ```
