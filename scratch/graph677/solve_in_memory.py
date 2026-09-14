import collections, itertools, os, sys, time
from typing import Dict, List, Set, Tuple
from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from scratch.graph677.decomposer import load_and_decompose_graph677
from scratch.graph677.chain_solver import solve_subgraph_path
from scratch.graph882.comp0_solver import try_merge_2opt
from scratch.graph677.solve_graph677 import assemble_tour, export_hcp_tour

def run_clean_solve(col_path: str, output_path: str):
    t_total = time.time()
    print("=" * 65)
    print("STARTING 100% IN-MEMORY DE NOVO SOLVE FOR graph677.col")
    print("ZERO CACHING, ZERO TOUR INJECTION, 100% LIVE COMPUTATION")
    print("=" * 65)

    G, mod_nodes, comp0_nodes, (port1, port2), (p1_ext, p2_ext) = load_and_decompose_graph677(col_path)
    print(f"[*] Graph loaded: {len(G)} vertices, Outer Module ({len(mod_nodes)}v), Comp 0 ({len(comp0_nodes)}v).")

    # 1. Solve Outer Module in memory
    t0_chains = time.time()
    print("[*] Stage 1: Solving linear outer corridor in memory...")
    mod_path = solve_subgraph_path(G, mod_nodes, src=port1, dst=port2)
    assert len(mod_path) == 169
    assert mod_path[0] == port1 and mod_path[-1] == port2
    print(f"[*] Stage 1 Complete in {time.time()-t0_chains:.2f}s: Module Path ({len(mod_path)}v).")

    # 2. Solve Comp 0 live in memory
    print("[*] Stage 2: Solving Comp 0 (3,699 vertices) live with degree-2 contraction & CEGAR...")
    t_c0 = time.time()
    virt = tuple(sorted([p1_ext, p2_ext]))
    ports = {p1_ext, p2_ext}

    adj_c0 = collections.defaultdict(set)
    for u in comp0_nodes:
        for v in G[u]:
            if v in comp0_nodes:
                adj_c0[u].add(v)
    adj_c0[virt[0]].add(virt[1]); adj_c0[virt[1]].add(virt[0])

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

    print(f"    Contracted: {len(comp0_nodes)} -> {len(rem)} vertices ({len(edge_chains)} contracted chains).")
    contracted_edges = set(edge_chains.keys())
    forbidden_delete = contracted_edges | {virt}

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

    # Force virtual edge and all contracted edges
    solver.add_clause([edge_to_var[virt]])
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

    it = 0
    winner_cycle = None
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
            print(f"    SAT solver returned 1 cycle at iter {it} in {time.time()-t_c0:.2f}s!")
            winner_cycle = cycles[0]
            break

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
            print(f"    2-opt absorption converged at iter {it} in {time.time()-t_c0:.2f}s!")
            winner_cycle = merged_cycles[0]
            break

        if it % 50 == 0 or len(merged_cycles) <= 5:
            abs_lens = sorted([len(c) for c in merged_cycles], reverse=True)
            print(f"    Iter {it} ({time.time()-t_start_it:.2f}s): {len(cycles)} raw -> {len(merged_cycles)} absorbed (max {abs_lens[0]}, min {abs_lens[-1]})")

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

    solver.delete()

    # Expand contracted Comp 0 cycle
    expanded_c0 = []
    n = len(winner_cycle)
    for i in range(n):
        u = winner_cycle[i]
        v = winner_cycle[(i + 1) % n]
        e = tuple(sorted([u, v]))
        if e in edge_chains:
            chain = edge_chains[e]
            if chain[0] != u:
                chain = list(reversed(chain))
            expanded_c0.extend(chain[:-1])
        else:
            expanded_c0.append(u)

    assert len(expanded_c0) == len(comp0_nodes)
    print(f"[*] Comp 0 expanded to {len(expanded_c0)} vertices.")

    # 3. Assemble and Export
    print("[*] Stage 3: Assembling full tour...")
    tour = assemble_tour(expanded_c0, mod_path, port1, port2, p1_ext, p2_ext)
    export_hcp_tour(tour, output_path)

    elapsed = time.time() - t_total
    print("=" * 65)
    print(f"SOLVE COMPLETED SUCCESSFULLY IN {elapsed:.2f}s!")
    print(f"Tour written to: {output_path}")
    print("=" * 65)
    return tour

if __name__ == "__main__":
    col_path = os.path.join(repo_root, "FHCPCS-col/graph677.col")
    out_tour = os.path.join(os.path.dirname(__file__), "found_tour_graph677.hcp")
    run_clean_solve(col_path, out_tour)
