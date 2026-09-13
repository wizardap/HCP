# Two-Block Bisection & Degree-2 Contraction Solver for graph710.col Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and execute a certified two-block bisection and degree-2 contraction solver for `FHCPCS-col/graph710.col` ($N = 4,064, M = 6,800$), producing a 100% verified sound Hamiltonian cycle in `scratch/graph710/found_tour_graph710.hcp` in $\le 1,800$s without tour injection.

**Architecture:** Bisection via 2-vertex cut $\{1876, 2491\}$ into Block A (887v) and Block B (3,179v) with forced port directions; Block A CaDiCaL CEGAR solver (~2s); Block B degree-2 recursive contraction (922 vertices contracted) + 454 static chordless triangle cuts + CaDiCaL CEGAR (~180s); parallel multiprocessing execution; deterministic tour stitching $T = P_A[:-1] + P_B[:-1]$.

**Tech Stack:** Python 3, PySAT (CaDiCaL 1.9.5), multiprocessing.

## Global Constraints

- Target Graph: `FHCPCS-col/graph710.col` ($N = 4,064, M = 6,800$).
- Zero Tour Injection: Never read, preload, or inspect `.tou` reference files (e.g. `FHCPCS-col/graph710.tou`).
- Zero Phantom Edges: Every edge in the tour must exist in `G[u]` from raw `graph710.col`.
- Complete Tour Certification: Independent validation using `python3 scratch/verify_benchmarks.py --graph FHCPCS-col/graph710.col --tour scratch/graph710/found_tour_graph710.hcp` must report `Validation Result: PASS - CERTIFIED SOUND` with exit code 0.
- Runtime limit: $\le 1,800$s (expected $\sim 185$s wall-clock).

---

### Task 1: Graph Loading, 2-Vertex Cut Bisection & Partition Invariant Verification

**Files:**
- Create: `scratch/graph710/decomposer.py`
- Test: `scratch/graph710/test_decomposer.py`

**Interfaces:**
- Consumes: Raw graph `FHCPCS-col/graph710.col`
- Produces: `load_and_decompose_graph710(col_path: str) -> Tuple[Dict[int, Set[int]], Set[int], Set[int], int, int]`
  where:
  - `G` is the full adjacency dictionary of `graph710.col` ($|V| = 4,064, |E| = 6,800$).
  - `V_A` is the set of 887 vertices in Block A (includes cut ports 1876 and 2491).
  - `V_B` is the set of 3,179 vertices in Block B (includes cut ports 1876 and 2491).
  - `port_u = 1876` and `port_v = 2491`.

- [ ] **Step 1: Write the failing test**

```python
# scratch/graph710/test_decomposer.py
import pytest
from scratch.graph710.decomposer import load_and_decompose_graph710

def test_decomposer_invariants():
    col_path = "FHCPCS-col/graph710.col"
    G, V_A, V_B, port_u, port_v = load_and_decompose_graph710(col_path)

    # 1. Graph sizes
    assert len(G) == 4064
    num_edges = sum(len(nbrs) for nbrs in G.values()) // 2
    assert num_edges == 6800

    # 2. Cut vertices
    assert port_u == 1876
    assert port_v == 2491
    assert port_v not in G[port_u], "No direct edge between cut ports in G"

    # 3. Block sizes
    assert len(V_A) == 887
    assert len(V_B) == 3179

    # 4. Overlap and union invariants
    assert V_A & V_B == {port_u, port_v}
    assert V_A | V_B == set(G.keys())
    assert len(V_A) + len(V_B) - 2 == 4064

    # 5. Forced port degrees into blocks
    interior_A = V_A - {port_u, port_v}
    interior_B = V_B - {port_u, port_v}

    # port_u (1876) has exactly 1 edge into Block A interior: (1876, 3878)
    u_nbrs_A = G[port_u] & interior_A
    assert len(u_nbrs_A) == 1
    assert 3878 in u_nbrs_A
    # port_u has 3 edges into Block B interior
    u_nbrs_B = G[port_u] & interior_B
    assert len(u_nbrs_B) == 3

    # port_v (2491) has exactly 1 edge into Block B interior: (2491, 1671)
    v_nbrs_B = G[port_v] & interior_B
    assert len(v_nbrs_B) == 1
    assert 1671 in v_nbrs_B
    # port_v has 2 edges into Block A interior
    v_nbrs_A = G[port_v] & interior_A
    assert len(v_nbrs_A) == 2
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pytest scratch/graph710/test_decomposer.py -v`
Expected: FAIL with `ModuleNotFoundError: No module named 'scratch.graph710.decomposer'`

- [ ] **Step 3: Write implementation**

```python
# scratch/graph710/decomposer.py
import collections
from typing import Dict, Set, Tuple

def load_graph(col_path: str) -> Dict[int, Set[int]]:
    G = collections.defaultdict(set)
    with open(col_path, "r") as f:
        for line in f:
            if line.startswith("e "):
                parts = line.split()
                u, v = int(parts[1]), int(parts[2])
                G[u].add(v)
                G[v].add(u)
    return dict(G)

def load_and_decompose_graph710(col_path: str) -> Tuple[Dict[int, Set[int]], Set[int], Set[int], int, int]:
    G = load_graph(col_path)
    assert len(G) == 4064

    port_u = 1876
    port_v = 2491
    cut = {port_u, port_v}
    rem_nodes = set(G.keys()) - cut

    # Find connected components in G \ cut
    visited = set()
    comps = []
    for u in rem_nodes:
        if u not in visited:
            c = []
            q = [u]
            visited.add(u)
            for x in q:
                c.append(x)
                for nbr in G[x]:
                    if nbr in rem_nodes and nbr not in visited:
                        visited.add(nbr)
                        q.append(nbr)
            comps.append(set(c))

    assert len(comps) == 2, f"Expected 2 components, found {len(comps)}"
    comps.sort(key=len)
    comp_a, comp_b = comps[0], comps[1]

    assert len(comp_a) == 885
    assert len(comp_b) == 3177

    V_A = comp_a | cut
    V_B = comp_b | cut

    return G, V_A, V_B, port_u, port_v
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pytest scratch/graph710/test_decomposer.py -v`
Expected: PASS (all partition and cut invariants verified)

- [ ] **Step 5: Commit**

```bash
git add scratch/graph710/decomposer.py scratch/graph710/test_decomposer.py
git commit -m "feat(graph710): add graph decomposer and 2-vertex cut verification test"
```

---

### Task 2: Block SAT Solvers & Parallel Solving

**Files:**
- Create: `scratch/graph710/block_solver.py`
- Create: `scratch/graph710/solve_blocks_parallel.py`
- Test: `scratch/graph710/test_block_solver.py`
- Output: `scratch/graph710/block_paths.json`

**Interfaces:**
- Consumes: `load_and_decompose_graph710` from `scratch/graph710/decomposer.py`
- Produces: `solve_block_a(G, V_A, port_u, port_v) -> List[int]`
- Produces: `solve_block_b(G, V_B, port_u, port_v) -> List[int]`
- Produces: `solve_both_blocks_parallel(col_path: str, cache_path: str) -> Dict[str, List[int]]`
  where `scratch/graph710/block_paths.json` contains:
  - `"A"`: list of 887 vertices starting at 1876 and ending at 2491.
  - `"B"`: list of 3,179 vertices starting at 2491 and ending at 1876.

- [ ] **Step 1: Write the failing test**

```python
# scratch/graph710/test_block_solver.py
import json, os, pytest
from scratch.graph710.decomposer import load_and_decompose_graph710

def test_cached_block_paths_valid():
    base_dir = os.path.dirname(os.path.abspath(__file__))
    cache_file = os.path.join(base_dir, "block_paths.json")
    assert os.path.exists(cache_file), "block_paths.json missing, run solve_blocks_parallel.py first"

    col_path = "FHCPCS-col/graph710.col"
    G, V_A, V_B, port_u, port_v = load_and_decompose_graph710(col_path)

    with open(cache_file, "r") as f:
        paths = json.load(f)

    assert "A" in paths and "B" in paths
    path_a = paths["A"]
    path_b = paths["B"]

    # Block A assertions
    assert len(path_a) == 887
    assert len(set(path_a)) == 887
    assert set(path_a) == V_A
    assert path_a[0] == port_u and path_a[-1] == port_v
    for i in range(len(path_a) - 1):
        assert path_a[i+1] in G[path_a[i]], f"Phantom edge in path_a: ({path_a[i]}, {path_a[i+1]})"

    # Block B assertions
    assert len(path_b) == 3179
    assert len(set(path_b)) == 3179
    assert set(path_b) == V_B
    assert path_b[0] == port_v and path_b[-1] == port_u
    for i in range(len(path_b) - 1):
        assert path_b[i+1] in G[path_b[i]], f"Phantom edge in path_b: ({path_b[i]}, {path_b[i+1]})"
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pytest scratch/graph710/test_block_solver.py -v`
Expected: FAIL with `AssertionError: block_paths.json missing`

- [ ] **Step 3: Implement block solvers and parallel runner**

Create `scratch/graph710/block_solver.py`:
```python
# scratch/graph710/block_solver.py
import collections, time
from typing import Dict, List, Set, Tuple
from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType

def solve_block_a(G: Dict[int, Set[int]], V_A: Set[int], port_u: int, port_v: int) -> List[int]:
    """
    Solve Block A (887 vertices) using CaDiCaL CEGAR with virtual edge (port_u, port_v).
    """
    t0 = time.time()
    edges = set()
    for u in V_A:
        for v in G[u]:
            if v in V_A and u < v:
                edges.add((u, v))
    virt_edge = tuple(sorted([port_u, port_v]))
    edges.add(virt_edge)
    edge_list = sorted(list(edges))
    edge_to_var = {e: i + 1 for i, e in enumerate(edge_list)}
    inc_edges = collections.defaultdict(list)
    for e in edge_list:
        inc_edges[e[0]].append(edge_to_var[e])
        inc_edges[e[1]].append(edge_to_var[e])

    solver = Cadical195()
    top = len(edge_list) + 1

    # Exactly-2 degree constraints
    for u in V_A:
        lits = inc_edges[u]
        clauses = CardEnc.equals(lits=lits, bound=2, top_id=top, encoding=EncType.cardnetwrk)
        for c in clauses:
            solver.add_clause(c)
            for lit in c:
                top = max(top, abs(lit) + 1)

    # Force virtual edge to True
    solver.add_clause([edge_to_var[virt_edge]])

    # CEGAR loop
    it = 0
    while True:
        it += 1
        if not solver.solve():
            raise RuntimeError("Block A UNSAT!")
        model = set(solver.get_model())
        active = [e for e in edge_list if edge_to_var[e] in model]
        adj = collections.defaultdict(list)
        for u, v in active:
            adj[u].append(v)
            adj[v].append(u)

        visited = set()
        cycles = []
        for u in V_A:
            if u not in visited:
                cyc = []
                curr, prev = u, None
                while curr not in visited:
                    visited.add(curr)
                    cyc.append(curr)
                    nbrs = adj[curr]
                    nxt = nbrs[0] if nbrs[0] != prev else nbrs[1]
                    prev, curr = curr, nxt
                cycles.append(cyc)

        if len(cycles) == 1:
            print(f"[Block A] Converged at iter {it} in {time.time()-t0:.2f}s! ({len(cycles[0])} vertices)")
            cyc = cycles[0]
            idx_u = cyc.index(port_u)
            n = len(cyc)
            if cyc[(idx_u + 1) % n] == port_v:
                cyc = list(reversed(cyc))
                idx_u = cyc.index(port_u)
            assert cyc[(idx_u - 1) % n] == port_v
            # Path starts at port_u and ends at port_v
            path = cyc[idx_u:] + cyc[:idx_u]
            assert path[0] == port_u and path[-1] == port_v and len(path) == 887
            solver.delete()
            return path

        for cyc in cycles:
            cut_lits = []
            cyc_set = set(cyc)
            for u in cyc:
                for v in G[u]:
                    if v in V_A and v not in cyc_set:
                        e = tuple(sorted([u, v]))
                        cut_lits.append(edge_to_var[e])
            if cut_lits:
                clauses = CardEnc.atleast(lits=cut_lits, bound=2, top_id=top, encoding=EncType.cardnetwrk)
                for c in clauses:
                    solver.add_clause(c)
                    for lit in c:
                        top = max(top, abs(lit) + 1)
            else:
                neg_clause = [-edge_to_var[tuple(sorted([cyc[i], cyc[(i+1)%len(cyc)]]))] for i in range(len(cyc))]
                solver.add_clause(neg_clause)

def solve_block_b(G: Dict[int, Set[int]], V_B: Set[int], port_u: int, port_v: int) -> List[int]:
    """
    Solve Block B (3,179 vertices) with degree-2 contraction and 454 triangle cuts.
    Returns path starting at port_v (2491) and ending at port_u (1876).
    """
    t0 = time.time()
    adj_b = collections.defaultdict(set)
    for u in V_B:
        for v in G[u]:
            if v in V_B:
                adj_b[u].add(v)
                adj_b[v].add(u)
    # Add virtual edge
    adj_b[port_u].add(port_v)
    adj_b[port_v].add(port_u)

    # Degree-2 recursive contraction (excluding ports)
    rem = set(V_B)
    edge_chains = {}
    while True:
        d2 = [u for u in rem if u not in {port_u, port_v} and len(adj_b[u]) == 2]
        if not d2:
            break
        v = d2[0]
        u, w = list(adj_b[v])
        adj_b[u].remove(v)
        adj_b[w].remove(v)
        del adj_b[v]
        rem.remove(v)
        e_uv = tuple(sorted([u, v]))
        e_vw = tuple(sorted([v, w]))
        chain_uv = edge_chains.pop(e_uv, [u, v])
        chain_vw = edge_chains.pop(e_vw, [v, w])
        if chain_uv[-1] != v:
            chain_uv = list(reversed(chain_uv))
        if chain_vw[0] != v:
            chain_vw = list(reversed(chain_vw))
        merged_chain = chain_uv[:-1] + chain_vw
        e_uw = tuple(sorted([u, w]))
        adj_b[u].add(w)
        adj_b[w].add(u)
        edge_chains[e_uw] = merged_chain

    print(f"[Block B] Contracted: {len(V_B)} -> {len(rem)} vertices ({len(edge_chains)} contracted chains).")
    assert len(rem) == 2257

    contracted_edges = set(edge_chains.keys())
    edges = set()
    for u in rem:
        for v in adj_b[u]:
            if u < v:
                edges.add((u, v))
    edge_list = sorted(list(edges))
    edge_to_var = {e: i + 1 for i, e in enumerate(edge_list)}
    inc_edges = collections.defaultdict(list)
    for e in edge_list:
        inc_edges[e[0]].append(edge_to_var[e])
        inc_edges[e[1]].append(edge_to_var[e])

    solver = Cadical195()
    top = len(edge_list) + 1

    # Exactly-2 degree constraints on contracted graph
    for u in rem:
        lits = inc_edges[u]
        clauses = CardEnc.equals(lits=lits, bound=2, top_id=top, encoding=EncType.cardnetwrk)
        for c in clauses:
            solver.add_clause(c)
            for lit in c:
                top = max(top, abs(lit) + 1)

    # Force virtual edge to True
    virt_edge = tuple(sorted([port_u, port_v]))
    solver.add_clause([edge_to_var[virt_edge]])

    # Force all contracted edges to True
    for ce in contracted_edges:
        solver.add_clause([edge_to_var[ce]])

    # Static chordless triangle cuts in contracted graph
    triangles = []
    rem_list = sorted(list(rem))
    for i, u in enumerate(rem_list):
        for v in adj_b[u]:
            if v > u:
                for w in adj_b[v]:
                    if w > v and w in adj_b[u]:
                        triangles.append((u, v, w))
    for u, v, w in triangles:
        e1 = tuple(sorted([u, v]))
        e2 = tuple(sorted([v, w]))
        e3 = tuple(sorted([w, u]))
        solver.add_clause([-edge_to_var[e1], -edge_to_var[e2], -edge_to_var[e3]])
    print(f"[Block B] Added {len(triangles)} static triangle cuts. Starting CEGAR loop...")

    it = 0
    while True:
        it += 1
        t_start_it = time.time()
        if not solver.solve():
            raise RuntimeError("Block B UNSAT!")
        model = set(solver.get_model())
        active = [e for e in edge_list if edge_to_var[e] in model]
        adj = collections.defaultdict(list)
        for u, v in active:
            adj[u].append(v)
            adj[v].append(u)

        visited = set()
        cycles = []
        for u in rem:
            if u not in visited:
                cyc = []
                curr, prev = u, None
                while curr not in visited:
                    visited.add(curr)
                    cyc.append(curr)
                    nbrs = adj[curr]
                    nxt = nbrs[0] if nbrs[0] != prev else nbrs[1]
                    prev, curr = curr, nxt
                cycles.append(cyc)

        if len(cycles) == 1:
            print(f"[Block B] Converged at iter {it} in {time.time()-t0:.2f}s! ({len(cycles[0])} contracted vertices)")
            cyc = cycles[0]
            # Expand contracted edges along cycle
            expanded = []
            n = len(cyc)
            for i in range(n):
                u = cyc[i]
                v = cyc[(i + 1) % n]
                e = tuple(sorted([u, v]))
                if e in edge_chains:
                    chain = edge_chains[e]
                    if chain[0] != u:
                        chain = list(reversed(chain))
                    expanded.extend(chain[:-1])
                else:
                    expanded.append(u)

            print(f"[Block B] Expanded cycle length: {len(expanded)} (expected 3179).")
            assert len(expanded) == 3179
            assert set(expanded) == V_B

            idx_v = expanded.index(port_v)
            n_exp = len(expanded)
            if expanded[(idx_v + 1) % n_exp] == port_u:
                expanded = list(reversed(expanded))
                idx_v = expanded.index(port_v)
            assert expanded[(idx_v - 1) % n_exp] == port_u
            # Path starts at port_v (2491) and ends at port_u (1876)
            path = expanded[idx_v:] + expanded[:idx_v]
            assert path[0] == port_v and path[-1] == port_u and len(path) == 3179
            solver.delete()
            return path

        if it % 10 == 0 or len(cycles) <= 5:
            cyc_lens = sorted([len(c) for c in cycles], reverse=True)
            print(f"[Block B] Iter {it} ({time.time()-t_start_it:.2f}s): {len(cycles)} cycles. Max: {cyc_lens[0]}, Min: {cyc_lens[-1]}")

        for cyc in cycles:
            cut_lits = []
            cyc_set = set(cyc)
            for u in cyc:
                for v in adj_b[u]:
                    if v not in cyc_set:
                        e = tuple(sorted([u, v]))
                        cut_lits.append(edge_to_var[e])
            if cut_lits:
                clauses = CardEnc.atleast(lits=cut_lits, bound=2, top_id=top, encoding=EncType.cardnetwrk)
                for c in clauses:
                    solver.add_clause(c)
                    for lit in c:
                        top = max(top, abs(lit) + 1)
            else:
                neg_clause = [-edge_to_var[tuple(sorted([cyc[i], cyc[(i+1)%len(cyc)]]))] for i in range(len(cyc))]
                solver.add_clause(neg_clause)
```

Create `scratch/graph710/solve_blocks_parallel.py`:
```python
# scratch/graph710/solve_blocks_parallel.py
import json, multiprocessing, os, sys, time
from typing import Dict, List
from scratch.graph710.decomposer import load_and_decompose_graph710
from scratch.graph710.block_solver import solve_block_a, solve_block_b

def _worker_a(col_path: str):
    G, V_A, _, port_u, port_v = load_and_decompose_graph710(col_path)
    print("Worker A started...")
    path_a = solve_block_a(G, V_A, port_u, port_v)
    return "A", path_a

def _worker_b(col_path: str):
    G, _, V_B, port_u, port_v = load_and_decompose_graph710(col_path)
    print("Worker B started...")
    path_b = solve_block_b(G, V_B, port_u, port_v)
    return "B", path_b

def solve_both_blocks_parallel(col_path: str, cache_path: str) -> Dict[str, List[int]]:
    t0 = time.time()
    os.makedirs(os.path.dirname(os.path.abspath(cache_path)), exist_ok=True)
    if os.path.exists(cache_path):
        with open(cache_path, "r") as f:
            data = json.load(f)
        if "A" in data and "B" in data and len(data["A"]) == 887 and len(data["B"]) == 3179:
            print("Both block paths already cached!")
            return data

    with multiprocessing.Pool(processes=2) as pool:
        res_a = pool.apply_async(_worker_a, (col_path,))
        res_b = pool.apply_async(_worker_b, (col_path,))
        k_a, path_a = res_a.get()
        k_b, path_b = res_b.get()

    result = {k_a: path_a, k_b: path_b}
    with open(cache_path, "w") as f:
        json.dump(result, f)
    print(f"Both blocks solved and cached to {cache_path} in {time.time()-t0:.2f}s.")
    return result

if __name__ == "__main__":
    base_dir = os.path.dirname(os.path.abspath(__file__))
    col_path = "FHCPCS-col/graph710.col"
    cache_path = os.path.join(base_dir, "block_paths.json")
    solve_both_blocks_parallel(col_path, cache_path)
```

- [ ] **Step 4: Execute parallel solver and run test to verify it passes**

Run: `python3 scratch/graph710/solve_blocks_parallel.py`
Run: `pytest scratch/graph710/test_block_solver.py -v`
Expected: PASS (both blocks solved and validated on raw DIMACS edge list)

- [ ] **Step 5: Commit**

```bash
git add scratch/graph710/block_solver.py scratch/graph710/solve_blocks_parallel.py scratch/graph710/test_block_solver.py scratch/graph710/block_paths.json
git commit -m "feat(graph710): add block SAT solvers, parallel runner, and verified path cache"
```

---

### Task 3: Full Tour Assembly & Independent Soundness Certification

**Files:**
- Create: `scratch/graph710/solve_graph710.py`
- Output: `scratch/graph710/found_tour_graph710.hcp`
- Test: `scratch/verify_benchmarks.py`

**Interfaces:**
- Consumes: `scratch/graph710/block_paths.json` and `load_and_decompose_graph710`
- Produces: `scratch/graph710/found_tour_graph710.hcp`
- Verification: `python3 scratch/verify_benchmarks.py --graph FHCPCS-col/graph710.col --tour scratch/graph710/found_tour_graph710.hcp`

- [ ] **Step 1: Implement tour assembler**

Create `scratch/graph710/solve_graph710.py`:
```python
# scratch/graph710/solve_graph710.py
import json, os, sys, time
from scratch.graph710.decomposer import load_and_decompose_graph710
from scratch.graph710.solve_blocks_parallel import solve_both_blocks_parallel

def assemble_and_verify_tour():
    t0 = time.time()
    base_dir = os.path.dirname(os.path.abspath(__file__))
    col_path = "FHCPCS-col/graph710.col"
    cache_path = os.path.join(base_dir, "block_paths.json")
    tour_path = os.path.join(base_dir, "found_tour_graph710.hcp")

    G, V_A, V_B, port_u, port_v = load_and_decompose_graph710(col_path)
    paths = solve_both_blocks_parallel(col_path, cache_path)

    P_A = paths["A"]  # 1876 -> ... -> 2491 (887 vertices)
    P_B = paths["B"]  # 2491 -> ... -> 1876 (3179 vertices)

    assert P_A[0] == port_u and P_A[-1] == port_v
    assert P_B[0] == port_v and P_B[-1] == port_u

    # Assemble tour: omit last element of each path to avoid duplicating cut vertices
    tour = P_A[:-1] + P_B[:-1]
    assert len(tour) == 4064
    assert len(set(tour)) == 4064
    assert set(tour) == set(G.keys())

    # Verify all edges along the cycle exist in raw G
    for i in range(len(tour)):
        u = tour[i]
        v = tour[(i + 1) % len(tour)]
        assert v in G[u], f"Phantom edge in assembled tour: ({u}, {v})"

    print(f"Tour assembly verified sound: 4064 unique vertices, 4064 valid DIMACS edges!")

    # Write HCP file
    with open(tour_path, "w") as f:
        f.write("NAME : graph710.col\n")
        f.write("TYPE : TOUR\n")
        f.write(f"DIMENSION : {len(tour)}\n")
        f.write("TOUR_SECTION\n")
        for node in tour:
            f.write(f"{node}\n")
        f.write("-1\n")
        f.write("EOF\n")

    print(f"Tour written to {tour_path} in {time.time()-t0:.2f}s.")
    return tour_path

if __name__ == "__main__":
    assemble_and_verify_tour()
```

- [ ] **Step 2: Run `solve_graph710.py` to generate the tour**

Run: `python3 scratch/graph710/solve_graph710.py`
Expected: Tour assembled and written to `scratch/graph710/found_tour_graph710.hcp`.

- [ ] **Step 3: Run independent benchmark verification**

Run: `python3 scratch/verify_benchmarks.py --graph FHCPCS-col/graph710.col --tour scratch/graph710/found_tour_graph710.hcp`
Expected:
`Validation Result: PASS - CERTIFIED SOUND`
`Exit code: 0`

- [ ] **Step 4: Commit**

```bash
git add scratch/graph710/solve_graph710.py scratch/graph710/found_tour_graph710.hcp
git commit -m "feat(graph710): add full tour assembler and certified sound Hamiltonian cycle"
```
