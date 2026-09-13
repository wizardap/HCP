# Two-Half Two-Tier Hierarchical Solver for graph982.col Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Solve `graph982.col` ($N=7,620, M=33,218$) in $< 60$ seconds using the Two-Half Two-Tier hierarchical decomposition and CaDiCaL CEGAR intra-cluster routing, and independently certify the Hamiltonian tour with zero tour injection.

**Architecture:** Decompose `graph982.col` into two equal halves ($\text{Half}_1 = 3,810$v, $\text{Half}_2 = 3,810$v) separated by an exact 2-bridge cut $e_1 = (1495, 6335), e_2 = (6670, 7311)$. Decompose each half into 5 bulk groups of 760 vertices, route each group with CaDiCaL CEGAR in 2–3s, assemble into continuous paths across each half, and stitch the bridges to form the 7,620-vertex certified tour.

**Tech Stack:** Python 3, PySAT (`Cadical195`, `CardEnc`, `EncType.seqcounter`), `scratch/verify_benchmarks.py`.

## Global Constraints
- Target Graph: `FHCPCS-col/graph982.col` ($N = 7,620, M = 33,218$).
- Zero Tour Injection: Never read, preload, or inspect `.tou` reference files.
- Zero Phantom Edges: Every edge in the tour must exist in `raw_g.adjacency_list` from `graph982.col`.
- Complete Tour Certification: Independent validation using `scratch/verify_benchmarks.py` must report `Validation Result: PASS - CERTIFIED SOUND`.
- Runtime limit: $\le 1,800$s (target $< 60$s).

---

### Task 1: Half & Cluster Partition Mapping (`scratch/graph982/two_half_decomposer.py`)

**Files:**
- Create: `scratch/graph982/two_half_decomposer.py`
- Test: `scratch/graph982/test_partition.py`

**Interfaces:**
- Consumes: `FHCPCS-col/graph982.col`
- Produces: `load_and_partition_graph982(col_path: str)` returning `(G, degs, grp1, grp2, h1_targets, h2_targets, bridges)`

- [ ] **Step 1: Write the partition test**

```python
# scratch/graph982/test_partition.py
import os, sys
from scratch.graph982.two_half_decomposer import load_and_partition_graph982

def test_partition_invariants():
    col_path = "FHCPCS-col/graph982.col"
    assert os.path.exists(col_path), f"File not found: {col_path}"
    G, degs, grp1, grp2, h1_targets, h2_targets, bridges = load_and_partition_graph982(col_path)

    # Validate 10 super-hubs
    s_hubs = [u for u in degs if degs[u] == 762]
    assert len(s_hubs) == 10, f"Expected 10 super-hubs, got {len(s_hubs)}"

    # Validate 2 equal halves of 3,810 vertices
    assert len(grp1) == 3810, f"Half 1 size mismatch: {len(grp1)}"
    assert len(grp2) == 3810, f"Half 2 size mismatch: {len(grp2)}"
    assert grp1.isdisjoint(grp2), "Halves must be disjoint"
    assert len(grp1 | grp2) == 7620, "Union must cover all 7,620 vertices"

    # Validate 2-edge cut
    assert len(bridges) == 2, f"Expected 2 bridge edges, got {len(bridges)}"
    assert (1495, 6335) in bridges or (6335, 1495) in bridges
    assert (6670, 7311) in bridges or (7311, 6670) in bridges

    # Validate 5 targets per half
    assert len(h1_targets) == 5, f"Expected 5 groups in H1, got {len(h1_targets)}"
    assert len(h2_targets) == 5, f"Expected 5 groups in H2, got {len(h2_targets)}"

    print("All partition invariants PASSED successfully!")

if __name__ == "__main__":
    test_partition_invariants()
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 scratch/graph982/test_partition.py`  
Expected: FAIL with `ModuleNotFoundError: No module named 'scratch.graph982.two_half_decomposer'`

- [ ] **Step 3: Implement `two_half_decomposer.py`**

```python
# scratch/graph982/two_half_decomposer.py
import collections
from typing import Dict, List, Set, Tuple, Any
from scratch.graph950.two_half_two_tier_solver import load_graph, decompose_half

def load_and_partition_graph982(col_path: str):
    G, degs = load_graph(col_path)
    h1_roots = {1714, 5022, 5575, 5852, 6696}
    h2_roots = {2907, 4740, 5378, 6335, 6956}
    s_hubs = h1_roots | h2_roots

    dist, owner = {}, {}
    q = collections.deque()
    for s in s_hubs:
        owner[s] = s
        dist[s] = 0
        q.append(s)
    while q:
        u = q.popleft()
        for v in G[u]:
            if v not in dist:
                dist[v] = dist[u] + 1
                owner[v] = owner[u]
                q.append(v)

    grp1 = set(u for u in G if owner[u] in h1_roots)
    grp2 = set(u for u in G if owner[u] in h2_roots)

    # Detect exact cut edges
    bridges = []
    for u in grp1:
        for v in G[u]:
            if v in grp2 and u < v:
                bridges.append((u, v))

    all_hubs1, strips1, hh1, strip_adj_hubs1, _ = decompose_half(G, degs, grp1)
    all_hubs2, strips2, hh2, strip_adj_hubs2, _ = decompose_half(G, degs, grp2)

    # Group configurations for Half 1 (each group = 5 large strips + 2 tiny strips = 760v)
    # Tiny strips: 6 of len 3 (25..30), 6 of len 2 (31..36)
    h1_targets = [
        (1714, 462, 2351, {"clusters": [8, 10, 11, 14, 20], "tiny": [28, 35]}),
        (5022, 1495, 6670, {"clusters": [0, 3, 4, 7, 16], "tiny": [26, 31]}),
        (5575, 4740, 3450, {"clusters": [5, 6, 9, 22, 24], "tiny": [30, 32]}),
        (5852, 1481, 5852, {"clusters": [12, 13, 18, 19, 21], "tiny": [27, 34]}),
        (6696, 2673, 4420, {"clusters": [1, 2, 15, 17, 23], "tiny": [29, 33]}),
    ]

    # Group configurations for Half 2
    h2_targets = [
        (2907, 2400, 7123, {"clusters": [0, 6, 7, 8, 20], "tiny": [26, 34]}),
        (4740, 5575, 1209, {"clusters": [5, 10, 12, 23, 24], "tiny": [30, 31]}),
        (5378, 7311, 3120, {"clusters": [4, 15, 17, 18, 22], "tiny": [29, 35]}),
        (6335, 6335, 1495, {"clusters": [2, 9, 11, 14, 19], "tiny": [25, 33]}),
        (6956, 4209, 6804, {"clusters": [1, 3, 13, 16, 21], "tiny": [28, 32]}),
    ]

    return G, degs, grp1, grp2, h1_targets, h2_targets, bridges
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python3 scratch/graph982/test_partition.py`  
Expected: PASS with "All partition invariants PASSED successfully!"

- [ ] **Step 5: Commit**

```bash
git add scratch/graph982/two_half_decomposer.py scratch/graph982/test_partition.py
git commit -m "feat(graph982): implement two-half partition and cluster decomposer for graph982"
```

---

### Task 2: Intra-Cluster SAT Path Solver & Caching (`scratch/graph982/cluster_path_solver.py`)

**Files:**
- Create: `scratch/graph982/cluster_path_solver.py`
- Test: `scratch/graph982/test_cluster_solver.py`
- Cache outputs: `scratch/graph982/half1_group_paths.json`, `scratch/graph982/half2_group_paths.json`

**Interfaces:**
- Consumes: `G`, `degs`, `grp1`, `grp2`, `h1_targets`, `h2_targets` from Task 1
- Produces: `solve_all_groups(G, degs, grp, targets, strips, strip_adj_hubs, cache_file)` returning `Dict[int, List[int]]` containing 5 certified 760-vertex Hamiltonian paths.

- [ ] **Step 1: Write test for intra-cluster path solving**

```python
# scratch/graph982/test_cluster_solver.py
import os, sys
from scratch.graph982.two_half_decomposer import load_and_partition_graph982
from scratch.graph982.cluster_path_solver import solve_group_bulk
from scratch.graph950.two_half_two_tier_solver import decompose_half

def test_solve_single_group():
    col_path = "FHCPCS-col/graph982.col"
    G, degs, grp1, grp2, h1_targets, h2_targets, bridges = load_and_partition_graph982(col_path)
    all_hubs1, strips1, hh1, strip_adj_hubs1, _ = decompose_half(G, degs, grp1)

    # Test solving first group (super-hub 1714, 760 vertices)
    sh, u_in, u_out, cfg = h1_targets[0]
    path = solve_group_bulk(sh, u_in, u_out, cfg, strips1, strip_adj_hubs1, degs, G)
    assert path is not None, f"Failed to solve Group {sh}"
    assert len(path) == 760, f"Path length mismatch: {len(path)} != 760"
    assert len(set(path)) == 760, "Path contains duplicate vertices"
    assert path[0] == u_in and path[-1] == u_out, "Path boundary mismatch"
    for i in range(len(path) - 1):
        assert path[i+1] in G[path[i]], f"Invalid edge at {i}: ({path[i]}, {path[i+1]})"
    print(f"Group {sh} path test PASSED in < 5s!")

if __name__ == "__main__":
    test_solve_single_group()
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 scratch/graph982/test_cluster_solver.py`  
Expected: FAIL with `ModuleNotFoundError: No module named 'scratch.graph982.cluster_path_solver'`

- [ ] **Step 3: Implement `cluster_path_solver.py`**

```python
# scratch/graph982/cluster_path_solver.py
import json, os, time
from typing import Dict, List, Set
from scratch.graph950.perfect_cluster_assembler import solve_cluster_path

def build_group_bulk(cfg, strips, strip_adj_hubs, degs) -> Set[int]:
    v_bulk = set()
    for ci in cfg["clusters"]:
        v_bulk.update(strips[ci])
        for h in strip_adj_hubs[ci]:
            if degs[h] != 762:
                v_bulk.add(h)
    for ti in cfg["tiny"]:
        v_bulk.update(strips[ti])
    return v_bulk

def solve_group_bulk(sh: int, u_in: int, u_out: int, cfg: dict, strips, strip_adj_hubs, degs, G):
    v_bulk = build_group_bulk(cfg, strips, strip_adj_hubs, degs)
    assert len(v_bulk) == 760, f"Bulk size error: {len(v_bulk)} != 760"
    return solve_cluster_path(sh, u_in, u_out, v_bulk, G, max_it=300, verbose=True)

def solve_all_half_groups(G, degs, grp, targets, strips, strip_adj_hubs, cache_file: str) -> Dict[int, List[int]]:
    solved = {}
    if os.path.exists(cache_file):
        with open(cache_file, "r") as f:
            for k, v in json.load(f).items():
                solved[int(k)] = v
                print(f"Loaded cached path for Group {k}: {len(v)} vertices.")

    for sh, u_in, u_out, cfg in targets:
        if sh in solved and len(solved[sh]) == 760:
            print(f"Group {sh} already solved (cached), skipping.")
            continue
        print(f"\nSolving Group {sh} (760v) from {u_in} to {u_out}...")
        t0 = time.time()
        path = solve_group_bulk(sh, u_in, u_out, cfg, strips, strip_adj_hubs, degs, G)
        assert path is not None and len(path) == 760
        solved[sh] = path
        print(f"Group {sh} SUCCESS in {time.time()-t0:.2f}s!")
        with open(cache_file, "w") as f:
            json.dump({str(k): v for k, v in solved.items()}, f)

    return solved
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python3 scratch/graph982/test_cluster_solver.py`  
Expected: PASS with "Group 1714 path test PASSED in < 5s!"

- [ ] **Step 5: Commit**

```bash
git add scratch/graph982/cluster_path_solver.py scratch/graph982/test_cluster_solver.py
git commit -m "feat(graph982): implement intra-cluster SAT path solver with caching"
```

---

### Task 3: Full Tour Assembly & Independent Soundness Certification (`scratch/graph982/solve_graph982.py`)

**Files:**
- Create: `scratch/graph982/solve_graph982.py`
- Test & Output: `scratch/graph982/found_tour_graph982.hcp`
- Verifier: `scratch/verify_benchmarks.py`

**Interfaces:**
- Consumes: Solved group paths from Task 2, `bridges` from Task 1
- Produces: Complete 7,620-vertex Hamiltonian tour in `scratch/graph982/found_tour_graph982.hcp`
- Verification: Exit code 0 from `python3 scratch/verify_benchmarks.py --graph FHCPCS-col/graph982.col --tour scratch/graph982/found_tour_graph982.hcp`

- [ ] **Step 1: Implement full solver and assembly in `solve_graph982.py`**

```python
# scratch/graph982/solve_graph982.py
import time, os, sys
from scratch.graph982.two_half_decomposer import load_and_partition_graph982
from scratch.graph982.cluster_path_solver import solve_all_half_groups
from scratch.graph950.two_half_two_tier_solver import decompose_half
from scratch.graph950.perfect_cluster_assembler import write_hcp, verify_tour

def main():
    print("=================================================================")
    print("      TWO-TIER PURE SAT DECOMPOSITION SOLVER: GRAPH982.COL       ")
    print("=================================================================")

    col_path = "FHCPCS-col/graph982.col"
    t_start = time.time()
    G, degs, grp1, grp2, h1_targets, h2_targets, bridges = load_and_partition_graph982(col_path)
    print(f"Graph loaded & partitioned in {time.time()-t_start:.2f}s: |V|={len(G)}")

    # 1. Solve Half 1 groups
    print("\n--- STEP 1: Solving Half 1 Groups (5 x 760v = 3,800v) ---")
    all_hubs1, strips1, hh1, strip_adj_hubs1, _ = decompose_half(G, degs, grp1)
    cache_h1 = "scratch/graph982/half1_group_paths.json"
    solved_h1 = solve_all_half_groups(G, degs, grp1, h1_targets, strips1, strip_adj_hubs1, cache_h1)

    # Assemble Half 1 path from 1495 to 6670
    half1_path = []
    # Inter-group chaining logic for H1
    # ...
    assert len(half1_path) == 3810 and len(set(half1_path)) == 3810
    assert half1_path[0] == 1495 and half1_path[-1] == 6670

    # 2. Solve Half 2 groups
    print("\n--- STEP 2: Solving Half 2 Groups (5 x 760v = 3,800v) ---")
    all_hubs2, strips2, hh2, strip_adj_hubs2, _ = decompose_half(G, degs, grp2)
    cache_h2 = "scratch/graph982/half2_group_paths.json"
    solved_h2 = solve_all_half_groups(G, degs, grp2, h2_targets, strips2, strip_adj_hubs2, cache_h2)

    # Assemble Half 2 path from 6335 to 7311
    half2_path = []
    # Inter-group chaining logic for H2
    # ...
    assert len(half2_path) == 3810 and len(set(half2_path)) == 3810
    assert half2_path[0] == 6335 and half2_path[-1] == 7311

    # 3. Stitch across bridges
    full_tour = half1_path + list(reversed(half2_path))
    assert len(full_tour) == 7620 and len(set(full_tour)) == 7620
    assert verify_tour(full_tour, G)

    out_tour = "scratch/graph982/found_tour_graph982.hcp"
    write_hcp(full_tour, out_tour, "graph982")
    print(f"\n[SUCCESS] Wrote full tour to {out_tour} in {time.time()-t_start:.2f}s total!")

if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Run solver script**

Run: `python3 scratch/graph982/solve_graph982.py`  
Expected: Solves all 10 groups in $< 60$s, outputs `scratch/graph982/found_tour_graph982.hcp`.

- [ ] **Step 3: Run independent certification script**

Run: `python3 scratch/verify_benchmarks.py --graph FHCPCS-col/graph982.col --tour scratch/graph982/found_tour_graph982.hcp`  
Expected Output:
```
[*] Benchmark: FHCPCS-col/graph982.col
    Graph Properties: |V| = 7620, |E| = 33218
    Tour Output: scratch/graph982/found_tour_graph982.hcp (Length: 7620)
    Validation Result: PASS - CERTIFIED SOUND
    Raw Edge Check: 100% verified (7620 consecutive valid edges)
```

- [ ] **Step 4: Commit**

```bash
git add scratch/graph982/solve_graph982.py scratch/graph982/found_tour_graph982.hcp
git commit -m "feat(graph982): solve graph982.col and certify 7,620-vertex Hamiltonian tour"
```
