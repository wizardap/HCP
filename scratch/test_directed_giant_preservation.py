import collections, time, os, sys
from pysat.solvers import Cadical195

def solve_directed_giant_preservation(col_path, max_iters=200):
    t0 = time.time()
    print(f"[*] Loading {col_path}...")
    G = collections.defaultdict(set)
    with open(col_path) as f:
        for line in f:
            if line.startswith("e "):
                _, u, v = line.split()
                u, v = int(u), int(v)
                G[u].add(v); G[v].add(u)
    
    nv = len(G)
    deg2 = [u for u in G if len(G[u]) == 2]
    print(f"[*] Graph loaded: |V|={nv}, Deg-2 nodes={len(deg2)}")
    
    if len(deg2) * 3 != nv:
        raise ValueError(f"Graph is not a 3-block graph! deg2*3={len(deg2)*3} != {nv}")
        
    blocks = []
    v_partner = {}
    for v in deg2:
        u, w = list(G[v])
        blocks.append((u, w, v))
        v_partner[u] = w; v_partner[w] = u
        
    non_deg2 = set(G.keys()) - set(deg2)
    color = {}
    q = [blocks[0][0]]
    color[blocks[0][0]] = 0
    while q:
        curr = q.pop()
        c = color[curr]
        vp = v_partner[curr]
        if vp not in color:
            color[vp] = 1 - c
            q.append(vp)
        for nxt in G[curr]:
            if nxt != vp and nxt in non_deg2:
                if nxt not in color:
                    color[nxt] = 1 - c
                    q.append(nxt)
                    
    n_dir = len(blocks)
    node_to_id = {}
    id_to_pair = {}
    for idx, (u, w, v) in enumerate(blocks):
        node_to_id[u] = idx
        node_to_id[w] = idx
        in_v = u if color[u] == 0 else w
        out_v = w if in_v == u else u
        id_to_pair[idx] = (in_v, out_v, v)
        
    dir_adj = collections.defaultdict(list)
    in_adj = collections.defaultdict(list)
    arcs = set()
    for idx in range(n_dir):
        in_v, out_v, _ = id_to_pair[idx]
        for nxt in sorted(G[out_v]):
            if nxt != in_v and nxt in node_to_id:
                target_idx = node_to_id[nxt]
                if nxt == id_to_pair[target_idx][0]:
                    arc = (idx, target_idx)
                    if arc not in arcs:
                        arcs.add(arc)
                        dir_adj[idx].append(target_idx)
                        in_adj[target_idx].append(idx)
                        
    arc_list = sorted(list(arcs))
    arc_to_var = {arc: i + 1 for i, arc in enumerate(arc_list)}
    
    print(f"[*] Directed macro-graph: {n_dir} nodes, {len(arc_list)} arcs.")
    
    # Static 1-in, 1-out degree clauses
    static_clauses = []
    for u in range(n_dir):
        out_lits = [arc_to_var[(u, v)] for v in dir_adj[u]]
        static_clauses.append(out_lits)
        for i in range(len(out_lits)):
            for j in range(i + 1, len(out_lits)):
                static_clauses.append([-out_lits[i], -out_lits[j]])
        in_lits = [arc_to_var[(v, u)] for v in in_adj[u]]
        static_clauses.append(in_lits)
        for i in range(len(in_lits)):
            for j in range(i + 1, len(in_lits)):
                static_clauses.append([-in_lits[i], -in_lits[j]])
                
    # Add static short cycle mutexes (lengths 2, 3, 4)
    c2_count = 0
    for u in range(n_dir):
        for v in dir_adj[u]:
            if v > u and u in dir_adj[v]:
                static_clauses.append([-arc_to_var[(u, v)], -arc_to_var[(v, u)]])
                c2_count += 1
                
    c3_count = 0
    for u in range(n_dir):
        for v in dir_adj[u]:
            if v != u:
                for w in dir_adj[v]:
                    if w != u and w != v and u in dir_adj[w]:
                        if u < v and u < w:
                            static_clauses.append([-arc_to_var[(u, v)], -arc_to_var[(v, w)], -arc_to_var[(w, u)]])
                            c3_count += 1

    c4_count = 0
    for u in range(n_dir):
        for v in dir_adj[u]:
            for w in dir_adj[v]:
                if w != u and w != v:
                    for z in dir_adj[w]:
                        if z != u and z != v and z != w and u in dir_adj[z]:
                            if u == min(u, v, w, z):
                                static_clauses.append([-arc_to_var[(u, v)], -arc_to_var[(v, w)], -arc_to_var[(w, z)], -arc_to_var[(z, u)]])
                                c4_count += 1
                                
    print(f"[*] Added static short cycle mutexes: {c2_count} 2-cycles, {c3_count} 3-cycles, {c4_count} 4-cycles (total {c2_count+c3_count+c4_count}).")
    
    solver = Cadical195(bootstrap_with=static_clauses)
    solver.configure({"chrono": 1})
    seen_cuts = {tuple(sorted(cl)) for cl in static_clauses}
    
    it = 0
    winner_tour = None
    best_giant_len = 0
    
    while it < max_iters:
        it += 1
        t_it = time.time()
        if not solver.solve():
            print(f"[!] UNSAT at iteration {it}!")
            return None
            
        model = set(solver.get_model())
        active_arcs = [arc for arc in arc_list if arc_to_var[arc] in model]
        succ = {u: v for u, v in active_arcs}
        
        visited = set()
        cycles = []
        for u in range(n_dir):
            if u not in visited:
                cyc = []; curr = u
                while curr not in visited:
                    visited.add(curr); cyc.append(curr); curr = succ[curr]
                cycles.append(cyc)
        cycles.sort(key=len, reverse=True)
        
        giant = cycles[0]
        if len(giant) > best_giant_len:
            best_giant_len = len(giant)
            
        dt = time.time() - t_it
        print(f"    Iter {it} ({dt*1000:.1f}ms, total {time.time()-t0:.2f}s): {len(cycles)} cycles, Giant: {len(giant)}/{n_dir} ({len(giant)*100/n_dir:.1f}%), smallest: {len(cycles[-1])}", flush=True)
        
        if len(cycles) == 1:
            print(f"[*] Converged at Iter {it} with 1 Single Cycle in {time.time()-t0:.2f}s!")
            winner_tour = cycles[0]
            break
            
        # Guide solver phases towards the giant cycle
        giant_lits = [arc_to_var[(giant[i], giant[(i + 1) % len(giant)])] for i in range(len(giant))]
        solver.set_phases(giant_lits)
        
        # Add cuts:
        # 1. DO NOT add negative cuts on giant if giant > n_dir // 2!
        # 2. For ALL cycles <= n_dir // 2:
        #    Add negative cycle cut + dual boundary cuts
        for cyc in cycles:
            if len(cyc) <= n_dir // 2:
                neg_cl = [-arc_to_var[(cyc[i], cyc[(i + 1) % len(cyc)])] for i in range(len(cyc))]
                t_neg = tuple(sorted(neg_cl))
                if t_neg not in seen_cuts:
                    seen_cuts.add(t_neg)
                    solver.add_clause(neg_cl)
                    
                c_set = set(cyc)
                out_lits = [arc_to_var[(u, v)] for u in cyc for v in dir_adj[u] if v not in c_set]
                if out_lits:
                    t_out = tuple(sorted(out_lits))
                    if t_out not in seen_cuts:
                        seen_cuts.add(t_out)
                        solver.add_clause(out_lits)
                        
                in_lits = [arc_to_var[(v, u)] for u in cyc for v in in_adj[u] if v not in c_set]
                if in_lits:
                    t_in = tuple(sorted(in_lits))
                    if t_in not in seen_cuts:
                        seen_cuts.add(t_in)
                        solver.add_clause(in_lits)
            else:
                # If cycle > n_dir // 2, we only add negative cut if there are NO small cycles (only 2 large cycles)
                if len(cycles) == 2:
                    neg_cl = [-arc_to_var[(cyc[i], cyc[(i + 1) % len(cyc)])] for i in range(len(cyc))]
                    t_neg = tuple(sorted(neg_cl))
                    if t_neg not in seen_cuts:
                        seen_cuts.add(t_neg)
                        solver.add_clause(neg_cl)
                        
    if winner_tour is None:
        print(f"[!] Failed to converge within {max_iters} iterations.")
        return None
        
    print("[*] Reconstructing raw Hamiltonian tour...")
    raw_tour = []
    for idx in winner_tour:
        in_v, out_v, mid_v = id_to_pair[idx]
        raw_tour.append(in_v)
        raw_tour.append(mid_v)
        raw_tour.append(out_v)
        
    seen = set()
    for v in raw_tour:
        assert v not in seen, f"Duplicate vertex {v}!"
        seen.add(v)
    assert len(seen) == nv, f"Not all vertices visited: {len(seen)} != {nv}"
    for i in range(len(raw_tour)):
        u = raw_tour[i]
        v = raw_tour[(i + 1) % len(raw_tour)]
        assert v in G[u], f"Edge ({u}, {v}) does not exist in G!"
        
    print(f"=================================================================")
    print(f"[✓] 100% CERTIFIED SOUND HAMILTONIAN TOUR FOUND IN {time.time()-t0:.2f}s!")
    print(f"=================================================================")
    return raw_tour

if __name__ == "__main__":
    col = sys.argv[1] if len(sys.argv) > 1 else "FHCPCS-col/graph868.col"
    solve_directed_giant_preservation(col)
