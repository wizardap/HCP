#!/usr/bin/env python3
"""
Two-Half Two-Tier Decomposed HCP Solver for graph950.col
Exploits the 2-bridge cut of graph950:
- Half 1: 3,310 vertices (155 Hubs, 37 Strips)
- Half 2: 3,310 vertices (155 Hubs, 37 Strips)
- Bridge edges: (164, 6080) and (5835, 5540)

Zero Tour Injection: 100% derived from FHCPCS-col/graph950.col
"""

import os, sys, time, collections
from typing import Dict, List, Set, Tuple, Any, Optional
from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType

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

def partition_halves(G: Dict[int, Set[int]], degs: Dict[int, int]):
    s_hubs = [u for u in degs if degs[u] > 300]
    dist, owner = {}, {}
    q = collections.deque()
    for s in s_hubs:
        owner[s] = s; dist[s] = 0; q.append(s)
    while q:
        u = q.popleft()
        for v in G[u]:
            if v not in dist:
                dist[v] = dist[u] + 1; owner[v] = owner[u]; q.append(v)

    g1_roots = {5787, 4785, 164, 4000, 5835}
    g2_roots = {1171, 6080, 4540, 2803, 5540}

    grp1 = set(u for u in G if owner[u] in g1_roots)
    grp2 = set(u for u in G if owner[u] in g2_roots)
    assert len(grp1) == 3310 and len(grp2) == 3310
    return grp1, grp2

def decompose_half(G: Dict[int, Set[int]], degs: Dict[int, int], half_nodes: Set[int]):
    all_hubs = sorted([u for u in half_nodes if degs[u] >= 20])
    hub_set = set(all_hubs)
    bulk = half_nodes - hub_set
    
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
    strips.sort(key=len, reverse=True)
    
    hh_edges = []
    for u in all_hubs:
        for v in G[u]:
            if v in hub_set and u < v:
                hh_edges.append((u, v))
                
    strip_adj_hubs = collections.defaultdict(set)
    hub_adj_strips = collections.defaultdict(set)
    for si, s in enumerate(strips):
        for u in s:
            for nbr in G[u]:
                if nbr in hub_set:
                    strip_adj_hubs[si].add(nbr)
                    hub_adj_strips[nbr].add(si)
                    
    return all_hubs, strips, hh_edges, dict(strip_adj_hubs), dict(hub_adj_strips)

def solve_strip_exact(G: Dict[int, Set[int]], strip_verts: List[int], dem: Dict[int, int], K: int):
    verts = strip_verts
    v_set = set(verts)
    edges = [(u, v) for u in verts for v in G[u] if v in v_set and u < v]
    
    nv = len(edges)
    var_e = {e: i+1 for i, e in enumerate(edges)}
    for u, v in edges: var_e[(v, u)] = var_e[(u, v)]
    
    var_end = {}
    for u in verts:
        nv += 1; var_end[u] = nv
        
    clauses = []
    for u in verts:
        inc = [var_e[(u, v)] for v in G[u] if v in v_set]
        if not inc:
            return False, None, None
        clauses.append(inc)
        for i in range(len(inc)):
            for j in range(i+1, len(inc)):
                for k in range(j+1, len(inc)):
                    clauses.append([-inc[i], -inc[j], -inc[k]])
        for i in range(len(inc)):
            for j in range(i+1, len(inc)):
                clauses.append([-var_end[u], -inc[i], -inc[j]])
        for i in range(len(inc)):
            clauses.append([var_end[u]] + [inc[j] for j in range(len(inc)) if j != i])
            
    ends_list = [var_end[u] for u in verts]
    if len(ends_list) < 2 * K:
        return False, None, None
    elif len(ends_list) == 2 * K:
        for l in ends_list:
            clauses.append([l])
    else:
        cnf = CardEnc.equals(lits=ends_list, bound=2*K, top_id=nv, encoding=EncType.seqcounter)
        nv = max(nv, cnf.nv); clauses.extend(cnf.clauses)
    
    var_h = {}
    for h, req in dem.items():
        if req > 0:
            for u in verts:
                if h in G[u]:
                    nv += 1; var_h[(u, h)] = nv
                    clauses.append([-nv, var_end[u]])
                    
    for u in verts:
        u_h_lits = [var_h[(u, h)] for h in dem if (u, h) in var_h]
        for i in range(len(u_h_lits)):
            for j in range(i+1, len(u_h_lits)):
                clauses.append([-u_h_lits[i], -u_h_lits[j]])
                
    for h, req in dem.items():
        if req > 0:
            h_lits = [var_h[(u, h)] for u in verts if (u, h) in var_h]
            if len(h_lits) < req:
                return False, None, None
            elif len(h_lits) == req:
                for l in h_lits:
                    clauses.append([l])
            else:
                cnf_h = CardEnc.equals(lits=h_lits, bound=req, top_id=nv, encoding=EncType.seqcounter)
                nv = max(nv, cnf_h.nv); clauses.extend(cnf_h.clauses)
            
    with Cadical195(bootstrap_with=clauses) as solver:
        for it in range(30):
            if not solver.solve():
                return False, None, None
            model = set(solver.get_model())
            
            adj_p = {u: [] for u in verts}
            chosen_edges = []
            for u, v in edges:
                if var_e[(u, v)] in model:
                    adj_p[u].append(v); adj_p[v].append(u)
                    chosen_edges.append((u, v))
                    
            vis = set()
            cycles = []
            for u in verts:
                if u not in vis and len(adj_p[u]) == 2:
                    cyc = [u]; vis.add(u)
                    curr = adj_p[u][0]; prev = u; is_cyc = False
                    while curr != u:
                        vis.add(curr); cyc.append(curr)
                        nxts = [w for w in adj_p[curr] if w != prev]
                        if not nxts: break
                        prev, curr = curr, nxts[0]
                        if curr == u:
                            is_cyc = True; break
                    if is_cyc:
                        cycles.append(cyc)
                        
            if not cycles:
                hub_match = []
                for (u, h), v_id in var_h.items():
                    if v_id in model:
                        hub_match.append((u, h))
                return True, chosen_edges, hub_match
                
            for cyc in cycles:
                cyc_e = [var_e[(cyc[k], cyc[(k+1)%len(cyc)])] for k in range(len(cyc))]
                solver.add_clause([-e for e in cyc_e])
                
    return False, None, None

def solve_half(
    G: Dict[int, Set[int]],
    degs: Dict[int, int],
    half_nodes: Set[int],
    start_hub: int,
    end_hub: int,
    half_name: str
) -> List[int]:
    print(f"\n====================================================================")
    print(f"Solving {half_name} ({len(half_nodes)} vertices: {start_hub} -> {end_hub})...")
    print(f"====================================================================")
    t0 = time.time()
    
    all_hubs, strips, hh_edges, strip_adj_hubs, hub_adj_strips = decompose_half(G, degs, half_nodes)
    print(f"{half_name}: {len(all_hubs)} Hubs, {len(strips)} Strips, {len(hh_edges)} HH edges.")
    
    # Precompute tiny valid pairs
    tiny_invalid_pairs = []
    for si, s in enumerate(strips):
        if len(s) < 10:
            hubs = sorted(list(strip_adj_hubs[si]))
            for i in range(len(hubs)):
                for j in range(i, len(hubs)):
                    h1, h2 = hubs[i], hubs[j]
                    dem = {h1: 1, h2: 1} if h1 != h2 else {h1: 2}
                    sat, _, _ = solve_strip_exact(G, s, dem, K=1)
                    if not sat:
                        tiny_invalid_pairs.append((si, h1, h2))
    print(f"Precomputed and forbidden {len(tiny_invalid_pairs)} invalid pairs on tiny strips.")
    
    # Coordinator SAT instance
    solver = Cadical195()
    nv = 0
    var_hh = {}
    for u, v in hh_edges:
        nv += 1; var_hh[(u, v)] = nv; var_hh[(v, u)] = nv
        
    var_d1 = {}; var_d2 = {}
    for si, s in enumerate(strips):
        for h in sorted(strip_adj_hubs.get(si, set())):
            nv += 1; var_d1[(si, h)] = nv
            nv += 1; var_d2[(si, h)] = nv
            solver.add_clause([-var_d2[(si, h)], var_d1[(si, h)]])
            
    for si, h1, h2 in tiny_invalid_pairs:
        if h1 == h2:
            solver.add_clause([-var_d2[(si, h1)]])
        else:
            solver.add_clause([-var_d1[(si, h1)], -var_d1[(si, h2)]])
            
    # Hub degrees
    for h in all_hubs:
        inc_hh = [var_hh[(h, nbr)] for nbr in G[h] if (h, nbr) in var_hh]
        inc_s1 = [var_d1[(si, h)] for si in hub_adj_strips.get(h, set()) if (si, h) in var_d1]
        inc_s2 = [var_d2[(si, h)] for si in hub_adj_strips.get(h, set()) if (si, h) in var_d2]
        all_lits = inc_hh + inc_s1 + inc_s2
        target = 1 if h in (start_hub, end_hub) else 2
        cnf = CardEnc.equals(lits=all_lits, bound=target, top_id=nv, encoding=EncType.seqcounter)
        nv = max(nv, cnf.nv)
        for cl in cnf.clauses: solver.add_clause(cl)
        
    for si, s in enumerate(strips):
        adj_hubs = sorted(strip_adj_hubs.get(si, set()))
        strip_lits = [var_d1[(si, h)] for h in adj_hubs] + [var_d2[(si, h)] for h in adj_hubs]
        if len(s) < 10:
            cnf = CardEnc.equals(lits=strip_lits, bound=2, top_id=nv, encoding=EncType.seqcounter)
            nv = max(nv, cnf.nv)
            for cl in cnf.clauses: solver.add_clause(cl)
        else:
            k_vars = []
            for k in (2, 3, 4, 5):
                nv += 1; k_vars.append(nv)
            cnf_k = CardEnc.equals(lits=k_vars, bound=1, top_id=nv, encoding=EncType.seqcounter)
            nv = max(nv, cnf_k.nv)
            for cl in cnf_k.clauses: solver.add_clause(cl)
            for idx, k in enumerate((2, 3, 4, 5)):
                target = 2 * k
                cnf_eq = CardEnc.equals(lits=strip_lits, bound=target, top_id=nv, encoding=EncType.seqcounter)
                nv = max(nv, cnf_eq.nv)
                for cl in cnf_eq.clauses: solver.add_clause([-k_vars[idx]] + cl)
                
    macro_it = 0
    while True:
        macro_it += 1
        if not solver.solve():
            print(f"Coordinator UNSAT at macro it {macro_it}")
            return None
        model = set(solver.get_model())
        
        demands = {}
        for si in range(len(strips)):
            d_map = {}
            for h in sorted(strip_adj_hubs.get(si, set())):
                d = (1 if var_d1[(si, h)] in model else 0) + (1 if var_d2[(si, h)] in model else 0)
                if d > 0: d_map[h] = d
            demands[si] = d_map
            
        strip_edges_all = []
        strip_matching_all = []
        strip_failed = False
        
        for si, s in enumerate(strips):
            dem = demands[si]
            tot_d = sum(dem.values())
            K = tot_d // 2 if tot_d >= 2 else (1 if len(s) < 10 else 4)
            sat, s_edges, s_match = solve_strip_exact(G, s, dem, K)
            if not sat:
                strip_failed = True
                conflict = []
                for h, d in dem.items():
                    if d >= 1: conflict.append(-var_d1[(si, h)])
                    if d >= 2: conflict.append(-var_d2[(si, h)])
                solver.add_clause(conflict)
                break
            else:
                strip_edges_all.extend(s_edges)
                strip_matching_all.extend(s_match)
                
        if strip_failed:
            continue
            
        chosen_hh = [e for e in hh_edges if var_hh[e] in model]
        
        # Build 2-factor on half nodes
        adj_half = collections.defaultdict(list)
        for u, v in strip_edges_all:
            adj_half[u].append(v); adj_half[v].append(u)
        for u, v in chosen_hh:
            adj_half[u].append(v); adj_half[v].append(u)
        for u, h in strip_matching_all:
            adj_half[u].append(h); adj_half[h].append(u)
            
        # Verify degrees
        deg_ok = all(len(adj_half[u]) == (1 if u in (start_hub, end_hub) else 2) for u in half_nodes)
        if not deg_ok:
            print("  Degree check failed!")
            continue
            
        # Trace path from start_hub
        path = [start_hub]
        curr = start_hub; prev = None; vis = {start_hub}
        while curr != end_hub:
            nxts = [w for w in adj_half[curr] if w != prev]
            if not nxts: break
            nxt = nxts[0]
            path.append(nxt); vis.add(nxt)
            prev, curr = curr, nxt
            
        cycles = []
        for u in sorted(half_nodes):
            if u not in vis:
                cyc = [u]; vis.add(u)
                c_curr = u; c_prev = None
                while True:
                    nxts = [w for w in adj_half[c_curr] if w != c_prev]
                    if not nxts or nxts[0] == u: break
                    nxt = nxts[0]
                    c_prev, c_curr = c_curr, nxt; vis.add(c_curr); cyc.append(c_curr)
                cycles.append(cyc)
                
        print(f"  Macro It {macro_it:2d} ({time.time()-t0:.2f}s): path={len(path)}/{len(half_nodes)}, cycles={len(cycles)}")
        
        if len(path) == len(half_nodes) and curr == end_hub and not cycles:
            print(f"===> SUCCESS! {half_name} Hamiltonian Path SOLVED in {time.time()-t0:.2f}s at it {macro_it}!")
            return path
            
        # Subtour elimination cut on Hubs
        for cyc in cycles:
            c_hubs = set(cyc) & set(all_hubs)
            if c_hubs:
                cut_lits = []
                for h in c_hubs:
                    for nbr in G[h]:
                        if (h, nbr) in var_hh and nbr not in c_hubs:
                            cut_lits.append(var_hh[(h, nbr)])
                    for si in hub_adj_strips.get(h, set()):
                        out_hubs = strip_adj_hubs[si] - c_hubs
                        if out_hubs:
                            cut_lits.append(var_d1[(si, h)])
                if cut_lits:
                    solver.add_clause(cut_lits)
                    
    return None

def verify_full_tour(tour: List[int], G: Dict[int, Set[int]]) -> bool:
    n = len(G)
    if len(tour) != n:
        print(f"Verification Error: tour length {len(tour)} != {n}")
        return False
    if len(set(tour)) != n:
        print(f"Verification Error: duplicates found ({len(set(tour))} unique)")
        return False
    for i in range(n):
        u = tour[i]
        v = tour[(i + 1) % n]
        if v not in G[u]:
            print(f"Verification Error: edge ({u}, {v}) not in G!")
            return False
    return True

def write_certified_tour(tour: List[int], out_path: str):
    os.makedirs(os.path.dirname(os.path.abspath(out_path)), exist_ok=True)
    with open(out_path, 'w') as f:
        f.write("NAME : graph950_certified.tour\n")
        f.write("TYPE : TOUR\n")
        f.write(f"DIMENSION : {len(tour)}\n")
        f.write("TOUR_SECTION\n")
        for v in tour:
            f.write(f"{v}\n")
        f.write("-1\n")
        f.write("EOF\n")
    print(f"Tour saved to {out_path}!")

def main():
    col_path = "FHCPCS-col/graph950.col"
    print("=" * 70)
    print("TWO-HALF TWO-TIER DECOMPOSED SOLVER FOR graph950 (N = 6,620)")
    print("Zero Tour Injection — Rigorous Independent Derivation")
    print("=" * 70)
    
    t_start = time.time()
    G, degs = load_graph(col_path)
    grp1, grp2 = partition_halves(G, degs)
    print(f"Decomposition: Half 1 = {len(grp1)} vertices, Half 2 = {len(grp2)} vertices.")
    print(f"Bridge 1: (164, 6080) in G: {6080 in G[164]}")
    print(f"Bridge 2: (5835, 5540) in G: {5540 in G[5835]}")
    
    # Solve Half 1: 164 -> 5835
    path1 = solve_half(G, degs, grp1, start_hub=164, end_hub=5835, half_name="Half 1")
    if not path1:
        print("Failed to solve Half 1!")
        sys.exit(1)
        
    # Solve Half 2: 5540 -> 6080
    path2 = solve_half(G, degs, grp2, start_hub=5540, end_hub=6080, half_name="Half 2")
    if not path2:
        print("Failed to solve Half 2!")
        sys.exit(1)
        
    # Stitch together across the 2 bridges
    print("\nStitching Half 1 and Half 2 across the 2 bridge cuts...")
    assert path1[0] == 164 and path1[-1] == 5835
    assert path2[0] == 5540 and path2[-1] == 6080
    
    full_tour = path1 + path2
    print(f"Full stitched tour length: {len(full_tour)} (expected 6620)")
    
    # Standalone verification
    print("Running independent verification...")
    if verify_full_tour(full_tour, G):
        print("=" * 70)
        print("*** 100% CERTIFIED SINGLE HAMILTONIAN TOUR FOUND FOR graph950! ***")
        print(f"*** Total Solving Time: {time.time()-t_start:.2f}s ***")
        print("=" * 70)
        out_tour = "scratch/graph950_certified.tour"
        write_certified_tour(full_tour, out_tour)
        print("ALL CERTIFICATIONS PASSED 100%!")
    else:
        print("Verification FAILED!")
        sys.exit(1)

if __name__ == '__main__':
    main()
