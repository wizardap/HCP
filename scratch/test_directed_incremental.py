import collections, time, os, sys
from pysat.solvers import Cadical195

def solve_bipartite_directed_macro(col_path, max_iters=200):
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
        
    # Extract blocks
    blocks = []
    v_partner = {}
    for v in deg2:
        u, w = list(G[v])
        blocks.append((u, w, v))
        v_partner[u] = w
        v_partner[w] = u
        
    # 2-Coloring for block orientation
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
        elif color[vp] == c:
            raise ValueError(f"Graph block structure is not bipartite! Conflict at partner ({curr}, {vp})")
        for nxt in G[curr]:
            if nxt != vp and nxt in non_deg2:
                if nxt not in color:
                    color[nxt] = 1 - c
                    q.append(nxt)
                elif color[nxt] == c:
                    raise ValueError(f"Graph block structure is not bipartite! Conflict at edge ({curr}, {nxt})")
                    
    n_dir = len(blocks)
    node_to_id = {}
    id_to_pair = {}
    for idx, (u, w, v) in enumerate(blocks):
        node_to_id[u] = idx
        node_to_id[w] = idx
        in_v = u if color[u] == 0 else w
        out_v = w if in_v == u else u
        id_to_pair[idx] = (in_v, out_v, v)
        
    # Build directed adjacency: arc from u_id to v_id exists if out_v connects to in_v
    dir_adj = collections.defaultdict(list)
    in_adj = collections.defaultdict(list)
    arcs = set()
    for idx in range(n_dir):
        in_v, out_v, _ = id_to_pair[idx]
        for nxt in sorted(G[out_v]):
            if nxt != in_v and nxt in node_to_id:
                target_idx = node_to_id[nxt]
                # nxt must be the in_v of target_idx
                if nxt == id_to_pair[target_idx][0]:
                    arc = (idx, target_idx)
                    if arc not in arcs:
                        arcs.add(arc)
                        dir_adj[idx].append(target_idx)
                        in_adj[target_idx].append(idx)
                        
    arc_list = sorted(list(arcs))
    arc_to_var = {arc: i + 1 for i, arc in enumerate(arc_list)}
    var_to_arc = {i + 1: arc for i, arc in enumerate(arc_list)}
    
    print(f"[*] Directed macro-graph built: {n_dir} nodes, {len(arc_list)} arcs. Variables: {len(arc_list)}")
    
    # Static clauses: exactly 1 out-arc and 1 in-arc per node
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
                
    print(f"[*] Initial CNF: {len(static_clauses)} static clauses. Starting Pure Incremental CaDiCaL (0% reseeding)...")
    solver = Cadical195(bootstrap_with=static_clauses)
    
    # 2-opt directed absorption
    def absorb_2opt(giant, small_cycles):
        curr_giant = list(giant)
        remaining = []
        pass_absorbed = 0
        for small in small_cycles:
            n_g = len(curr_giant)
            n_s = len(small)
            small_set = set(small)
            giant_pos = {v: i for i, v in enumerate(curr_giant)}
            small_pos = {v: i for i, v in enumerate(small)}
            found = None
            for u1 in curr_giant:
                for v2 in dir_adj[u1]:
                    if v2 in small_set:
                        i1 = giant_pos[u1]
                        v1 = curr_giant[(i1 + 1) % n_g]
                        j2 = small_pos[v2]
                        u2 = small[(j2 + n_s - 1) % n_s]
                        if v1 in dir_adj[u2]:
                            found = (i1, j2)
                            break
                if found:
                    break
            if found:
                i1, j2 = found
                new_giant = curr_giant[:i1+1] + [small[(j2 + k) % n_s] for k in range(n_s)] + curr_giant[i1+1:]
                curr_giant = new_giant
                pass_absorbed += 1
            else:
                remaining.append(small)
        return curr_giant, remaining, pass_absorbed
        
    it = 0
    winner_tour = None
    seen_cuts = set()
    
    while it < max_iters:
        it += 1
        t_it = time.time()
        if not solver.solve():
            print(f"[!] UNSAT at iteration {it}!")
            return None
            
        model = set(solver.get_model())
        active_arcs = [arc for arc in arc_list if arc_to_var[arc] in model]
        succ = {u: v for u, v in active_arcs}
        
        # Trace cycles
        visited = set()
        cycles = []
        for u in range(n_dir):
            if u not in visited:
                cyc = []
                curr = u
                while curr not in visited:
                    visited.add(curr)
                    cyc.append(curr)
                    curr = succ[curr]
                cycles.append(cyc)
                
        cycles.sort(key=len, reverse=True)
        
        if len(cycles) == 1:
            print(f"[*] SAT Converged at Iter {it} with 1 Single Cycle in {time.time()-t0:.2f}s!")
            winner_tour = cycles[0]
            break
            
        # Try absorption
        giant = cycles[0]
        giant, remaining, n_abs = absorb_2opt(giant, cycles[1:])
        
        dt = time.time() - t_it
        print(f"    Iter {it} ({dt*1000:.1f}ms, total {time.time()-t0:.2f}s): {len(cycles)} raw -> {len(remaining)+1} after 2-opt (giant: {len(giant)}/{n_dir})", flush=True)
        
        if not remaining:
            print(f"[*] 2-opt Absorption Converged at Iter {it} in {time.time()-t0:.2f}s! Giant: {len(giant)}/{n_dir}")
            winner_tour = giant
            break
            
        # Add negative cuts for all raw cycles < n_dir
        for cyc in cycles:
            if len(cyc) < n_dir:
                neg_clause = [-arc_to_var[(cyc[i], cyc[(i + 1) % len(cyc)])] for i in range(len(cyc))]
                t_cl = tuple(sorted(neg_clause))
                if t_cl not in seen_cuts:
                    seen_cuts.add(t_cl)
                    solver.add_clause(neg_clause)
                    
                # Dual boundary cuts for small cycles
                if len(cyc) <= 24:
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
                            
    if winner_tour is None:
        print(f"[!] Failed to converge within {max_iters} iterations.")
        return None
        
    # Reconstruct raw Hamiltonian tour
    # For each macro-node idx: in_v -> v (deg2 node) -> out_v
    print("[*] Reconstructing raw 5,544-vertex Hamiltonian tour...")
    raw_tour = []
    for idx in winner_tour:
        in_v, out_v, mid_v = id_to_pair[idx]
        raw_tour.append(in_v)
        raw_tour.append(mid_v)
        raw_tour.append(out_v)
        
    print(f"[✓] Tour reconstructed: length={len(raw_tour)}")
    # Verify tour sound
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
    solve_bipartite_directed_macro(col)
