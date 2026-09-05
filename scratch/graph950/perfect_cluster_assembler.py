#!/usr/bin/env python3
"""
Perfect Hierarchical Cluster Assembler & Solver for graph950.col
Decomposes the 6620-vertex graph into 10 macro-clusters (661 vertices each)
and 4 inter-cluster strips (10 vertices total).
Solves each cluster independently as a single continuous Hamiltonian path
and concatenates them across the 2-edge bridge cut into 1 single unified Hamiltonian cycle.
"""

import collections, time, os, sys
from typing import Dict, List, Set, Tuple, Optional
from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType
from scratch.graph950.two_tier_decomposer import load_graph, decompose_graph

def solve_cluster_path(
    c_id: int,
    u_in: int,
    u_out: int,
    c_verts: Set[int],
    G: Dict[int, Set[int]],
    max_it: int = 150,
    verbose: bool = True
) -> Optional[List[int]]:
    """
    Solves a single continuous Hamiltonian path on c_verts from u_in to u_out.
    Enforces deg=1 at u_in, u_out and deg=2 at all other vertices in c_verts.
    Eliminates subcycles using exact >=2 cut-crossing clauses and cycle-blocking clauses.
    """
    t0 = time.time()
    edges = []
    G_c = collections.defaultdict(set)
    for u in c_verts:
        for v in G[u]:
            if v in c_verts and u < v:
                edges.append((u, v))
                G_c[u].add(v)
                G_c[v].add(u)

    var_e = {e: i + 1 for i, e in enumerate(edges)}
    for u, v in edges:
        var_e[(v, u)] = var_e[(u, v)]

    nv = len(edges)
    clauses = []
    for u in c_verts:
        inc = [var_e[(u, v)] for v in G_c[u]]
        target = 1 if u in (u_in, u_out) else 2
        cnf = CardEnc.equals(lits=inc, bound=target, top_id=nv, encoding=EncType.seqcounter)
        nv = max(nv, cnf.nv)
        clauses.extend(cnf.clauses)

    solver = Cadical195(bootstrap_with=clauses)
    for it in range(max_it):
        t_it = time.time()
        sat = solver.solve()
        if not sat:
            if verbose:
                print(f"Cluster {c_id} UNSAT at it {it}")
            return None
        model = set(solver.get_model())
        adj = collections.defaultdict(list)
        for (u, v) in edges:
            if var_e[(u, v)] in model:
                adj[u].append(v)
                adj[v].append(u)

        # Trace path from u_in to u_out
        path = [u_in]
        curr = u_in
        prev = None
        vis = {u_in}
        while curr != u_out:
            nxts = [w for w in adj[curr] if w != prev]
            if not nxts:
                break
            nxt = nxts[0]
            path.append(nxt)
            vis.add(nxt)
            prev, curr = curr, nxt

        cycles = []
        for u in c_verts:
            if u not in vis:
                cyc = [u]
                vis.add(u)
                curr_c = u
                prev_c = None
                while True:
                    nxts = [w for w in adj[curr_c] if w != prev_c]
                    if not nxts or nxts[0] == u:
                        break
                    nxt = nxts[0]
                    cyc.append(nxt)
                    vis.add(nxt)
                    prev_c, curr_c = curr_c, nxt
                cycles.append(cyc)

        if verbose and (it % 20 == 0 or len(cycles) <= 3):
            print(f"Cluster {c_id} It {it:2d} ({time.time()-t_it:.2f}s): path_len={len(path)}/{len(c_verts)}, cycles={len(cycles)}")

        if not cycles and len(path) == len(c_verts) and curr == u_out:
            if verbose:
                print(f"SUCCESS! Cluster {c_id} SOLVED in {time.time()-t0:.2f}s at it {it}!")
            return path

        for cyc in cycles:
            s_set = set(cyc)
            cut_edges = []
            for u in cyc:
                for v in G_c[u]:
                    if v not in s_set:
                        cut_edges.append(var_e[(u, v)])
            if cut_edges:
                solver.add_clause(cut_edges)
                if len(cut_edges) <= 10:
                    for idx_e, e_lit in enumerate(cut_edges):
                        others = [cut_edges[j] for j in range(len(cut_edges)) if j != idx_e]
                        solver.add_clause([-e_lit] + others)
            # Cycle-blocking clause
            c_edges = [var_e[(cyc[k], cyc[(k+1)%len(cyc)])] for k in range(len(cyc))]
            solver.add_clause([-e for e in c_edges])

    return None

def verify_tour(tour: List[int], G: Dict[int, Set[int]]) -> bool:
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

def write_hcp(tour: List[int], out_path: str):
    os.makedirs(os.path.dirname(os.path.abspath(out_path)), exist_ok=True)
    with open(out_path, 'w') as f:
        f.write("NAME : graph950.hcp.tour\n")
        f.write("TYPE : TOUR\n")
        f.write(f"DIMENSION : {len(tour)}\n")
        f.write("TOUR_SECTION\n")
        for v in tour:
            f.write(f"{v}\n")
        f.write("-1\n")
        f.write("EOF\n")
    print(f"Certified tour successfully written to {out_path}")
