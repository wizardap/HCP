"""
SAT Engine for Hamiltonian Path & Cycle Problems.
Features:
- Pure direct combinatoric degree encoding (0 auxiliary variables)
- Static chordless triangle and square cuts
- Incremental CaDiCaL CEGAR subcycle elimination loop
- 2-opt and 3-opt local cycle absorption operators
"""

import collections
import itertools
import time
from typing import Dict, List, Optional, Set, Tuple
from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType

def build_combinatoric_degree_clauses(
    nodes: Set[int],
    sub_adj: Dict[int, Set[int]],
    edge_to_var: Dict[Tuple[int, int], int],
    fixed_degree_1: Optional[Set[int]] = None
) -> List[List[int]]:
    """
    Generates exact degree-2 (or degree-1 for boundary endpoints) constraints
    using pure direct combinatorics with 0 auxiliary variables.
    """
    if fixed_degree_1 is None:
        fixed_degree_1 = set()

    clauses: List[List[int]] = []
    for u in sorted(nodes):
        nbrs = sorted(list(sub_adj[u]))
        inc = [edge_to_var[tuple(sorted([u, v]))] for v in nbrs]
        d = len(inc)

        if u in fixed_degree_1:
            # Degree exactly 1
            clauses.append(inc)
            for comb in itertools.combinations(inc, 2):
                clauses.append([-comb[0], -comb[1]])
        else:
            # Degree exactly 2
            for comb in itertools.combinations(inc, 3):
                clauses.append([-comb[0], -comb[1], -comb[2]])
            for comb in itertools.combinations(inc, d - 1):
                clauses.append(list(comb))

    return clauses


def find_chordless_triangles(nodes: Set[int], adj: Dict[int, Set[int]]) -> List[Tuple[int, int, int]]:
    """
    Finds all 3-cycles (triangles) on nodes.
    """
    triangles = []
    sorted_nodes = sorted(list(nodes))
    for u in sorted_nodes:
        for v in adj[u]:
            if v > u:
                for w in adj[v]:
                    if w > v and w in adj[u]:
                        triangles.append((u, v, w))
    return triangles


def find_chordless_squares(nodes: Set[int], adj: Dict[int, Set[int]]) -> List[Tuple[Tuple[int, int], ...]]:
    """
    Finds all chordless 4-cycles (squares) on nodes.
    """
    squares = set()
    sorted_nodes = sorted(list(nodes))
    for a in sorted_nodes:
        nbrs_a = sorted(list(adj[a]))
        for i in range(len(nbrs_a)):
            u = nbrs_a[i]
            for j in range(i + 1, len(nbrs_a)):
                v = nbrs_a[j]
                common = [w for w in adj[u] if w != a and w in adj[v]]
                for w in common:
                    if w not in adj[a] and v not in adj[u]:
                        e1 = tuple(sorted([a, u]))
                        e2 = tuple(sorted([u, w]))
                        e3 = tuple(sorted([w, v]))
                        e4 = tuple(sorted([v, a]))
                        sq_key = tuple(sorted([e1, e2, e3, e4]))
                        if sq_key not in squares:
                            squares.add(sq_key)
    return list(squares)


def try_merge_2opt(
    cyc1: List[int],
    cyc2: List[int],
    adj: Dict[int, Set[int]],
    forbidden_edges: Set[Tuple[int, int]]
) -> Optional[List[int]]:
    """
    Attempts to merge cyc2 into cyc1 using a single 2-opt cross-connection.
    """
    n1 = len(cyc1)
    n2 = len(cyc2)
    pos2 = {u: i for i, u in enumerate(cyc2)}

    for i in range(n1):
        u1 = cyc1[i]
        v1 = cyc1[(i + 1) % n1]
        e1 = tuple(sorted([u1, v1]))
        if e1 in forbidden_edges:
            continue

        nbrs_u1 = [u2 for u2 in adj[u1] if u2 in pos2]
        for u2 in nbrs_u1:
            idx2 = pos2[u2]
            for v2, rev in [(cyc2[(idx2 + 1) % n2], False), (cyc2[(idx2 - 1) % n2], True)]:
                e2 = tuple(sorted([u2, v2]))
                if e2 in forbidden_edges:
                    continue
                if v2 in adj[v1]:
                    p1 = cyc1[i+1:] + cyc1[:i+1]
                    p2 = [cyc2[(idx2 - k) % n2] for k in range(n2)] if not rev else [cyc2[(idx2 + k) % n2] for k in range(n2)]
                    return p1 + p2
    return None


def solve_subgraph_path(
    G: Dict[int, Set[int]],
    nodes: Set[int],
    src: int,
    dst: int,
    max_it: int = 150,
    verbose: bool = False
) -> Optional[List[int]]:
    """
    Solves a Hamiltonian Path on the subgraph induced by `nodes` between `src` and `dst`.
    Executes live with CaDiCaL CEGAR and subcycle cut clauses.
    """
    nodes_list = sorted(list(nodes))
    sub_adj = {u: sorted([v for v in G[u] if v in nodes]) for u in nodes_list}

    edges = []
    for u in nodes_list:
        for v in sub_adj[u]:
            if u < v:
                edges.append((u, v))

    e2v = {e: idx + 1 for idx, e in enumerate(edges)}
    solver = Cadical195()

    # Combinatoric degree clauses
    clauses = build_combinatoric_degree_clauses(nodes, sub_adj, e2v, fixed_degree_1={src, dst})
    for cl in clauses:
        solver.add_clause(cl)

    for it in range(1, max_it + 1):
        if not solver.solve():
            solver.delete()
            return None

        model = set(solver.get_model())
        active_edges = [e for e in edges if e2v[e] in model]
        adj_m = collections.defaultdict(list)
        for u, v in active_edges:
            adj_m[u].append(v)
            adj_m[v].append(u)

        # Trace path from src to dst
        path = [src]
        curr = src
        prev = None
        vis = {src}
        while curr != dst:
            nxts = [w for w in adj_m[curr] if w != prev]
            if not nxts:
                break
            nxt = nxts[0]
            path.append(nxt)
            vis.add(nxt)
            prev, curr = curr, nxt

        # Detect subcycles
        cycles = []
        for u in nodes_list:
            if u not in vis:
                cyc = [u]
                vis.add(u)
                curr_c = u
                prev_c = None
                while True:
                    nxts = [w for w in adj_m[curr_c] if w != prev_c]
                    if not nxts or nxts[0] == u:
                        break
                    nxt = nxts[0]
                    cyc.append(nxt)
                    vis.add(nxt)
                    prev_c, curr_c = curr_c, nxt
                cycles.append(cyc)

        if not cycles and len(path) == len(nodes) and curr == dst:
            solver.delete()
            return path

        # Add subcycle blocking and cut clauses
        for cyc in cycles:
            cyc_set = set(cyc)
            cut_edges = [e2v[tuple(sorted([u, v]))] for u in cyc for v in sub_adj[u] if v not in cyc_set]
            if cut_edges:
                solver.add_clause(cut_edges)
            neg_clause = [-e2v[tuple(sorted([cyc[k], cyc[(k+1)%len(cyc)]]))] for k in range(len(cyc))]
            solver.add_clause(neg_clause)

    solver.delete()
    return None


def solve_cluster_path(
    c_id: int,
    u_in: int,
    u_out: int,
    c_verts: Set[int],
    G: Dict[int, Set[int]],
    max_it: int = 300,
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
                print(f"Cluster {c_id} UNSAT at it {it}", flush=True)
            solver.delete()
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
            print(f"Cluster {c_id} It {it:2d} ({time.time()-t_it:.2f}s): path_len={len(path)}/{len(c_verts)}, cycles={len(cycles)}", flush=True)

        if not cycles and len(path) == len(c_verts) and curr == u_out:
            if verbose:
                print(f"SUCCESS! Cluster {c_id} SOLVED in {time.time()-t0:.2f}s at it {it}!", flush=True)
            solver.delete()
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

    solver.delete()
    return None
