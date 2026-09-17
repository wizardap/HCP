import collections, time
from scratch.engine.graph_loader import load_dimacs
from scratch.engine.decomposer import decompose_modular_graph
from scratch.engine.sat_merger import solve_local_hp_sat
from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType

def run_test():
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

    edges = sorted([tuple(sorted((u, v))) for u in rem for v in adj_c0[u] if u < v])
    edge_to_var = {e: i + 1 for i, e in enumerate(edges)}
    inc_edges = collections.defaultdict(list)
    for e in edges:
        inc_edges[e[0]].append(edge_to_var[e])
        inc_edges[e[1]].append(edge_to_var[e])

    solver = Cadical195()
    top = len(edges) + 1
    for u in sorted(rem):
        cl = CardEnc.equals(lits=inc_edges[u], bound=2, top_id=top, encoding=EncType.cardnetwrk)
        for c in cl:
            solver.add_clause(c)
            for lit in c:
                top = max(top, abs(lit) + 1)

    for ve in virt_set:
        solver.add_clause([edge_to_var[ve]])
    for ce in contracted_edges:
        solver.add_clause([edge_to_var[ce]])

    # Run until we hit the 2-cycle state:
    for it in range(1, 300):
        solver.solve()
        model = set(solver.get_model())
        active_edges = [e for e in edges if edge_to_var[e] in model]
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

        if len(cycles) == 2:
            c1, c2 = max(cycles, key=len), min(cycles, key=len)
            print(f"[*] Reached 2 cycles at iter {it}: len(C1)={len(c1)}, len(C2)={len(c2)}")
            s1, s2 = set(c1), set(c2)
            cross_edges = [(u, v) for u in s2 for v in adj_c0[u] if v in s1]
            print(f"Cross edges count: {len(cross_edges)}")
            print(f"Cross edges: {cross_edges}")

            # Test finding Hamiltonian path in C2 for each pair of cross edges:
            valid_pairs = []
            t0 = time.time()
            for i in range(len(cross_edges)):
                u1, v1 = cross_edges[i]
                for j in range(i + 1, len(cross_edges)):
                    u2, v2 = cross_edges[j]
                    if u1 == u2:
                        continue
                    hp = solve_local_hp_sat(c2, u1, u2, adj_c0, forbidden_delete)
                    if hp is not None:
                        print(f"  [+] Found valid HP in C2 connecting {u1} -> {u2} (ext {v1} -> {v2}) in {time.time()-t0:.3f}s!")
                        valid_pairs.append(((u1, v1), (u2, v2), hp))
                        break
                if valid_pairs:
                    break
            print(f"Total valid pairs found: {len(valid_pairs)}")
            return

        for cyc in cycles:
            if len(cyc) <= len(rem) // 2:
                c_set = set(cyc)
                cut_vars = [edge_to_var[tuple(sorted((u, v)))] for u in cyc for v in adj_c0[u] if v not in c_set]
                solver.add_clause(cut_vars)
                cyc_edges = [edge_to_var[tuple(sorted((cyc[k], cyc[(k+1)%len(cyc)])))] for k in range(len(cyc))]
                solver.add_clause([-e for e in cyc_edges])

if __name__ == "__main__":
    run_test()
