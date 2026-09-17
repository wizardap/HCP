import time, collections
from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType

from scratch.engine.graph_loader import load_dimacs
from scratch.engine.decomposer import detect_bridge_corridors
from scratch.engine.sat_merger import solve_local_hp_sat
from scratch.engine.comp0_solver import try_merge_2opt, try_merge_3opt

def inspect_2cycles():
    G = load_dimacs('FHCPCS-col/graph944.col')
    corridors, comp0_nodes = detect_bridge_corridors(G)
    virtual_edges = [c['ext_ports'] for c in corridors]
    virt_set = {tuple(sorted(e)) for e in virtual_edges}
    ports = {p for e in virt_set for p in e}

    adj_c0 = collections.defaultdict(set)
    for u in comp0_nodes:
        for v in G[u]:
            if v in comp0_nodes:
                adj_c0[u].add(v)
    for u, v in virt_set:
        adj_c0[u].add(v)
        adj_c0[v].add(u)

    # Recursive degree-2 contraction
    rem = set(comp0_nodes)
    edge_chains = {}
    while True:
        d2 = [u for u in sorted(rem) if u not in ports and len(adj_c0[u]) == 2]
        if not d2:
            break
        v = d2[0]
        u, w = list(adj_c0[v])
        adj_c0[u].remove(v)
        adj_c0[w].remove(v)
        del adj_c0[v]
        rem.remove(v)
        e_uv = tuple(sorted([u, v]))
        e_vw = tuple(sorted([v, w]))
        c_uv = edge_chains.pop(e_uv, [u, v])
        c_vw = edge_chains.pop(e_vw, [v, w])
        if c_uv[-1] != v: c_uv = list(reversed(c_uv))
        if c_vw[0] != v: c_vw = list(reversed(c_vw))
        e_new = tuple(sorted([u, w]))
        edge_chains[e_new] = c_uv + c_vw[1:]
        adj_c0[u].add(w)
        adj_c0[w].add(u)

    print(f'[*] Contracted Comp 0: {len(comp0_nodes)} -> {len(rem)} vertices.')
    contracted_edges = set(edge_chains.keys())

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
    for u in sorted(rem):
        clauses = CardEnc.equals(lits=inc_edges[u], bound=2, top_id=top, encoding=EncType.cardnetwrk)
        for cl in clauses:
            solver.add_clause(cl)
            for lit in cl:
                top = max(top, abs(lit) + 1)

    for ve in virt_set:
        solver.add_clause([edge_to_var[ve]])
    for ce in contracted_edges:
        solver.add_clause([edge_to_var[ce]])

    # Triangles and squares
    rem_list = sorted(list(rem))
    for u in rem_list:
        for v in adj_c0[u]:
            if v > u:
                for w in adj_c0[v]:
                    if w > v and w in adj_c0[u]:
                        solver.add_clause([-edge_to_var[tuple(sorted([u, v]))],
                                           -edge_to_var[tuple(sorted([v, w]))],
                                           -edge_to_var[tuple(sorted([w, u]))]])

    it = 0
    forbidden_delete = contracted_edges | set(virt_set)

    while it < 400:
        it += 1
        t_it = time.time()
        if not solver.solve():
            print('UNSAT!')
            break

        model = set(solver.get_model())
        active_edges = [e for e in edge_list if edge_to_var[e] in model]
        adj = collections.defaultdict(list)
        for u, v in active_edges:
            adj[u].append(v)
            adj[v].append(u)

        visited = set()
        cycles = []
        for u in sorted(rem):
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

        if it % 20 == 0 or len(merged) <= 3:
            abs_lens = sorted([len(c) for c in merged], reverse=True)
            print(f'Iter {it} ({time.time()-t_it:.2f}s): {len(cycles)} raw -> {len(merged)} absorbed (lens: {abs_lens})')

        if len(merged) == 2:
            c1, c2 = merged[0], merged[1]
            s1, s2 = set(c1), set(c2)
            cross = [(u, v) for u in s1 for v in adj_c0[u] if v in s2]
            ports1 = sorted(list({u for u, v in cross}))
            ports2 = sorted(list({v for u, v in cross}))
            print(f'[FOUND 2 CYCLES AT ITER {it}!]')
            print(f'  Cycle 1: {len(c1)} vertices, Boundary ports in C1: {len(ports1)} ({ports1[:10]}...)')
            print(f'  Cycle 2: {len(c2)} vertices, Boundary ports in C2: {len(ports2)} ({ports2[:10]}...)')
            print(f'  Cross edges between C1 and C2: {len(cross)}')
            for e in cross[:15]:
                print(f'    cross edge: {e}')

            # Test if C2 can be solved as an HP between any pair of ports in C2!
            print(f'Testing HP on C2 ({len(c2)} vertices) between all pairs of ports in C2:')
            found_hp = False
            for p_a in ports2:
                for p_b in ports2:
                    if p_a < p_b:
                        t_hp = time.time()
                        hp = solve_local_hp_sat(c2, p_a, p_b, adj_c0, forbidden_delete)
                        if hp:
                            print(f'  SUCCESS! Found HP on C2 from {p_a} to {p_b} in {time.time()-t_hp:.3f}s!')
                            found_hp = True
                            break
                if found_hp:
                    break
            if found_hp:
                print('Breakthrough confirmed!')
                break

        # Universal cocycle cuts
        for cyc in cycles:
            c_set = set(cyc)
            cut_e = [tuple(sorted([u, v])) for u in cyc for v in adj_c0[u] if v not in c_set]
            solver.add_clause([edge_to_var[e] for e in cut_e])
            if len(cyc) <= len(rem) // 2:
                neg_c = [-edge_to_var[tuple(sorted([cyc[i], cyc[(i + 1) % len(cyc)]]))] for i in range(len(cyc))]
                solver.add_clause(neg_c)

    solver.delete()

if __name__ == '__main__':
    inspect_2cycles()
