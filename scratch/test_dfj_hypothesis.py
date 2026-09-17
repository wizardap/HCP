import collections, time, os, sys
from pysat.solvers import Cadical195

def test_dfj(col_path):
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
    var_to_arc = {i + 1: arc for i, arc in enumerate(arc_list)}
    
    print(f"[*] Directed macro-graph: {n_dir} nodes, {len(arc_list)} arcs.")
    
    solver = Cadical195()
    
    # 2-factor clauses
    for u in range(n_dir):
        out_lits = [arc_to_var[(u, v)] for v in dir_adj[u]]
        solver.add_clause(out_lits)
        for i in range(len(out_lits)):
            for j in range(i + 1, len(out_lits)):
                solver.add_clause([-out_lits[i], -out_lits[j]])
                
        in_lits = [arc_to_var[(v, u)] for v in in_adj[u]]
        solver.add_clause(in_lits)
        for i in range(len(in_lits)):
            for j in range(i + 1, len(in_lits)):
                solver.add_clause([-in_lits[i], -in_lits[j]])
                
    # Static short cycles
    # 2-cycles
    for u in range(n_dir):
        for v in dir_adj[u]:
            if v > u and u in dir_adj[v]:
                solver.add_clause([-arc_to_var[(u, v)], -arc_to_var[(v, u)]])
                
    # 3-cycles
    for u in range(n_dir):
        for v in dir_adj[u]:
            for w in dir_adj[v]:
                if w != u and u in dir_adj[w]:
                    if u < v and u < w:
                        solver.add_clause([-arc_to_var[(u, v)], -arc_to_var[(v, w)], -arc_to_var[(w, u)]])
                        
    # 4-cycles
    for u in range(n_dir):
        for v in dir_adj[u]:
            for w in dir_adj[v]:
                if w != u:
                    for x in dir_adj[w]:
                        if x != u and x != v and u in dir_adj[x]:
                            if u < v and u < w and u < x:
                                solver.add_clause([-arc_to_var[(u, v)], -arc_to_var[(v, w)], -arc_to_var[(w, x)], -arc_to_var[(x, u)]])

    print("[*] Base solver initialized.")
    
    phase_hints = []
    round_num = 0
    while True:
        round_num += 1
        t_r0 = time.time()
        
        # No phase hints in pure SAT
                
        res = solver.solve()
        t_solve = time.time() - t_r0
        if not res:
            print("UNSAT! No Hamiltonian cycle exists!")
            return False
            
        model = set(solver.get_model())
        succ = {}
        for (u, v), var in arc_to_var.items():
            if var in model:
                succ[u] = v
                
        visited = set()
        cycles = []
        for start in range(n_dir):
            if start not in visited:
                c = []
                curr = start
                while curr not in visited:
                    visited.add(curr)
                    c.append(curr)
                    curr = succ[curr]
                cycles.append(c)
                
        cycles.sort(key=len, reverse=True)
        top5 = [len(c) for c in cycles[:5]]
        print(f"Round {round_num:2d} ({t_solve*1000:6.1f}ms): {len(cycles):3d} cycles, Giant: {len(cycles[0])}/{n_dir} ({100*len(cycles[0])/n_dir:.1f}%), top5: {top5}")
        
        if len(cycles) == 1:
            print(f"*** FOUND HAMILTONIAN CYCLE in {round_num} rounds, {time.time()-t0:.2f}s! ***")
            return True
            
        # Update phase hints to giant
        phase_hints = [arc_to_var[(cycles[0][i], cycles[0][(i + 1) % len(cycles[0])])] for i in range(len(cycles[0]))]
        
        # DFJ BOUNDARY CUTS:
        # Every cycle < n_dir MUST have an arc leaving and entering!
        for c in cycles:
            c_set = set(c)
            # Out-boundary cut: at least one arc must leave c
            out_boundary = [arc_to_var[(u, v)] for u in c for v in dir_adj[u] if v not in c_set]
            if out_boundary:
                solver.add_clause(out_boundary)
            # In-boundary cut: at least one arc must enter c
            in_boundary = [arc_to_var[(v, u)] for u in c for v in in_adj[u] if v not in c_set]
            if in_boundary:
                solver.add_clause(in_boundary)
                
            # For small cycles (len <= 20), also add negative cut
            if len(c) <= 20:
                solver.add_clause([-arc_to_var[(c[i], c[(i + 1) % len(c)])] for i in range(len(c))])

if __name__ == "__main__":
    test_dfj("FHCPCS-col/graph868.col")
