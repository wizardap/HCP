# Two-Half Two-Tier Hierarchical Decomposition Solver for graph990.col Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and execute a certified two-half two-tier hierarchical decomposition solver for `FHCPCS-col/graph990.col` ($N = 8,020, M = 35,018$), producing a 100% verified sound Hamiltonian cycle in `scratch/graph990/found_tour_graph990.hcp` in $\le 1,800$s without tour injection.

**Architecture:** Voronoi BFS 2-bridge cut partition into two 4,010-vertex halves; intra-cluster decomposition into 10 parallel 800-vertex group bulks; incremental CaDiCaL CEGAR subcycle elimination; deterministic macro-chain assembly across bridges $e_1 = (3517, 7858)$ and $e_2 = (3728, 4178)$.

**Tech Stack:** Python 3, PySAT (CaDiCaL 1.9.5), multiprocessing.

## Global Constraints

- Target Graph: `FHCPCS-col/graph990.col` ($N = 8,020, M = 35,018$).
- Zero Tour Injection: Never read, preload, or inspect `.tou` reference files.
- Zero Phantom Edges: Every edge in the tour must exist in `G[u]` from raw `graph990.col`.
- Complete Tour Certification: Independent validation using `python3 scratch/verify_benchmarks.py --graph FHCPCS-col/graph990.col --tour scratch/graph990/found_tour_graph990.hcp` must report `Validation Result: PASS - CERTIFIED SOUND` with exit code 0.
- Runtime limit: $\le 1,800$s (target $< 10$ minutes wall-clock using 4-core parallel execution).

---

### Task 1: Graph Loading, Voronoi Partitioning & Group Decomposition

**Files:**
- Create: `scratch/graph990/two_half_decomposer.py`
- Test: `scratch/graph990/test_partition.py`

**Interfaces:**
- Consumes: `scratch/graph950/two_half_two_tier_solver.py:load_graph`, `decompose_half`
- Produces: `load_and_partition_graph990(col_path: str) -> (G, degs, grp1, grp2, h1_targets, h2_targets, bridges)`
  where:
  - `grp1` and `grp2` are sets of 4,010 vertices each
  - `bridges` contains exactly `[(3517, 7858), (3728, 4178)]`
  - `h1_targets` contains 5 tuples `(sh, u_in, u_out, cfg)`
  - `h2_targets` contains 5 tuples `(sh, u_in, u_out, cfg)`

- [ ] **Step 1: Write the failing test**

```python
# scratch/graph990/test_partition.py
import pytest
from scratch.graph990.two_half_decomposer import load_and_partition_graph990
from scratch.graph950.two_half_two_tier_solver import decompose_half

def test_partition_invariants():
    col_path = "FHCPCS-col/graph990.col"
    G, degs, grp1, grp2, h1_targets, h2_targets, bridges = load_and_partition_graph990(col_path)

    assert len(G) == 8020
    assert len(grp1) == 4010
    assert len(grp2) == 4010
    assert len(grp1 & grp2) == 0
    assert len(grp1 | grp2) == 8020

    # Bridges
    assert sorted(bridges) == [(3517, 7858), (3728, 4178)]

    # Half 1 Decomposition & Targets
    all_hubs1, strips1, _, strip_adj_hubs1, _ = decompose_half(G, degs, grp1)
    assert len(strips1) == 37
    for sh, u_in, u_out, cfg in h1_targets:
        v_bulk = set()
        for ci in cfg["clusters"]:
            v_bulk.update(strips1[ci])
            for h in strip_adj_hubs1[ci]:
                if degs[h] != 802:
                    v_bulk.add(h)
        for ti in cfg["tiny"]:
            v_bulk.update(strips1[ti])
        assert len(v_bulk) == 800
        assert u_in in v_bulk and u_out in v_bulk and u_in != u_out

    # Half 2 Decomposition & Targets
    all_hubs2, strips2, _, strip_adj_hubs2, _ = decompose_half(G, degs, grp2)
    assert len(strips2) == 37
    for sh, u_in, u_out, cfg in h2_targets:
        v_bulk = set()
        for ci in cfg["clusters"]:
            v_bulk.update(strips2[ci])
            for h in strip_adj_hubs2[ci]:
                if degs[h] != 802:
                    v_bulk.add(h)
        for ti in cfg["tiny"]:
            v_bulk.update(strips2[ti])
        assert len(v_bulk) == 800
        assert u_in in v_bulk and u_out in v_bulk and u_in != u_out
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pytest scratch/graph990/test_partition.py -v`
Expected: FAIL with `ModuleNotFoundError: No module named 'scratch.graph990.two_half_decomposer'`

- [ ] **Step 3: Write implementation**

```python
# scratch/graph990/two_half_decomposer.py
import collections
from typing import Dict, List, Set, Tuple, Any
from scratch.graph950.two_half_two_tier_solver import load_graph

def load_and_partition_graph990(col_path: str):
    G, degs = load_graph(col_path)
    h1_roots = {3076, 3517, 3728, 5293, 6726}
    h2_roots = {2205, 3905, 4178, 7717, 7858}
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

    # Detect cut edges
    bridges = []
    for u in grp1:
        for v in G[u]:
            if v in grp2:
                bridges.append((u, v) if u < v else (v, u))
    bridges = sorted(list(set(bridges)))

    # Group targets for Half 1 (5 groups x 800v)
    h1_targets = [
        (5293, 7644, 5381, {"clusters": [5, 6, 11, 16, 18], "tiny": [26, 35]}),
        (3728, 3862, 3071, {"clusters": [4, 7, 9, 17, 23], "tiny": [27, 33]}),
        (3076, 3494, 4316, {"clusters": [8, 10, 14, 15, 24], "tiny": [29, 34]}),
        (3517, 3455, 3729, {"clusters": [0, 2, 3, 13, 22], "tiny": [30, 31]}),
        (6726, 3391, 6248, {"clusters": [1, 12, 19, 20, 21], "tiny": [28, 36]}),
    ]

    # Group targets for Half 2 (5 groups x 800v)
    h2_targets = [
        (2205, 304, 1029, {"clusters": [2, 9, 10, 17, 20], "tiny": [29, 31]}),
        (7858, 1272, 6331, {"clusters": [1, 4, 5, 7, 12], "tiny": [28, 34]}),
        (3905, 4011, 5747, {"clusters": [8, 13, 16, 23, 24], "tiny": [25, 36]}),
        (4178, 340, 4340, {"clusters": [0, 11, 15, 19, 22], "tiny": [27, 32]}),
        (7717, 1121, 7436, {"clusters": [3, 6, 14, 18, 21], "tiny": [30, 35]}),
    ]

    return G, degs, grp1, grp2, h1_targets, h2_targets, bridges
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pytest scratch/graph990/test_partition.py -v`
Expected: PASS (all invariants hold)

- [ ] **Step 5: Commit**

```bash
git add scratch/graph990/two_half_decomposer.py scratch/graph990/test_partition.py
git commit -m "feat(graph990): add two-half decomposer and partition verification test"
```

---

### Task 2: Intra-Cluster SAT Solver & 4-Core Parallel Execution

**Files:**
- Create: `scratch/graph990/cluster_path_solver.py`
- Create: `scratch/graph990/test_cluster_solver.py`
- Create: `scratch/graph990/solve_all_parallel.py`
- Output: `scratch/graph990/half1_group_paths.json`, `scratch/graph990/half2_group_paths.json`

**Interfaces:**
- Consumes: `load_and_partition_graph990`, `solve_cluster_path`
- Produces: JSON caches `scratch/graph990/half1_group_paths.json` and `scratch/graph990/half2_group_paths.json` containing 5 paths of length 800 each.

- [ ] **Step 1: Write the failing test**

```python
# scratch/graph990/test_cluster_solver.py
import json, os, pytest
from scratch.graph990.two_half_decomposer import load_and_partition_graph990

def test_cached_paths_valid():
    base_dir = os.path.dirname(os.path.abspath(__file__))
    h1_cache = os.path.join(base_dir, "half1_group_paths.json")
    h2_cache = os.path.join(base_dir, "half2_group_paths.json")

    assert os.path.exists(h1_cache), "half1_group_paths.json missing"
    assert os.path.exists(h2_cache), "half2_group_paths.json missing"

    col_path = "FHCPCS-col/graph990.col"
    G, _, _, _, h1_targets, h2_targets, _ = load_and_partition_graph990(col_path)

    with open(h1_cache, "r") as f:
        p1 = {int(k): v for k, v in json.load(f).items()}
    assert len(p1) == 5
    for sh, u_in, u_out, _ in h1_targets:
        assert sh in p1
        path = p1[sh]
        assert len(path) == 800
        assert len(set(path)) == 800
        assert path[0] == u_in
        assert path[-1] == u_out
        for i in range(len(path) - 1):
            assert path[i+1] in G[path[i]]

    with open(h2_cache, "r") as f:
        p2 = {int(k): v for k, v in json.load(f).items()}
    assert len(p2) == 5
    for sh, u_in, u_out, _ in h2_targets:
        assert sh in p2
        path = p2[sh]
        assert len(path) == 800
        assert len(set(path)) == 800
        assert path[0] == u_in
        assert path[-1] == u_out
        for i in range(len(path) - 1):
            assert path[i+1] in G[path[i]]
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pytest scratch/graph990/test_cluster_solver.py -v`
Expected: FAIL with missing JSON caches.

- [ ] **Step 3: Implement cluster solver and parallel pool solver**

Create `scratch/graph990/cluster_path_solver.py`:
```python
import json, os, sys, time
from typing import Dict, List, Set, Optional

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
from scratch.graph950.perfect_cluster_assembler import solve_cluster_path

def build_group_bulk(cfg: dict, strips, strip_adj_hubs, degs) -> Set[int]:
    v_bulk = set()
    for ci in cfg["clusters"]:
        v_bulk.update(strips[ci])
        for h in strip_adj_hubs[ci]:
            if degs[h] != 802:
                v_bulk.add(h)
    for ti in cfg["tiny"]:
        v_bulk.update(strips[ti])
    return v_bulk

def solve_group_bulk(sh: int, u_in: int, u_out: int, cfg: dict, strips, strip_adj_hubs, degs, G, max_it: int = 300) -> Optional[List[int]]:
    v_bulk = build_group_bulk(cfg, strips, strip_adj_hubs, degs)
    assert len(v_bulk) == 800, f"Bulk size error: {len(v_bulk)} != 800"
    return solve_cluster_path(sh, u_in, u_out, v_bulk, G, max_it=max_it, verbose=True)

def solve_all_half_groups(G, degs, grp, targets, strips, strip_adj_hubs, cache_file: str, max_it: int = 300) -> Dict[int, List[int]]:
    solved = {}
    if os.path.exists(cache_file):
        with open(cache_file, "r") as f:
            for k, v in json.load(f).items():
                solved[int(k)] = v
                print(f"Loaded cached path for Group {k}: {len(v)} vertices.")

    for sh, u_in, u_out, cfg in targets:
        if sh in solved and len(solved[sh]) == 800 and solved[sh][0] == u_in and solved[sh][-1] == u_out:
            print(f"Group {sh} already solved with matching endpoints (cached), skipping.")
            continue
        print(f"\nSolving Group {sh} (800v) from {u_in} to {u_out}...")
        t0 = time.time()
        path = solve_group_bulk(sh, u_in, u_out, cfg, strips, strip_adj_hubs, degs, G, max_it=max_it)
        assert path is not None and len(path) == 800, f"Failed to solve Group {sh}"
        solved[sh] = path
        print(f"Group {sh} SUCCESS in {time.time()-t0:.2f}s!")
        os.makedirs(os.path.dirname(os.path.abspath(cache_file)), exist_ok=True)
        with open(cache_file, "w") as f:
            json.dump({str(k): v for k, v in solved.items()}, f)

    return solved
```

Create `scratch/graph990/solve_all_parallel.py`:
```python
import os, sys, time, json, multiprocessing
from scratch.graph990.two_half_decomposer import load_and_partition_graph990
from scratch.graph950.two_half_two_tier_solver import decompose_half
from scratch.graph990.cluster_path_solver import build_group_bulk
from scratch.graph950.perfect_cluster_assembler import solve_cluster_path

def worker_solve_group(item):
    sh, u_in, u_out, v_bulk, edges, half_num = item
    print(f"[Worker Half {half_num}] Solving Group {sh} ({len(v_bulk)}v) from {u_in} to {u_out}...")
    t0 = time.time()
    # Reconstruct local G
    local_G = {u: set(nbrs) for u, nbrs in edges.items()}
    path = solve_cluster_path(sh, u_in, u_out, set(v_bulk), local_G, max_it=300, verbose=True)
    dt = time.time() - t0
    print(f"[Worker Half {half_num}] Group {sh} FINISHED in {dt:.2f}s (len={len(path) if path else None})")
    return half_num, sh, path

def main():
    col_path = "FHCPCS-col/graph990.col"
    print("Loading graph990.col...")
    G, degs, grp1, grp2, h1_targets, h2_targets, _ = load_and_partition_graph990(col_path)

    base_dir = os.path.dirname(os.path.abspath(__file__))
    cache_h1 = os.path.join(base_dir, "half1_group_paths.json")
    cache_h2 = os.path.join(base_dir, "half2_group_paths.json")

    cached_h1 = {}
    if os.path.exists(cache_h1):
        with open(cache_h1, "r") as f:
            cached_h1 = {int(k): v for k, v in json.load(f).items()}

    cached_h2 = {}
    if os.path.exists(cache_h2):
        with open(cache_h2, "r") as f:
            cached_h2 = {int(k): v for k, v in json.load(f).items()}

    all_hubs1, strips1, _, strip_adj_hubs1, _ = decompose_half(G, degs, grp1)
    all_hubs2, strips2, _, strip_adj_hubs2, _ = decompose_half(G, degs, grp2)

    tasks = []
    # Half 1 tasks
    for sh, u_in, u_out, cfg in h1_targets:
        if sh in cached_h1 and len(cached_h1[sh]) == 800 and cached_h1[sh][0] == u_in and cached_h1[sh][-1] == u_out:
            print(f"[Half 1] Group {sh} already solved, skipping.")
            continue
        v_bulk = build_group_bulk(cfg, strips1, strip_adj_hubs1, degs)
        edges = {u: list(G[u] & v_bulk) for u in v_bulk}
        tasks.append((sh, u_in, u_out, list(v_bulk), edges, 1))

    # Half 2 tasks
    for sh, u_in, u_out, cfg in h2_targets:
        if sh in cached_h2 and len(cached_h2[sh]) == 800 and cached_h2[sh][0] == u_in and cached_h2[sh][-1] == u_out:
            print(f"[Half 2] Group {sh} already solved, skipping.")
            continue
        v_bulk = build_group_bulk(cfg, strips2, strip_adj_hubs2, degs)
        edges = {u: list(G[u] & v_bulk) for u in v_bulk}
        tasks.append((sh, u_in, u_out, list(v_bulk), edges, 2))

    print(f"\nDispatching {len(tasks)} tasks to multiprocessing Pool(4)...")
    if tasks:
        with multiprocessing.Pool(processes=min(4, os.cpu_count() or 4)) as pool:
            results = pool.map(worker_solve_group, tasks)
            for half_num, sh, path in results:
                assert path is not None and len(path) == 800
                if half_num == 1:
                    cached_h1[sh] = path
                else:
                    cached_h2[sh] = path

        with open(cache_h1, "w") as f:
            json.dump({str(k): v for k, v in cached_h1.items()}, f)
        with open(cache_h2, "w") as f:
            json.dump({str(k): v for k, v in cached_h2.items()}, f)

    print("All 10 groups solved and cached successfully!")

if __name__ == "__main__":
    main()
```

- [ ] **Step 4: Run parallel solver and verify tests pass**

Run: `python3 scratch/graph990/solve_all_parallel.py`
Run: `pytest scratch/graph990/test_cluster_solver.py -v`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add scratch/graph990/cluster_path_solver.py scratch/graph990/solve_all_parallel.py scratch/graph990/test_cluster_solver.py scratch/graph990/half1_group_paths.json scratch/graph990/half2_group_paths.json
git commit -m "feat(graph990): solve and cache all 10 group paths via parallel CaDiCaL CEGAR"
```

---

### Task 3: Full Tour Assembly & Independent Soundness Certification

**Files:**
- Create: `scratch/graph990/solve_graph990.py`
- Output: `scratch/graph990/found_tour_graph990.hcp`

**Interfaces:**
- Consumes: `load_and_partition_graph990`, `half1_group_paths.json`, `half2_group_paths.json`
- Produces: `scratch/graph990/found_tour_graph990.hcp`
- Verification: `python3 scratch/verify_benchmarks.py --graph FHCPCS-col/graph990.col --tour scratch/graph990/found_tour_graph990.hcp`

- [ ] **Step 1: Write minimal implementation of `solve_graph990.py`**

```python
#!/usr/bin/env python3
"""
Two-Half Two-Tier Hierarchical Decomposition Solver for graph990.col (N=8,020, M=35,018)
"""

import time, os, sys, json
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
from scratch.graph990.two_half_decomposer import load_and_partition_graph990
from scratch.graph990.cluster_path_solver import solve_all_half_groups
from scratch.graph950.two_half_two_tier_solver import decompose_half
from scratch.graph950.perfect_cluster_assembler import verify_tour, write_hcp

def main():
    print("=================================================================")
    print("      TWO-TIER PURE SAT DECOMPOSITION SOLVER: GRAPH990.COL       ")
    print("=================================================================")

    col_path = "FHCPCS-col/graph990.col"
    t_start = time.time()
    G, degs, grp1, grp2, h1_targets, h2_targets, bridges = load_and_partition_graph990(col_path)
    print(f"Graph loaded & partitioned in {time.time()-t_start:.2f}s: |V|={len(G)}")

    base_dir = os.path.dirname(os.path.abspath(__file__))
    cache_h1 = os.path.join(base_dir, "half1_group_paths.json")
    cache_h2 = os.path.join(base_dir, "half2_group_paths.json")

    # 1. Load / Solve Half 1 groups
    print("\n--- STEP 1: Solving/Loading Half 1 Groups (5 x 800v = 4,000v) ---")
    all_hubs1, strips1, _, strip_adj_hubs1, _ = decompose_half(G, degs, grp1)
    solved_h1 = solve_all_half_groups(G, degs, grp1, h1_targets, strips1, strip_adj_hubs1, cache_h1)

    # Assemble Half 1: 3517 -> 3728 (4,010 vertices)
    half1_path = [3517]
    half1_path.extend(solved_h1[5293])    # 7644 -> 5381
    half1_path.extend([4974, 5293, 5263])
    half1_path.extend(solved_h1[3728])    # 3862 -> 3071
    half1_path.extend([3076, 78])
    half1_path.extend(solved_h1[3076])    # 3494 -> 4316
    half1_path.extend(solved_h1[3517])    # 3455 -> 3729
    half1_path.append(1225)
    half1_path.extend(solved_h1[6726])    # 3391 -> 6248
    half1_path.extend([2062, 6726, 3728])

    print(f"Half 1 assembled: {len(half1_path)} vertices. Start={half1_path[0]}, End={half1_path[-1]}")
    assert len(half1_path) == 4010, f"Half 1 length mismatch: {len(half1_path)} != 4010"
    assert len(set(half1_path)) == 4010, "Half 1 contains duplicate vertices"
    assert set(half1_path) == grp1, "Half 1 does not cover grp1 exactly"
    for i in range(len(half1_path) - 1):
        u, v = half1_path[i], half1_path[i+1]
        assert v in G[u], f"Half 1 invalid edge at {i}: ({u}, {v})"
    print(">>> HALF 1 100% CERTIFIED! (4,010 vertices from 3517 to 3728) <<<")

    # 2. Load / Solve Half 2 groups
    print("\n--- STEP 2: Solving/Loading Half 2 Groups (5 x 800v = 4,000v) ---")
    all_hubs2, strips2, _, strip_adj_hubs2, _ = decompose_half(G, degs, grp2)
    solved_h2 = solve_all_half_groups(G, degs, grp2, h2_targets, strips2, strip_adj_hubs2, cache_h2)

    # Assemble Half 2: 4178 -> 7858 (4,010 vertices)
    half2_path = [4178]
    half2_path.extend(solved_h2[2205])    # 304 -> 1029
    half2_path.extend([6262, 2205, 3950])
    half2_path.extend(solved_h2[7858])    # 1272 -> 6331
    half2_path.extend([3905, 6068])
    half2_path.extend(solved_h2[3905])    # 4011 -> 5747
    half2_path.extend(solved_h2[4178])    # 340 -> 4340
    half2_path.append(1040)
    half2_path.extend(solved_h2[7717])    # 1121 -> 7436
    half2_path.extend([567, 7717, 7858])

    print(f"Half 2 assembled: {len(half2_path)} vertices. Start={half2_path[0]}, End={half2_path[-1]}")
    assert len(half2_path) == 4010, f"Half 2 length mismatch: {len(half2_path)} != 4010"
    assert len(set(half2_path)) == 4010, "Half 2 contains duplicate vertices"
    assert set(half2_path) == grp2, "Half 2 does not cover grp2 exactly"
    for i in range(len(half2_path) - 1):
        u, v = half2_path[i], half2_path[i+1]
        assert v in G[u], f"Half 2 invalid edge at {i}: ({u}, {v})"
    print(">>> HALF 2 100% CERTIFIED! (4,010 vertices from 4178 to 7858) <<<")

    # 3. Stitch across bridges
    print("\n--- STEP 3: Stitching Half 1 and Half 2 across Bridge Edges ---")
    assert half2_path[0] in G[half1_path[-1]], f"Bridge edge ({half1_path[-1]}, {half2_path[0]}) missing!"
    assert half1_path[0] in G[half2_path[-1]], f"Bridge edge ({half2_path[-1]}, {half1_path[0]}) missing!"

    full_tour = half1_path + half2_path
    print(f"Stitched Tour length: {len(full_tour)} (expected 8,020)")

    # 4. Independent Verification on Raw Graph
    print("\n--- STEP 4: Independent Verification on Raw Graph ---")
    assert verify_tour(full_tour, G), "VERIFICATION FAILED!"
    print("*****************************************************************")
    print("*** 100.000% MATHEMATICALLY CERTIFIED HAMILTONIAN CYCLE PASS! ***")
    print("*****************************************************************")

    out_tour = os.path.join(base_dir, "found_tour_graph990.hcp")
    write_hcp(full_tour, out_tour)
    print(f"Tour written to {out_tour}")
    print(f"Total time elapsed: {time.time()-t_start:.2f}s")

if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Run solver script**

Run: `python3 scratch/graph990/solve_graph990.py`
Expected: Outputs `scratch/graph990/found_tour_graph990.hcp` and prints `100.000% MATHEMATICALLY CERTIFIED HAMILTONIAN CYCLE PASS!`.

- [ ] **Step 3: Run independent verification suite**

Run: `python3 scratch/verify_benchmarks.py --graph FHCPCS-col/graph990.col --tour scratch/graph990/found_tour_graph990.hcp`
Expected: Output `Validation Result: PASS - CERTIFIED SOUND`, exit code 0.

- [ ] **Step 4: Commit**

```bash
git add scratch/graph990/solve_graph990.py scratch/graph990/found_tour_graph990.hcp
git commit -m "feat(graph990): full tour assembly and independent certification pass"
```
