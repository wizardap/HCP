import collections, itertools, time
from typing import Dict, List, Optional, Set, Tuple
from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType
from scratch.engine.graph_loader import load_dimacs
from scratch.engine.decomposer import decompose_modular_graph
from scratch.engine.comp0_solver import try_merge_2opt, try_merge_3opt, try_patch_merge

def inspect_and_merge():
    G = load_dimacs('FHCPCS-col/graph944.col')
    corrs, comp0_nodes = decompose_modular_graph(G)
    virtual_edges = [c['ext_ports'] for c in corrs]
    virt_set = {tuple(sorted(e)) for e in virtual_edges}
    ports = set()
    for e in virt_set:
        ports.update(e)

    adj_c0 = collections.defaultdict(set)
    for u in comp0_nodes:
        for v in G[u]:
            if v in comp0_nodes:
                adj_c0[u].add(v)
    for u, v in virt_set:
        adj_c0[u].add(v)

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
        if c_uv[-1] != v:
            c_uv = list(reversed(c_uv))
        if c_vw[0] != v:
            c_vw = list(reversed(c_vw))
        e_new = tuple(sorted([u, w]))
        edge_chains[e_new] = c_uv + c_vw[1:]
        adj_c0[u].add(w)
        adj_c0[w].add(u)

    print(f"Contracted: {len(comp0_nodes)} -> {len(rem)}")
    contracted_edges = set(edge_chains.keys())
    forbidden_delete = virt_set | contracted_edges

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
        lits = inc_edges[u]
        clauses = CardEnc.equals(lits=lits, bound=2, top_id=top, encoding=EncType.cardnetwrk)
        for cl in clauses:
            solver.add_clause(cl)
            for lit in cl:
                top = max(top, abs(lit) + 1)

    for ve in virt_set:
        solver.add_clause([edge_to_var[ve]])
    for ce in contracted_edges:
        solver.add_clause([edge_to_var[ce]])

    # Triangles
    rem_list = sorted(list(rem))
    for u in rem_list:
        nbrs = sorted(list(adj_c0[u]))
        for i in range(len(nbrs)):
            for j in range(i + 1, len(nbrs)):
                v, w = nbrs[i], nbrs[j]
                if w in adj_c0[v] and u < v < w:
                    e1, e2, e3 = tuple(sorted([u, v])), tuple(sorted([v, w])), tuple(sorted([w, u]))
                    solver.add_clause([-edge_to_var[e1], -edge_to_var[e2], -edge_to_var[e3]])

    for it in range(1, 200):
        assert solver.solve(), "UNSAT in Comp 0!"
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

        # 2-opt merge
        merged = list(cycles)
        merged_any = True
        while merged_any and len(merged) > 1:
            merged_any = False
            merged.sort(key=len, reverse=True)
            for i in range(len(merged)):
                for j in range(i + 1, len(merged)):
                    res = try_merge_2opt(merged[i], merged[j], adj_c0, forbidden_delete)
                    if res is not None:
                        merged.pop(j)
                        merged[i] = res
                        merged_any = True
                        break
                if merged_any:
                    break

        if len(merged) == 2:
            print(f"\n[!] STOPPED AT ITER {it} WITH 2 CYCLES: len(C1)={len(merged[0])}, len(C2)={len(merged[1])}", flush=True)
            c1, c2 = merged[0], merged[1]
            s1, s2 = set(c1), set(c2)
            cross_edges = [(u, v) for u in s2 for v in adj_c0[u] if v in s1]
            print(f"Total cross edges between C2 and C1: {len(cross_edges)}", flush=True)

            print("Testing try_merge_2opt...", flush=True)
            r2 = try_merge_2opt(c1, c2, adj_c0, forbidden_delete)
            print("2-opt result:", r2 is not None, flush=True)

            print("Testing try_merge_3opt...", flush=True)
            r3 = try_merge_3opt(c1, c2, adj_c0, forbidden_delete)
            print("3-opt result:", r3 is not None, flush=True)

            print("Testing try_patch_merge (window=10)...", flush=True)
            rp10 = try_patch_merge(c1, c2, adj_c0, forbidden_delete, max_window=10)
            print("patch(10) result:", rp10 is not None, flush=True)

            print("Testing try_patch_merge (window=20)...", flush=True)
            rp20 = try_patch_merge(c1, c2, adj_c0, forbidden_delete, max_window=20)
            print("patch(20) result:", rp20 is not None, flush=True)

            # Let's also check SAT local splice:
            # We want to find a Hamiltonian cycle on C1 + C2 by keeping most of C1 fixed
            # and solving SAT locally!
            return

        for cyc in cycles:
            if len(cyc) <= len(rem) // 2:
                c_set = set(cyc)
                cut_vars = [edge_to_var[tuple(sorted((u, v)))] for u in cyc for v in adj_c0[u] if v not in c_set]
                solver.add_clause(cut_vars)
                cyc_edges = [edge_to_var[tuple(sorted((cyc[k], cyc[(k+1)%len(cyc)])))] for k in range(len(cyc))]
                solver.add_clause([-e for e in cyc_edges])

if __name__ == "__main__":
    inspect_and_merge()
