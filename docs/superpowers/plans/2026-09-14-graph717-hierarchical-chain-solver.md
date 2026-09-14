# Hierarchical Linear-Chain & Central-Block Solver for graph717.col Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and execute a certified hierarchical linear-chain and central-block solver for `FHCPCS-col/graph717.col` ($N = 4,122, M = 7,638$), producing a 100% verified sound Hamiltonian cycle in `scratch/graph717/found_tour_graph717.hcp` in $\le 1,800$s without tour injection.

**Architecture:** Decompose graph into two 509-vertex linear chains (each comprising three 167-vertex modules) and one 3,108-vertex central block Comp 0; solve the 6 modules with CaDiCaL CEGAR and assemble Chain 1 and Chain 2; solve Comp 0 via degree-2 contraction (922 vertices contracted) + 376 static triangle cuts + CaDiCaL CEGAR with virtual edges; deterministically splice chains into the central cycle.

**Tech Stack:** Python 3, PySAT (CaDiCaL 1.9.5), multiprocessing.

## Global Constraints

- Target Graph: `FHCPCS-col/graph717.col` ($N = 4,122, M = 7,638$).
- Zero Tour Injection: Never read, preload, or inspect `.tou` reference files (e.g. `FHCPCS-col/graph717.tou`).
- Zero Phantom Edges: Every edge in the tour must exist in `G[u]` from raw `graph717.col`.
- Complete Tour Certification: Independent validation using `python3 scratch/verify_benchmarks.py --graph FHCPCS-col/graph717.col --tour scratch/graph717/found_tour_graph717.hcp` must report `Validation Result: PASS - CERTIFIED SOUND` with exit code 0.
- Runtime limit: $\le 1,800$s.

---

### Task 1: Graph Loading, 16-Cut Decomposition & Partition Invariant Verification

**Files:**
- Create: `scratch/graph717/decomposer.py`
- Test: `scratch/graph717/test_decomposer.py`

**Interfaces:**
- Consumes: Raw graph `FHCPCS-col/graph717.col`
- Produces: `load_and_decompose_graph717(col_path: str) -> Tuple[Dict[int, Set[int]], Dict[Tuple[int, int], Set[int]], Set[int], Set[int], Set[int]]`
  where:
  - `G`: full adjacency dictionary ($|V| = 4,122, |E| = 7,638$).
  - `modules`: dict mapping each module's port pair `(u, v)` to its set of 167 internal vertices.
  - `chain1_nodes`: set of 509 vertices in Chain 1.
  - `chain2_nodes`: set of 509 vertices in Chain 2.
  - `comp0_nodes`: set of 3,108 vertices in Comp 0 (including 4 interface ports).

- [ ] **Step 1: Write the failing test**

```python
# scratch/graph717/test_decomposer.py
import os, pytest
from scratch.graph717.decomposer import load_and_decompose_graph717

def test_decomposer_invariants():
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
    col_path = os.path.join(repo_root, "FHCPCS-col/graph717.col")
    G, modules, chain1_nodes, chain2_nodes, comp0_nodes = load_and_decompose_graph717(col_path)

    # 1. Graph sizes
    assert len(G) == 4122
    num_edges = sum(len(nbrs) for nbrs in G.values()) // 2
    assert num_edges == 7638

    # 2. Module count and sizes
    assert len(modules) == 6
    for ports, mod in modules.items():
        assert len(mod) == 167
        u, v = ports
        assert u in G and v in G
        assert len(G[u] & mod) == 13
        assert len(G[v] & mod) == 13

    # 3. Chain and Comp 0 sizes
    assert len(chain1_nodes) == 509
    assert len(chain2_nodes) == 509
    assert len(comp0_nodes) == 3108

    # 4. Overlaps and union
    assert chain1_nodes & chain2_nodes == set()
    ports_c0 = {255, 1955, 2609, 3358}
    assert (chain1_nodes | chain2_nodes) & comp0_nodes == ports_c0
    assert (chain1_nodes | chain2_nodes | comp0_nodes) == set(G.keys())
    assert len(chain1_nodes) + len(chain2_nodes) + len(comp0_nodes) - 4 == 4122

    # 5. Bridge edges
    assert 1389 in G[255]
    assert 1213 in G[2677]
    assert 773 in G[2681]
    assert 1955 in G[702]

    assert 3986 in G[3358]
    assert 1177 in G[1016]
    assert 577 in G[2467]
    assert 2609 in G[540]
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pytest scratch/graph717/test_decomposer.py -v`
Expected: FAIL with `ModuleNotFoundError: No module named 'scratch.graph717.decomposer'`

- [ ] **Step 3: Write implementation**

```python
# scratch/graph717/decomposer.py
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

def load_and_decompose_graph717(col_path: str) -> Tuple[Dict[int, Set[int]], Dict[Tuple[int, int], Set[int]], Set[int], Set[int], Set[int]]:
    G = load_graph(col_path)
    assert len(G) == 4122

    cut_nodes = {255, 540, 577, 702, 773, 1016, 1177, 1213, 1389, 1955, 2467, 2609, 2677, 2681, 3358, 3986}
    rem_16 = set(G.keys()) - cut_nodes

    visited = set()
    comps = []
    for u in rem_16:
        if u not in visited:
            c = []
            q = [u]
            visited.add(u)
            for x in q:
                c.append(x)
                for nbr in G[x]:
                    if nbr in rem_16 and nbr not in visited:
                        visited.add(nbr)
                        q.append(nbr)
            comps.append(set(c))

    assert len(comps) == 7

    modules = {}
    comp0_internal = None
    for c in comps:
        if len(c) == 3104:
            comp0_internal = c
        elif len(c) == 167:
            ports = tuple(sorted([nbr for u in c for nbr in G[u] if nbr in cut_nodes]))
            ports_set = tuple(sorted(list(set(ports))))
            assert len(ports_set) == 2
            modules[ports_set] = c

    assert len(modules) == 6
    assert comp0_internal is not None

    ports_c0 = {255, 1955, 2609, 3358}
    comp0_nodes = comp0_internal | ports_c0

    # Chain 1: modules (1389, 2677), (1213, 2681), (702, 773) + intermediate cut nodes + ports 255, 1955
    mod2 = modules[tuple(sorted([1389, 2677]))]
    mod4 = modules[tuple(sorted([1213, 2681]))]
    mod6 = modules[tuple(sorted([702, 773]))]
    chain1_nodes = mod2 | mod4 | mod6 | {255, 1389, 2677, 1213, 2681, 773, 702, 1955}
    assert len(chain1_nodes) == 509

    # Chain 2: modules (1016, 3986), (1177, 2467), (540, 577) + intermediate cut nodes + ports 3358, 2609
    mod1 = modules[tuple(sorted([1016, 3986]))]
    mod5 = modules[tuple(sorted([1177, 2467]))]
    mod3 = modules[tuple(sorted([540, 577]))]
    chain2_nodes = mod1 | mod5 | mod3 | {3358, 3986, 1016, 1177, 2467, 577, 540, 2609}
    assert len(chain2_nodes) == 509

    return G, modules, chain1_nodes, chain2_nodes, comp0_nodes
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pytest scratch/graph717/test_decomposer.py -v`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add scratch/graph717/decomposer.py scratch/graph717/test_decomposer.py
git commit -m "feat(graph717): add graph decomposer and 16-cut verification test"
```

---

### Task 2: Modular Solvers & Chain Construction

**Files:**
- Create: `scratch/graph717/chain_solver.py`
- Test: `scratch/graph717/test_chain_solver.py`
- Output: `scratch/graph717/chains.json`

**Interfaces:**
- Consumes: `load_and_decompose_graph717` from `scratch/graph717/decomposer.py`
- Produces: `solve_and_assemble_chains(col_path: str, cache_path: str) -> Dict[str, List[int]]`
  where `scratch/graph717/chains.json` contains:
  - `"chain1"`: list of 509 unique vertices starting at 255 and ending at 1955.
  - `"chain2"`: list of 509 unique vertices starting at 3358 and ending at 2609.

- [ ] **Step 1: Write the failing test**

```python
# scratch/graph717/test_chain_solver.py
import json, os, pytest
from scratch.graph717.decomposer import load_and_decompose_graph717

def test_cached_chains_valid():
    base_dir = os.path.dirname(os.path.abspath(__file__))
    cache_file = os.path.join(base_dir, "chains.json")
    assert os.path.exists(cache_file), "chains.json missing, run chain_solver.py first"

    repo_root = os.path.abspath(os.path.join(base_dir, "../.."))
    col_path = os.path.join(repo_root, "FHCPCS-col/graph717.col")
    G, _, chain1_nodes, chain2_nodes, _ = load_and_decompose_graph717(col_path)

    with open(cache_file, "r") as f:
        data = json.load(f)

    assert "chain1" in data and "chain2" in data
    c1 = data["chain1"]
    c2 = data["chain2"]

    # Chain 1 assertions
    assert len(c1) == 509
    assert len(set(c1)) == 509
    assert set(c1) == chain1_nodes
    assert c1[0] == 255 and c1[-1] == 1955
    for i in range(len(c1) - 1):
        assert c1[i+1] in G[c1[i]], f"Phantom edge in c1: ({c1[i]}, {c1[i+1]})"

    # Chain 2 assertions
    assert len(c2) == 509
    assert len(set(c2)) == 509
    assert set(c2) == chain2_nodes
    assert c2[0] == 3358 and c2[-1] == 2609
    for i in range(len(c2) - 1):
        assert c2[i+1] in G[c2[i]], f"Phantom edge in c2: ({c2[i]}, {c2[i+1]})"
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pytest scratch/graph717/test_chain_solver.py -v`
Expected: FAIL with `AssertionError: chains.json missing`

- [ ] **Step 3: Implement `scratch/graph717/chain_solver.py`**

```python
# scratch/graph717/chain_solver.py
import collections, json, multiprocessing, os, time
from typing import Dict, List, Set, Tuple
from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType
from scratch.graph717.decomposer import load_and_decompose_graph717

def solve_module_path(G: Dict[int, Set[int]], mod: Set[int], u_port: int, v_port: int) -> List[int]:
    """
    Solve Hamiltonian path in G[mod + {u, v}] between u_port and v_port.
    Returns path of 169 vertices starting at u_port and ending at v_port.
    """
    t0 = time.time()
    V_mod = mod | {u_port, v_port}
    virt = tuple(sorted([u_port, v_port]))
    edges = set()
    for u in V_mod:
        for v in G[u]:
            if v in V_mod and u < v:
                edges.add((u, v))
    edges.add(virt)
    edge_list = sorted(list(edges))
    edge_to_var = {e: i + 1 for i, e in enumerate(edge_list)}
    inc_edges = collections.defaultdict(list)
    for e in edge_list:
        inc_edges[e[0]].append(edge_to_var[e])
        inc_edges[e[1]].append(edge_to_var[e])

    solver = Cadical195()
    top = len(edge_list) + 1

    for u in V_mod:
        lits = inc_edges[u]
        clauses = CardEnc.equals(lits=lits, bound=2, top_id=top, encoding=EncType.cardnetwrk)
        for cl in clauses:
            solver.add_clause(cl)
            for lit in cl:
                top = max(top, abs(lit) + 1)

    solver.add_clause([edge_to_var[virt]])

    it = 0
    while True:
        it += 1
        if not solver.solve():
            raise RuntimeError(f"Module ({u_port}, {v_port}) UNSAT!")
        model = set(solver.get_model())
        active = [e for e in edge_list if edge_to_var[e] in model]
        adj = collections.defaultdict(list)
        for u, v in active:
            adj[u].append(v); adj[v].append(u)

        visited = set()
        cycles = []
        for u in V_mod:
            if u not in visited:
                cyc = []
                curr, prev = u, None
                while curr not in visited:
                    visited.add(curr); cyc.append(curr)
                    nbrs = adj[curr]
                    nxt = nbrs[0] if nbrs[0] != prev else nbrs[1]
                    prev, curr = curr, nxt
                cycles.append(cyc)

        if len(cycles) == 1:
            cyc = cycles[0]
            n = len(cyc)
            idx_u = cyc.index(u_port)
            if cyc[(idx_u + 1) % n] == v_port:
                cyc = list(reversed(cyc))
                idx_u = cyc.index(u_port)
            assert cyc[(idx_u - 1) % n] == v_port
            path = cyc[idx_u:] + cyc[:idx_u]
            assert path[0] == u_port and path[-1] == v_port and len(path) == 169
            solver.delete()
            return path

        for cyc in cycles:
            neg_clause = [-edge_to_var[tuple(sorted([cyc[i], cyc[(i+1)%len(cyc)]]))] for i in range(len(cyc))]
            solver.add_clause(neg_clause)

def _worker_mod(args):
    col_path, mod_ports, u_start, v_end = args
    G, modules, _, _, _ = load_and_decompose_graph717(col_path)
    mod = modules[mod_ports]
    p = solve_module_path(G, mod, u_start, v_end)
    return mod_ports, p

def solve_and_assemble_chains(col_path: str, cache_path: str) -> Dict[str, List[int]]:
    if os.path.exists(cache_path):
        with open(cache_path, "r") as f:
            data = json.load(f)
        if "chain1" in data and "chain2" in data and len(data["chain1"]) == 509 and len(data["chain2"]) == 509:
            print("Both chains already solved and cached!")
            return data

    G, modules, _, _, _ = load_and_decompose_graph717(col_path)

    # 6 module tasks: (col_path, mod_ports, u_start, v_end)
    tasks = [
        (col_path, tuple(sorted([1389, 2677])), 1389, 2677),
        (col_path, tuple(sorted([1213, 2681])), 1213, 2681),
        (col_path, tuple(sorted([702, 773])), 773, 702),
        (col_path, tuple(sorted([1016, 3986])), 3986, 1016),
        (col_path, tuple(sorted([1177, 2467])), 1177, 2467),
        (col_path, tuple(sorted([540, 577])), 577, 540),
    ]

    print("Solving all 6 modules in parallel using multiprocessing...")
    solved_mods = {}
    with multiprocessing.Pool(processes=min(6, os.cpu_count() or 4)) as pool:
        for mod_ports, path in pool.map(_worker_mod, tasks):
            solved_mods[mod_ports] = path
            print(f"Module {mod_ports} solved: {len(path)} vertices.")

    # Assemble Chain 1: 255 -> [1389..2677] -> [1213..2681] -> [773..702] -> 1955
    p_m2 = solved_mods[tuple(sorted([1389, 2677]))]
    if p_m2[0] != 1389: p_m2 = list(reversed(p_m2))
    p_m4 = solved_mods[tuple(sorted([1213, 2681]))]
    if p_m4[0] != 1213: p_m4 = list(reversed(p_m4))
    p_m6 = solved_mods[tuple(sorted([702, 773]))]
    if p_m6[0] != 773: p_m6 = list(reversed(p_m6))

    chain1 = [255] + p_m2 + p_m4 + p_m6 + [1955]
    assert len(chain1) == 509

    # Assemble Chain 2: 3358 -> [3986..1016] -> [1177..2467] -> [577..540] -> 2609
    p_m1 = solved_mods[tuple(sorted([1016, 3986]))]
    if p_m1[0] != 3986: p_m1 = list(reversed(p_m1))
    p_m5 = solved_mods[tuple(sorted([1177, 2467]))]
    if p_m5[0] != 1177: p_m5 = list(reversed(p_m5))
    p_m3 = solved_mods[tuple(sorted([540, 577]))]
    if p_m3[0] != 577: p_m3 = list(reversed(p_m3))

    chain2 = [3358] + p_m1 + p_m5 + p_m3 + [2609]
    assert len(chain2) == 509

    res = {"chain1": chain1, "chain2": chain2}
    os.makedirs(os.path.dirname(os.path.abspath(cache_path)), exist_ok=True)
    with open(cache_path, "w") as f:
        json.dump(res, f)
    print(f"Saved both chains to {cache_path}.")
    return res

if __name__ == "__main__":
    base_dir = os.path.dirname(os.path.abspath(__file__))
    repo_root = os.path.abspath(os.path.join(base_dir, "../.."))
    col_path = os.path.join(repo_root, "FHCPCS-col/graph717.col")
    cache_path = os.path.join(base_dir, "chains.json")
    solve_and_assemble_chains(col_path, cache_path)
```

- [ ] **Step 4: Execute `chain_solver.py` and run test to verify it passes**

Run: `python3 scratch/graph717/chain_solver.py`
Run: `pytest scratch/graph717/test_chain_solver.py -v`
Expected: PASS (both chains valid on raw DIMACS edge list)

- [ ] **Step 5: Commit**

```bash
git add scratch/graph717/chain_solver.py scratch/graph717/test_chain_solver.py scratch/graph717/chains.json
git commit -m "feat(graph717): add modular chain solver and verified linear chains cache"
```

---

### Task 3: Central Block Comp 0 Solver (Contraction + CEGAR)

**Files:**
- Create: `scratch/graph717/comp0_solver.py`
- Test: `scratch/graph717/test_comp0_solver.py`
- Output: `scratch/graph717/comp0_cycle.json`

**Interfaces:**
- Consumes: `load_and_decompose_graph717`
- Produces: `solve_comp0(col_path: str, cache_path: str) -> List[int]`
  where `scratch/graph717/comp0_cycle.json` contains:
  - list of 3,108 unique vertices forming a Hamiltonian cycle in $G[\text{Comp0} \cup \text{ports}] \cup \{(255, 1955), (3358, 2609)\}$, containing both virtual edges $(255, 1955)$ and $(3358, 2609)$.

- [ ] **Step 1: Write the failing test**

```python
# scratch/graph717/test_comp0_solver.py
import json, os, pytest
from scratch.graph717.decomposer import load_and_decompose_graph717

def test_cached_comp0_cycle_valid():
    base_dir = os.path.dirname(os.path.abspath(__file__))
    cache_file = os.path.join(base_dir, "comp0_cycle.json")
    assert os.path.exists(cache_file), "comp0_cycle.json missing, run comp0_solver.py first"

    repo_root = os.path.abspath(os.path.join(base_dir, "../.."))
    col_path = os.path.join(repo_root, "FHCPCS-col/graph717.col")
    G, _, _, _, comp0_nodes = load_and_decompose_graph717(col_path)

    with open(cache_file, "r") as f:
        cyc = json.load(f)

    assert len(cyc) == 3108
    assert len(set(cyc)) == 3108
    assert set(cyc) == comp0_nodes

    # Check that both virtual edges (255, 1955) and (3358, 2609) exist in the cycle
    virt1 = tuple(sorted([255, 1955]))
    virt2 = tuple(sorted([3358, 2609]))
    n = len(cyc)
    cycle_edges = {tuple(sorted([cyc[i], cyc[(i+1)%n]])) for i in range(n)}
    assert virt1 in cycle_edges, "Virtual edge (255, 1955) missing from Comp 0 cycle"
    assert virt2 in cycle_edges, "Virtual edge (3358, 2609) missing from Comp 0 cycle"

    # All other edges must exist in raw G
    for e in cycle_edges - {virt1, virt2}:
        assert e[1] in G[e[0]], f"Phantom edge in comp0 cycle: {e}"
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pytest scratch/graph717/test_comp0_solver.py -v`
Expected: FAIL with `AssertionError: comp0_cycle.json missing`

- [ ] **Step 3: Implement `scratch/graph717/comp0_solver.py`**

```python
# scratch/graph717/comp0_solver.py
import collections, json, os, time
from typing import Dict, List, Set, Tuple
from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType
from scratch.graph717.decomposer import load_and_decompose_graph717

def solve_comp0(col_path: str, cache_path: str) -> List[int]:
    if os.path.exists(cache_path):
        with open(cache_path, "r") as f:
            cyc = json.load(f)
        if len(cyc) == 3108 and len(set(cyc)) == 3108:
            print("Comp 0 cycle already cached!")
            return cyc

    t0 = time.time()
    G, _, _, _, comp0_nodes = load_and_decompose_graph717(col_path)
    ports = {255, 1955, 2609, 3358}

    adj_c0 = collections.defaultdict(set)
    for u in comp0_nodes:
        for v in G[u]:
            if v in comp0_nodes:
                adj_c0[u].add(v)

    # Virtual edges
    adj_c0[255].add(1955); adj_c0[1955].add(255)
    adj_c0[3358].add(2609); adj_c0[2609].add(3358)

    # Degree-2 recursive contraction (excluding the 4 ports)
    rem = set(comp0_nodes)
    edge_chains = {}
    while True:
        d2 = [u for u in rem if u not in ports and len(adj_c0[u]) == 2]
        if not d2:
            break
        v = d2[0]
        u, w = list(adj_c0[v])
        adj_c0[u].remove(v); adj_c0[w].remove(v)
        del adj_c0[v]; rem.remove(v)
        e_uv = tuple(sorted([u, v]))
        e_vw = tuple(sorted([v, w]))
        chain_uv = edge_chains.pop(e_uv, [u, v])
        chain_vw = edge_chains.pop(e_vw, [v, w])
        if chain_uv[-1] != v: chain_uv = list(reversed(chain_uv))
        if chain_vw[0] != v: chain_vw = list(reversed(chain_vw))
        merged_chain = chain_uv[:-1] + chain_vw
        e_uw = tuple(sorted([u, w]))
        adj_c0[u].add(w); adj_c0[w].add(u)
        edge_chains[e_uw] = merged_chain

    print(f"[Comp 0] Contracted: {len(comp0_nodes)} -> {len(rem)} vertices ({len(edge_chains)} contracted chains).")
    assert len(rem) == 2186

    contracted_edges = set(edge_chains.keys())
    edges = set()
    for u in rem:
        for v in adj_c0[u]:
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

    # Exactly-2 degree constraints
    for u in rem:
        lits = inc_edges[u]
        clauses = CardEnc.equals(lits=lits, bound=2, top_id=top, encoding=EncType.cardnetwrk)
        for cl in clauses:
            solver.add_clause(cl)
            for lit in cl:
                top = max(top, abs(lit) + 1)

    # Force virtual edges
    virt1 = tuple(sorted([255, 1955]))
    virt2 = tuple(sorted([3358, 2609]))
    solver.add_clause([edge_to_var[virt1]])
    solver.add_clause([edge_to_var[virt2]])

    # Force all contracted edges
    for ce in contracted_edges:
        solver.add_clause([edge_to_var[ce]])

    # Static chordless triangle cuts in contracted graph
    triangles = []
    rem_list = sorted(list(rem))
    for u in rem_list:
        for v in adj_c0[u]:
            if v > u:
                for w in adj_c0[v]:
                    if w > v and w in adj_c0[u]:
                        triangles.append((u, v, w))
    for u, v, w in triangles:
        e1 = tuple(sorted([u, v]))
        e2 = tuple(sorted([v, w]))
        e3 = tuple(sorted([w, u]))
        solver.add_clause([-edge_to_var[e1], -edge_to_var[e2], -edge_to_var[e3]])

    print(f"[Comp 0] Added {len(triangles)} static triangle cuts. Starting CEGAR loop...")
    it = 0
    while True:
        it += 1
        t_start_it = time.time()
        if not solver.solve():
            raise RuntimeError("Comp 0 UNSAT!")
        model = set(solver.get_model())
        active = [e for e in edge_list if edge_to_var[e] in model]
        adj = collections.defaultdict(list)
        for u, v in active:
            adj[u].append(v); adj[v].append(u)

        visited = set()
        cycles = []
        for u in rem:
            if u not in visited:
                cyc = []
                curr, prev = u, None
                while curr not in visited:
                    visited.add(curr); cyc.append(curr)
                    nbrs = adj[curr]
                    nxt = nbrs[0] if nbrs[0] != prev else nbrs[1]
                    prev, curr = curr, nxt
                cycles.append(cyc)

        if len(cycles) == 1:
            print(f"[Comp 0] Converged at iter {it} in {time.time()-t0:.2f}s! ({len(cycles[0])} contracted vertices)")
            cyc = cycles[0]
            # Expand contracted chains
            expanded = []
            n = len(cyc)
            for i in range(n):
                u = cyc[i]
                v = cyc[(i + 1) % n]
                e = tuple(sorted([u, v]))
                if e in edge_chains:
                    chain = edge_chains[e]
                    if chain[0] != u: chain = list(reversed(chain))
                    expanded.extend(chain[:-1])
                else:
                    expanded.append(u)

            print(f"[Comp 0] Expanded cycle length: {len(expanded)} (expected 3108).")
            assert len(expanded) == 3108
            assert set(expanded) == comp0_nodes
            solver.delete()

            os.makedirs(os.path.dirname(os.path.abspath(cache_path)), exist_ok=True)
            with open(cache_path, "w") as f:
                json.dump(expanded, f)
            print(f"Saved Comp 0 cycle to {cache_path}.")
            return expanded

        if it % 10 == 0 or len(cycles) <= 5:
            cyc_lens = sorted([len(c) for c in cycles], reverse=True)
            print(f"[Comp 0] Iter {it} ({time.time()-t_start_it:.2f}s): {len(cycles)} cycles. Max: {cyc_lens[0]}, Min: {cyc_lens[-1]}")

        for cyc in cycles:
            cut_lits = []
            cyc_set = set(cyc)
            for u in cyc:
                for v in adj_c0[u]:
                    if v not in cyc_set:
                        e = tuple(sorted([u, v]))
                        cut_lits.append(edge_to_var[e])
            if cut_lits:
                clauses = CardEnc.atleast(lits=cut_lits, bound=2, top_id=top, encoding=EncType.cardnetwrk)
                for cl in clauses:
                    solver.add_clause(cl)
                    for lit in cl:
                        top = max(top, abs(lit) + 1)
            else:
                neg_clause = [-edge_to_var[tuple(sorted([cyc[i], cyc[(i+1)%len(cyc)]]))] for i in range(len(cyc))]
                solver.add_clause(neg_clause)

if __name__ == "__main__":
    base_dir = os.path.dirname(os.path.abspath(__file__))
    repo_root = os.path.abspath(os.path.join(base_dir, "../.."))
    col_path = os.path.join(repo_root, "FHCPCS-col/graph717.col")
    cache_path = os.path.join(base_dir, "comp0_cycle.json")
    solve_comp0(col_path, cache_path)
```

- [ ] **Step 4: Run `comp0_solver.py` and run test to verify it passes**

Run: `python3 scratch/graph717/comp0_solver.py`
Run: `pytest scratch/graph717/test_comp0_solver.py -v`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add scratch/graph717/comp0_solver.py scratch/graph717/test_comp0_solver.py scratch/graph717/comp0_cycle.json
git commit -m "feat(graph717): add Comp 0 solver with degree-2 contraction and cycle cache"
```

---

### Task 4: Full Tour Assembly & Independent Soundness Certification

**Files:**
- Create: `scratch/graph717/solve_graph717.py`
- Output: `scratch/graph717/found_tour_graph717.hcp`
- Test: `scratch/verify_benchmarks.py`

**Interfaces:**
- Consumes: `scratch/graph717/chains.json` and `scratch/graph717/comp0_cycle.json`
- Produces: `scratch/graph717/found_tour_graph717.hcp`
- Verification: `python3 scratch/verify_benchmarks.py --graph FHCPCS-col/graph717.col --tour scratch/graph717/found_tour_graph717.hcp`

- [ ] **Step 1: Implement tour assembler**

Create `scratch/graph717/solve_graph717.py`:
```python
# scratch/graph717/solve_graph717.py
import json, os, sys, time
from scratch.graph717.decomposer import load_and_decompose_graph717
from scratch.graph717.chain_solver import solve_and_assemble_chains
from scratch.graph717.comp0_solver import solve_comp0

def assemble_and_verify_tour():
    t0 = time.time()
    base_dir = os.path.dirname(os.path.abspath(__file__))
    repo_root = os.path.abspath(os.path.join(base_dir, "../.."))
    col_path = os.path.join(repo_root, "FHCPCS-col/graph717.col")
    chains_path = os.path.join(base_dir, "chains.json")
    comp0_path = os.path.join(base_dir, "comp0_cycle.json")
    tour_path = os.path.join(base_dir, "found_tour_graph717.hcp")

    G, _, _, _, _ = load_and_decompose_graph717(col_path)
    chains = solve_and_assemble_chains(col_path, chains_path)
    comp0_cyc = solve_comp0(col_path, comp0_path)

    c1 = chains["chain1"]  # 255 -> ... -> 1955 (509 vertices)
    c2 = chains["chain2"]  # 3358 -> ... -> 2609 (509 vertices)

    # In comp0_cyc, find virtual edges (255, 1955) and (3358, 2609)
    # Replace the virtual edge with the chain interior
    # Build final tour
    n = len(comp0_cyc)
    final_tour = []
    i = 0
    while i < n:
        u = comp0_cyc[i]
        v = comp0_cyc[(i + 1) % n]
        e = tuple(sorted([u, v]))
        if e == tuple(sorted([255, 1955])):
            # splice chain 1
            if c1[0] == u:
                splice = c1[:-1]
            else:
                splice = list(reversed(c1))[:-1]
            final_tour.extend(splice)
        elif e == tuple(sorted([3358, 2609])):
            # splice chain 2
            if c2[0] == u:
                splice = c2[:-1]
            else:
                splice = list(reversed(c2))[:-1]
            final_tour.extend(splice)
        else:
            final_tour.append(u)
        i += 1

    assert len(final_tour) == 4122
    assert len(set(final_tour)) == 4122
    assert set(final_tour) == set(G.keys())

    # Verify all edges exist in raw G
    for i in range(len(final_tour)):
        u = final_tour[i]
        v = final_tour[(i + 1) % len(final_tour)]
        assert v in G[u], f"Phantom edge in tour: ({u}, {v})"

    print(f"Tour verified sound: 4122 unique vertices, 4122 raw DIMACS edges!")

    # Write HCP format
    with open(tour_path, "w") as f:
        f.write("NAME : graph717.col\n")
        f.write("TYPE : TOUR\n")
        f.write(f"DIMENSION : {len(final_tour)}\n")
        f.write("TOUR_SECTION\n")
        for node in final_tour:
            f.write(f"{node}\n")
        f.write("-1\n")
        f.write("EOF\n")

    print(f"Tour written to {tour_path} in {time.time()-t0:.2f}s.")
    return tour_path

if __name__ == "__main__":
    assemble_and_verify_tour()
```

- [ ] **Step 2: Run `solve_graph717.py` to generate the tour**

Run: `python3 scratch/graph717/solve_graph717.py`
Expected: Tour assembled and written to `scratch/graph717/found_tour_graph717.hcp`.

- [ ] **Step 3: Run independent benchmark verification**

Run: `python3 scratch/verify_benchmarks.py --graph FHCPCS-col/graph717.col --tour scratch/graph717/found_tour_graph717.hcp`
Expected:
`Validation Result: PASS - CERTIFIED SOUND`
`Exit code: 0`

- [ ] **Step 4: Commit**

```bash
git add scratch/graph717/solve_graph717.py scratch/graph717/found_tour_graph717.hcp
git commit -m "feat(graph717): add full tour assembler and certified sound Hamiltonian cycle"
```
