# Support Running All Data Formats in HCP/data Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow `run_experiments.py` and `run_CRE_experiments.py` to recursively scan and execute on all converted graph datasets under `HCP/data/`.

**Architecture:** Create a pre-processing Python script `scripts/convert_data.py` to translate all `.hcp` and `.txt` files under `data/` to standard DIMACS edge format and store them in corresponding subdirectories under `graphs/`. Update both experiment runner scripts to recursively discover `.edge` files and dynamically handle paths.

**Tech Stack:** Python 3, standard file and path utilities.

## Global Constraints

- Do not alter the raw source datasets in `HCP/data/`.
- Converted outputs under `graphs/` must conform to the standard DIMACS edge format (`p edge <nodes> <edges>` and `e <u> <v>`).
- Subdirectory structure of `data/` must be mirrored in `graphs/`.

---

### Task 1: Create Data Conversion Script

**Files:**
- Create: `scripts/convert_data.py`

**Interfaces:**
- Consumes: Raw files in `HCP/data/` (TSPLIB `.hcp` and custom `.txt` files)
- Produces: Standard DIMACS `.edge` files in matching subdirectories under `HCP/graphs/`

- [ ] **Step 1: Write the conversion script**

Create `scripts/convert_data.py` with the following content:

```python
import os
import sys

def convert_hcp(in_path, out_path):
    nNode = None
    edges = []
    in_edge_section = False
    with open(in_path, 'r') as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            if line.upper().startswith("DIMENSION"):
                if ":" in line:
                    nNode = int(line.split(":")[1].strip())
                else:
                    nNode = int(line.split()[1].strip())
            elif line.upper().startswith("EDGE_DATA_SECTION"):
                in_edge_section = True
            elif in_edge_section:
                if line == "-1" or line.upper().startswith("EOF"):
                    break
                parts = line.split()
                if len(parts) == 2:
                    edges.append((int(parts[0]), int(parts[1])))
    
    if nNode is None:
        raise ValueError(f"Could not parse DIMENSION from {in_path}")
    
    nEdge = len(edges)
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    with open(out_path, 'w') as f:
        f.write(f"p edge {nNode} {nEdge}\n")
        for u, v in edges:
            f.write(f"e {u} {v}\n")

def convert_txt(in_path, out_path):
    nNode = None
    nEdge = None
    edges = []
    with open(in_path, 'r') as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            if line.startswith("p edge"):
                parts = line.split()
                nNode = int(parts[2])
                nEdge = int(parts[3])
            else:
                parts = line.split()
                if len(parts) == 2:
                    edges.append((int(parts[0]), int(parts[1])))
    
    if nNode is None or nEdge is None:
        raise ValueError(f"Could not parse 'p edge' header from {in_path}")
         
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    with open(out_path, 'w') as f:
        f.write(f"p edge {nNode} {nEdge}\n")
        for u, v in edges:
            f.write(f"e {u} {v}\n")

def main():
    script_dir = os.path.dirname(os.path.abspath(__file__))
    data_dir = os.path.join(script_dir, "../data")
    graphs_dir = os.path.join(script_dir, "../graphs")
    
    for root, dirs, files in os.walk(data_dir):
        for file in files:
            in_path = os.path.join(root, file)
            rel_path = os.path.relpath(in_path, data_dir)
            
            # Form destination filename
            base, ext = os.path.splitext(rel_path)
            out_path = os.path.join(graphs_dir, base + ".edge")
            
            print(f"Converting {rel_path} -> {base}.edge...")
            if file.endswith(".hcp"):
                convert_hcp(in_path, out_path)
            elif file.endswith(".txt"):
                convert_txt(in_path, out_path)
            else:
                print(f"Skipping unknown file format: {file}")

if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Run the script to convert the data**

Run: `python3 scripts/convert_data.py`
Expected output: Converts all 136 files and writes them to matching subdirectories under `graphs/`.

- [ ] **Step 3: Verify output files exist and are correct**

Run: `head -n 10 graphs/fhcpcs/graph1.edge`
Expected output:
```
p edge 66 101
e 1 3
e 1 9
e 1 61
...
```

Run: `head -n 10 graphs/vset/v-39-3.edge`
Expected output:
```
p edge 39 115
e 1 32
e 1 23
e 1 2
...
```

- [ ] **Step 4: Commit the conversion script**

Run:
```bash
git add scripts/convert_data.py
git commit -m "feat: add batch data conversion script to translate all datasets to DIMACS"
```

---

### Task 2: Update `run_experiments.py`

**Files:**
- Modify: `scripts/run_experiments.py`

**Interfaces:**
- Consumes: `.edge` files in nested subdirectories under `graphs/`
- Produces: Correct solution paths in matching nested subdirectories under `solution_paths/`

- [ ] **Step 1: Update the file search logic and solution paths output**

In `scripts/run_experiments.py`, replace lines 28-34:

```python
    # Find all .edge files in graphs/
    files = [f for f in os.listdir(graphs_dir) if f.endswith(".edge")]
    # Sort files numerically if possible
    def get_num(filename):
        match = re.search(r'\d+', filename)
        return int(match.group()) if match else filename
    files.sort(key=get_num)
```

with:

```python
    # Find all .edge files in graphs/ recursively
    files = []
    for root, _, filenames in os.walk(graphs_dir):
        for f in filenames:
            if f.endswith(".edge"):
                rel_path = os.path.relpath(os.path.join(root, f), graphs_dir)
                files.append(rel_path)
    
    # Sort files numerically if possible
    def get_num(filename):
        base = os.path.basename(filename)
        match = re.search(r'\d+', base)
        return int(match.group()) if match else filename
    files.sort(key=get_num)
```

Also, modify lines 161 and 288 (the target path for shutil.copy):
In the incremental block (around line 161):
```python
                            dest_path = os.path.join(solution_paths_dir, f"{graph_name}.path")
```
replace with:
```python
                            dest_path = os.path.join(solution_paths_dir, f"{graph_name}.path")
                            os.makedirs(os.path.dirname(dest_path), exist_ok=True)
```

In the non-incremental block (around line 288):
```python
                            dest_path = os.path.join(solution_paths_dir, f"{graph_name}.path")
```
replace with:
```python
                            dest_path = os.path.join(solution_paths_dir, f"{graph_name}.path")
                            os.makedirs(os.path.dirname(dest_path), exist_ok=True)
```

- [ ] **Step 2: Run a dry run to verify execution on a small subset**

Temporarily edit `scripts/run_experiments.py` or run a single file, or check that it handles a nested file correctly by moving one test file.
Specifically, let's run `run_experiments.py` on a specific file, or check that it initializes without errors.
Run: `python3 scripts/run_experiments.py --non-incremental` (and terminate it early if needed, or check if it starts and runs on the first few graphs correctly).

- [ ] **Step 3: Commit updates to `run_experiments.py`**

Run:
```bash
git add scripts/run_experiments.py
git commit -m "refactor: update run_experiments.py to recurse subdirectories and copy solution paths dynamically"
```

---

### Task 3: Update `run_CRE_experiments.py`

**Files:**
- Modify: `scripts/run_CRE_experiments.py`

**Interfaces:**
- Consumes: `.edge` files in nested subdirectories under `graphs/`

- [ ] **Step 1: Update the file search logic**

In `scripts/run_CRE_experiments.py`, replace lines 26-32:

```python
    script_dir = os.path.dirname(os.path.abspath(__file__))
    files = [f for f in os.listdir(GRAPHS_DIR) if f.endswith(".edge")]

    def get_num(filename):
        match = re.search(r'\d+', filename)
        return int(match.group()) if match else filename
    files.sort(key=get_num)
```

with:

```python
    script_dir = os.path.dirname(os.path.abspath(__file__))
    files = []
    for root, _, filenames in os.walk(GRAPHS_DIR):
        for f in filenames:
            if f.endswith(".edge"):
                rel_path = os.path.relpath(os.path.join(root, f), GRAPHS_DIR)
                files.append(rel_path)

    def get_num(filename):
        base = os.path.basename(filename)
        match = re.search(r'\d+', base)
        return int(match.group()) if match else filename
    files.sort(key=get_num)
```

- [ ] **Step 2: Run a test to verify execution**

Run: `python3 scripts/run_CRE_experiments.py -g v-39-3`
Expected output:
Verify that the output table displays `vset/v-39-3.edge` and completes successfully.

- [ ] **Step 3: Commit updates to `run_CRE_experiments.py`**

Run:
```bash
git add scripts/run_CRE_experiments.py
git commit -m "refactor: update run_CRE_experiments.py to recursively find edge files"
```
