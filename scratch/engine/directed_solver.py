import os, sys, collections, time
repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)
from pysat.solvers import Cadical195
from scratch.engine.graph_loader import load_dimacs
from scratch.engine.assembler import export_hcp_tour

def solve_directed_bipartite_3block(col_path: str, output_path: str = None):
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
        v_partner[u] = w; v_partner[w] = u
        
    non_deg2 = set(G.keys()) - set(deg2)
    color = {blocks[0][0]: 0}
    q = [blocks[0][0]]
    while q:
        curr = q.pop(); c = color[curr]; vp = v_partner[curr]
        if vp not in color: color[vp] = 1 - c; q.append(vp)
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
                    arcs.add(arc)
                    dir_adj[idx].append(node_to_id[nxt])
                    in_adj[node_to_id[nxt]].append(idx)
                    
    arc_list = sorted(list(arcs))
    arc_to_var = {arc: i + 1 for i, arc in enumerate(arc_list)}
    dir_adj_set = {u: set(dir_adj[u]) for u in range(n_dir)}
    
    print(f"[*] Graph reduced to {n_dir} directed nodes, {len(arc_list)} arcs.")
    
    solver = Cadical195()
    
    # 2-factor clauses: exactly 1 out-arc and 1 in-arc per node
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
                
    # Static cuts for short cycles (lengths 2 to 6)
    t_cycles = time.time()
    static_cuts_count = 0
    for target_len in range(2, 7):
        cuts_this_len = 0
        for start in range(n_dir):
            path = [start]
            def dfs(u, d):
                nonlocal cuts_this_len, static_cuts_count
                if d == target_len:
                    if start in dir_adj_set[u]:
                        clause = [-arc_to_var[(path[i], path[i+1])] for i in range(len(path)-1)]
                        clause.append(-arc_to_var[(path[-1], start)])
                        solver.add_clause(clause)
                        c_set = set(path)
                        out_b = [arc_to_var[(x, y)] for x in path for y in dir_adj[x] if y not in c_set]
                        if out_b:
                            solver.add_clause(out_b)
                        in_b = [arc_to_var[(y, x)] for x in path for y in in_adj[x] if y not in c_set]
                        if in_b:
                            solver.add_clause(in_b)
                        cuts_this_len += 1
                        static_cuts_count += 3
                    return
                for v in dir_adj[u]:
                    if v > start and v not in path:
                        path.append(v)
                        dfs(v, d + 1)
                        path.pop()
            dfs(start, 1)
        print(f"  Pre-cut cycles of len {target_len}: {cuts_this_len} ({time.time()-t_cycles:.2f}s)")

    print(f"[*] Base solver initialized with {static_cuts_count} static cuts in {time.time()-t0:.2f}s.")
    
    def try_directed_2opt(c1, c2):
        """
        Merge two directed cycles c1 and c2 using directed 2-opt.
        Arc u1 -> v1 in c1, arc u2 -> v2 in c2.
        Cross arcs: u1 -> v2 and u2 -> v1.
        Merged cycle: v1 -> ... -> u1 -> v2 -> ... -> u2 -> v1.
        """
        n1 = len(c1)
        n2 = len(c2)
        pos2 = {u: i for i, u in enumerate(c2)}
        for i in range(n1):
            u1 = c1[i]
            v1 = c1[(i + 1) % n1]
            nbrs_u1 = [v2 for v2 in dir_adj_set[u1] if v2 in pos2]
            for v2 in nbrs_u1:
                idx2 = pos2[v2]
                u2 = c2[(idx2 - 1) % n2]
                if v1 in dir_adj_set[u2]:
                    # Found valid 2-opt merge!
                    # p1: v1 -> ... -> u1
                    p1 = [c1[(i + 1 + k) % n1] for k in range(n1)]
                    # p2: v2 -> ... -> u2
                    p2 = [c2[(idx2 + k) % n2] for k in range(n2)]
                    merged = p1 + p2
                    return merged
        return None

    round_num = 0
    seen_cuts = set()
    winner_cycle = None
    
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
                
        # Splicer: apply directed 2-opt
        merged = list(cycles)
        merged_any = True
        while merged_any and len(merged) > 1:
            merged_any = False
            merged.sort(key=len, reverse=True)
            for i in range(len(merged)):
                for j in range(i + 1, len(merged)):
                    res_m = try_directed_2opt(merged[i], merged[j])
                    if res_m is not None:
                        merged.pop(j)
                        merged[i] = res_m
                        merged_any = True
                        break
                if merged_any:
                    break
                    
        top_raw = sorted([len(c) for c in cycles], reverse=True)[:5]
        top_abs = sorted([len(c) for c in merged], reverse=True)[:5]
        print(f"Round {round_num:2d} ({t_solve*1000:6.1f}ms): {len(cycles):3d} raw (top5 {top_raw}) -> {len(merged):3d} merged (Giant: {len(merged[0])}/{n_dir} = {100*len(merged[0])/n_dir:.1f}%)", flush=True)
        
        if len(merged) == 1:
            print(f"*** FOUND DIRECTED HAMILTONIAN CYCLE in {round_num} rounds, {time.time()-t0:.2f}s! ***", flush=True)
            winner_cycle = merged[0]
            break
            
        # Cuts:
        # Cocycle cuts and negative cuts on raw cycles
        for c in cycles:
            if len(c) <= n_dir // 2:
                c_set = set(c)
                out_b = [arc_to_var[(u, v)] for u in c for v in dir_adj[u] if v not in c_set]
                if out_b:
                    t_out = tuple(sorted(out_b))
                    if t_out not in seen_cuts:
                        seen_cuts.add(t_out)
                        solver.add_clause(out_b)
                in_b = [arc_to_var[(v, u)] for u in c for v in in_adj[u] if v not in c_set]
                if in_b:
                    t_in = tuple(sorted(in_b))
                    if t_in not in seen_cuts:
                        seen_cuts.add(t_in)
                        solver.add_clause(in_b)

            neg_clause = [-arc_to_var[(c[i], c[(i+1)%len(c)])] for i in range(len(c))]
            t_neg = tuple(sorted(neg_clause))
            if t_neg not in seen_cuts:
                seen_cuts.add(t_neg)
                solver.add_clause(neg_clause)

        # Cocycle cuts on merged macro-cycles
        if len(merged) > 1:
            for c in merged:
                if len(c) <= n_dir // 2:
                    c_set = set(c)
                    out_b = [arc_to_var[(u, v)] for u in c for v in dir_adj[u] if v not in c_set]
                    if out_b:
                        t_out = tuple(sorted(out_b))
                        if t_out not in seen_cuts:
                            seen_cuts.add(t_out)
                            solver.add_clause(out_b)
                    in_b = [arc_to_var[(v, u)] for u in c for v in in_adj[u] if v not in c_set]
                    if in_b:
                        t_in = tuple(sorted(in_b))
                        if t_in not in seen_cuts:
                            seen_cuts.add(t_in)
                            solver.add_clause(in_b)

    # Reconstruct original Hamiltonian cycle
    print("[*] Reconstructing original graph tour...")
    full_tour = []
    for idx in winner_cycle:
        in_v, out_v, v = id_to_pair[idx]
        full_tour.extend([in_v, v, out_v])
        
    assert len(full_tour) == nv, f"Tour length {len(full_tour)} != {nv}"
    assert len(set(full_tour)) == nv, "Duplicate vertices in tour!"
    
    # Soundness verification against raw graph
    for i in range(nv):
        x = full_tour[i]
        y = full_tour[(i + 1) % nv]
        assert y in G[x], f"Raw graph edge violation: ({x}, {y})!"
        
    print(f"[✓] 100% SOUND AND COMPLETE TOUR VERIFIED ON {col_path}!")
    if output_path:
        os.makedirs(os.path.dirname(output_path), exist_ok=True)
        export_hcp_tour(full_tour, os.path.basename(col_path), output_path)
        print(f"[✓] Wrote verified tour to {output_path}!")
    return full_tour

if __name__ == "__main__":
    col = sys.argv[1] if len(sys.argv) > 1 else "FHCPCS-col/graph868.col"
    out = sys.argv[2] if len(sys.argv) > 2 else "scratch/graph868/found_tour_graph868.hcp"
    solve_directed_bipartite_3block(col, out)
