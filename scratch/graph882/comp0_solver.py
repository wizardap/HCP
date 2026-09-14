import collections, itertools, json, os, sys, time
from typing import Dict, List, Set, Tuple
from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from scratch.graph882.decomposer import load_and_decompose_graph882

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
        nbrs_u1 = [u2 for u2 in adj[u1] if u2 in pos2]
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

def _expand_and_save_cycle(cyc: List[int], edge_chains: Dict[Tuple[int, int], List[int]], comp0_nodes: Set[int], cache_path: str, t0: float, solver: Cadical195) -> List[int]:
    expanded = []
    n = len(cyc)
    for i in range(n):
        u = cyc[i]
        v = cyc[(i + 1) % n]
        e = tuple(sorted([u, v]))
        if e in edge_chains:
            c = edge_chains[e]
            if c[0] != u:
                c = list(reversed(c))
            expanded.extend(c[:-1])
        else:
            expanded.append(u)

    assert len(expanded) == 5501, f"Expected 5,501 vertices, got {len(expanded)}"
    assert len(set(expanded)) == 5501, "Duplicate vertices in expanded Comp 0 cycle!"
    assert set(expanded) == comp0_nodes, "Comp 0 nodes mismatch!"

    os.makedirs(os.path.dirname(os.path.abspath(cache_path)), exist_ok=True)
    with open(cache_path, "w") as f:
        json.dump(expanded, f)
    print(f"[*] Comp 0 solved and saved ({len(expanded)} vertices) in {time.time() - t0:.2f}s!")
    solver.delete()
    return expanded

def solve_comp0(G: Dict[int, Set[int]], comp0_nodes: Set[int]) -> List[int]:
    cache_path = os.path.join(os.path.dirname(__file__), "comp0_cycle.json")
    if os.path.exists(cache_path):
        with open(cache_path, "r") as f:
            cycle = json.load(f)
            if len(cycle) == 5501 and set(cycle) == comp0_nodes:
                return cycle

    t0 = time.time()
    virt1 = tuple(sorted([2080, 2117]))
    virt2 = tuple(sorted([3811, 5066]))
    ports = {2080, 2117, 3811, 5066}

    adj_c0 = collections.defaultdict(set)
    for u in comp0_nodes:
        for v in G[u]:
            if v in comp0_nodes:
                adj_c0[u].add(v)
    adj_c0[virt1[0]].add(virt1[1]); adj_c0[virt1[1]].add(virt1[0])
    adj_c0[virt2[0]].add(virt2[1]); adj_c0[virt2[1]].add(virt2[0])

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
        e_uv = tuple(sorted([u, v])); e_vw = tuple(sorted([v, w]))
        c_uv = edge_chains.pop(e_uv, [u, v]); c_vw = edge_chains.pop(e_vw, [v, w])
        if c_uv[-1] != v: c_uv = list(reversed(c_uv))
        if c_vw[0] != v: c_vw = list(reversed(c_vw))
        e_uw = tuple(sorted([u, w]))
        adj_c0[u].add(w); adj_c0[w].add(u)
        assert e_uw not in edge_chains, f"Degree-2 contraction collision on edge {e_uw}"
        edge_chains[e_uw] = c_uv[:-1] + c_vw

    print(f"[Comp 0] Contracted: {len(comp0_nodes)} -> {len(rem)} vertices ({len(edge_chains)} contracted chains).")
    assert len(rem) == 4273

    contracted_edges = set(edge_chains.keys())
    forbidden_delete = contracted_edges | {virt1, virt2}

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

    # Force virtual edges and all contracted edges
    solver.add_clause([edge_to_var[virt1]])
    solver.add_clause([edge_to_var[virt2]])
    for ce in contracted_edges:
        solver.add_clause([edge_to_var[ce]])

    # Static chordless triangle cuts in contracted graph
    rem_list = sorted(list(rem))
    for u in rem_list:
        for v in adj_c0[u]:
            if v > u:
                for w in adj_c0[v]:
                    if w > v and w in adj_c0[u]:
                        solver.add_clause([-edge_to_var[tuple(sorted([u, v]))], -edge_to_var[tuple(sorted([v, w]))], -edge_to_var[tuple(sorted([w, u]))]])

    # Static chordless square cuts in contracted graph
    squares = set()
    for a in rem_list:
        nbrs_a = sorted(list(adj_c0[a]))
        for i in range(len(nbrs_a)):
            u = nbrs_a[i]
            for j in range(i+1, len(nbrs_a)):
                v = nbrs_a[j]
                common = [w for w in adj_c0[u] if w != a and w in adj_c0[v]]
                for w in common:
                    if w not in adj_c0[a] and v not in adj_c0[u]:
                        e1 = tuple(sorted([a, u]))
                        e2 = tuple(sorted([u, w]))
                        e3 = tuple(sorted([w, v]))
                        e4 = tuple(sorted([v, a]))
                        sq_key = tuple(sorted([e1, e2, e3, e4]))
                        if sq_key not in squares:
                            squares.add(sq_key)
                            solver.add_clause([-edge_to_var[e1], -edge_to_var[e2], -edge_to_var[e3], -edge_to_var[e4]])

    print(f"[Comp 0] Starting CEGAR loop with 2-opt absorption...")
    it = 0
    while True:
        it += 1
        t_start_it = time.time()
        if not solver.solve():
            solver.delete()
            raise RuntimeError(f"Comp 0 UNSAT at iteration {it}!")

        model = set(solver.get_model())
        active_edges = [e for e in edge_list if edge_to_var[e] in model]
        adj = collections.defaultdict(list)
        for u, v in active_edges:
            adj[u].append(v); adj[v].append(u)

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
            print(f"[Comp 0] SAT solver returned 1 cycle at iter {it} in {time.time()-t0:.2f}s! ({len(cycles[0])} contracted vertices)")
            return _expand_and_save_cycle(cycles[0], edge_chains, comp0_nodes, cache_path, t0, solver)

        # 2-opt cycle absorption
        merged_cycles = list(cycles)
        merged_any = True
        while merged_any and len(merged_cycles) > 1:
            merged_any = False
            merged_cycles.sort(key=len, reverse=True)
            for i in range(len(merged_cycles)):
                for j in range(i + 1, len(merged_cycles)):
                    res = try_merge_2opt(merged_cycles[i], merged_cycles[j], adj_c0, forbidden_delete)
                    if res is not None:
                        merged_cycles.pop(j)
                        merged_cycles[i] = res
                        merged_any = True
                        break
                if merged_any:
                    break

        if len(merged_cycles) == 1:
            print(f"[Comp 0] 2-opt absorption converged at iter {it} in {time.time()-t0:.2f}s! ({len(merged_cycles[0])} contracted vertices)")
            return _expand_and_save_cycle(merged_cycles[0], edge_chains, comp0_nodes, cache_path, t0, solver)

        if it % 5 == 0 or len(merged_cycles) <= 10:
            cyc_lens = sorted([len(c) for c in cycles], reverse=True)
            abs_lens = sorted([len(c) for c in merged_cycles], reverse=True)
            print(f"[Comp 0] Iter {it} ({time.time()-t_start_it:.2f}s): {len(cycles)} raw (max {cyc_lens[0]}, min {cyc_lens[-1]}) -> {len(merged_cycles)} absorbed (max {abs_lens[0]}, min {abs_lens[-1]})")

        # Cocycle cuts and negative cuts for raw cycles
        for cyc in cycles:
            if len(cyc) <= len(rem) // 2:
                cyc_set = set(cyc)
                cut_edges = [tuple(sorted([u, v])) for u in cyc for v in adj_c0[u] if v not in cyc_set]
                solver.add_clause([edge_to_var[e] for e in cut_edges])
            neg_clause = [-edge_to_var[tuple(sorted([cyc[i], cyc[(i+1)%len(cyc)]]))] for i in range(len(cyc))]
            solver.add_clause(neg_clause)

        # Cocycle cuts for absorbed macro-cycles
        if len(merged_cycles) > 1:
            for cyc in merged_cycles:
                if len(cyc) <= len(rem) // 2:
                    cyc_set = set(cyc)
                    cut_edges = [tuple(sorted([u, v])) for u in cyc for v in adj_c0[u] if v not in cyc_set]
                    solver.add_clause([edge_to_var[e] for e in cut_edges])

if __name__ == "__main__":
    col_path = os.path.join(repo_root, "FHCPCS-col/graph882.col")
    G, _, _, _, comp0_nodes = load_and_decompose_graph882(col_path)
    solve_comp0(G, comp0_nodes)
