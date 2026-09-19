import collections, time
from typing import Dict, List, Set, Tuple
from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType

def try_merge_2opt(cyc1: List[int], cyc2: List[int], adj: Dict[int, Set[int]], forbidden_edges: Set[Tuple[int, int]]):
    n1 = len(cyc1)
    n2 = len(cyc2)
    pos2 = {u: i for i, u in enumerate(cyc2)}
    for i in range(n1):
        u1 = cyc1[i]
        v1 = cyc1[(i + 1) % n1]
        e1 = tuple(sorted([u1, v1]))
        if e1 in forbidden_edges:
            continue
        nbrs_u1 = adj[u1].intersection(pos2.keys())
        for u2 in nbrs_u1:
            idx2 = pos2[u2]
            for v2, rev in [(cyc2[(idx2 + 1) % n2], False), (cyc2[(idx2 - 1) % n2], True)]:
                e2 = tuple(sorted([u2, v2]))
                if e2 in forbidden_edges:
                    continue
                if v2 in adj[v1]:
                    p1 = cyc1[i+1:] + cyc1[:i+1]
                    if not rev:
                        p2 = [cyc2[(idx2 - k) % n2] for k in range(n2)]
                    else:
                        p2 = [cyc2[(idx2 + k) % n2] for k in range(n2)]
                    merged = p1 + p2
                    return merged
    return None

def _format_path_b(cyc: List[int], edge_chains: Dict[Tuple[int, int], List[int]], V_B: Set[int], port_u: int, port_v: int, solver) -> List[int]:
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
            if len(cyc) <= len(V_A) // 2:
                cyc_set = set(cyc)
                cut_edges = [tuple(sorted([u, v])) for u in cyc for v in G[u] if v in V_A and v not in cyc_set]
                solver.add_clause([edge_to_var[e] for e in cut_edges])
            neg_clause = [-edge_to_var[tuple(sorted([cyc[i], cyc[(i+1)%len(cyc)]]))] for i in range(len(cyc))]
            solver.add_clause(neg_clause)

def solve_block_b(G: Dict[int, Set[int]], V_B: Set[int], port_u: int, port_v: int) -> List[int]:
    """
    Solve Block B (3,179 vertices) with degree-2 contraction, static cuts,
    zero-aux cocycle cuts, and 2-opt cycle absorption.
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
    virt_edge = tuple(sorted([port_u, port_v]))
    forbidden_delete = contracted_edges | {virt_edge}

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

    # Static chordless square cuts (4-cycles) in contracted graph
    squares = set()
    for a in rem_list:
        nbrs_a = sorted(list(adj_b[a]))
        for i in range(len(nbrs_a)):
            u = nbrs_a[i]
            for j in range(i + 1, len(nbrs_a)):
                v = nbrs_a[j]
                common = [w for w in adj_b[u] if w != a and w in adj_b[v]]
                for w in common:
                    if w not in adj_b[a] and v not in adj_b[u]:
                        e1 = tuple(sorted([a, u]))
                        e2 = tuple(sorted([u, w]))
                        e3 = tuple(sorted([w, v]))
                        e4 = tuple(sorted([v, a]))
                        sq_key = tuple(sorted([e1, e2, e3, e4]))
                        if sq_key not in squares:
                            squares.add(sq_key)
                            solver.add_clause([-edge_to_var[e1], -edge_to_var[e2], -edge_to_var[e3], -edge_to_var[e4]])

    print(f"[Block B] Added {len(triangles)} static triangle cuts and {len(squares)} static square cuts. Starting LEAN CEGAR loop...")

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
            return _format_path_b(cycles[0], edge_chains, V_B, port_u, port_v, solver)

        # 2-opt cycle absorption
        merged_cycles = list(cycles)
        merged_any = True
        while merged_any and len(merged_cycles) > 1:
            merged_any = False
            merged_cycles.sort(key=len, reverse=True)
            for i in range(len(merged_cycles)):
                for j in range(i + 1, len(merged_cycles)):
                    res = try_merge_2opt(merged_cycles[i], merged_cycles[j], adj_b, forbidden_delete)
                    if res is not None:
                        merged_cycles.pop(j)
                        merged_cycles[i] = res
                        merged_any = True
                        break
                if merged_any:
                    break

        if len(merged_cycles) == 1:
            print(f"[Block B] 2-opt absorption converged at iter {it} in {time.time()-t0:.2f}s! ({len(merged_cycles[0])} contracted vertices)")
            return _format_path_b(merged_cycles[0], edge_chains, V_B, port_u, port_v, solver)

        if it % 10 == 0 or len(cycles) <= 10:
            cyc_lens = sorted([len(c) for c in cycles], reverse=True)
            print(f"[Block B] Iter {it} ({time.time()-t_start_it:.2f}s): {len(cycles)} cycles (absorbed -> {len(merged_cycles)}). Max: {cyc_lens[0]}, Min: {cyc_lens[-1]}")

        # Cocycle and negative cuts (DFK cuts): zero auxiliary variables
        for cyc in cycles:
            if len(cyc) <= len(rem) // 2:
                cyc_set = set(cyc)
                cut_edges = [tuple(sorted([u, v])) for u in cyc for v in adj_b[u] if v not in cyc_set]
                solver.add_clause([edge_to_var[e] for e in cut_edges])
            neg_clause = [-edge_to_var[tuple(sorted([cyc[i], cyc[(i+1)%len(cyc)]]))] for i in range(len(cyc))]
            solver.add_clause(neg_clause)
