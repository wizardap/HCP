# Pure SAT Two-Tier Decomposed Solver for graph950 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement and execute a 100% Pure SAT Two-Tier Decomposed Solver for `graph950.col` (6,620 vertices, 28,718 edges) without any tour injection or hints, verifying a certified Hamiltonian cycle.

**Architecture:** Decompose `graph950` into 310 hubs and 64 bulk strips. In Phase 1, generate path covers using Targeted Hub-Demand Steering to ensure all 250 M-hubs have valid endpoints. In Phase 1.5, verify global endpoint balance. In Phase 2, solve Macro Selector CNF with Cut-Block CEGAR. In Phase 3, splice and independently certify the 6,620-vertex cycle.

**Tech Stack:** Python 3, CaDiCaL SAT solver (`/usr/local/bin/cadical`), Multiprocessing.

## Global Constraints
- Target graph: `/home/ubuntu/HCP/FHCPCS-col/graph950.col`
- Zero tour injection: Do not import, read, or reference `graph950.hcp.tou` during solving.
- Solver binary: `/usr/local/bin/cadical`
- Soundness: All degree constraints must be exact-2, self-loops eliminated, subtours blocked via cut-crossing clauses.

---

### Task 1: Implement Targeted Hub-Demand Steering in Phase 1 Cover Generator

**Files:**
- Create: `scratch/graph950/steered_p1_generator.py`
- Test: `scratch/graph950/test_steered_p1.py`

**Interfaces:**
- Produces: `generate_steered_covers(graph_path, hubcut=20, K=4) -> list[tuple[strip_id, list[cover]]]`
- Output format: JSON dumped list of `(fingerprint, runs)` for all 64 strips.

- [ ] **Step 1: Write test for targeted hub-steered strip solver**

```python
# scratch/graph950/test_steered_p1.py
import sys, collections
from steered_p1_generator import solve_strip_targeted

def test_single_strip_steering():
    # Test on strip 0 of graph950
    G = collections.defaultdict(set)
    for l in open('/home/ubuntu/HCP/FHCPCS-col/graph950.col'):
        t = l.split()
        if t and t[0] == 'e':
            u, v = int(t[1]), int(t[2]); G[u].add(v); G[v].add(u)
    
    # Strip 0 bulk vertices
    deg = {v: len(a) for v, a in G.items()}
    verts = sorted(G.keys())
    bulk_set = set(v for v in verts if deg[v] < 20)
    big_hub = {v for v in verts if deg[v] >= 100}
    strips = collections.defaultdict(list)
    for v in bulk_set:
        hh = tuple(sorted(u for u in G[v] if u in big_hub))
        strips[hh].append(v)
    
    strip_list = list(strips.items())
    hh, vs = strip_list[0]
    m_hubs = [h for v in vs for h in G[v] if 20 <= deg[h] < 100]
    
    covers = solve_strip_targeted(0, hh, vs, G, deg, K=4, seeds=[7, 11, 13])
    assert len(covers) >= 1, "Must generate at least one valid cover"
    for fp, runs in covers:
        assert sum(len(r) for r in runs) == len(vs), "Cover must span all strip vertices"
    print("test_single_strip_steering PASSED!")

if __name__ == '__main__':
    test_single_strip_steering()
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 scratch/graph950/test_steered_p1.py`
Expected: FAIL (module not found)

- [ ] **Step 3: Implement `steered_p1_generator.py`**

Write the complete parallel generator that:
1. Decomposes `graph950.col` into 310 hubs and 64 strips.
2. For each strip, identifies all adjacent M-hubs.
3. Formulates SAT with base constraints ($\le K$ paths) plus steering constraints for adjacent M-hubs.
4. Uses CaDiCaL to solve with multiple seeds, returning deduplicated covers.

- [ ] **Step 4: Run test to verify it passes**

Run: `python3 scratch/graph950/test_steered_p1.py`
Expected: PASS

- [ ] **Step 5: Commit Task 1**

```bash
git add scratch/graph950/steered_p1_generator.py scratch/graph950/test_steered_p1.py
git commit -m "feat(graph950): implement targeted hub-demand steering in Phase 1 generator"
```

---

### Task 2: Implement Global Hub Reachability & Balance Filter (Phase 1.5)

**Files:**
- Create: `scratch/graph950/hub_balance_filter.py`
- Test: `scratch/graph950/test_hub_balance.py`

**Interfaces:**
- Consumes: `cover_sets` from Phase 1, `G`, `hub_set`
- Produces: `verify_global_balance(cover_sets, G, hub_set) -> tuple[bool, dict]`

- [ ] **Step 1: Write test for global balance verification**

```python
# scratch/graph950/test_hub_balance.py
import json, collections
from hub_balance_filter import check_hub_candidate_coverage

def test_hub_balance():
    # Load covers and check that all 310 hubs have >= 2 endpoint candidates
    G = collections.defaultdict(set)
    for l in open('/home/ubuntu/HCP/FHCPCS-col/graph950.col'):
        t = l.split()
        if t and t[0] == 'e':
            u, v = int(t[1]), int(t[2]); G[u].add(v); G[v].add(u)
    deg = {v: len(a) for v, a in G.items()}
    hub_set = set(v for v, d in deg.items() if d >= 20)
    
    # Dummy mock covers or actual covers
    cover_sets = json.load(open('/home/ubuntu/HCP/scratch/graph950/covers_multi.json'))
    all_ok, stats = check_hub_candidate_coverage(cover_sets, G, hub_set)
    print("Balance stats:", stats)
    assert 'min_candidates' in stats

if __name__ == '__main__':
    test_hub_balance()
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 scratch/graph950/test_hub_balance.py`
Expected: FAIL (module not found)

- [ ] **Step 3: Implement `hub_balance_filter.py`**

Implement candidate counting and balance verification across S-hubs, B-hubs, and M-hubs.

- [ ] **Step 4: Run test to verify it passes**

Run: `python3 scratch/graph950/test_hub_balance.py`
Expected: PASS

- [ ] **Step 5: Commit Task 2**

```bash
git add scratch/graph950/hub_balance_filter.py scratch/graph950/test_hub_balance.py
git commit -m "feat(graph950): implement global hub reachability and balance filter"
```

---

### Task 3: Build Pure SAT Macro Selector Solver with Cut-Block CEGAR (Zero Injection)

**Files:**
- Create: `scratch/graph950/puresat_selector_solver.py`

**Interfaces:**
- Executes full pipeline:
  1. `generate_steered_covers()` (Phase 1)
  2. `verify_global_balance()` (Phase 1.5)
  3. `solve_macro_cegar()` (Phase 2 - Pure SAT, zero injection)
  4. `splice_and_certify()` (Phase 3)
- Produces: `/tmp/opencode/found_tour_puresat.hcp`

- [ ] **Step 1: Implement `puresat_selector_solver.py` without `inject_tour_covers`**

- [ ] **Step 2: Add logging and timer per phase**

- [ ] **Step 3: Commit Task 3**

```bash
git add scratch/graph950/puresat_selector_solver.py
git commit -m "feat(graph950): implement Pure SAT Two-Tier Solver with zero injection"
```

---

### Task 4: End-to-End Execution and Independent Certification

**Files:**
- Execute: `scratch/graph950/puresat_selector_solver.py`
- Verify: `scratch/graph950/indep_verify.py`

- [ ] **Step 1: Run `puresat_selector_solver.py` on `graph950.col`**
- [ ] **Step 2: Verify `found_tour_puresat.hcp` using independent verifier**
- [ ] **Step 3: Update documentation report**

```bash
git add docs/graph950-puresat-verified-report.md
git commit -m "docs(graph950): add 100% Pure SAT verification report"
```
