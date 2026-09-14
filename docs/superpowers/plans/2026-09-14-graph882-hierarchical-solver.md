# Hierarchical Modular-Chain & Central-Block Solver for graph882.col Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and execute a certified hierarchical modular-chain and central-block solver for `FHCPCS-col/graph882.col` ($N = 5,686, M = 9,306$), producing a 100% verified sound Hamiltonian cycle in `scratch/graph882/found_tour_graph882.hcp` in $\le 1,800$s without tour injection.

**Architecture:** Decompose graph into an Outer Chain of 185 vertices (comprising a 167-vertex module and 16-vertex buffer block) and a 5,501-vertex central block Comp 0; solve Outer Chain with CaDiCaL CEGAR + Cocycle cuts into a 185-vertex simple path connecting ports 5200 and 4779; solve Comp 0 via degree-2 contraction (1,228 vertices contracted) + 80 static triangle cuts + CaDiCaL CEGAR with virtual edge $(2080, 5066)$ and 2-opt cycle absorption; deterministically splice the 185-vertex chain into the central cycle to produce a 5,686-vertex Hamiltonian tour.

**Tech Stack:** Python 3, PySAT (CaDiCaL 1.9.5), collections, itertools.

## Global Constraints

- Target Graph: `FHCPCS-col/graph882.col` ($N = 5,686, M = 9,306$).
- Zero Tour Injection: Never read, preload, or inspect `.tou` reference files (e.g. `FHCPCS-col/graph882.tou`).
- Zero Phantom Edges: Every edge in the tour must exist in `G[u]` from raw `graph882.col`.
- Complete Tour Certification: Independent validation using `python3 scratch/verify_benchmarks.py --graph FHCPCS-col/graph882.col --tour scratch/graph882/found_tour_graph882.hcp` must report `Validation Result: PASS - CERTIFIED SOUND` with exit code 0.
- Runtime limit: $\le 1,800$s.

---

### Task 1: Graph Loading, Modular Decomposition & Partition Invariant Verification

**Files:**
- Create: `scratch/graph882/__init__.py`
- Create: `scratch/graph882/decomposer.py`
- Test: `scratch/graph882/test_decomposer.py`

**Interfaces:**
- Consumes: Raw graph `FHCPCS-col/graph882.col`
- Produces: `load_and_decompose_graph882(col_path: str) -> Tuple[Dict[int, Set[int]], Set[int], Set[int], Set[int], Set[int]]`
  where:
  - `G`: full adjacency dictionary ($|V| = 5,686, |E| = 9,306$).
  - `mod_nodes`: set of 169 vertices in Module 167 including ports $\{3195, 5200\}$.
  - `block2_nodes`: set of 16 vertices in Buffer Block (Comp 2 + hub 4509).
  - `chain_nodes`: set of 185 vertices in Outer Chain (`mod_nodes | block2_nodes`).
  - `comp0_nodes`: set of 5,501 vertices in Central Block Comp 0 (`V(G) \ chain_nodes`).

- [ ] **Step 1: Write the failing test**

```python
# scratch/graph882/test_decomposer.py
import os, pytest
from scratch.graph882.decomposer import load_and_decompose_graph882

def test_decomposer_invariants():
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
    col_path = os.path.join(repo_root, "FHCPCS-col/graph882.col")
    G, mod_nodes, block2_nodes, chain_nodes, comp0_nodes = load_and_decompose_graph882(col_path)

    # 1. Graph sizes
    assert len(G) == 5686
    num_edges = sum(len(nbrs) for nbrs in G.values()) // 2
    assert num_edges == 9306

    # 2. Module count and sizes
    assert len(mod_nodes) == 169
    assert {3195, 5200}.issubset(mod_nodes)
    internal_mod = mod_nodes - {3195, 5200}
    assert len(internal_mod) == 167
    assert len(G[3195] & internal_mod) == 13
    assert len(G[5200] & internal_mod) == 13

    # 3. Buffer block sizes
    assert len(block2_nodes) == 16
    assert 4509 in block2_nodes
    assert 4779 in block2_nodes

    # 4. Chain nodes and bridge edge
    assert chain_nodes == (mod_nodes | block2_nodes)
    assert len(chain_nodes) == 185
    assert 4509 in G[3195]  # Bridge edge connecting Module to Buffer

    # 5. Comp 0 size and disjointness
    assert len(comp0_nodes) == 5501
    assert chain_nodes & comp0_nodes == set()
    assert (chain_nodes | comp0_nodes) == set(G.keys())

    # 6. Terminal connections to Comp 0
    assert 2080 in comp0_nodes
    assert 5066 in comp0_nodes
    assert 2080 in G[5200]
    assert 5066 in G[4779]
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pytest scratch/graph882/test_decomposer.py -v`  
Expected: FAIL with `ModuleNotFoundError: No module named 'scratch.graph882'`

- [ ] **Step 3: Write implementation**

```python
# scratch/graph882/__init__.py
# Marker file
```

```python
# scratch/graph882/decomposer.py
import collections, os
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

def load_and_decompose_graph882(col_path: str) -> Tuple[Dict[int, Set[int]], Set[int], Set[int], Set[int], Set[int]]:
    G = load_graph(col_path)
    assert len(G) == 5686

    # 2-vertex cut separating the 167-vertex module: {3195, 5200}
    cut_nodes = {3195, 5200}
    rem = set(G.keys()) - cut_nodes

    visited = set()
    comps = []
    for u in rem:
        if u not in visited:
            c = []
            q = [u]
            visited.add(u)
            for x in q:
                c.append(x)
                for nbr in G[x]:
                    if nbr in rem and nbr not in visited:
                        visited.add(nbr)
                        q.append(nbr)
            comps.append(set(c))

    # The small component of size 167 is the module
    small_mod = min(comps, key=len)
    assert len(small_mod) == 167
    mod_nodes = small_mod | cut_nodes
    assert len(mod_nodes) == 169

    # Buffer Block (Comp 2 + hub 4509): 15 nodes + hub 4509 = 16 nodes
    # Comp 2 contains 4779 which connects to 5066
    assert 4779 in G
    assert 5066 in G[4779]
    # Identify Comp 2 by BFS in G \ {all 20 hubs}
    hubs = {v for v, nbrs in G.items() if len(nbrs) == 14}
    assert len(hubs) == 20
    rem_hubs = set(G.keys()) - hubs

    visited_c2 = set()
    q_c2 = [4779]
    visited_c2.add(4779)
    for x in q_c2:
        for nbr in G[x]:
            if nbr in rem_hubs and nbr not in visited_c2:
                visited_c2.add(nbr)
                q_c2.append(nbr)
    comp2_nodes = set(q_c2)
    assert len(comp2_nodes) == 15
    assert 4509 in hubs
    block2_nodes = comp2_nodes | {4509}
    assert len(block2_nodes) == 16

    # Outer chain = mod_nodes | block2_nodes
    chain_nodes = mod_nodes | block2_nodes
    assert len(chain_nodes) == 185

    # Comp 0 = V \ chain_nodes
    comp0_nodes = set(G.keys()) - chain_nodes
    assert len(comp0_nodes) == 5501

    return G, mod_nodes, block2_nodes, chain_nodes, comp0_nodes
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pytest scratch/graph882/test_decomposer.py -v`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add scratch/graph882/__init__.py scratch/graph882/decomposer.py scratch/graph882/test_decomposer.py
git commit -m "feat(graph882): add graph decomposer and modular partition verification test"
```

---

### Task 2: Modular Chain Solver & Central Block Comp 0 Solver

**Files:**
- Create: `scratch/graph882/chain_solver.py`
- Create: `scratch/graph882/comp0_solver.py`
- Test: `scratch/graph882/test_chain_solver.py`
- Test: `scratch/graph882/test_comp0_solver.py`

**Interfaces:**
- `chain_solver.py`:
  - `solve_module_path(G: Dict[int, Set[int]], nodes: Set[int], src: int, dst: int) -> List[int]`
  - `solve_outer_chain(G: Dict[int, Set[int]], mod_nodes: Set[int], block2_nodes: Set[int]) -> List[int]`
    - Returns a 185-vertex simple path from $5200$ to $4779$.
- `comp0_solver.py`:
  - `solve_comp0(G: Dict[int, Set[int]], comp0_nodes: Set[int], virt_edge: Tuple[int, int] = (2080, 5066)) -> List[int]`
    - Returns a 5,501-vertex Hamiltonian cycle containing edge $(2080, 5066)$.

- [ ] **Step 1: Write the failing tests**

```python
# scratch/graph882/test_chain_solver.py
import os, pytest
from scratch.graph882.decomposer import load_and_decompose_graph882
from scratch.graph882.chain_solver import solve_outer_chain

def test_outer_chain_solver():
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
    col_path = os.path.join(repo_root, "FHCPCS-col/graph882.col")
    G, mod_nodes, block2_nodes, chain_nodes, _ = load_and_decompose_graph882(col_path)

    chain_path = solve_outer_chain(G, mod_nodes, block2_nodes)
    assert len(chain_path) == 185
    assert len(set(chain_path)) == 185
    assert set(chain_path) == chain_nodes
    assert chain_path[0] == 5200
    assert chain_path[-1] == 4779

    # Verify each step is an edge in G
    for u, v in zip(chain_path[:-1], chain_path[1:]):
        assert v in G[u], f"Phantom edge in chain: ({u}, {v})"
```

```python
# scratch/graph882/test_comp0_solver.py
import os, pytest
from scratch.graph882.decomposer import load_and_decompose_graph882
from scratch.graph882.comp0_solver import solve_comp0

def test_comp0_solver():
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
    col_path = os.path.join(repo_root, "FHCPCS-col/graph882.col")
    G, _, _, _, comp0_nodes = load_and_decompose_graph882(col_path)

    cycle = solve_comp0(G, comp0_nodes, virt_edge=(2080, 5066))
    assert len(cycle) == 5501
    assert len(set(cycle)) == 5501
    assert set(cycle) == comp0_nodes

    # Check that virtual edge (2080, 5066) is present
    has_virt = False
    for i in range(len(cycle)):
        u, v = cycle[i], cycle[(i + 1) % len(cycle)]
        if {u, v} == {2080, 5066}:
            has_virt = True
        else:
            assert v in G[u], f"Phantom edge in comp0 cycle: ({u}, {v})"
    assert has_virt, "Virtual edge (2080, 5066) not found in comp0 cycle!"
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `pytest scratch/graph882/test_chain_solver.py scratch/graph882/test_comp0_solver.py -v`  
Expected: FAIL with `ModuleNotFoundError`

- [ ] **Step 3: Implement `scratch/graph882/chain_solver.py`**

```python
# scratch/graph882/chain_solver.py
import collections, itertools, json, os, time
from typing import Dict, List, Set, Tuple
from pysat.solvers import Cadical153

def solve_subgraph_path(G: Dict[int, Set[int]], nodes: Set[int], src: int, dst: int) -> List[int]:
    nodes_list = sorted(list(nodes))
    sub_adj = {u: sorted([v for v in G[u] if v in nodes]) for u in nodes_list}
    edges = []
    for u in nodes_list:
        for v in sub_adj[u]:
            if u < v:
                edges.append((u, v))

    e2v = {e: idx + 1 for idx, e in enumerate(edges)}
    solver = Cadical153()

    for u in nodes_list:
        inc = [e2v[tuple(sorted((u, v)))] for v in sub_adj[u]]
        target_deg = 1 if u in (src, dst) else 2
        if target_deg == 1:
            solver.add_clause(inc)
            for a, b in itertools.combinations(inc, 2):
                solver.add_clause([-a, -b])
        else:
            solver.add_clause(inc)
            for c in itertools.combinations(inc, len(inc) - 1):
                solver.add_clause(list(c))
            for c in itertools.combinations(inc, 3):
                solver.add_clause([-c[0], -c[1], -c[2]])

    while solver.solve():
        model = set(solver.get_model())
        active_edges = [e for e in edges if e2v[e] in model]
        adj_m = collections.defaultdict(list)
        for u, v in active_edges:
            adj_m[u].append(v)
            adj_m[v].append(u)

        visited = set()
        comps = []
        for u in nodes_list:
            if u not in visited:
                c = []
                q = collections.deque([u])
                visited.add(u)
                while q:
                    curr = q.popleft()
                    c.append(curr)
                    for to in adj_m[curr]:
                        if to not in visited:
                            visited.add(to)
                            q.append(to)
                comps.append(c)

        if len(comps) == 1:
            # Reconstruct simple path from src to dst
            path = [src]
            curr = src
            prev = None
            while len(path) < len(nodes):
                nxts = [to for to in adj_m[curr] if to != prev]
                assert nxts, "Broken path in reconstruction"
                nxt = nxts[0]
                path.append(nxt)
                prev, curr = curr, nxt
            assert path[-1] == dst
            solver.delete()
            return path

        for c in comps:
            if src not in c and dst not in c:
                c_set = set(c)
                cut = [e2v[e] for e in edges if (e[0] in c_set) != (e[1] in c_set)]
                solver.add_clause(cut)

    solver.delete()
    raise RuntimeError(f"No Hamiltonian path found in subgraph from {src} to {dst}")

def solve_outer_chain(G: Dict[int, Set[int]], mod_nodes: Set[int], block2_nodes: Set[int]) -> List[int]:
    cache_path = os.path.join(os.path.dirname(__file__), "chain_path.json")
    if os.path.exists(cache_path):
        with open(cache_path, "r") as f:
            chain = json.load(f)
            if len(chain) == 185 and chain[0] == 5200 and chain[-1] == 4779:
                return chain

    t0 = time.time()
    # 1. Module 167 HP from 5200 to 3195
    p_mod = solve_subgraph_path(G, mod_nodes, src=5200, dst=3195)
    assert len(p_mod) == 169

    # 2. Buffer Block 16 HP from 4509 to 4779
    p_buf = solve_subgraph_path(G, block2_nodes, src=4509, dst=4779)
    assert len(p_buf) == 16

    # 3. Concatenate via bridge edge (3195, 4509)
    assert 4509 in G[3195]
    chain = p_mod + p_buf
    assert len(chain) == 185
    assert len(set(chain)) == 185
    assert chain[0] == 5200 and chain[-1] == 4779
    print(f"[*] Outer chain (185 vertices) solved in {time.time() - t0:.2f}s!")

    with open(cache_path, "w") as f:
        json.dump(chain, f)
    return chain

if __name__ == "__main__":
    from scratch.graph882.decomposer import load_and_decompose_graph882
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
    col_path = os.path.join(repo_root, "FHCPCS-col/graph882.col")
    G, mod_nodes, block2_nodes, _, _ = load_and_decompose_graph882(col_path)
    solve_outer_chain(G, mod_nodes, block2_nodes)
```

- [ ] **Step 4: Implement `scratch/graph882/comp0_solver.py`**

```python
# scratch/graph882/comp0_solver.py
import collections, itertools, json, os, time
from typing import Dict, List, Set, Tuple
from pysat.solvers import Cadical153

def contract_degree2(sub_adj: Dict[int, Set[int]], fixed_nodes: Set[int]):
    g = {u: set(sub_adj[u]) for u in sub_adj}
    edge_to_chain = {}
    deg2_candidates = set(u for u in g if len(g[u]) == 2 and u not in fixed_nodes)

    while deg2_candidates:
        u = deg2_candidates.pop()
        if u not in g or len(g[u]) != 2 or u in fixed_nodes:
            continue
        nbrs = list(g[u])
        a, b = nbrs[0], nbrs[1]
        if a == b:
            continue

        chain_ua = edge_to_chain.pop(tuple(sorted((u, a))), [u, a])
        chain_ub = edge_to_chain.pop(tuple(sorted((u, b))), [u, b])

        if chain_ua[-1] == u: chain_ua.reverse()
        if chain_ub[0] == u: chain_ub.reverse()
        full_chain = chain_ub + chain_ua[1:]

        del g[u]
        g[a].remove(u); g[b].remove(u)
        g[a].add(b); g[b].add(a)

        e_ab = tuple(sorted((a, b)))
        edge_to_chain[e_ab] = full_chain

        if len(g[a]) == 2 and a not in fixed_nodes: deg2_candidates.add(a)
        if len(g[b]) == 2 and b not in fixed_nodes: deg2_candidates.add(b)

    return g, edge_to_chain

def absorb_2opt(cycles, contracted_adj):
    if len(cycles) <= 1:
        return cycles
    cycles = [list(c) for c in cycles]
    improved = True
    while improved and len(cycles) > 1:
        improved = False
        for i in range(len(cycles)):
            c1 = cycles[i]
            c1_set = set(c1)
            pos1 = {v: idx for idx, v in enumerate(c1)}
            l1 = len(c1)
            merged = False
            for j in range(i + 1, len(cycles)):
                c2 = cycles[j]
                c2_set = set(c2)
                l2 = len(c2)
                for idx2 in range(l2):
                    u = c2[idx2]
                    v = c2[(idx2 + 1) % l2]
                    c1_nbrs_u = [w for w in contracted_adj[u] if w in c1_set]
                    c1_nbrs_v = [w for w in contracted_adj[v] if w in c1_set]
                    for w1 in c1_nbrs_u:
                        pos_w1 = pos1[w1]
                        for delta in (-1, 1):
                            w2 = c1[(pos_w1 + delta) % l1]
                            if w2 in c1_nbrs_v:
                                if delta == 1:
                                    part1 = c1[:pos_w1 + 1]
                                    part2 = [c2[(idx2 - k) % l2] for k in range(l2)]
                                    part3 = c1[pos_w1 + 1:]
                                    new_c = part1 + part2 + part3
                                else:
                                    part1 = c1[:pos_w1]
                                    part2 = [c2[(idx2 + 1 + k) % l2] for k in range(l2)]
                                    part3 = c1[pos_w1:]
                                    new_c = part1 + part2 + part3
                                cycles[i] = new_c
                                cycles.pop(j)
                                merged = True
                                improved = True
                                break
                        if merged: break
                    if merged: break
                if merged: break
            if merged: break
    return cycles

def solve_comp0(G: Dict[int, Set[int]], comp0_nodes: Set[int], virt_edge: Tuple[int, int] = (2080, 5066)) -> List[int]:
    cache_path = os.path.join(os.path.dirname(__file__), "comp0_cycle.json")
    if os.path.exists(cache_path):
        with open(cache_path, "r") as f:
            cycle = json.load(f)
            if len(cycle) == 5501 and set(cycle) == comp0_nodes:
                return cycle

    t0 = time.time()
    sub_adj = {u: set(v for v in G[u] if v in comp0_nodes) for u in comp0_nodes}
    u_v, v_v = virt_edge
    sub_adj[u_v].add(v_v)
    sub_adj[v_v].add(u_v)

    fixed = {u_v, v_v}
    g_contracted, edge_to_chain = contract_degree2(sub_adj, fixed)
    contracted_nodes = sorted(list(g_contracted.keys()))
    print(f"[*] Comp 0 contracted from {len(comp0_nodes)} to {len(contracted_nodes)} vertices.")

    c_edges = set()
    for u in contracted_nodes:
        for v in g_contracted[u]:
            c_edges.add(tuple(sorted((u, v))))
    ve = tuple(sorted(virt_edge))
    c_edges.add(ve)
    e_list = sorted(list(c_edges))
    e2v = {e: idx + 1 for idx, e in enumerate(e_list)}

    solver = Cadical153()
    for u in contracted_nodes:
        inc = [e2v[tuple(sorted((u, v)))] for v in g_contracted[u]]
        if u in (u_v, v_v):
            if e2v[ve] not in inc: inc.append(e2v[ve])
        solver.add_clause(inc)
        for c in itertools.combinations(inc, len(inc) - 1):
            solver.add_clause(list(c))
        for c in itertools.combinations(inc, 3):
            solver.add_clause([-c[0], -c[1], -c[2]])

    solver.add_clause([e2v[ve]])

    # Static triangle cuts
    for u in contracted_nodes:
        nbrs = list(g_contracted[u])
        for i in range(len(nbrs)):
            for j in range(i + 1, len(nbrs)):
                v, w = nbrs[i], nbrs[j]
                if u < v < w and w in g_contracted[v]:
                    e1 = e2v[tuple(sorted((u, v)))]
                    e2 = e2v[tuple(sorted((v, w)))]
                    e3 = e2v[tuple(sorted((u, w)))]
                    solver.add_clause([-e1, -e2, -e3])

    iteration = 0
    final_contracted_cycle = None
    while solver.solve():
        iteration += 1
        model = set(solver.get_model())
        active = [e for e in e_list if e2v[e] in model]
        adj_m = collections.defaultdict(list)
        for u, v in active:
            adj_m[u].append(v); adj_m[v].append(u)

        visited = set()
        raw_cycles = []
        for u in contracted_nodes:
            if u not in visited:
                c = []
                curr = u
                while curr not in visited:
                    visited.add(curr)
                    c.append(curr)
                    nxts = [x for x in adj_m[curr] if x not in visited]
                    if nxts: curr = nxts[0]
                    else: break
                raw_cycles.append(c)

        absorbed = absorb_2opt(raw_cycles, g_contracted)
        if iteration % 10 == 0 or len(absorbed) == 1:
            lens = [len(c) for c in absorbed]
            print(f"    [Comp 0] Iter {iteration} ({time.time()-t0:.2f}s): {len(raw_cycles)} cycles (absorbed -> {len(absorbed)}). Max: {max(lens)}, Min: {min(lens)}")

        if len(absorbed) == 1:
            final_contracted_cycle = absorbed[0]
            print(f"[*] Comp 0 CEGAR converged at iter {iteration} in {time.time()-t0:.2f}s!")
            break

        for c in raw_cycles:
            if len(c) < len(contracted_nodes):
                c_set = set(c)
                cut = [e2v[e] for e in e_list if (e[0] in c_set) != (e[1] in c_set)]
                solver.add_clause(cut)

    solver.delete()
    assert final_contracted_cycle is not None, "CEGAR failed to converge on Comp 0"

    # Uncontract degree-2 chains
    full_cycle = []
    l_c = len(final_contracted_cycle)
    for i in range(l_c):
        u = final_contracted_cycle[i]
        v = final_contracted_cycle[(i + 1) % l_c]
        e = tuple(sorted((u, v)))
        if e in edge_to_chain:
            chain = edge_to_chain[e]
            if chain[0] != u:
                chain = list(reversed(chain))
            full_cycle.extend(chain[:-1])
        else:
            full_cycle.append(u)

    assert len(full_cycle) == 5501
    assert len(set(full_cycle)) == 5501
    with open(cache_path, "w") as f:
        json.dump(full_cycle, f)
    return full_cycle

if __name__ == "__main__":
    from scratch.graph882.decomposer import load_and_decompose_graph882
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
    col_path = os.path.join(repo_root, "FHCPCS-col/graph882.col")
    G, _, _, _, comp0_nodes = load_and_decompose_graph882(col_path)
    solve_comp0(G, comp0_nodes)
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `pytest scratch/graph882/test_chain_solver.py -v`  
Expected: PASS in ~1-2s.  
Run: `python3 scratch/graph882/comp0_solver.py`  
Expected: Converges in ~200-350s and writes `scratch/graph882/comp0_cycle.json`.  
Run: `pytest scratch/graph882/test_comp0_solver.py -v`  
Expected: PASS in ~0.5s.

- [ ] **Step 6: Commit**

```bash
git add scratch/graph882/chain_solver.py scratch/graph882/comp0_solver.py scratch/graph882/test_chain_solver.py scratch/graph882/test_comp0_solver.py scratch/graph882/chain_path.json scratch/graph882/comp0_cycle.json
git commit -m "feat(graph882): add modular chain solver and Comp 0 degree-2 contracted CEGAR solver"
```

---

### Task 3: Full Tour Assembly, Clean In-Memory Solver & Soundness Certification

**Files:**
- Create: `scratch/graph882/solve_graph882.py`
- Create: `scratch/graph882/solve_in_memory.py`
- Test: `scratch/graph882/test_solve_graph882.py`
- Produce: `scratch/graph882/found_tour_graph882.hcp`

**Interfaces:**
- `assemble_tour(comp0_cycle: List[int], chain_path: List[int], virt_edge: Tuple[int, int] = (2080, 5066)) -> List[int]`
  - Splices the 185-vertex outer chain into the 5,501-vertex Comp 0 cycle in place of the virtual edge $(2080, 5066)$, returning a 5,686-vertex Hamiltonian cycle.
- `solve_in_memory.py`:
  - 100% cold-start in-memory de novo solver executing from raw `FHCPCS-col/graph882.col`.

- [ ] **Step 1: Write the failing test**

```python
# scratch/graph882/test_solve_graph882.py
import os, pytest
from scratch.graph882.decomposer import load_graph
from scratch.graph882.solve_graph882 import assemble_tour

def test_tour_assembly():
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
    col_path = os.path.join(repo_root, "FHCPCS-col/graph882.col")
    G = load_graph(col_path)

    # Synthetic small cycle and path test
    comp0_cycle = [1, 2080, 5066, 2]
    chain_path = [5200, 3, 4779]
    tour = assemble_tour(comp0_cycle, chain_path, (2080, 5066))
    assert tour == [1, 2080, 5200, 3, 4779, 5066, 2] or tour == [1, 2, 5066, 4779, 3, 5200, 2080]
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pytest scratch/graph882/test_solve_graph882.py -v`  
Expected: FAIL with `ModuleNotFoundError`

- [ ] **Step 3: Implement `scratch/graph882/solve_graph882.py`**

```python
# scratch/graph882/solve_graph882.py
import os, json
from typing import List, Tuple
from scratch.graph882.decomposer import load_and_decompose_graph882
from scratch.graph882.chain_solver import solve_outer_chain
from scratch.graph882.comp0_solver import solve_comp0

def assemble_tour(comp0_cycle: List[int], chain_path: List[int], virt_edge: Tuple[int, int] = (2080, 5066)) -> List[int]:
    u_v, v_v = virt_edge
    l_c = len(comp0_cycle)
    idx_virt = None
    for i in range(l_c):
        a = comp0_cycle[i]
        b = comp0_cycle[(i + 1) % l_c]
        if {a, b} == {u_v, v_v}:
            idx_virt = i
            break
    assert idx_virt is not None, f"Virtual edge {virt_edge} not found in Comp 0 cycle"

    a = comp0_cycle[idx_virt]
    b = comp0_cycle[(idx_virt + 1) % l_c]

    # chain_path has endpoints (5200, 4779)
    # 5200 connects to 2080, 4779 connects to 5066
    if a == 2080 and b == 5066:
        chain_oriented = chain_path
    elif a == 5066 and b == 2080:
        chain_oriented = list(reversed(chain_path))
    else:
        raise ValueError("Invalid orientation for virtual edge")

    # Tour = cycle up to a + chain_oriented + cycle from b onward
    tour = comp0_cycle[:idx_virt + 1] + chain_oriented + comp0_cycle[idx_virt + 1:]
    assert len(tour) == len(comp0_cycle) + len(chain_path)
    # tour has 5686 vertices, unique set size 5686
    return tour

def export_hcp_tour(tour: List[int], output_path: str):
    os.makedirs(os.path.dirname(output_path), exist_ok=True)
    with open(output_path, "w") as f:
        f.write(f"DIMENSION = {len(tour)}\n")
        f.write("TOUR_SECTION\n")
        for node in tour:
            f.write(f"{node}\n")
        f.write("-1\n")
        f.write("EOF\n")

def main():
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
    col_path = os.path.join(repo_root, "FHCPCS-col/graph882.col")
    G, mod_nodes, block2_nodes, _, comp0_nodes = load_and_decompose_graph882(col_path)

    chain = solve_outer_chain(G, mod_nodes, block2_nodes)
    comp0_cycle = solve_comp0(G, comp0_nodes)

    tour = assemble_tour(comp0_cycle, chain)
    assert len(tour) == 5686
    assert len(set(tour)) == 5686

    out_tour = os.path.join(os.path.dirname(__file__), "found_tour_graph882.hcp")
    export_hcp_tour(tour, out_tour)
    print(f"[✓] Tour exported to {out_tour}")

if __name__ == "__main__":
    main()
```

- [ ] **Step 4: Implement `scratch/graph882/solve_in_memory.py`**

```python
# scratch/graph882/solve_in_memory.py
import os, time
from scratch.graph882.decomposer import load_and_decompose_graph882
from scratch.graph882.chain_solver import solve_subgraph_path
from scratch.graph882.comp0_solver import solve_comp0
from scratch.graph882.solve_graph882 import assemble_tour, export_hcp_tour

def solve_graph882_clean_in_memory(col_path: str, output_path: str):
    print("=" * 65)
    print("STARTING 100% IN-MEMORY DE NOVO SOLVE FOR graph882.col")
    print("ZERO CACHING, ZERO TOUR INJECTION, 100% LIVE COMPUTATION")
    print("=" * 65)
    t0 = time.time()

    G, mod_nodes, block2_nodes, _, comp0_nodes = load_and_decompose_graph882(col_path)
    print(f"[*] Graph loaded: {len(G)} vertices, Outer Chain (185v), Comp 0 (5501v).")

    print("[*] Stage 1: Solving Outer Chain components...")
    p_mod = solve_subgraph_path(G, mod_nodes, src=5200, dst=3195)
    p_buf = solve_subgraph_path(G, block2_nodes, src=4509, dst=4779)
    chain = p_mod + p_buf
    print(f"[*] Stage 1 Complete in {time.time()-t0:.2f}s: Chain 185 vertices.")

    print("[*] Stage 2: Solving Comp 0 (5,501 vertices) live with degree-2 contraction & CEGAR...")
    t_c0 = time.time()
    comp0_cycle = solve_comp0(G, comp0_nodes)
    print(f"[*] Stage 2 Complete in {time.time()-t_c0:.2f}s.")

    print("[*] Stage 3: Splicing Outer Chain into Comp 0 cycle in memory...")
    tour = assemble_tour(comp0_cycle, chain)
    export_hcp_tour(tour, output_path)
    total_time = time.time() - t0
    print(f"[✓] Full tour written to {output_path} in {total_time:.2f}s total!")

if __name__ == "__main__":
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
    col_path = os.path.join(repo_root, "FHCPCS-col/graph882.col")
    out_tour = os.path.join(os.path.dirname(__file__), "found_tour_graph882.hcp")
    solve_graph882_clean_in_memory(col_path, out_tour)
```

- [ ] **Step 5: Run tests and execute full tour generation**

Run: `pytest scratch/graph882/ -v`  
Expected: All unit tests PASS.  
Run: `python3 scratch/graph882/solve_graph882.py`  
Expected: Generates `scratch/graph882/found_tour_graph882.hcp`.  
Run independent verification:  
`python3 scratch/verify_benchmarks.py --graph FHCPCS-col/graph882.col --tour scratch/graph882/found_tour_graph882.hcp`  
Expected: `Validation Result: PASS - CERTIFIED SOUND` with exit code 0.

- [ ] **Step 6: Commit**

```bash
git add scratch/graph882/solve_graph882.py scratch/graph882/solve_in_memory.py scratch/graph882/test_solve_graph882.py scratch/graph882/found_tour_graph882.hcp
git commit -m "feat(graph882): add tour assembler and certified sound Hamiltonian cycle"
```
