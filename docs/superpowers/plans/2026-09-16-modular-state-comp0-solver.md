# Modular State Equivalence Comp 0 Solver Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a generic, sound, and propagation-efficient Modular State Equivalence Solver in Python for the 5 large FHCP challenge graphs (`graph965.col`, `graph966.col`, `graph971.col`, `graph994.col`, `graph998.col`), reducing Comp 0 search space from $2^{5000}$ edges to $k$ boolean selector variables, achieving 100% de novo in-memory solves within the 1800s limit.

**Architecture:** 
1. `BipartiteModuleDetector`: Identifies all 44-vertex 3-SAT variable modules and their external interface ports $(p_{\text{in}}, p_{\text{out}})$ via virtual-edge quotient clustering.
2. `ModuleDualPathExtractor`: Solves the dual canonical Hamiltonian paths $T_i$ (True) and $F_i$ (False) for each isolated module in $<0.001$s using micro-SAT.
3. `ModularComp0Solver`: Injects selector variables $b_i \in \{0, 1\}$ and equivalence clauses for $T_i, F_i$, completely eliminating intra-module cycle shattering, and drives a fast Macro-CEGAR loop.
4. `UnifiedSolver` Integration: Dynamically dispatches Comp 0 to the modular solver when $|V_{\text{core}}| > 4,600$, maintaining backward compatibility with smaller graphs.

**Tech Stack:** Python 3.13, PySAT (`pysat.solvers.Cadical195`, `pysat.card.CardEnc`), DIMACS `.col`, TSPLIB `.hcp`, pytest.

## Global Constraints

- 100% In-Memory Live Computation: Zero precomputed `.tou` / `.pkl` caching, zero tour injection.
- Verification Soundness: Every tour must achieve `PASS - CERTIFIED SOUND` on raw uncontracted graph $G$ via `scratch/verify_benchmarks.py`.
- Hard 1800s Limit: Every benchmark run must be strictly wrapped with `timeout 1800`.
- Interface Preservation: Must not break existing solved graphs (`graph882`, `graph717`, `graph668`, `graph944`).

---

### Task 1: Implement `BipartiteModuleDetector`

**Files:**
- Create: `scratch/engine/bipartite_module_detector.py`
- Test: `scratch/engine/tests/test_bipartite_module_detector.py`

**Interfaces:**
- Consumes: `G: Dict[int, Set[int]]`, `comp0_nodes: Set[int]`, `virtual_edges: List[Tuple[int, int]]`
- Produces: `detect_variable_modules(G: Dict[int, Set[int]], comp0_nodes: Set[int], virtual_edges: List[Tuple[int, int]]) -> List[Dict]`
  Each returned dict has:
  ```python
  {
      'id': int,
      'nodes': Set[int],            # 44 contracted vertices (or 66 raw vertices)
      'virtual_edges': Set[Tuple[int, int]], # 22 virtual edges inside module
      'ports': Tuple[int, int],      # (p_in, p_out) interface vertices connecting externally
      'internal_nodes': Set[int],   # 42 nodes strictly internal to the module
  }
  ```

- [ ] **Step 1: Write the failing test**

Create `scratch/engine/tests/test_bipartite_module_detector.py`:
```python
import os, pytest
from scratch.engine.graph_loader import load_dimacs
from scratch.engine.decomposer import detect_bridge_corridors
from scratch.engine.bipartite_module_detector import detect_variable_modules

def test_detect_modules_graph868():
    col_path = "FHCPCS-col/graph868.col"
    if not os.path.exists(col_path):
        pytest.skip("graph868.col not found")
    G = load_dimacs(col_path)
    corridors, c0_nodes = detect_bridge_corridors(G)
    modules = detect_variable_modules(G, set(c0_nodes), [])
    assert len(modules) == 42, f"Expected 42 modules for graph868, got {len(modules)}"
    for m in modules:
        assert len(m['nodes']) == 88 or len(m['nodes']) == 44, f"Unexpected module size: {len(m['nodes'])}"
        assert len(m['ports']) == 2, f"Module must have exactly 2 interface ports"

def test_detect_modules_graph965():
    col_path = "FHCPCS-col/graph965.col"
    if not os.path.exists(col_path):
        pytest.skip("graph965.col not found")
    G = load_dimacs(col_path)
    corridors, c0_nodes = detect_bridge_corridors(G)
    ve = [c['ext_ports'] for c in corridors]
    modules = detect_variable_modules(G, set(c0_nodes), ve)
    assert len(modules) >= 30, f"Expected >= 30 modules for graph965, got {len(modules)}"
    for m in modules:
        assert len(m['ports']) == 2
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pytest scratch/engine/tests/test_bipartite_module_detector.py -v`
Expected: FAIL with `ModuleNotFoundError: No module named 'scratch.engine.bipartite_module_detector'`

- [ ] **Step 3: Implement `bipartite_module_detector.py`**

Create `scratch/engine/bipartite_module_detector.py`:
```python
import collections
from typing import Dict, List, Set, Tuple

def detect_variable_modules(
    G: Dict[int, Set[int]],
    comp0_nodes: Set[int],
    virtual_edges: List[Tuple[int, int]]
) -> List[Dict]:
    """
    Clusters 44-vertex (or 22-vertex elementary gadget) variable modules in Comp 0.
    """
    virt_set = {tuple(sorted(e)) for e in virtual_edges}
    ports_corridor = set()
    for u, v in virt_set:
        ports_corridor.add(u)
        ports_corridor.add(v)

    c0_set = set(comp0_nodes)
    adj_c0 = {u: set(v for v in G[u] if v in c0_set) for u in comp0_nodes}
    for u, v in virt_set:
        adj_c0[u].add(v)
        adj_c0[v].add(u)

    # 1. Identify degree-2 vertices in Comp 0
    deg2 = [u for u in comp0_nodes if u not in ports_corridor and len(adj_c0[u]) == 2]
    v_partner = {}
    for v in deg2:
        nbrs = list(adj_c0[v])
        if len(nbrs) == 2:
            u, w = nbrs[0], nbrs[1]
            v_partner[u] = w
            v_partner[w] = u

    contracted_nodes = set(v_partner.keys())
    # Subgraph on contracted endpoints
    adj_contracted = {u: set(v for v in adj_c0[u] if v in contracted_nodes) for u in contracted_nodes}

    # 2. Cluster elementary gadgets (22 vertices each)
    d2_in_c = [u for u in contracted_nodes if len(adj_contracted[u]) == 2]
    gadget_adj = collections.defaultdict(set)
    for u in d2_in_c:
        for v in adj_contracted[u]:
            gadget_adj[u].add(v)
            gadget_adj[v].add(u)
    for u, w in v_partner.items():
        gadget_adj[u].add(w)
        gadget_adj[w].add(u)

    visited = set()
    elementary_gadgets = []
    for u in sorted(contracted_nodes):
        if u not in visited:
            comp = []
            q = [u]
            visited.add(u)
            for curr in q:
                comp.append(curr)
                for nxt in gadget_adj[curr]:
                    if nxt not in visited:
                        visited.add(nxt)
                        q.append(nxt)
            if len(comp) >= 10:
                elementary_gadgets.append(sorted(comp))

    # 3. Group elementary gadgets into 44-vertex modules
    node_to_eg = {}
    for gid, g_nodes in enumerate(elementary_gadgets):
        for u in g_nodes:
            node_to_eg[u] = gid

    # Connectivity between elementary gadgets
    eg_links = collections.defaultdict(collections.Counter)
    for u in contracted_nodes:
        if u in node_to_eg:
            g1 = node_to_eg[u]
            for v in adj_contracted[u]:
                if v in node_to_eg:
                    g2 = node_to_eg[v]
                    if g1 != g2:
                        eg_links[g1][g2] += 1

    # Merge pairs with maximum mutual connectivity (weight >= 4)
    merged_modules = []
    used_eg = set()
    for g1 in range(len(elementary_gadgets)):
        if g1 in used_eg:
            continue
        best_g2 = None
        best_w = 0
        for g2, w in eg_links[g1].items():
            if g2 not in used_eg and w > best_w:
                best_w = w
                best_g2 = g2
        if best_g2 is not None and best_w >= 4:
            used_eg.add(g1)
            used_eg.add(best_g2)
            merged_modules.append(sorted(elementary_gadgets[g1] + elementary_gadgets[best_g2]))
        else:
            used_eg.add(g1)
            merged_modules.append(elementary_gadgets[g1])

    # 4. Form final module dicts and identify interface ports
    modules = []
    for mid, m_nodes in enumerate(merged_modules):
        m_set = set(m_nodes)
        # Interface ports are nodes with neighbors outside m_set in adj_c0
        ext_ports = []
        int_nodes = set()
        for u in m_nodes:
            ext_nbrs = [v for v in adj_c0[u] if v not in m_set]
            if ext_nbrs:
                ext_ports.append(u)
            else:
                int_nodes.add(u)
        
        # Virtual edges in module
        m_ves = set()
        for u in m_nodes:
            if u in v_partner and v_partner[u] in m_set and u < v_partner[u]:
                m_ves.add((u, v_partner[u]))

        modules.append({
            'id': mid,
            'nodes': set(m_nodes),
            'virtual_edges': m_ves,
            'ports': tuple(sorted(ext_ports[:2])) if len(ext_ports) >= 2 else tuple(sorted(ext_ports)),
            'internal_nodes': int_nodes,
        })

    return modules
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pytest scratch/engine/tests/test_bipartite_module_detector.py -v`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add scratch/engine/bipartite_module_detector.py scratch/engine/tests/test_bipartite_module_detector.py
git commit -m "feat(module): implement BipartiteModuleDetector for 44-vertex variable gadgets"
```

---

### Task 2: Implement `ModuleDualPathExtractor`

**Files:**
- Create: `scratch/engine/dual_path_extractor.py`
- Test: `scratch/engine/tests/test_dual_path_extractor.py`

**Interfaces:**
- Consumes: `G_adj: Dict[int, Set[int]]`, `module: Dict`
- Produces: `extract_module_dual_paths(G_adj: Dict[int, Set[int]], module: Dict) -> Tuple[Set[Tuple[int, int]], Set[Tuple[int, int]]]`
  Returns `(E_True, E_False)` where each is a set of canonical edges forming a spanning path across `module['ports']`.

- [ ] **Step 1: Write the failing test**

Create `scratch/engine/tests/test_dual_path_extractor.py`:
```python
import os, pytest
from scratch.engine.graph_loader import load_dimacs
from scratch.engine.bipartite_module_detector import detect_variable_modules
from scratch.engine.dual_path_extractor import extract_module_dual_paths

def test_dual_path_extraction_graph868():
    col_path = "FHCPCS-col/graph868.col"
    if not os.path.exists(col_path):
        pytest.skip("graph868.col not found")
    G = load_dimacs(col_path)
    modules = detect_variable_modules(G, set(G.keys()), [])
    assert len(modules) > 0
    m0 = modules[0]
    e_true, e_false = extract_module_dual_paths(G, m0)
    assert len(e_true) > 0, "True path must not be empty"
    assert len(e_false) > 0, "False path must not be empty"
    assert e_true != e_false, "True and False paths must be distinct"
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pytest scratch/engine/tests/test_dual_path_extractor.py -v`
Expected: FAIL with `ModuleNotFoundError: No module named 'scratch.engine.dual_path_extractor'`

- [ ] **Step 3: Implement `dual_path_extractor.py`**

Create `scratch/engine/dual_path_extractor.py`:
```python
import collections
from typing import Dict, Set, Tuple
from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType

def extract_module_dual_paths(
    G_adj: Dict[int, Set[int]],
    module: Dict
) -> Tuple[Set[Tuple[int, int]], Set[Tuple[int, int]]]:
    """
    Computes the two canonical spanning Hamiltonian path configurations T_i and F_i
    for an isolated variable module across its interface ports.
    """
    m_nodes = set(module['nodes'])
    ports = module['ports']
    if len(ports) != 2:
        return set(), set()
    p_in, p_out = ports

    # Edges within module
    edges = set()
    for u in m_nodes:
        for v in G_adj[u]:
            if v in m_nodes and u < v:
                edges.add((u, v))
    
    # Virtual closure edge between ports to turn Hamiltonian Path into Hamiltonian Cycle
    virt_edge = tuple(sorted([p_in, p_out]))
    edges.add(virt_edge)

    edge_list = sorted(list(edges))
    edge_to_var = {e: i + 1 for i, e in enumerate(edge_list)}
    inc_edges = collections.defaultdict(list)
    for e in edge_list:
        inc_edges[e[0]].append(edge_to_var[e])
        inc_edges[e[1]].append(edge_to_var[e])

    solver = Cadical195()
    top = len(edge_list) + 1

    # Degree-2 constraint on every node in module
    for u in m_nodes:
        clauses = CardEnc.equals(lits=inc_edges[u], bound=2, top_id=top, encoding=EncType.cardnetwrk)
        for cl in clauses:
            solver.add_clause(cl)
            for lit in cl:
                top = max(top, abs(lit) + 1)

    # Force virtual edge
    solver.add_clause([edge_to_var[virt_edge]])

    # Force internal virtual edges (contracted degree-2 chains)
    for ve in module['virtual_edges']:
        if ve in edge_to_var:
            solver.add_clause([edge_to_var[ve]])

    paths = []
    # Extract up to 2 distinct configurations
    while len(paths) < 2:
        if not solver.solve():
            break
        model = set(solver.get_model())
        active_edges = {e for e in edge_list if edge_to_var[e] in model}
        
        # Verify connectivity (no subcycles)
        adj_model = collections.defaultdict(list)
        for u, v in active_edges:
            adj_model[u].append(v); adj_model[v].append(u)
        
        visited = set()
        cycles = []
        for u in m_nodes:
            if u not in visited:
                cyc = []
                curr, prev = u, None
                while curr not in visited:
                    visited.add(curr); cyc.append(curr)
                    nbrs = adj_model[curr]
                    if len(nbrs) < 2:
                        break
                    nxt = nbrs[0] if nbrs[0] != prev else nbrs[1]
                    prev, curr = curr, nxt
                cycles.append(cyc)

        if len(cycles) == 1 and len(cycles[0]) == len(m_nodes):
            # Remove virtual closure edge to get pure path edges
            path_edges = active_edges - {virt_edge}
            paths.append(path_edges)
            # Add blocking clause to find the alternative configuration
            solver.add_clause([-edge_to_var[e] for e in active_edges if e != virt_edge])
        else:
            # Subcycle cut
            for cyc in cycles:
                solver.add_clause([-edge_to_var[tuple(sorted([cyc[i], cyc[(i+1)%len(cyc)]]))] for i in range(len(cyc))])

    solver.delete()
    if len(paths) == 2:
        return paths[0], paths[1]
    elif len(paths) == 1:
        return paths[0], paths[0]
    return set(), set()
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pytest scratch/engine/tests/test_dual_path_extractor.py -v`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add scratch/engine/dual_path_extractor.py scratch/engine/tests/test_dual_path_extractor.py
git commit -m "feat(module): implement ModuleDualPathExtractor with micro-SAT solving"
```

---

### Task 3: Implement `ModularComp0Solver`

**Files:**
- Create: `scratch/engine/modular_comp0_solver.py`
- Test: `scratch/engine/tests/test_modular_comp0_solver.py`

**Interfaces:**
- Consumes: `G: Dict[int, Set[int]]`, `comp0_nodes: Set[int]`, `virtual_edges: List[Tuple[int, int]]`, `time_limit: int = 1800`
- Produces: `solve_modular_comp0(G: Dict[int, Set[int]], comp0_nodes: Set[int], virtual_edges: List[Tuple[int, int]], time_limit: int = 1800) -> List[int]`
  Returns an uncontracted Hamiltonian cycle visiting 100% of `comp0_nodes`.

- [ ] **Step 1: Write the failing test**

Create `scratch/engine/tests/test_modular_comp0_solver.py`:
```python
import os, pytest
from scratch.engine.graph_loader import load_dimacs
from scratch.engine.decomposer import detect_bridge_corridors
from scratch.engine.modular_comp0_solver import solve_modular_comp0

def test_modular_comp0_solve_graph882():
    col_path = "FHCPCS-col/graph882.col"
    if not os.path.exists(col_path):
        pytest.skip("graph882.col not found")
    G = load_dimacs(col_path)
    corridors, c0_nodes = detect_bridge_corridors(G)
    ve = [c['ext_ports'] for c in corridors]
    cycle = solve_modular_comp0(G, set(c0_nodes), ve, time_limit=300)
    assert len(cycle) == len(c0_nodes), f"Expected {len(c0_nodes)}, got {len(cycle)}"
    assert len(set(cycle)) == len(c0_nodes), "Duplicate vertices in cycle"
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pytest scratch/engine/tests/test_modular_comp0_solver.py -v`
Expected: FAIL with `ModuleNotFoundError: No module named 'scratch.engine.modular_comp0_solver'`

- [ ] **Step 3: Implement `modular_comp0_solver.py`**

Create `scratch/engine/modular_comp0_solver.py`:
```python
import collections, time
from typing import Dict, List, Set, Tuple
from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType
from scratch.engine.bipartite_module_detector import detect_variable_modules
from scratch.engine.dual_path_extractor import extract_module_dual_paths
from scratch.engine.comp0_solver import try_merge_2opt, try_merge_3opt, sat_merge_cycles

def solve_modular_comp0(
    G: Dict[int, Set[int]],
    comp0_nodes: Set[int],
    virtual_edges: List[Tuple[int, int]],
    time_limit: int = 1800
) -> List[int]:
    t0 = time.time()
    virt_set = {tuple(sorted(e)) for e in virtual_edges}
    ports = set()
    for e in virt_set:
        ports.update(e)

    print(f"[*] Starting Modular State Equivalence Comp 0 Solver on {len(comp0_nodes)} vertices...")
    
    # 1. Detect modules
    modules = detect_variable_modules(G, comp0_nodes, virtual_edges)
    print(f"    Detected {len(modules)} variable modules in Comp 0.")

    # 2. Extract dual paths for each module
    module_paths = []
    all_module_nodes = set()
    for m in modules:
        t_m = extract_module_dual_paths(G, m)
        module_paths.append(t_m)
        all_module_nodes.update(m['nodes'])

    # 3. Contract degree-2 vertices in Comp 0
    adj_c0 = collections.defaultdict(set)
    for u in comp0_nodes:
        for v in G[u]:
            if v in comp0_nodes:
                adj_c0[u].add(v)
    for u, v in virt_set:
        adj_c0[u].add(v); adj_c0[v].add(u)

    rem = set(comp0_nodes)
    edge_chains = {}
    while True:
        d2 = [u for u in sorted(rem) if u not in ports and len(adj_c0[u]) == 2]
        if not d2:
            break
        v = d2[0]
        u, w = list(adj_c0[v])
        adj_c0[u].remove(v); adj_c0[w].remove(v)
        del adj_c0[v]; rem.remove(v)
        e_uv = tuple(sorted([u, v])); e_vw = tuple(sorted([v, w]))
        c_uv = edge_chains.pop(e_uv, [u, v]); c_vw = edge_chains.pop(e_vw, [v, w])
        if c_uv[-1] != v: c_uv = list(reversed(c_uv))
        if c_vw[0] != v: c_vw = list(reversed(c_vw))
        e_new = tuple(sorted([u, w]))
        edge_chains[e_new] = c_uv + c_vw[1:]
        adj_c0[u].add(w); adj_c0[w].add(u)

    contracted_edges = set(edge_chains.keys())
    forbidden_delete = virt_set | contracted_edges

    # 4. Map edges to variables
    edges = set()
    for u in sorted(rem):
        for v in sorted(adj_c0[u]):
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

    # Degree-2 constraint on every node
    for u in sorted(rem):
        clauses = CardEnc.equals(lits=inc_edges[u], bound=2, top_id=top, encoding=EncType.cardnetwrk)
        for cl in clauses:
            solver.add_clause(cl)
            for lit in cl:
                top = max(top, abs(lit) + 1)

    # Force virtual and contracted edges
    for ve in virt_set:
        solver.add_clause([edge_to_var[ve]])
    for ce in contracted_edges:
        solver.add_clause([edge_to_var[ce]])

    # 5. Inject Module State Selector Variables (b_i)
    selector_vars = {}
    for mid, (m, (t_path, f_path)) in enumerate(zip(modules, module_paths)):
        if not t_path or not f_path or t_path == f_path:
            continue
        b_var = top
        top += 1
        selector_vars[mid] = b_var

        common = t_path & f_path
        true_only = t_path - f_path
        false_only = f_path - t_path

        for e in common:
            if e in edge_to_var:
                solver.add_clause([edge_to_var[e]])
        for e in true_only:
            if e in edge_to_var:
                var_e = edge_to_var[e]
                solver.add_clause([-b_var, var_e])
                solver.add_clause([b_var, -var_e])
        for e in false_only:
            if e in edge_to_var:
                var_e = edge_to_var[e]
                solver.add_clause([b_var, var_e])
                solver.add_clause([-b_var, -var_e])

    print(f"    Injected {len(selector_vars)} module selector variables. Top var: {top}.")

    # 6. Macro-CEGAR Loop
    it = 0
    winner_cycle = None
    while True:
        it += 1
        t_it = time.time()
        if time.time() - t0 > time_limit:
            solver.delete()
            raise TimeoutError(f"Modular Comp 0 reached {time_limit}s limit at iter {it}!")

        if not solver.solve():
            solver.delete()
            raise RuntimeError("Modular Comp 0 SAT problem proved UNSAT!")

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
            print(f"[*] Modular Comp 0 converged at iter {it} in {time.time()-t0:.2f}s!", flush=True)
            winner_cycle = cycles[0]
            break

        # Splicer
        merged = list(cycles)
        merged_any = True
        while merged_any and len(merged) > 1:
            merged_any = False
            merged.sort(key=len, reverse=True)
            for i in range(len(merged)):
                for j in range(i + 1, len(merged)):
                    res = try_merge_2opt(merged[i], merged[j], adj_c0, forbidden_delete)
                    if res is None and len(merged) <= 12:
                        res = try_merge_3opt(merged[i], merged[j], adj_c0, forbidden_delete)
                    if res is not None:
                        merged.pop(j)
                        merged[i] = res
                        merged_any = True
                        break
                if merged_any:
                    break

        if len(merged) == 1:
            print(f"[*] Modular Comp 0 2-opt converged at iter {it} in {time.time()-t0:.2f}s!", flush=True)
            winner_cycle = merged[0]
            break

        print(f"    Iter {it} ({time.time()-t_it:.2f}s, total {time.time()-t0:.1f}s): {len(cycles)} raw -> {len(merged)} macro-cycles", flush=True)

        # Universal cocycle cuts
        for cyc in cycles:
            c_set = set(cyc)
            cut_e = [tuple(sorted([u, v])) for u in cyc for v in adj_c0[u] if v not in c_set]
            solver.add_clause([edge_to_var[e] for e in cut_e])
            if len(cyc) <= len(rem) // 2:
                neg_c = [-edge_to_var[tuple(sorted([cyc[i], cyc[(i + 1) % len(cyc)]]))] for i in range(len(cyc))]
                solver.add_clause(neg_c)

    solver.delete()

    # Expand contracted degree-2 chains
    expanded = []
    n = len(winner_cycle)
    for i in range(n):
        u = winner_cycle[i]
        v = winner_cycle[(i + 1) % n]
        e = tuple(sorted([u, v]))
        if e in edge_chains:
            chain = edge_chains[e]
            if chain[0] != u:
                chain = list(reversed(chain))
            expanded.extend(chain[:-1])
        else:
            expanded.append(u)

    assert len(expanded) == len(comp0_nodes), f"Expected {len(comp0_nodes)}, got {len(expanded)}"
    return expanded
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pytest scratch/engine/tests/test_modular_comp0_solver.py -v`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add scratch/engine/modular_comp0_solver.py scratch/engine/tests/test_modular_comp0_solver.py
git commit -m "feat(engine): implement ModularComp0Solver with Macro-CEGAR loop"
```

---

### Task 4: Integrate into `unified_solver.py` & End-to-End Certification of the 5 Large Graphs

**Files:**
- Modify: `scratch/engine/unified_solver.py:48-54`
- Modify: `scratch/engine/auto_corridor_pipeline.py:90-105`
- Test: `scratch/verify_benchmarks.py`

**Interfaces:**
- In `unified_solver.py`, automatically dispatch Comp 0 to `modular_comp0_solver` if `len(comp0_nodes) > 4600`, else fall back to standard `comp0_solver`.

- [ ] **Step 1: Modify `scratch/engine/unified_solver.py` to route modular graphs**

Update `scratch/engine/unified_solver.py`:
```python
    # Step 4: Solve Comp 0
    t_c0 = time.time()
    virtual_edges = [c['ext_ports'] for c in corridors]
    if len(comp0_nodes) > 4600:
        from scratch.engine.modular_comp0_solver import solve_modular_comp0
        print(f"[*] Stage 3: Large Comp 0 ({len(comp0_nodes)}v) detected -> routing to Modular State Equivalence Solver...")
        comp0_cycle = solve_modular_comp0(G, comp0_nodes, virtual_edges)
    else:
        print(f"[*] Stage 3: Solving Comp 0 ({len(comp0_nodes)}v) via degree-2 contraction & 2-opt CEGAR...")
        comp0_cycle = solve_comp0_core(G, comp0_nodes, virtual_edges)
    print(f"[*] Stage 3 Complete in {time.time()-t_c0:.2f}s.")
```

- [ ] **Step 2: Update `auto_corridor_pipeline.py` queue with all 5 remaining hard instances**

Update queue to:
```python
    queue = [
        "graph965.col",
        "graph966.col",
        "graph971.col",
        "graph994.col",
        "graph998.col"
    ]
```

- [ ] **Step 3: Run pipeline on `graph965.col` with `timeout 1800`**

Run: `timeout 1800 python3 scratch/engine/unified_solver.py FHCPCS-col/graph965.col scratch/engine/found_tour_graph965.hcp`
Expected: Solve completed in $< 1800$s (target $< 120$s).

- [ ] **Step 4: Verify soundness with benchmark verifier**

Run: `python3 scratch/verify_benchmarks.py --graph FHCPCS-col/graph965.col --tour scratch/engine/found_tour_graph965.hcp`
Expected: `PASS - CERTIFIED SOUND` (7,102 consecutive valid edges verified).

- [ ] **Step 5: Commit and register into `scratch/verify_benchmarks.py`**

```bash
git add scratch/engine/unified_solver.py scratch/engine/auto_corridor_pipeline.py scratch/verify_benchmarks.py
git commit -m "feat(engine): integrate Modular State Equivalence Solver into unified solver"
```
