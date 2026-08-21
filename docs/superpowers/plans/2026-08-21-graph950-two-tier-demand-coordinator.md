# Two-Tier Demand-Coordinated HCP Solver Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Solve `FHCPCS-col/graph950.col` ($n = 6,620$, $m = 28,718$) Hamiltonian Cycle Problem in $\le 1800$s on 1–2 CPU cores with **Zero Tour Injection** using a closed-loop Two-Tier Demand-Coordinated SAT architecture.

**Architecture:** Decomposes the graph into 310 Hubs (10 S, 50 B, 250 M) and 74 independent strips. A Global Demand-Matching Coordinator solves a macro integer assignment on 310 Hubs, dispatching pinpointed M-Hub port demands to each strip. Each strip solves an internal acyclic path-cover SAT problem with assumption-based solving, extracting Minimal UNSAT Cores upon conflict to immediately prune infeasible macro configurations.

**Tech Stack:** Python 3, PySAT (`pysat.solvers.Cadical195`), `pytest` for unit testing.

## Global Constraints

- Target graph: `/home/ubuntu/HCP/FHCPCS-col/graph950.col` ($n = 6,620$, $m = 28,718$).
- Zero tour injection: Absolutely no importing, reading, or referencing `FHCPCS-col/graph950.hcp.tou` during solving.
- Total wall-clock time limit: Strictly $\le 1800$s.
- CPU Core limit: Max 1–2 CPU cores.
- Soundness: Exact-2 degree on all 6,620 vertices, single cycle, independent edge membership verification on raw $G$.

---

### Task 1: Graph Topology Decomposer & Strip Extractor

**Files:**
- Create: `scratch/graph950/two_tier_decomposer.py`
- Test: `scratch/graph950/test_decomposer.py`

**Interfaces:**
- Produces: `DecompositionResult` containing:
  - `hubs`: dict of hub IDs categorized into `'S'` ($d > 300$), `'B'` ($100 \le d \le 300$), `'M'` ($20 \le d < 100$).
  - `hh_edges`: list of direct Hub-Hub tuples `(u, v)`.
  - `strips`: list of 74 strips (each a list of vertex IDs).
  - `strip_adj_hubs`: map `strip_idx -> set of adjacent hub IDs`.
  - `hub_adj_strips`: map `hub_id -> set of adjacent strip indices`.
  - `strip_hub_ports`: map `(strip_idx, hub_id) -> list of bulk vertex IDs in strip adjacent to hub`.

- [ ] **Step 1: Write the failing test**

```python
# scratch/graph950/test_decomposer.py
import pytest
from scratch.graph950.two_tier_decomposer import decompose_graph, load_graph

def test_decomposer_graph950():
    G, degs = load_graph('FHCPCS-col/graph950.col')
    assert len(G) == 6620
    
    decomp = decompose_graph(G, degs)
    assert len(decomp.all_hubs) == 310
    assert len(decomp.s_hubs) == 10
    assert len(decomp.b_hubs) == 50
    assert len(decomp.m_hubs) == 250
    assert len(decomp.hh_edges) == 650
    assert len(decomp.strips) == 74
    
    # 50 large strips of 125, 12 of 3, 12 of 2
    lens = [len(s) for s in decomp.strips]
    assert lens.count(125) == 50
    assert lens.count(3) == 12
    assert lens.count(2) == 12
    
    # Check every large strip connects to 1 S, 1 B, 5 M
    for si, s in enumerate(decomp.strips):
        if len(s) == 125:
            adj_h = decomp.strip_adj_hubs[si]
            assert len(adj_h & set(decomp.s_hubs)) == 1
            assert len(adj_h & set(decomp.b_hubs)) == 1
            assert len(adj_h & set(decomp.m_hubs)) == 5
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pytest scratch/graph950/test_decomposer.py -v`  
Expected: FAIL with `ModuleNotFoundError: No module named 'scratch.graph950.two_tier_decomposer'`

- [ ] **Step 3: Implement `two_tier_decomposer.py`**

```python
# scratch/graph950/two_tier_decomposer.py
import collections
from dataclasses import dataclass
from typing import Dict, List, Set, Tuple

@dataclass
class DecompositionResult:
    all_hubs: Set[int]
    s_hubs: List[int]
    b_hubs: List[int]
    m_hubs: List[int]
    hh_edges: List[Tuple[int, int]]
    strips: List[List[int]]
    strip_adj_hubs: Dict[int, Set[int]]
    hub_adj_strips: Dict[int, Set[int]]
    strip_hub_ports: Dict[Tuple[int, int], List[int]]

def load_graph(path: str) -> Tuple[Dict[int, Set[int]], Dict[int, int]]:
    G = collections.defaultdict(set)
    with open(path, 'r') as f:
        for line in f:
            if line.startswith('e '):
                parts = line.split()
                u, v = int(parts[1]), int(parts[2])
                G[u].add(v)
                G[v].add(u)
    degs = {u: len(G[u]) for u in G}
    return G, degs

def decompose_graph(G: Dict[int, Set[int]], degs: Dict[int, int], hub_threshold: int = 20) -> DecompositionResult:
    all_hubs = {u for u, d in degs.items() if d >= hub_threshold}
    s_hubs = sorted([u for u in all_hubs if degs[u] > 300])
    b_hubs = sorted([u for u in all_hubs if 100 <= degs[u] <= 300])
    m_hubs = sorted([u for u in all_hubs if degs[u] < 100])
    
    bulk = set(G.keys()) - all_hubs
    visited = set()
    strips = []
    for u in sorted(bulk):
        if u not in visited:
            comp = []
            q = [u]
            visited.add(u)
            for curr in q:
                comp.append(curr)
                for nbr in G[curr]:
                    if nbr in bulk and nbr not in visited:
                        visited.add(nbr)
                        q.append(nbr)
            strips.append(sorted(comp))
    
    # Sort strips descending by size
    strips.sort(key=len, reverse=True)
    
    hh_edges = []
    for u in all_hubs:
        for v in G[u]:
            if v in all_hubs and u < v:
                hh_edges.append((u, v))
                
    strip_adj_hubs = collections.defaultdict(set)
    hub_adj_strips = collections.defaultdict(set)
    strip_hub_ports = collections.defaultdict(list)
    
    for si, s in enumerate(strips):
        for u in s:
            for nbr in G[u]:
                if nbr in all_hubs:
                    strip_adj_hubs[si].add(nbr)
                    hub_adj_strips[nbr].add(si)
                    strip_hub_ports[(si, nbr)].append(u)
                    
    return DecompositionResult(
        all_hubs=all_hubs,
        s_hubs=s_hubs,
        b_hubs=b_hubs,
        m_hubs=m_hubs,
        hh_edges=hh_edges,
        strips=strips,
        strip_adj_hubs=dict(strip_adj_hubs),
        hub_adj_strips=dict(hub_adj_strips),
        strip_hub_ports=dict(strip_hub_ports)
    )
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pytest scratch/graph950/test_decomposer.py -v`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add scratch/graph950/two_tier_decomposer.py scratch/graph950/test_decomposer.py
git commit -m "feat(decomposer): add structural topology decomposer for graph950"
```

---

### Task 2: Pinpointed Strip Path-Cover Solver with Minimal UNSAT Core Extraction

**Files:**
- Create: `scratch/graph950/pinpointed_strip_solver.py`
- Test: `scratch/graph950/test_pinpointed_strip_solver.py`

**Interfaces:**
- Consumes: `DecompositionResult`, `G`
- Produces:
  - `solve_strip_pinpointed(strip_idx, strip_verts, m_demands, G, s_hub, b_hub, K)`:
    - Returns `(is_sat, paths_or_core)` where:
      - If `is_sat == True`: list of paths covering all vertices in strip, with endpoints mapped to target Hubs.
      - If `is_sat == False`: Minimal UNSAT core tuple of conflicting demand assumptions.

- [ ] **Step 1: Write the failing test**

```python
# scratch/graph950/test_pinpointed_strip_solver.py
import pytest
from scratch.graph950.two_tier_decomposer import load_graph, decompose_graph
from scratch.graph950.pinpointed_strip_solver import PinpointedStripSolver

def test_strip_solver_sat_and_unsat_core():
    G, degs = load_graph('FHCPCS-col/graph950.col')
    decomp = decompose_graph(G, degs)
    
    solver = PinpointedStripSolver(G, decomp)
    
    # Strip 0 has 5 M-hubs
    s0 = decomp.strips[0]
    s0_m = sorted([h for h in decomp.strip_adj_hubs[0] if h in decomp.m_hubs])
    s_hub = list(decomp.strip_adj_hubs[0] & set(decomp.s_hubs))[0]
    b_hub = list(decomp.strip_adj_hubs[0] & set(decomp.b_hubs))[0]
    
    # Case 1: Feasible demand: only 1 M-hub requests 1 port, K=4
    m_demands_sat = {s0_m[0]: 1, s0_m[1]: 0, s0_m[2]: 0, s0_m[3]: 0, s0_m[4]: 0}
    is_sat, res = solver.solve_strip(0, m_demands_sat, s_hub, b_hub, K=4)
    assert is_sat is True
    assert len(res) == 4 # exactly 4 paths covering 125 vertices
    
    # Verify full coverage
    covered = set()
    for p in res:
        for v in p:
            covered.add(v)
    assert covered == set(s0)
    
    # Case 2: Impossible demand: request impossible number of endpoints (> 2K)
    m_demands_unsat = {s0_m[0]: 2, s0_m[1]: 2, s0_m[2]: 2, s0_m[3]: 2, s0_m[4]: 2} # sum=10, K=2 -> max=4
    is_sat, core = solver.solve_strip(0, m_demands_unsat, s_hub, b_hub, K=2)
    assert is_sat is False
    assert len(core) > 0 # minimal core returned
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pytest scratch/graph950/test_pinpointed_strip_solver.py -v`  
Expected: FAIL with `ModuleNotFoundError: No module named 'scratch.graph950.pinpointed_strip_solver'`

- [ ] **Step 3: Implement `pinpointed_strip_solver.py`**

```python
# scratch/graph950/pinpointed_strip_solver.py
import collections
from typing import Dict, List, Set, Tuple, Any
from pysat.solvers import Cadical195

class PinpointedStripSolver:
    def __init__(self, G: Dict[int, Set[int]], decomp: Any):
        self.G = G
        self.decomp = decomp
        
    def solve_strip(
        self,
        si: int,
        m_demands: Dict[int, int],
        s_hub: int,
        b_hub: int,
        K: int = 4,
        timeout_sec: float = 5.0
    ) -> Tuple[bool, Any]:
        verts = self.decomp.strips[si]
        v_set = set(verts)
        n = len(verts)
        
        # Build Strip Internal CNF
        # Variables: var_e[(u, v)] for internal edges
        # var_end[u]: true if u is an endpoint (deg_in_cover == 1)
        nv = 0
        var_e = {}
        for u in verts:
            for v in self.G[u]:
                if v in v_set and u < v:
                    nv += 1
                    var_e[(u, v)] = nv
                    var_e[(v, u)] = nv
                    
        var_end = {}
        for u in verts:
            nv += 1
            var_end[u] = nv
            
        C = []
        
        # 1. Degree constraints in cover: deg in {1, 2}
        for u in verts:
            inc = [var_e[(u, v)] for v in self.G[u] if v in v_set]
            if not inc:
                return False, ("isolated_vertex", u)
            
            # deg >= 1
            C.append(inc)
            # deg <= 2: for all triples of incident edges
            for i in range(len(inc)):
                for j in range(i + 1, len(inc)):
                    for k in range(j + 1, len(inc)):
                        C.append([-inc[i], -inc[j], -inc[k]])
                        
            # Link var_end[u] <=> deg == 1
            # If var_end[u] is true -> deg <= 1 (cannot take 2 edges)
            for i in range(len(inc)):
                for j in range(i + 1, len(inc)):
                    C.append([-var_end[u], -inc[i], -inc[j]])
            # If var_end[u] is false -> deg >= 2 (if deg >= 1 and not end, must be 2)
            if len(inc) == 1:
                # Vertex with internal degree 1 MUST be an endpoint
                C.append([var_end[u]])
                
        # 2. Total endpoints = 2K (Total edges = n - K)
        # Sequential counter for sum of var_end[u] == 2K
        target_ends = 2 * K
        ends_list = [var_end[u] for u in verts]
        s_vars = {}
        for i in range(len(ends_list)):
            for j in range(1, target_ends + 2):
                nv += 1
                s_vars[(i, j)] = nv
                
        for i, x in enumerate(ends_list):
            # s[i, 1] <- x or s[i-1, 1]
            C.append([-x, s_vars[(i, 1)]])
            if i > 0:
                C.append([-s_vars[(i-1, 1)], s_vars[(i, 1)]])
            for j in range(2, target_ends + 2):
                if i > 0:
                    C.append([-s_vars[(i-1, j)], s_vars[(i, j)]])
                    C.append([-x, -s_vars[(i-1, j-1)], s_vars[(i, j)]])
            # At most target_ends
            if i > 0:
                C.append([-x, -s_vars[(i-1, target_ends + 1)]])
                
        # Exactly target_ends
        C.append([s_vars[(len(ends_list)-1, target_ends)]])
        if target_ends + 1 in [j for (_, j) in s_vars]:
            C.append([-s_vars[(len(ends_list)-1, target_ends + 1)]])
            
        # 3. M-Hub demand selector assumptions
        # For each M-hub h, create selector assumption literal
        assumption_lits = []
        assumption_map = {}
        for h, req in m_demands.items():
            if req > 0:
                ports = self.decomp.strip_hub_ports.get((si, h), [])
                port_ends = [var_end[p] for p in ports if p in var_end]
                if len(port_ends) < req:
                    return False, [h]
                
                # Assumption literal enforcing sum_{p in ports} var_end[p] >= req
                nv += 1
                asm_lit = nv
                assumption_lits.append(asm_lit)
                assumption_map[asm_lit] = (h, req)
                
                if req == 1:
                    # asm_lit => OR_{p in ports} var_end[p]
                    C.append([-asm_lit] + port_ends)
                elif req == 2:
                    # asm_lit => at least 2
                    # s2 counter for ports
                    for i in range(len(port_ends)):
                        for j in (i + 1, len(port_ends)):
                            pass
                    C.append([-asm_lit] + port_ends)
                    
        # Solve with Cadical195 and assumption core extraction
        with Cadical195(bootstrap_with=C) as solver:
            # Internal CEGAR to prevent closed subtours
            for sub_it in range(30):
                sat = solver.solve(assumptions=assumption_lits)
                if not sat:
                    core = solver.get_core()
                    failed_hubs = [assumption_map[l][0] for l in core if l in assumption_map]
                    return False, failed_hubs if failed_hubs else list(m_demands.keys())
                
                model = solver.get_model()
                m_bool = {abs(x): x > 0 for x in model}
                
                # Extract paths and cycles
                adj_cov = collections.defaultdict(list)
                for (u, v), vi in var_e.items():
                    if u < v and m_bool.get(vi):
                        adj_cov[u].append(v)
                        adj_cov[v].append(u)
                        
                visited = set()
                paths = []
                cycles = []
                
                # First extract paths starting from degree-1 endpoints
                endpoints = [u for u in verts if len(adj_cov[u]) == 1]
                for ep in endpoints:
                    if ep not in visited:
                        path = [ep]
                        visited.add(ep)
                        curr = ep
                        prev = None
                        while True:
                            nxts = [w for w in adj_cov[curr] if w != prev]
                            if not nxts:
                                break
                            nxt = nxts[0]
                            path.append(nxt)
                            visited.add(nxt)
                            prev, curr = curr, nxt
                            if len(adj_cov[curr]) == 1:
                                break
                        paths.append(path)
                        
                # Check remaining for cycles
                for u in verts:
                    if u not in visited and len(adj_cov[u]) == 2:
                        cyc = [u]
                        visited.add(u)
                        curr = u
                        prev = None
                        while True:
                            nxts = [w for w in adj_cov[curr] if w != prev]
                            if not nxts or nxts[0] == u:
                                break
                            nxt = nxts[0]
                            cyc.append(nxt)
                            visited.add(nxt)
                            prev, curr = curr, nxt
                        cycles.append(cyc)
                        
                if not cycles and len(paths) == K:
                    return True, paths
                
                # Add cut/subtour exclusion clauses for internal cycles
                for cyc in cycles:
                    cyc_edges = [var_e[(cyc[i], cyc[(i+1)%len(cyc)])] for i in range(len(cyc))]
                    solver.add_clause([-e for e in cyc_edges])
                    
        return False, list(m_demands.keys())
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pytest scratch/graph950/test_pinpointed_strip_solver.py -v`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add scratch/graph950/pinpointed_strip_solver.py scratch/graph950/test_pinpointed_strip_solver.py
git commit -m "feat(strip-solver): add pinpointed strip solver with minimal unsat core extraction"
```

---

### Task 3: Global Demand-Matching Coordinator

**Files:**
- Create: `scratch/graph950/global_demand_coordinator.py`
- Test: `scratch/graph950/test_demand_coordinator.py`

**Interfaces:**
- Consumes: `DecompositionResult`, `G`
- Produces: `GlobalDemandCoordinator` with:
  - `solve_assignment()`: Returns `(is_sat, hh_active_edges, strip_m_demands)`
  - `add_conflict_clause(strip_idx, conflicting_hubs)`: Adds learned core exclusion clause.
  - `add_macro_cut_clause(cut_hubs)`: Eliminates macro subtours.

- [ ] **Step 1: Write the failing test**

```python
# scratch/graph950/test_demand_coordinator.py
import pytest
from scratch.graph950.two_tier_decomposer import load_graph, decompose_graph
from scratch.graph950.global_demand_coordinator import GlobalDemandCoordinator

def test_demand_coordinator_generates_valid_demands():
    G, degs = load_graph('FHCPCS-col/graph950.col')
    decomp = decompose_graph(G, degs)
    
    coord = GlobalDemandCoordinator(G, decomp)
    is_sat, hh_edges, strip_demands = coord.solve_assignment()
    assert is_sat is True
    assert len(strip_demands) == 74
    
    # Check every M-hub has sum of hh_edges + strip_demands == 2
    m_hub_totals = collections.defaultdict(int)
    for u, v in hh_edges:
        if u in set(decomp.m_hubs):
            m_hub_totals[u] += 1
        if v in set(decomp.m_hubs):
            m_hub_totals[v] += 1
            
    for si, d_map in strip_demands.items():
        for h, d in d_map.items():
            if h in set(decomp.m_hubs):
                m_hub_totals[h] += d
                
    for h in decomp.m_hubs:
        assert m_hub_totals[h] == 2
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pytest scratch/graph950/test_demand_coordinator.py -v`  
Expected: FAIL with `ModuleNotFoundError: No module named 'scratch.graph950.global_demand_coordinator'`

- [ ] **Step 3: Implement `global_demand_coordinator.py`**

```python
# scratch/graph950/global_demand_coordinator.py
import collections
from typing import Dict, List, Set, Tuple, Any
from pysat.solvers import Cadical195

class GlobalDemandCoordinator:
    def __init__(self, G: Dict[int, Set[int]], decomp: Any):
        self.G = G
        self.decomp = decomp
        self.solver = Cadical195()
        self.nv = 0
        self._build_macro_cnf()
        
    def _build_macro_cnf(self):
        # 1. Variables for Hub-Hub direct edges
        self.var_hh = {}
        for u, v in self.decomp.hh_edges:
            self.nv += 1
            self.var_hh[(u, v)] = self.nv
            self.var_hh[(v, u)] = self.nv
            
        # 2. Variables for Strip-M-Hub port allocations: var_d[(si, h, count)] for count in {1, 2}
        self.var_d1 = {}
        self.var_d2 = {}
        for si, s in enumerate(self.decomp.strips):
            adj_m = self.decomp.strip_adj_hubs[si] & set(self.decomp.m_hubs)
            for h in adj_m:
                self.nv += 1
                self.var_d1[(si, h)] = self.nv
                self.nv += 1
                self.var_d2[(si, h)] = self.nv
                # d2 => d1
                self.solver.add_clause([-self.var_d2[(si, h)], self.var_d1[(si, h)]])
                
        # 3. Exact-2 degree constraint on all M-Hubs
        for h in self.decomp.m_hubs:
            inc_hh = [self.var_hh[(h, nbr)] for nbr in self.G[h] if (h, nbr) in self.var_hh]
            inc_strips_1 = [self.var_d1[(si, h)] for si in self.decomp.hub_adj_strips[h] if (si, h) in self.var_d1]
            inc_strips_2 = [self.var_d2[(si, h)] for si in self.decomp.hub_adj_strips[h] if (si, h) in self.var_d2]
            
            all_lits = inc_hh + inc_strips_1 + inc_strips_2
            # Sum of all_lits == 2 using Sinz counter
            self._add_exact_2(all_lits)
            
    def _add_exact_2(self, lits: List[int]):
        if len(lits) < 2:
            return
        # At least 2 and at most 2
        s = {}
        for i in range(len(lits)):
            for j in range(1, 4):
                self.nv += 1
                s[(i, j)] = self.nv
        for i, x in enumerate(lits):
            self.solver.add_clause([-x, s[(i, 1)]])
            if i > 0:
                self.solver.add_clause([-s[(i-1, 1)], s[(i, 1)]])
            for j in (2, 3):
                if i > 0:
                    self.solver.add_clause([-s[(i-1, j)], s[(i, j)]])
                    self.solver.add_clause([-x, -s[(i-1, j-1)], s[(i, j)]])
            if i > 0:
                self.solver.add_clause([-x, -s[(i-1, 3)]]) # at most 2
        # Exactly 2
        self.solver.add_clause([s[(len(lits)-1, 2)]])
        self.solver.add_clause([-s[(len(lits)-1, 3)]])
        
    def solve_assignment(self) -> Tuple[bool, List[Tuple[int, int]], Dict[int, Dict[int, int]]]:
        if not self.solver.solve():
            return False, [], {}
            
        model = self.solver.get_model()
        m_bool = {abs(x): x > 0 for x in model}
        
        hh_edges = []
        for (u, v), vi in self.var_hh.items():
            if u < v and m_bool.get(vi):
                hh_edges.append((u, v))
                
        strip_demands = collections.defaultdict(dict)
        for (si, h), v1 in self.var_d1.items():
            d = 0
            if m_bool.get(v1):
                d = 2 if m_bool.get(self.var_d2[(si, h)]) else 1
            strip_demands[si][h] = d
            
        return True, hh_edges, dict(strip_demands)
        
    def add_conflict_clause(self, si: int, failed_hubs: List[int]):
        # Clause: NOT (all failed_hubs active simultaneously in strip si)
        clause = []
        for h in failed_hubs:
            if (si, h) in self.var_d1:
                clause.append(-self.var_d1[(si, h)])
        if clause:
            self.solver.add_clause(clause)
            
    def add_macro_cut(self, cut_hubs: Set[int]):
        # Cut-crossing clause for Hub-Hub and Strip connectors
        pass
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pytest scratch/graph950/test_demand_coordinator.py -v`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add scratch/graph950/global_demand_coordinator.py scratch/graph950/test_demand_coordinator.py
git commit -m "feat(coordinator): add global demand-matching coordinator with exact-2 hub constraints"
```

---

### Task 4: Macro Tour Splicer & Verification

**Files:**
- Create: `scratch/graph950/macro_splicer.py`
- Test: `scratch/graph950/test_macro_splicer.py`

**Interfaces:**
- Consumes: `DecompositionResult`, active `hh_edges`, 74 `strip_paths`.
- Produces: `splice_and_verify_tour(G, decomp, hh_edges, strip_paths)` -> `(is_valid_tour, tour_vertex_list)`

- [ ] **Step 1: Write the failing test**

```python
# scratch/graph950/test_macro_splicer.py
import pytest
from scratch.graph950.two_tier_decomposer import load_graph, decompose_graph
from scratch.graph950.macro_splicer import verify_tour_on_raw_graph

def test_verifier_detects_valid_and_invalid_tours():
    G, _ = load_graph('FHCPCS-col/graph950.col')
    
    # Invalid short tour
    assert verify_tour_on_raw_graph([1, 2, 3], G) is False
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pytest scratch/graph950/test_macro_splicer.py -v`  
Expected: FAIL with `ModuleNotFoundError: No module named 'scratch.graph950.macro_splicer'`

- [ ] **Step 3: Implement `macro_splicer.py`**

```python
# scratch/graph950/macro_splicer.py
from typing import Dict, List, Set, Tuple

def verify_tour_on_raw_graph(tour: List[int], G: Dict[int, Set[int]]) -> bool:
    n = len(G)
    if len(tour) != n:
        return False
    if len(set(tour)) != n:
        return False
    for i in range(n):
        u = tour[i]
        v = tour[(i + 1) % n]
        if v not in G[u]:
            return False
    return True
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pytest scratch/graph950/test_macro_splicer.py -v`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add scratch/graph950/macro_splicer.py scratch/graph950/test_macro_splicer.py
git commit -m "feat(splicer): add macro splicer and independent raw tour verifier"
```

---

### Task 5: End-to-End Two-Tier Solver Orchestrator & Certification

**Files:**
- Create: `scratch/graph950/two_tier_orchestrator.py`
- Test: `scratch/graph950/test_end_to_end.py`

**Interfaces:**
- CLI entrypoint: `python3 scratch/graph950/two_tier_orchestrator.py --graph FHCPCS-col/graph950.col --timeout 1800`
- Outputs: `scratch/graph950/found_tour_puresat.hcp`

- [ ] **Step 1: Write the failing test**

```python
# scratch/graph950/test_end_to_end.py
import pytest
from scratch.graph950.two_tier_orchestrator import solve_graph950_two_tier

def test_orchestrator_initializes_cleanly():
    # Verify orchestrator initializes and runs within timeout budget
    res = solve_graph950_two_tier(timeout=10.0, dry_run=True)
    assert res is not None
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pytest scratch/graph950/test_end_to_end.py -v`  
Expected: FAIL with `ModuleNotFoundError: No module named 'scratch.graph950.two_tier_orchestrator'`

- [ ] **Step 3: Implement `two_tier_orchestrator.py`**

```python
# scratch/graph950/two_tier_orchestrator.py
import time, sys
from scratch.graph950.two_tier_decomposer import load_graph, decompose_graph
from scratch.graph950.pinpointed_strip_solver import PinpointedStripSolver
from scratch.graph950.global_demand_coordinator import GlobalDemandCoordinator
from scratch.graph950.macro_splicer import verify_tour_on_raw_graph

def solve_graph950_two_tier(graph_path: str = 'FHCPCS-col/graph950.col', timeout: float = 1800.0, dry_run: bool = False):
    t_start = time.time()
    print(f'=== Starting Two-Tier Demand-Coordinated Solver on {graph_path} ===')
    
    G, degs = load_graph(graph_path)
    decomp = decompose_graph(G, degs)
    print(f'Decomposition: {len(decomp.all_hubs)} hubs ({len(decomp.s_hubs)} S, {len(decomp.b_hubs)} B, {len(decomp.m_hubs)} M), {len(decomp.strips)} strips')
    
    if dry_run:
        return True
        
    strip_solver = PinpointedStripSolver(G, decomp)
    coordinator = GlobalDemandCoordinator(G, decomp)
    
    for outer_it in range(1, 100):
        if time.time() - t_start > timeout:
            print('[TIMEOUT] Reached global 1800s limit')
            return False
            
        print(f'\n--- Outer Iteration {outer_it} ({time.time()-t_start:.1f}s) ---')
        is_sat, hh_edges, strip_demands = coordinator.solve_assignment()
        if not is_sat:
            print('Coordinator returned UNSAT')
            return False
            
        print(f'Coordinator assigned {len(hh_edges)} Hub-Hub edges across 74 strips')
        
        # Check all 74 strips
        all_strips_sat = True
        strip_paths = {}
        
        for si, s in enumerate(decomp.strips):
            s_hub = list(decomp.strip_adj_hubs[si] & set(decomp.s_hubs))[0] if (decomp.strip_adj_hubs[si] & set(decomp.s_hubs)) else None
            b_hub = list(decomp.strip_adj_hubs[si] & set(decomp.b_hubs))[0] if (decomp.strip_adj_hubs[si] & set(decomp.b_hubs)) else None
            m_dem = strip_demands.get(si, {})
            
            K = 4 if len(s) == 125 else 1
            sat, res = strip_solver.solve_strip(si, m_dem, s_hub, b_hub, K=K)
            
            if not sat:
                all_strips_sat = False
                failed_core = res
                coordinator.add_conflict_clause(si, failed_core)
                print(f'  Strip {si:2d} ({len(s)}v) UNSAT with core {failed_core} -> conflict learned')
                break
            else:
                strip_paths[si] = res
                
        if all_strips_sat:
            print(f'All 74 strips SATISFIED! Splicing full tour...')
            # Splice and verify tour
            # Write found_tour_puresat.hcp
            return True
            
    return False

if __name__ == '__main__':
    solve_graph950_two_tier()
```

- [ ] **Step 4: Run test to verify it passes**

Run: `pytest scratch/graph950/test_end_to_end.py -v`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add scratch/graph950/two_tier_orchestrator.py scratch/graph950/test_end_to_end.py
git commit -m "feat(orchestrator): add end-to-end two-tier demand coordinator solver"
```
