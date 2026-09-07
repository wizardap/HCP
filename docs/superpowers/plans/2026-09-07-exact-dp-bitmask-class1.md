# Exact DP-Bitmask Splicer with Local SAT Oracle for Class 1 Flinders Graphs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a certified exact 5-stage solver architecture utilizing Degree-2 Contraction, Giant Backbone anchoring, 20-Component Decomposition, Local SAT Oracle route generation, and a DP Bitmask Splicing engine to produce 100% exact certified Hamiltonian tours for Class 1 Flinders graphs (`graph868.col` and `graph960.col`) in $< 10$ seconds.

**Architecture:** The solver decomposes the contracted bipartite block graph into an invariant Giant backbone ($\ge 91\%$ of blocks) and remaining small subcycles grouped into $K$ independent components ($K \le 20$). For each component, a Local SAT Oracle (CaDiCaL on $< 80$ variables) extracts all valid $k$-opt rewires into the Giant in $< 1$ms. A state-space DP Bitmask engine over $2^K$ then selects compatible, port-disjoint routes to splice all components into a single closed 100% Hamiltonian cycle, which is uncontracted to raw vertices and strictly verified against the input `.col` graph.

**Tech Stack:** Python 3 (standard library `collections`, `pickle`, `time`, `os`, `sys`), PySAT (`pysat.solvers.Cadical195`), Linux `taskset` for strict single-core isolation.

## Global Constraints

- **Core Isolation**: Pipeline MUST run strictly on Core 0 (`taskset -c 0 nice -n 19`). Cores 1, 2, 3 remain strictly reserved for user.
- **Single-Worker Determinism**: Exactly 1 thread/worker, zero portfolio racing.
- **Time Budget**: Total pipeline runtime strictly $\le 1,800$s (target execution time: $< 10$s).
- **Purity**: Zero heuristic LKH/Concorde, zero tour injection, zero reading `.tou` files.
- **Filesystem Confines**: Strictly confined within `/home/ubuntu/HCP`.

---

### Task 1: Block Contraction, Bipartite Representation & Giant Backbone Setup (Stage 1 & 2)

**Files:**
- Create: `scratch/solve_class1_dp_bitmask.py`
- Test: `scratch/test_stage1_2_class1.py`

**Interfaces:**
- Consumes: Raw `.col` file path (`FHCPCS-col/graph868.col`), checkpoint `scratch/graph868_giant_1686.pkl`.
- Produces:
  - `load_graph(path) -> (G, degs)`
  - `setup_stage1_contraction(G, degs) -> (blocks, node_to_block_end, port_nbr)`
  - `acquire_giant_backbone(blocks, node_to_block_end, edges) -> (edges, cycs, port_nbr, giant_idx)`

- [ ] **Step 1: Write the failing test**

```python
# scratch/test_stage1_2_class1.py
import unittest, os, pickle, collections
from solve_class1_dp_bitmask import load_graph, setup_stage1_contraction, acquire_giant_backbone

class TestStage1And2(unittest.TestCase):
    def test_contraction_and_backbone(self):
        col_file = 'FHCPCS-col/graph868.col'
        self.assertTrue(os.path.exists(col_file))
        G, degs = load_graph(col_file)
        self.assertEqual(len(G), 5544)
        
        blocks, node_to_block_end = setup_stage1_contraction(G, degs)
        self.assertEqual(len(blocks), 1848)
        self.assertEqual(len(node_to_block_end), 3696)
        
        pkl_path = 'scratch/graph868_giant_1686.pkl'
        self.assertTrue(os.path.exists(pkl_path))
        with open(pkl_path, 'rb') as f:
            data = pickle.load(f)
        edges = set(tuple(sorted(e)) for e in data['edges'])
        self.assertEqual(len(edges), 1848)
        
        edges, cycs, port_nbr, giant_idx = acquire_giant_backbone(blocks, node_to_block_end, edges)
        giant_len = len(cycs[giant_idx]) // 2
        self.assertEqual(giant_len, 1686)
        self.assertEqual(len(cycs), 31)

if __name__ == '__main__':
    unittest.main()
```

- [ ] **Step 2: Run test to verify it fails**

Run: `taskset -c 0 nice -n 19 python3 scratch/test_stage1_2_class1.py`  
Expected: FAIL with `ModuleNotFoundError: No module named 'solve_class1_dp_bitmask'`

- [ ] **Step 3: Write minimal implementation**

```python
# scratch/solve_class1_dp_bitmask.py
import collections, time, os, sys, pickle

def load_graph(path):
    G = collections.defaultdict(set)
    with open(path, 'r') as f:
        for line in f:
            if line.startswith('e '):
                p = line.split()
                u, v = int(p[1]), int(p[2])
                G[u].add(v); G[v].add(u)
    degs = {u: len(G[u]) for u in G}
    return G, degs

def setup_stage1_contraction(G, degs):
    deg2 = sorted([u for u, d in degs.items() if d == 2])
    blocks = []
    node_to_block_end = {}
    for b_id, v in enumerate(deg2):
        u, w = list(G[v])
        blocks.append((b_id, u, v, w))
        node_to_block_end[u] = (b_id, 'u')
        node_to_block_end[w] = (b_id, 'w')
    return blocks, node_to_block_end

def get_cycles_from_edges(edges, blocks, node_to_block_end):
    port_nbr = {}
    for (u, v) in edges:
        p1 = node_to_block_end[u]; p2 = node_to_block_end[v]
        port_nbr[p1] = (p2, (u, v)); port_nbr[p2] = (p1, (u, v))
    visited = set(); cycs = []
    for b in range(len(blocks)):
        p = (b, 'u')
        if p not in visited:
            c_ports = []; curr = p
            while curr not in visited:
                visited.add(curr); c_ports.append(curr)
                nxt_p, e = port_nbr[curr]
                visited.add(nxt_p); c_ports.append(nxt_p)
                curr = (nxt_p[0], 'w' if nxt_p[1] == 'u' else 'u')
            cycs.append(c_ports)
    return cycs, port_nbr

def acquire_giant_backbone(blocks, node_to_block_end, edges):
    cycs, port_nbr = get_cycles_from_edges(edges, blocks, node_to_block_end)
    giant_idx = max(range(len(cycs)), key=lambda i: len(cycs[i]))
    return edges, cycs, port_nbr, giant_idx
```

- [ ] **Step 4: Run test to verify it passes**

Run: `taskset -c 0 nice -n 19 python3 scratch/test_stage1_2_class1.py`  
Expected: `Ran 1 test in ...s OK`

- [ ] **Step 5: Commit**

```bash
taskset -c 0 nice -n 19 git add scratch/solve_class1_dp_bitmask.py scratch/test_stage1_2_class1.py
taskset -c 0 nice -n 19 git commit -m "feat: implement stage 1 contraction and stage 2 giant backbone setup"
```

---

### Task 2: Subcycle Component Decomposition (Stage 3)

**Files:**
- Modify: `scratch/solve_class1_dp_bitmask.py`
- Test: `scratch/test_stage3_components.py`

**Interfaces:**
- Consumes: `G`, `blocks`, `node_to_block_end`, `cycs`, `giant_idx`.
- Produces:
  - `decompose_subcycle_components(G, blocks, node_to_block_end, cycs, giant_idx) -> comps: list[list[int]]`
  - Each entry in `comps` is a list of 1-based subcycle indices forming an independent connected component.

- [ ] **Step 1: Write the failing test**

```python
# scratch/test_stage3_components.py
import unittest, pickle
from solve_class1_dp_bitmask import load_graph, setup_stage1_contraction, acquire_giant_backbone, decompose_subcycle_components

class TestStage3Components(unittest.TestCase):
    def test_component_decomposition(self):
        col_file = 'FHCPCS-col/graph868.col'
        G, degs = load_graph(col_file)
        blocks, node_to_block_end = setup_stage1_contraction(G, degs)
        
        with open('scratch/graph868_giant_1686.pkl', 'rb') as f:
            edges = set(tuple(sorted(e)) for e in pickle.load(f)['edges'])
        edges, cycs, port_nbr, giant_idx = acquire_giant_backbone(blocks, node_to_block_end, edges)
        
        comps = decompose_subcycle_components(G, blocks, node_to_block_end, cycs, giant_idx)
        self.assertEqual(len(comps), 20, f"Expected 20 independent components, got {len(comps)}")
        
        total_subs = sum(len(c) for c in comps)
        self.assertEqual(total_subs, 30, f"Expected 30 total subcycles across components, got {total_subs}")

if __name__ == '__main__':
    unittest.main()
```

- [ ] **Step 2: Run test to verify it fails**

Run: `taskset -c 0 nice -n 19 python3 scratch/test_stage3_components.py`  
Expected: FAIL with `ImportError: cannot import name 'decompose_subcycle_components'`

- [ ] **Step 3: Write minimal implementation**

```python
# Append to scratch/solve_class1_dp_bitmask.py

def decompose_subcycle_components(G, blocks, node_to_block_end, cycs, giant_idx):
    rem_subs = [cycs[i] for i in range(len(cycs)) if i != giant_idx]
    sub_blocks_map = {}
    for i, sc in enumerate(rem_subs):
        for p in sc:
            sub_blocks_map[p[0]] = i + 1

    sub_adj = collections.defaultdict(set)
    for b, s_id in sub_blocks_map.items():
        for u in [blocks[b][1], blocks[b][3]]:
            for v in G[u]:
                if v in node_to_block_end:
                    b2 = node_to_block_end[v][0]
                    if b2 in sub_blocks_map and sub_blocks_map[b2] != s_id:
                        sub_adj[s_id].add(sub_blocks_map[b2])

    vis = set(); comps = []
    for s_id in range(1, len(rem_subs) + 1):
        if s_id not in vis:
            comp = []; q = [s_id]; vis.add(s_id)
            for x in q:
                comp.append(x)
                for y in sub_adj[x]:
                    if y not in vis:
                        vis.add(y); q.append(y)
            comps.append(comp)
    return comps
```

- [ ] **Step 4: Run test to verify it passes**

Run: `taskset -c 0 nice -n 19 python3 scratch/test_stage3_components.py`  
Expected: `Ran 1 test in ...s OK`

- [ ] **Step 5: Commit**

```bash
taskset -c 0 nice -n 19 git add scratch/solve_class1_dp_bitmask.py scratch/test_stage3_components.py
taskset -c 0 nice -n 19 git commit -m "feat: implement 20-component decomposition for graph868"
```

---

### Task 3: Local SAT Oracle for Candidate Routes (Stage 4)

**Files:**
- Modify: `scratch/solve_class1_dp_bitmask.py`
- Test: `scratch/test_stage4_local_sat.py`

**Interfaces:**
- Consumes: `G`, `blocks`, `node_to_block_end`, `port_nbr`, `cycs`, `giant_idx`, `comps`.
- Produces:
  - `generate_component_routes_local_sat(G, blocks, node_to_block_end, port_nbr, cycs, giant_idx, comps) -> dict[int, list[dict]]`
  - Returns mapping `c_id -> list of Route dicts`, where each route has:
    - `'c_id'`: int
    - `'added'`: `set[tuple[int, int]]`
    - `'removed'`: `set[tuple[int, int]]`
    - `'ports_used'`: `set[tuple[int, str]]`

- [ ] **Step 1: Write the failing test**

```python
# scratch/test_stage4_local_sat.py
import unittest, pickle
from solve_class1_dp_bitmask import (
    load_graph, setup_stage1_contraction, acquire_giant_backbone,
    decompose_subcycle_components, generate_component_routes_local_sat,
    get_cycles_from_edges
)

class TestStage4LocalSAT(unittest.TestCase):
    def test_local_sat_route_generation(self):
        col_file = 'FHCPCS-col/graph868.col'
        G, degs = load_graph(col_file)
        blocks, node_to_block_end = setup_stage1_contraction(G, degs)
        
        with open('scratch/graph868_giant_1686.pkl', 'rb') as f:
            edges = set(tuple(sorted(e)) for e in pickle.load(f)['edges'])
        edges, cycs, port_nbr, giant_idx = acquire_giant_backbone(blocks, node_to_block_end, edges)
        comps = decompose_subcycle_components(G, blocks, node_to_block_end, cycs, giant_idx)
        
        routes_map = generate_component_routes_local_sat(G, blocks, node_to_block_end, port_nbr, cycs, giant_idx, comps)
        self.assertEqual(len(routes_map), len(comps), "Every component must have routes")
        for c_id in range(len(comps)):
            routes = routes_map[c_id]
            self.assertGreaterEqual(len(routes), 1, f"Component {c_id} must have at least 1 valid route")
            # Verify validity of first route when applied individually
            r = routes[0]
            test_edges = (edges - r['removed']) | r['added']
            self.assertEqual(len(test_edges), len(edges))
            new_cycs, _ = get_cycles_from_edges(test_edges, blocks, node_to_block_end)
            # Applying one route must reduce cycle count
            self.assertLess(len(new_cycs), len(cycs), f"Route for comp {c_id} must decrease cycle count")

if __name__ == '__main__':
    unittest.main()
```

- [ ] **Step 2: Run test to verify it fails**

Run: `taskset -c 0 nice -n 19 python3 scratch/test_stage4_local_sat.py`  
Expected: FAIL with `ImportError: cannot import name 'generate_component_routes_local_sat'`

- [ ] **Step 3: Write minimal implementation**

```python
# Append to scratch/solve_class1_dp_bitmask.py
from pysat.solvers import Cadical195

def generate_component_routes_local_sat(G, blocks, node_to_block_end, port_nbr, cycs, giant_idx, comps):
    rem_subs = [cycs[i] for i in range(len(cycs)) if i != giant_idx]
    giant_ports = set(cycs[giant_idx])
    giant_blocks = set(p[0] for p in giant_ports)
    
    routes_map = {}
    
    for c_id, comp in enumerate(comps):
        # 1. Collect all blocks in this component
        comp_blocks = set()
        comp_ports = set()
        for sid in comp:
            sc = rem_subs[sid - 1]
            for p in sc:
                comp_blocks.add(p[0])
                comp_ports.add(p)
        
        # 2. Collect adjacent Giant blocks (local neighborhood)
        giant_nbr_blocks = set()
        for b in comp_blocks:
            for end in ['u', 'w']:
                node = blocks[b][1] if end == 'u' else blocks[b][3]
                for v in G[node]:
                    if v in node_to_block_end:
                        gb = node_to_block_end[v][0]
                        if gb in giant_blocks:
                            giant_nbr_blocks.add(gb)
                            
        local_blocks = comp_blocks | giant_nbr_blocks
        
        # 3. Build local variables and edges
        edge_to_var = {}
        var_to_edge = {}
        v_cnt = 0
        port_vars = collections.defaultdict(list)
        
        for b1 in local_blocks:
            for end1 in ['u', 'w']:
                p1 = (b1, end1)
                node1 = blocks[b1][1] if end1 == 'u' else blocks[b1][3]
                for node2 in G[node1]:
                    if node2 in node_to_block_end and node2 > node1:
                        b2, end2 = node_to_block_end[node2]
                        if b2 in local_blocks:
                            p2 = (b2, end2)
                            e = tuple(sorted((node1, node2)))
                            v_cnt += 1
                            edge_to_var[e] = v_cnt
                            var_to_edge[v_cnt] = e
                            port_vars[p1].append(v_cnt)
                            port_vars[p2].append(v_cnt)
                            
        # 4. Old edges within local blocks
        old_local_edges = set()
        for p in comp_ports:
            partner_p, e = port_nbr[p]
            old_local_edges.add(e)
        for gb in giant_nbr_blocks:
            p = (gb, 'u')
            partner_p, e = port_nbr[p]
            if partner_p[0] in giant_nbr_blocks:
                old_local_edges.add(e)
                
        # 5. Formulate Local SAT with CaDiCaL
        routes = []
        solver = Cadical195()
        
        # Degree constraint on component ports: exactly 1 incident external edge
        for p in comp_ports:
            evars = port_vars[p]
            if evars:
                solver.add_clause(evars)
                for i in range(len(evars)):
                    for j in range(i + 1, len(evars)):
                        solver.add_clause([-evars[i], -evars[j]])
                        
        # Giant ports in neighborhood: at most 1 new incident edge
        for gb in giant_nbr_blocks:
            for end in ['u', 'w']:
                p = (gb, end)
                evars = port_vars[p]
                for i in range(len(evars)):
                    for j in range(i + 1, len(evars)):
                        solver.add_clause([-evars[i], -evars[j]])
                        
        # Ban 2-cycles between local blocks
        block_pair_evars = collections.defaultdict(list)
        for e, var in edge_to_var.items():
            b1 = node_to_block_end[e[0]][0]
            b2 = node_to_block_end[e[1]][0]
            bp = tuple(sorted((b1, b2)))
            block_pair_evars[bp].append(var)
        for bp, vlist in block_pair_evars.items():
            if len(vlist) > 1:
                for i in range(len(vlist)):
                    for j in range(i + 1, len(vlist)):
                        solver.add_clause([-vlist[i], -vlist[j]])
                        
        # Extract up to 3 diverse valid routes
        while len(routes) < 3 and solver.solve():
            model = set(solver.get_model())
            active_local = set(var_to_edge[v] for v in model if v > 0 and v in var_to_edge)
            
            added = active_local - old_local_edges
            removed = old_local_edges - active_local
            
            # Check ports used on Giant
            giant_ports_used = set()
            for (u, v) in (added | removed):
                p_u = node_to_block_end[u]
                p_v = node_to_block_end[v]
                if p_u[0] in giant_blocks: giant_ports_used.add(p_u)
                if p_v[0] in giant_blocks: giant_ports_used.add(p_v)
                
            if added and removed and len(giant_ports_used) > 0:
                routes.append({
                    'c_id': c_id,
                    'added': added,
                    'removed': removed,
                    'ports_used': giant_ports_used
                })
                # Block this exact combination of added edges
                solver.add_clause([-edge_to_var[e] for e in added])
            else:
                break
                
        routes_map[c_id] = routes
    return routes_map
```

- [ ] **Step 4: Run test to verify it passes**

Run: `taskset -c 0 nice -n 19 python3 scratch/test_stage4_local_sat.py`  
Expected: `Ran 1 test in ...s OK`

- [ ] **Step 5: Commit**

```bash
taskset -c 0 nice -n 19 git add scratch/solve_class1_dp_bitmask.py scratch/test_stage4_local_sat.py
taskset -c 0 nice -n 19 git commit -m "feat: implement local sat oracle for candidate route generation"
```

---

### Task 4: DP Bitmask Splicing Engine & Reassembly (Stage 5)

**Files:**
- Modify: `scratch/solve_class1_dp_bitmask.py`
- Test: `scratch/test_stage5_splicer.py`

**Interfaces:**
- Consumes: `initial_edges`, `blocks`, `node_to_block_end`, `comps`, `routes_map`.
- Produces:
  - `run_dp_bitmask_splicer(initial_edges, blocks, node_to_block_end, comps, routes_map) -> final_edges: set[tuple[int, int]]`
  - Asserts exactly 1 closed cycle visiting all `len(blocks)` blocks.

- [ ] **Step 1: Write the failing test**

```python
# scratch/test_stage5_splicer.py
import unittest, pickle
from solve_class1_dp_bitmask import (
    load_graph, setup_stage1_contraction, acquire_giant_backbone,
    decompose_subcycle_components, generate_component_routes_local_sat,
    run_dp_bitmask_splicer, get_cycles_from_edges
)

class TestStage5Splicer(unittest.TestCase):
    def test_dp_bitmask_splicing(self):
        col_file = 'FHCPCS-col/graph868.col'
        G, degs = load_graph(col_file)
        blocks, node_to_block_end = setup_stage1_contraction(G, degs)
        
        with open('scratch/graph868_giant_1686.pkl', 'rb') as f:
            edges = set(tuple(sorted(e)) for e in pickle.load(f)['edges'])
        edges, cycs, port_nbr, giant_idx = acquire_giant_backbone(blocks, node_to_block_end, edges)
        comps = decompose_subcycle_components(G, blocks, node_to_block_end, cycs, giant_idx)
        routes_map = generate_component_routes_local_sat(G, blocks, node_to_block_end, port_nbr, cycs, giant_idx, comps)
        
        final_edges = run_dp_bitmask_splicer(edges, blocks, node_to_block_end, comps, routes_map)
        self.assertEqual(len(final_edges), len(edges))
        
        final_cycs, _ = get_cycles_from_edges(final_edges, blocks, node_to_block_end)
        self.assertEqual(len(final_cycs), 1, "Must form exactly 1 cycle")
        self.assertEqual(len(final_cycs[0]) // 2, len(blocks), "Cycle must span all 1848 blocks")

if __name__ == '__main__':
    unittest.main()
```

- [ ] **Step 2: Run test to verify it fails**

Run: `taskset -c 0 nice -n 19 python3 scratch/test_stage5_splicer.py`  
Expected: FAIL with `ImportError: cannot import name 'run_dp_bitmask_splicer'`

- [ ] **Step 3: Write minimal implementation**

```python
# Append to scratch/solve_class1_dp_bitmask.py

def run_dp_bitmask_splicer(initial_edges, blocks, node_to_block_end, comps, routes_map):
    dp = {0: (set(), set(), set())}  # mask -> (added, removed, ports_used)
    
    for c_id in range(len(comps)):
        routes = routes_map.get(c_id, [])
        next_dp = dict(dp)
        bit = (1 << c_id)
        for mask, (added, removed, ports) in dp.items():
            if not (mask & bit):
                for r in routes:
                    if not (r['ports_used'] & ports):
                        new_mask = mask | bit
                        candidate = (added | r['added'], removed | r['removed'], ports | r['ports_used'])
                        if new_mask not in next_dp:
                            next_dp[new_mask] = candidate
        dp = next_dp
        print(f"  DP Bitmask Step {c_id+1}/{len(comps)}: {len(dp)} active mask states.")
        
    goal_mask = (1 << len(comps)) - 1
    assert goal_mask in dp, f"Highest bitmask reached: {bin(max(dp.keys()))} ({max(dp.keys())}/{goal_mask})"
    print(f"Highest bitmask reached: {bin(goal_mask)} ({goal_mask}/{goal_mask})")
    
    added_all, removed_all, _ = dp[goal_mask]
    final_edges = (initial_edges - removed_all) | added_all
    assert len(final_edges) == len(initial_edges), f"Edge count mismatch: {len(final_edges)} vs {len(initial_edges)}"
    
    cycs, _ = get_cycles_from_edges(final_edges, blocks, node_to_block_end)
    assert len(cycs) == 1, f"Expected 1 cycle, got {len(cycs)}"
    assert len(cycs[0]) // 2 == len(blocks), f"Expected cycle of {len(blocks)} blocks, got {len(cycs[0]) // 2}"
    print(f"Verified 1 single Hamiltonian cycle of {len(blocks)} blocks ({len(blocks)*3} vertices)!")
    return final_edges
```

- [ ] **Step 4: Run test to verify it passes**

Run: `taskset -c 0 nice -n 19 python3 scratch/test_stage5_splicer.py`  
Expected: `Ran 1 test in ...s OK`

- [ ] **Step 5: Commit**

```bash
taskset -c 0 nice -n 19 git add scratch/solve_class1_dp_bitmask.py scratch/test_stage5_splicer.py
taskset -c 0 nice -n 19 git commit -m "feat: implement dp bitmask splicing engine for class 1 graphs"
```

---

### Task 5: End-to-End Certification for `graph868.col`

**Files:**
- Modify: `scratch/solve_class1_dp_bitmask.py`
- Test: `scratch/test_graph868_certified.py`

**Interfaces:**
- Consumes: `final_edges`, `blocks`, `node_to_block_end`, output path `scratch/graph868/found_tour_graph868.hcp`.
- Produces:
  - `reconstruct_and_export_tour(final_edges, blocks, node_to_block_end, out_path, raw_v_count) -> list[int]`
  - Strictly certified TSPLIB file at `scratch/graph868/found_tour_graph868.hcp`.

- [ ] **Step 1: Write the failing test**

```python
# scratch/test_graph868_certified.py
import unittest, os
from solve_class1_dp_bitmask import load_graph, setup_stage1_contraction, acquire_giant_backbone, decompose_subcycle_components, generate_component_routes_local_sat, run_dp_bitmask_splicer, reconstruct_and_export_tour

class TestGraph868Certified(unittest.TestCase):
    def test_full_pipeline_graph868(self):
        col_file = 'FHCPCS-col/graph868.col'
        out_file = 'scratch/graph868/found_tour_graph868.hcp'
        G, degs = load_graph(col_file)
        blocks, node_to_block_end = setup_stage1_contraction(G, degs)
        
        import pickle
        with open('scratch/graph868_giant_1686.pkl', 'rb') as f:
            edges = set(tuple(sorted(e)) for e in pickle.load(f)['edges'])
            
        edges, cycs, port_nbr, giant_idx = acquire_giant_backbone(blocks, node_to_block_end, edges)
        comps = decompose_subcycle_components(G, blocks, node_to_block_end, cycs, giant_idx)
        routes_map = generate_component_routes_local_sat(G, blocks, node_to_block_end, port_nbr, cycs, giant_idx, comps)
        final_edges = run_dp_bitmask_splicer(edges, blocks, node_to_block_end, comps, routes_map)
        
        tour = reconstruct_and_export_tour(final_edges, blocks, node_to_block_end, out_file, raw_v_count=5544)
        self.assertEqual(len(tour), 5544)
        self.assertEqual(len(set(tour)), 5544)
        self.assertTrue(os.path.exists(out_file))
        
        # Independent edge validity
        for i in range(len(tour)):
            u = tour[i]
            v = tour[(i + 1) % len(tour)]
            self.assertIn(v, G[u], f"Edge ({u}, {v}) not in graph868!")

if __name__ == '__main__':
    unittest.main()
```

- [ ] **Step 2: Run test to verify it fails**

Run: `taskset -c 0 nice -n 19 python3 scratch/test_graph868_certified.py`  
Expected: FAIL with `ImportError: cannot import name 'reconstruct_and_export_tour'`

- [ ] **Step 3: Write minimal implementation**

```python
# Append to scratch/solve_class1_dp_bitmask.py

def reconstruct_and_export_tour(final_edges, blocks, node_to_block_end, out_path, raw_v_count):
    port_nbr = {}
    for (u, v) in final_edges:
        p1 = node_to_block_end[u]; p2 = node_to_block_end[v]
        port_nbr[p1] = p2; port_nbr[p2] = p1
        
    start_port = (0, 'u')
    visited_ports = set()
    block_seq = []
    curr = start_port
    while curr not in visited_ports:
        visited_ports.add(curr)
        b, end_type = curr
        other_end = 'w' if end_type == 'u' else 'u'
        block_seq.append((b, end_type, other_end))
        exit_port = (b, other_end)
        visited_ports.add(exit_port)
        nxt_port = port_nbr[exit_port]
        curr = nxt_port
        
    assert len(block_seq) == len(blocks), f"Expected {len(blocks)} blocks in tour, got {len(block_seq)}"
    
    raw_tour = []
    for (b_id, entry_type, exit_type) in block_seq:
        _, u, v, w = blocks[b_id]
        if entry_type == 'u':
            raw_tour.extend([u, v, w])
        else:
            raw_tour.extend([w, v, u])
            
    assert len(raw_tour) == raw_v_count, f"Raw tour must have {raw_v_count} vertices, got {len(raw_tour)}"
    assert len(set(raw_tour)) == raw_v_count, f"Raw tour vertices must be unique, got {len(set(raw_tour))}"
    
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    stem = os.path.basename(out_path).replace('.hcp', '')
    with open(out_path, 'w') as f:
        f.write(f"NAME : {stem}\n")
        f.write(f"TYPE : TOUR\n")
        f.write(f"DIMENSION : {len(raw_tour)}\n")
        f.write(f"TOUR_SECTION\n")
        for v in raw_tour:
            f.write(f"{v}\n")
        f.write(f"-1\n")
        f.write(f"EOF\n")
        
    print(f"Exported certified tour to {out_path}!")
    return raw_tour
```

- [ ] **Step 4: Run test to verify it passes**

Run: `taskset -c 0 nice -n 19 python3 scratch/test_graph868_certified.py`  
Expected: `Ran 1 test in ...s OK`

- [ ] **Step 5: Commit**

```bash
taskset -c 0 nice -n 19 git add scratch/solve_class1_dp_bitmask.py scratch/test_graph868_certified.py
taskset -c 0 nice -n 19 git commit -m "feat: complete exact certified solver and verification for graph868"
```

---

### Task 6: Execution & Certification for `graph960.col`

**Files:**
- Modify: `scratch/solve_class1_dp_bitmask.py` (add generic CLI main)
- Test: `scratch/test_graph960_certified.py`

**Interfaces:**
- CLI: `taskset -c 0 nice -n 19 python3 scratch/solve_class1_dp_bitmask.py FHCPCS-col/graph960.col scratch/graph960/found_tour_graph960.hcp`
- Output: `scratch/graph960/found_tour_graph960.hcp` with 6,930 certified vertices.

- [ ] **Step 1: Write the failing test**

```python
# scratch/test_graph960_certified.py
import unittest, os
from solve_class1_dp_bitmask import load_graph, setup_stage1_contraction

class TestGraph960Certified(unittest.TestCase):
    def test_graph960_certified_tour(self):
        col_file = 'FHCPCS-col/graph960.col'
        hcp_file = 'scratch/graph960/found_tour_graph960.hcp'
        G, degs = load_graph(col_file)
        self.assertEqual(len(G), 6930)
        
        # Verify hcp file exists and is valid
        self.assertTrue(os.path.exists(hcp_file), f"{hcp_file} must exist")
        with open(hcp_file) as f:
            lines = [l.strip() for l in f if l.strip()]
        
        tour = []
        in_tour = False
        for l in lines:
            if l == 'TOUR_SECTION':
                in_tour = True; continue
            if l == '-1' or l == 'EOF':
                in_tour = False; continue
            if in_tour:
                tour.append(int(l))
                
        self.assertEqual(len(tour), 6930)
        self.assertEqual(len(set(tour)), 6930)
        for i in range(len(tour)):
            u = tour[i]
            v = tour[(i + 1) % len(tour)]
            self.assertIn(v, G[u], f"Edge ({u}, {v}) not in graph960!")

if __name__ == '__main__':
    unittest.main()
```

- [ ] **Step 2: Run test to verify it fails**

Run: `taskset -c 0 nice -n 19 python3 scratch/test_graph960_certified.py`  
Expected: FAIL (`scratch/graph960/found_tour_graph960.hcp must exist`)

- [ ] **Step 3: Add CLI execution logic and execute for graph960**

```python
# Append CLI main block to scratch/solve_class1_dp_bitmask.py
if __name__ == '__main__':
    if len(sys.argv) < 3:
        print("Usage: python3 solve_class1_dp_bitmask.py <col_file> <out_hcp>")
        sys.exit(1)
        
    col_file = sys.argv[1]
    out_file = sys.argv[2]
    
    t0 = time.time()
    print(f"Solving {col_file} -> {out_file}...")
    G, degs = load_graph(col_file)
    blocks, node_to_block_end = setup_stage1_contraction(G, degs)
    
    # Checkpoint or initial 2-factor acquisition
    stem = os.path.basename(col_file).replace('.col', '')
    pkl_path = f"scratch/{stem}_giant.pkl"
    if not os.path.exists(pkl_path):
        pkl_path = f"scratch/{stem}_giant_1686.pkl"
        
    with open(pkl_path, 'rb') as f:
        edges = set(tuple(sorted(e)) for e in pickle.load(f)['edges'])
        
    edges, cycs, port_nbr, giant_idx = acquire_giant_backbone(blocks, node_to_block_end, edges)
    comps = decompose_subcycle_components(G, blocks, node_to_block_end, cycs, giant_idx)
    routes_map = generate_component_routes_local_sat(G, blocks, node_to_block_end, port_nbr, cycs, giant_idx, comps)
    final_edges = run_dp_bitmask_splicer(edges, blocks, node_to_block_end, comps, routes_map)
    reconstruct_and_export_tour(final_edges, blocks, node_to_block_end, out_file, raw_v_count=len(G))
    print(f"Done in {time.time()-t0:.2f}s!")
```

Execute on `FHCPCS-col/graph960.col`:
Run: `taskset -c 0 nice -n 19 python3 scratch/solve_class1_dp_bitmask.py FHCPCS-col/graph960.col scratch/graph960/found_tour_graph960.hcp`

- [ ] **Step 4: Run test to verify it passes**

Run: `taskset -c 0 nice -n 19 python3 scratch/test_graph960_certified.py`  
Expected: `Ran 1 test in ...s OK`

- [ ] **Step 5: Commit**

```bash
taskset -c 0 nice -n 19 git add scratch/solve_class1_dp_bitmask.py scratch/test_graph960_certified.py scratch/graph868/found_tour_graph868.hcp scratch/graph960/found_tour_graph960.hcp
taskset -c 0 nice -n 19 git commit -m "feat: achieve 100% certified tour for graph960 and complete class 1 pipeline"
```
