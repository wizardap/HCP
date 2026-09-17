import time, collections
from scratch.engine.graph_loader import load_dimacs
from scratch.engine.comp0_solver import try_merge_2opt, try_merge_3opt, try_patch_merge
from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType

def solve_with_phase_saving(graph_file):
    t0 = time.time()
    G = load_dimacs(graph_file)
    adj = {u: set(G[u]) for u in G}
    rem = set(G.keys())
    edge_chains = {}
    
    # d2 contraction
    while True:
        d2 = [u for u in rem if len(adj[u]) == 2]
        if not d2:
            break
        v = d2[0]
        u, w = list(adj[v])
        adj[u].remove(v)
        adj[w].remove(v)
        del adj[v]
        rem.remove(v)
        e_uv = tuple(sorted([u, v]))
        e_vw = tuple(sorted([v, w]))
        c_uv = edge_chains.pop(e_uv, [u, v])
        c_vw = edge_chains.pop(e_vw, [v, w])
        if c_uv[-1] != v: c_uv = list(reversed(c_uv))
        if c_vw[0] != v: c_vw = list(reversed(c_vw))
        e_new = tuple(sorted([u, w]))
        edge_chains[e_new] = c_uv + c_vw[1:]
        adj[u].add(w)
        adj[w].add(u)
        
    print(f"Contracted: {len(G)} -> {len(rem)} vertices in {time.time()-t0:.2f}s")
    contracted_edges = set(edge_chains.keys())
    
    edges = sorted(list({tuple(sorted([u, v])) for u in rem for v in adj[u] if u < v}))
    edge_to_var = {e: i + 1 for i, e in enumerate(edges)}
    inc_edges = collections.defaultdict(list)
    for e in edges:
        inc_edges[e[0]].append(edge_to_var[e])
        inc_edges[e[1]].append(edge_to_var[e])
        
    solver = Cadical195()
    top = len(edges) + 1
    for u in sorted(rem):
        clauses = CardEnc.equals(inc_edges[u], 2, top_id=top, encoding=EncType.cardnetwrk)
        for cl in clauses:
            solver.add_clause(cl)
            for lit in cl:
                top = max(top, abs(lit) + 1)
                
    for ce in contracted_edges:
        if ce in edge_to_var:
            solver.add_clause([edge_to_var[ce]])
            
    # Static triangles
    for u in sorted(rem):
        for v in adj[u]:
            if v > u:
                for w in adj[v]:
                    if w > v and w in adj[u]:
                        solver.add_clause([-edge_to_var[tuple(sorted([u, v]))],
                                           -edge_to_var[tuple(sorted([v, w]))],
                                           -edge_to_var[tuple(sorted([w, u]))]])
                                           
    print(f"Formulation built in {time.time()-t0:.2f}s. Starting CEGAR with phase saving + 2/3-opt...")
    winner = None
    for it in range(1, 201):
        t_it = time.time()
        res = solver.solve()
        if not res:
            print("UNSAT!")
            return None
            
        raw_model = solver.get_model()
        model_set = set(raw_model)
        
        # Guide phase saving towards current 2-factor
        solver.set_phases([v if v in model_set else -v for v in range(1, len(edges) + 1)])
        
        # Extract cycles
        active = [e for e in edges if edge_to_var[e] in model_set]
        a_adj = collections.defaultdict(list)
        for u, v in active:
            a_adj[u].append(v)
            a_adj[v].append(u)
        vis = set()
        cycles = []
        for u in rem:
            if u not in vis:
                c = []
                curr, prev = u, None
                while curr not in vis:
                    vis.add(curr)
                    c.append(curr)
                    nbrs = a_adj[curr]
                    nxt = nbrs[0] if nbrs[0] != prev else nbrs[1]
                    prev, curr = curr, nxt
                cycles.append(c)
                
        if len(cycles) == 1:
            print(f"[✓] Exact SAT cycle found at iter {it} in {time.time()-t0:.2f}s!")
            winner = cycles[0]
            break
            
        # Try absorption: 2-opt first, then 3-opt, then patch merge
        merged = list(cycles)
        merged_any = True
        while merged_any and len(merged) > 1:
            merged_any = False
            merged.sort(key=len, reverse=True)
            for i in range(len(merged)):
                for j in range(i + 1, len(merged)):
                    # 2-opt
                    res2 = try_merge_2opt(merged[i], merged[j], adj, contracted_edges)
                    if res2 is not None:
                        merged.pop(j)
                        merged[i] = res2
                        merged_any = True
                        break
                    # 3-opt if j is small
                    if len(merged[j]) <= 50:
                        res3 = try_merge_3opt(merged[i], merged[j], adj, contracted_edges)
                        if res3 is not None:
                            merged.pop(j)
                            merged[i] = res3
                            merged_any = True
                            break
                    # patch merge if j is small
                    if len(merged[j]) <= 40:
                        resp = try_patch_merge(merged[i], merged[j], adj, contracted_edges, max_window=10)
                        if resp is not None:
                            merged.pop(j)
                            merged[i] = resp
                            merged_any = True
                            break
                if merged_any:
                    break
                    
        if len(merged) == 1:
            print(f"[✓] Absorbed 2/3-opt cycle found at iter {it} in {time.time()-t0:.2f}s!")
            winner = merged[0]
            break
            
        lens = sorted([len(c) for c in merged], reverse=True)
        if it <= 5 or it % 5 == 0 or len(merged) <= 10:
            print(f"    Iter {it} ({time.time()-t_it:.2f}s): {len(cycles)} raw -> {len(merged)} absorbed (max {lens[0]}, min {lens[-1]})", flush=True)
            
        # Add cocycle cuts and negative cuts on RAW cycles
        for cyc in cycles:
            if len(cyc) <= len(rem) // 2:
                c_set = set(cyc)
                cut_e = [tuple(sorted([u, v])) for u in cyc for v in adj[u] if v not in c_set]
                solver.add_clause([edge_to_var[e] for e in cut_e])
            neg_c = [-edge_to_var[tuple(sorted([cyc[i], cyc[(i+1)%len(cyc)]]))] for i in range(len(cyc))]
            solver.add_clause(neg_c)
            
    if winner:
        # Reconstruct full cycle
        full = []
        for i in range(len(winner)):
            u, v = winner[i], winner[(i+1)%len(winner)]
            e = tuple(sorted([u, v]))
            if e in edge_chains:
                chain = edge_chains[e]
                if chain[0] == u and chain[-1] == v:
                    full.extend(chain[:-1])
                elif chain[-1] == u and chain[0] == v:
                    full.extend(list(reversed(chain))[:-1])
                else:
                    raise ValueError("Chain orientation error")
            else:
                full.append(u)
        print(f"Full reconstructed tour length: {len(full)}")
        assert len(full) == len(G), f"Expected {len(G)}, got {len(full)}"
        assert len(set(full)) == len(G), "Duplicates found!"
        print(f"[✓] SUCCESS! Solved {graph_file} in {time.time()-t0:.2f}s total!")
        return full
    return None

if __name__ == '__main__':
    solve_with_phase_saving('FHCPCS-col/graph788.col')
