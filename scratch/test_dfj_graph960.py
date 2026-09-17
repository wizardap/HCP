import sys, collections, time, os
sys.path.insert(0, ".")
from pysat.solvers import Cadical195
from scratch.engine.graph_loader import load_dimacs

def test_dfj_960():
    col_path = "FHCPCS-col/graph960.col"
    t0 = time.time()
    print(f"[*] Loading {col_path}...")
    G = load_dimacs(col_path)
    nv = len(G)
    deg2 = [u for u in G if len(G[u]) == 2]
    
    blocks = []
    v_partner = {}
    for v in deg2:
        u, w = list(G[v])
        blocks.append((u, w, v))
        v_partner[u] = w
        v_partner[w] = u
        
    non_deg2 = set(G.keys()) - set(deg2)
    color = {blocks[0][0]: 0}
    q = [blocks[0][0]]
    while q:
        curr = q.pop()
        c = color[curr]
        vp = v_partner[curr]
        if vp not in color:
            color[vp] = 1 - c; q.append(vp)
        for nxt in G[curr]:
            if nxt != vp and nxt in non_deg2 and nxt not in color:
                color[nxt] = 1 - c; q.append(nxt)
                
    n_dir = len(blocks)
    node_to_id = {}
    id_to_pair = {}
    for idx, (u, w, v) in enumerate(blocks):
        node_to_id[u] = idx; node_to_id[w] = idx
        in_v = u if color[u] == 0 else w
        out_v = w if in_v == u else u
        id_to_pair[idx] = (in_v, out_v, v)
        
    dir_adj = collections.defaultdict(list)
    in_adj = collections.defaultdict(list)
    arcs = set()
    for idx in range(n_dir):
        in_v, out_v, _ = id_to_pair[idx]
        for nxt in sorted(G[out_v]):
            if nxt != in_v and nxt in node_to_id and nxt == id_to_pair[node_to_id[nxt]][0]:
                arc = (idx, node_to_id[nxt])
                if arc not in arcs:
                    arcs.add(arc); dir_adj[idx].append(node_to_id[nxt]); in_adj[node_to_id[nxt]].append(idx)
                    
    arc_list = sorted(list(arcs))
    arc_to_var = {arc: i + 1 for i, arc in enumerate(arc_list)}
    
    print(f"[*] graph960: {n_dir} directed nodes, {len(arc_list)} arcs.")
    
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
                
    # Static short cycles: 2-cycles, 3-cycles, 4-cycles
    for u in range(n_dir):
        for v in dir_adj[u]:
            if v > u and u in dir_adj[v]:
                solver.add_clause([-arc_to_var[(u, v)], -arc_to_var[(v, u)]])
                
    for u in range(n_dir):
        for v in dir_adj[u]:
            for w in dir_adj[v]:
                if w != u and u in dir_adj[w] and u < v and u < w:
                    solver.add_clause([-arc_to_var[(u, v)], -arc_to_var[(v, w)], -arc_to_var[(w, u)]])
                    
    for u in range(n_dir):
        for v in dir_adj[u]:
            for w in dir_adj[v]:
                if w != u:
                    for x in dir_adj[w]:
                        if x != u and x != v and u in dir_adj[x] and u < v and u < w and u < x:
                            solver.add_clause([-arc_to_var[(u, v)], -arc_to_var[(v, w)], -arc_to_var[(w, x)], -arc_to_var[(x, u)]])
                            
    print(f"[*] Base solver initialized in {time.time()-t0:.2f}s.")
    
    round_num = 0
    while True:
        round_num += 1
        t_r0 = time.time()
        res = solver.solve()
        t_solve = time.time() - t_r0
        if not res:
            print("UNSAT!")
            return False
            
        model = set(solver.get_model())
        succ = {u: v for (u, v), var in arc_to_var.items() if var in model}
        
        visited = set()
        cycles = []
        for start in range(n_dir):
            if start not in visited:
                c = []
                curr = start
                while curr not in visited:
                    visited.add(curr); c.append(curr); curr = succ[curr]
                cycles.append(c)
                
        cycles.sort(key=len, reverse=True)
        top5 = [len(c) for c in cycles[:5]]
        print(f"Round {round_num:2d} ({t_solve*1000:6.1f}ms): {len(cycles):3d} cycles, Giant: {len(cycles[0])}/{n_dir} ({100*len(cycles[0])/n_dir:.1f}%), top5: {top5}", flush=True)
        
        if len(cycles) == 1:
            print(f"*** FOUND HAMILTONIAN CYCLE in {round_num} rounds, {time.time()-t0:.2f}s! ***", flush=True)
            return True
            
        # Cuts:
        for c in cycles:
            c_set = set(c)
            out_b = [arc_to_var[(u, v)] for u in c for v in dir_adj[u] if v not in c_set]
            if out_b: solver.add_clause(out_b)
            in_b = [arc_to_var[(v, u)] for u in c for v in in_adj[u] if v not in c_set]
            if in_b: solver.add_clause(in_b)
            if len(c) <= 20:
                solver.add_clause([-arc_to_var[(c[i], c[(i+1)%len(c)])] for i in range(len(c))])

if __name__ == "__main__":
    test_dfj_960()
