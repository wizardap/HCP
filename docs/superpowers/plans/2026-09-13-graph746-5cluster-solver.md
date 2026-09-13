# 5-Cluster Macro-Ring Hierarchical Solver for graph746.col Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and execute a certified 5-cluster macro-ring hierarchical decomposition solver for `FHCPCS-col/graph746.col` ($N = 4,286, M = 18,286$), producing a 100% verified sound Hamiltonian cycle in `scratch/graph746/found_tour_graph746.hcp` in $\le 1,800$s without tour injection.

**Architecture:** 5-cluster symmetric decomposition into 855-vertex group bulks; intra-cluster CaDiCaL CEGAR subcycle elimination; multiprocessing Pool(4) parallel solving; deterministic 16-stage macro-ring assembly through 11 corridor nodes.

**Tech Stack:** Python 3, PySAT (CaDiCaL 1.9.5), multiprocessing.

## Global Constraints

- Target Graph: `FHCPCS-col/graph746.col` ($N = 4,286, M = 18,286$).
- Zero Tour Injection: Never read, preload, or inspect `.tou` reference files.
- Zero Phantom Edges: Every edge in the tour must exist in `G[u]` from raw `graph746.col`.
- Complete Tour Certification: Independent validation using `python3 scratch/verify_benchmarks.py --graph FHCPCS-col/graph746.col --tour scratch/graph746/found_tour_graph746.hcp` must report `Validation Result: PASS - CERTIFIED SOUND` with exit code 0.
- Runtime limit: $\le 1,800$s (target $< 3$ minutes wall-clock using 4-core parallel execution).

---

### Task 1: Graph Loading, Strip Extraction & 5-Cluster Group Decomposition

**Files:**
- Create: `scratch/graph746/cluster_decomposer.py`
- Test: `scratch/graph746/test_partition.py`

**Interfaces:**
- Consumes: `scratch/graph950/two_half_two_tier_solver.py:load_graph`
- Produces: `load_and_decompose_graph746(col_path: str) -> (G, degs, group_targets, macro_nodes, bulks)`
  where:
  - `group_targets` is a list of 5 tuples `(sh, u_in, u_out, cfg)`
  - `macro_nodes` is a set of 11 vertices: 5 super-hubs + 6 connector vertices
  - `bulks` is a dict mapping `sh -> set of 855 vertices`
  - All 5 bulks and `macro_nodes` form an exact partition of $V(G)$ (size 4,286)

- [ ] **Step 1: Write the failing test**

```python
# scratch/graph746/test_partition.py
import pytest
from scratch.graph746.cluster_decomposer import load_and_decompose_graph746

def test_partition_invariants():
    col_path = "FHCPCS-col/graph746.col"
    G, degs, group_targets, macro_nodes, bulks = load_and_decompose_graph746(col_path)

    assert len(G) == 4286
    assert len(group_targets) == 5
    assert len(macro_nodes) == 11

    # Super-hubs
    super_hubs = {1430, 3641, 3735, 3790, 3960}
    assert super_hubs.issubset(macro_nodes)

    # Check each group bulk
    all_bulk_nodes = set()
    for sh, u_in, u_out, cfg in group_targets:
        b = bulks[sh]
        assert len(b) == 855
        assert u_in in b and u_out in b and u_in != u_out
        assert not (b & macro_nodes), f"Group {sh} overlaps with macro nodes"
        assert not (b & all_bulk_nodes), f"Group {sh} overlaps with other bulks"
        all_bulk_nodes.update(b)

    assert len(all_bulk_nodes) == 5 * 855  # 4275
    assert all_bulk_nodes | macro_nodes == set(G.keys())
    assert len(all_bulk_nodes & macro_nodes) == 0

    # Check external connections of each group bulk
    port_pairs = {
        1430: (3003, 2623),
        3790: (2165, 1264),
        3960: (1025, 3498),
        3641: (3146, 2397),
        3735: (46, 3547),
    }
    for sh, u_in, u_out, _ in group_targets:
        expected_ports = set(port_pairs[sh])
        assert {u_in, u_out} == expected_ports
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pytest scratch/graph746/test_partition.py -v`
Expected: FAIL with `ModuleNotFoundError: No module named 'scratch.graph746.cluster_decomposer'`

- [ ] **Step 3: Write implementation**

```python
# scratch/graph746/cluster_decomposer.py
import collections
from typing import Dict, List, Set, Tuple, Any
from scratch.graph950.two_half_two_tier_solver import load_graph

def load_and_decompose_graph746(col_path: str):
    G, degs = load_graph(col_path)
    super_hubs = sorted([u for u in degs if degs[u] >= 500])
    assert super_hubs == [1430, 3641, 3735, 3790, 3960]

    hubs = set(u for u in degs if degs[u] >= 16)
    bulk = set(G.keys()) - hubs

    adj_bulk = {u: G[u] & bulk for u in bulk}
    visited = set()
    strips = []
    for u in bulk:
        if u not in visited:
            c = []
            q = [u]
            visited.add(u)
            for x in q:
                c.append(x)
                for y in adj_bulk[x]:
                    if y not in visited:
                        visited.add(y)
                        q.append(y)
            strips.append(c)
    strips.sort(key=len, reverse=True)

    sh_cfgs = {
        1430: {"large": [3, 8, 9, 13, 16], "med": [27, 29, 30, 34, 46], "tiny": [50, 57]},
        3641: {"large": [5, 11, 12, 15, 24], "med": [25, 32, 37, 44, 45], "tiny": [53, 56]},
        3735: {"large": [0, 2, 20, 21, 22], "med": [26, 28, 35, 36, 41], "tiny": [52, 58]},
        3790: {"large": [4, 10, 14, 17, 18], "med": [31, 33, 39, 43, 47], "tiny": [51, 59]},
        3960: {"large": [1, 6, 7, 19, 23], "med": [38, 40, 42, 48, 49], "tiny": [54, 61]}
    }

    strip_adj_hubs = collections.defaultdict(set)
    for si, s in enumerate(strips):
        for u in s:
            for nbr in G[u]:
                if nbr in hubs:
                    strip_adj_hubs[si].add(nbr)

    bulks = {}
    for sh, cfg in sh_cfgs.items():
        v = set()
        for si in cfg["large"] + cfg["med"]:
            v.update(strips[si])
            for h in strip_adj_hubs[si]:
                if degs[h] < 500:
                    v.add(h)
        for ti in cfg["tiny"]:
            v.update(strips[ti])
        bulks[sh] = v

    macro_nodes = set(G.keys()) - set().union(*bulks.values())
    assert len(macro_nodes) == 11

    group_targets = [
        (1430, 3003, 2623, sh_cfgs[1430]),
        (3790, 2165, 1264, sh_cfgs[3790]),
        (3960, 1025, 3498, sh_cfgs[3960]),
        (3641, 3146, 2397, sh_cfgs[3641]),
        (3735, 46, 3547, sh_cfgs[3735]),
    ]

    return G, degs, group_targets, macro_nodes, bulks
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pytest scratch/graph746/test_partition.py -v`
Expected: PASS (all partition invariants hold)

- [ ] **Step 5: Commit**

```bash
git add scratch/graph746/cluster_decomposer.py scratch/graph746/test_partition.py
git commit -m "feat(graph746): add cluster decomposer and partition verification test"
```

---

### Task 2: Intra-Cluster SAT Solver & 4-Core Parallel Execution

**Files:**
- Create: `scratch/graph746/cluster_path_solver.py`
- Create: `scratch/graph746/test_cluster_solver.py`
- Create: `scratch/graph746/solve_all_parallel.py`
- Output: `scratch/graph746/group_paths.json`

**Interfaces:**
- Consumes: `load_and_decompose_graph746`, `solve_cluster_path`
- Produces: JSON cache `scratch/graph746/group_paths.json` containing 5 paths of length 855 each.

- [ ] **Step 1: Write the failing test**

```python
# scratch/graph746/test_cluster_solver.py
import json, os, pytest
from scratch.graph746.cluster_decomposer import load_and_decompose_graph746

def test_cached_paths_valid():
    base_dir = os.path.dirname(os.path.abspath(__file__))
    cache_file = os.path.join(base_dir, "group_paths.json")

    assert os.path.exists(cache_file), "group_paths.json missing"

    col_path = "FHCPCS-col/graph746.col"
    G, _, group_targets, macro_nodes, bulks = load_and_decompose_graph746(col_path)

    with open(cache_file, "r") as f:
        paths = {int(k): v for k, v in json.load(f).items()}

    assert len(paths) == 5
    all_path_nodes = set()
    for sh, u_in, u_out, _ in group_targets:
        assert sh in paths
        path = paths[sh]
        assert len(path) == 855
        assert len(set(path)) == 855
        assert path[0] == u_in
        assert path[-1] == u_out
        assert set(path) == bulks[sh]
        assert not (set(path) & macro_nodes)
        for i in range(len(path) - 1):
            assert path[i+1] in G[path[i]]
        all_path_nodes.update(path)

    assert len(all_path_nodes) == 5 * 855
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pytest scratch/graph746/test_cluster_solver.py -v`
Expected: FAIL with missing JSON cache.

- [ ] **Step 3: Implement cluster solver and parallel pool solver**

Create `scratch/graph746/cluster_path_solver.py`:
```python
import json, os, sys, time
from typing import Dict, List, Set, Optional

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
from scratch.graph950.perfect_cluster_assembler import solve_cluster_path
from scratch.graph746.cluster_decomposer import load_and_decompose_graph746

def solve_all_groups(G, degs, group_targets, bulks, cache_file: str, max_it: int = 300) -> Dict[int, List[int]]:
    solved = {}
    if os.path.exists(cache_file):
        with open(cache_file, "r") as f:
            for k, v in json.load(f).items():
                solved[int(k)] = v
                print(f"Loaded cached path for Group {k}: {len(v)} vertices.")

    for sh, u_in, u_out, cfg in group_targets:
        if sh in solved and len(solved[sh]) == 855 and solved[sh][0] == u_in and solved[sh][-1] == u_out:
            print(f"Group {sh} already solved with matching endpoints (cached), skipping.")
            continue
        v_bulk = bulks[sh]
        assert len(v_bulk) == 855
        print(f"\nSolving Group {sh} (855v) from {u_in} to {u_out}...")
        t0 = time.time()
        path = solve_cluster_path(sh, u_in, u_out, v_bulk, G, max_it=max_it, verbose=True)
        assert path is not None and len(path) == 855, f"Failed to solve Group {sh}"
        solved[sh] = path
        print(f"Group {sh} SUCCESS in {time.time()-t0:.2f}s!")
        os.makedirs(os.path.dirname(os.path.abspath(cache_file)), exist_ok=True)
        with open(cache_file, "w") as f:
            json.dump({str(k): v for k, v in solved.items()}, f)

    return solved
```

Create `scratch/graph746/solve_all_parallel.py`:
```python
import os, sys, time, json, multiprocessing
from scratch.graph746.cluster_decomposer import load_and_decompose_graph746
from scratch.graph950.perfect_cluster_assembler import solve_cluster_path

def worker_solve_group(item):
    sh, u_in, u_out, v_bulk, edges = item
    print(f"[Worker] Solving Group {sh} ({len(v_bulk)}v) from {u_in} to {u_out}...")
    t0 = time.time()
    local_G = {u: set(nbrs) for u, nbrs in edges.items()}
    path = solve_cluster_path(sh, u_in, u_out, set(v_bulk), local_G, max_it=300, verbose=True)
    dt = time.time() - t0
    print(f"[Worker] Group {sh} FINISHED in {dt:.2f}s (len={len(path) if path else None})")
    return sh, path

def main():
    col_path = "FHCPCS-col/graph746.col"
    print("Loading graph746.col...")
    G, degs, group_targets, macro_nodes, bulks = load_and_decompose_graph746(col_path)

    base_dir = os.path.dirname(os.path.abspath(__file__))
    cache_file = os.path.join(base_dir, "group_paths.json")

    cached = {}
    if os.path.exists(cache_file):
        with open(cache_file, "r") as f:
            cached = {int(k): v for k, v in json.load(f).items()}

    tasks = []
    for sh, u_in, u_out, cfg in group_targets:
        if sh in cached and len(cached[sh]) == 855 and cached[sh][0] == u_in and cached[sh][-1] == u_out:
            print(f"Group {sh} already solved, skipping.")
            continue
        v_bulk = bulks[sh]
        edges = {u: list(G[u] & v_bulk) for u in v_bulk}
        tasks.append((sh, u_in, u_out, list(v_bulk), edges))

    print(f"\nDispatching {len(tasks)} tasks to multiprocessing Pool(4)...")
    if tasks:
        with multiprocessing.Pool(processes=min(4, os.cpu_count() or 4)) as pool:
            for sh, path in pool.imap_unordered(worker_solve_group, tasks):
                assert path is not None and len(path) == 855
                cached[sh] = path
                with open(cache_file, "w") as f:
                    json.dump({str(k): v for k, v in cached.items()}, f)

    print("All 5 groups solved and cached successfully!")

if __name__ == "__main__":
    main()
```

- [ ] **Step 4: Run parallel solver and verify tests pass**

Run: `python3 scratch/graph746/solve_all_parallel.py`
Run: `pytest scratch/graph746/test_cluster_solver.py -v`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add scratch/graph746/cluster_path_solver.py scratch/graph746/solve_all_parallel.py scratch/graph746/test_cluster_solver.py scratch/graph746/group_paths.json
git commit -m "feat(graph746): solve and cache all 5 group paths via parallel CaDiCaL CEGAR"
```

---

### Task 3: Full Tour Assembly & Independent Soundness Certification

**Files:**
- Create: `scratch/graph746/solve_graph746.py`
- Output: `scratch/graph746/found_tour_graph746.hcp`

**Interfaces:**
- Consumes: `load_and_decompose_graph746`, `group_paths.json`
- Produces: `scratch/graph746/found_tour_graph746.hcp`
- Verification: `python3 scratch/verify_benchmarks.py --graph FHCPCS-col/graph746.col --tour scratch/graph746/found_tour_graph746.hcp`

- [ ] **Step 1: Write minimal implementation of `solve_graph746.py`**

```python
#!/usr/bin/env python3
"""
5-Cluster Macro-Ring Hierarchical Decomposition Solver for graph746.col (N=4,286, M=18,286)
"""

import time, os, sys, json
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
from scratch.graph746.cluster_decomposer import load_and_decompose_graph746
from scratch.graph746.cluster_path_solver import solve_all_groups
from scratch.graph950.perfect_cluster_assembler import verify_tour, write_hcp

def main():
    print("=================================================================")
    print("      5-CLUSTER PURE SAT DECOMPOSITION SOLVER: GRAPH746.COL      ")
    print("=================================================================")

    col_path = "FHCPCS-col/graph746.col"
    t_start = time.time()
    G, degs, group_targets, macro_nodes, bulks = load_and_decompose_graph746(col_path)
    print(f"Graph loaded & decomposed in {time.time()-t_start:.2f}s: |V|={len(G)}")

    base_dir = os.path.dirname(os.path.abspath(__file__))
    cache_file = os.path.join(base_dir, "group_paths.json")

    print("\n--- STEP 1: Solving/Loading 5 Groups (5 x 855v = 4,275v) ---")
    solved = solve_all_groups(G, degs, group_targets, bulks, cache_file)

    print("\n--- STEP 2: Assembling 16-Stage Macro Hamiltonian Tour ---")
    # Traversal sequence:
    # 1430 -> 3566 -> G_1430(3003 -> 2623) -> G_3790(2165 -> 1264) -> 3692 -> G_3960(1025 -> 3498)
    # -> 3106 -> 3960 -> 3735 -> 2433 -> 3790 -> G_3641(3146 -> 2397) -> 2361 -> 3641 -> 1321
    # -> G_3735(46 -> 3547) -> (closes to 1430)

    full_tour = [1430, 3566]
    full_tour.extend(solved[1430])   # 3003 -> 2623
    full_tour.extend(solved[3790])   # 2165 -> 1264
    full_tour.append(3692)
    full_tour.extend(solved[3960])   # 1025 -> 3498
    full_tour.extend([3106, 3960, 3735, 2433, 3790])
    full_tour.extend(solved[3641])   # 3146 -> 2397
    full_tour.extend([2361, 3641, 1321])
    full_tour.extend(solved[3735])   # 46 -> 3547

    print(f"Assembled Tour length: {len(full_tour)} (expected 4,286)")
    assert len(full_tour) == 4286, f"Length mismatch: {len(full_tour)} != 4286"
    assert len(set(full_tour)) == 4286, "Tour contains duplicate vertices"
    assert set(full_tour) == set(G.keys()), "Tour does not cover all vertices of G"

    # Step 3: Independent Verification on Raw Graph
    print("\n--- STEP 3: Independent Verification on Raw Graph ---")
    assert verify_tour(full_tour, G), "VERIFICATION FAILED!"
    print("*****************************************************************")
    print("*** 100.000% MATHEMATICALLY CERTIFIED HAMILTONIAN CYCLE PASS! ***")
    print("*****************************************************************")

    out_tour = os.path.join(base_dir, "found_tour_graph746.hcp")
    write_hcp(full_tour, out_tour)
    print(f"Tour written to {out_tour}")
    print(f"Total time elapsed: {time.time()-t_start:.2f}s")

if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Run solver script**

Run: `python3 scratch/graph746/solve_graph746.py`
Expected: Outputs `scratch/graph746/found_tour_graph746.hcp` and prints `100.000% MATHEMATICALLY CERTIFIED HAMILTONIAN CYCLE PASS!`.

- [ ] **Step 3: Run independent verification suite**

Run: `python3 scratch/verify_benchmarks.py --graph FHCPCS-col/graph746.col --tour scratch/graph746/found_tour_graph746.hcp`
Expected: Output `Validation Result: PASS - CERTIFIED SOUND`, exit code 0.

- [ ] **Step 4: Commit**

```bash
git add scratch/graph746/solve_graph746.py scratch/graph746/found_tour_graph746.hcp
git commit -m "feat(graph746): full tour assembly and independent certification pass"
```
