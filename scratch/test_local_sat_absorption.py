import collections, time
from pysat.solvers import Cadical195

def test_local_sat_absorption(col_path="FHCPCS-col/graph868.col"):
    G = collections.defaultdict(set)
    with open(col_path) as f:
        for line in f:
            if line.startswith("e "):
                _, u, v = line.split()
                u, v = int(u), int(v)
                G[u].add(v); G[v].add(u)
    deg2 = [u for u in G if len(G[u]) == 2]
    blocks = [(list(G[v])[0], list(G[v])[1], v) for v in deg2]
    v_partner = {u: w for u, w, v in blocks}
    for u, w, v in blocks: v_partner[w] = u
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
    node_to_id = {u: idx for idx, (u, w, v) in enumerate(blocks)}
    for idx, (u, w, v) in enumerate(blocks): node_to_id[w] = idx
    id_to_pair = {idx: (u if color[u] == 0 else w, w if color[u] == 0 else u) for idx, (u, w, v) in enumerate(blocks)}

    dir_adj = collections.defaultdict(list)
    in_adj = collections.defaultdict(list)
    for idx in range(n_dir):
        in_v, out_v = id_to_pair[idx]
        for nxt in G[out_v]:
            if nxt != in_v and nxt in node_to_id and nxt == id_to_pair[node_to_id[nxt]][0]:
                dir_adj[idx].append(node_to_id[nxt])
                in_adj[node_to_id[nxt]].append(idx)

    # Let us get a set of cycles by running 1 solve
    arc_set = {(u, v) for u in range(n_dir) for v in dir_adj[u]}
    arc_list = sorted(list(arc_set))
    arc_to_var = {arc: i + 1 for i, arc in enumerate(arc_list)}
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

    for u in range(n_dir):
        for v in dir_adj[u]:
            if v > u and dir_adj[v].contains if hasattr(dir_adj[v], "contains") else u in dir_adj[v]:
                static_clauses.append([-arc_to_var[(u, v)], -arc_to_var[(v, u)]])

    solver = Cadical195(bootstrap_with=static_clauses)
    solver.configure({"chrono": 1})
    solver.solve()
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
    giant_set = set(giant)
    giant_pos = {v: i for i, v in enumerate(giant)}
    print(f"Giant has {len(giant)} nodes. Small cycles: {len(cycles)-1}")

    # Local SAT Splicer function
    def local_sat_splice(c_giant, c_small, max_window=10):
        # Find all nodes in giant that have arcs to or from c_small
        s_set = set(c_small)
        touch_in = [u for u in c_giant if any(v in s_set for v in dir_adj[u])]
        touch_out = [u for u in c_giant if any(v in s_set for v in in_adj[u])]
        
        # Test windows in giant of length w around touching nodes
        n_g = len(c_giant)
        g_pos = {v: i for i, v in enumerate(c_giant)}
        
        # Try windows between an entry and exit
        for u_entry in touch_in:
            idx_entry = g_pos[u_entry]
            for w in range(1, max_window + 1):
                idx_exit = (idx_entry + w) % n_g
                u_exit = c_giant[idx_exit]
                # u_entry connects to sub_nodes, u_exit receives from sub_nodes
                # Window nodes to be replaced: c_giant[idx_entry + 1 ... idx_exit - 1]
                if w == 1:
                    win_nodes = []
                else:
                    win_nodes = [c_giant[(idx_entry + k) % n_g] for k in range(1, w)]
                sub_nodes = win_nodes + c_small
                sub_set = set(sub_nodes)
                
                # Check if u_entry has arc into sub_nodes, and u_exit has arc from sub_nodes
                cand_starts = [v for v in dir_adj[u_entry] if v in sub_set]
                cand_ends = [v for v in in_adj[u_exit] if v in sub_set]
                if not cand_starts or not cand_ends:
                    continue
                    
                # Solve local Hamiltonian path through sub_nodes
                sub_arcs = [(u, v) for u in sub_nodes for v in dir_adj[u] if v in sub_set]
                s_arc_var = {a: i + 1 for i, a in enumerate(sub_arcs)}
                
                # Try start/end pairs
                for s_node in cand_starts:
                    for e_node in cand_ends:
                        if s_node == e_node and len(sub_nodes) > 1:
                            continue
                        sub_solver = Cadical195()
                        # Out degree: 1 for all except e_node (which has 0 in path)
                        for u in sub_nodes:
                            out_l = [s_arc_var[(u, v)] for v in dir_adj[u] if v in sub_set]
                            target_out = 0 if u == e_node else 1
                            if len(out_l) < target_out:
                                continue
                            if target_out == 1:
                                sub_solver.add_clause(out_l)
                                for i in range(len(out_l)):
                                    for j in range(i + 1, len(out_l)):
                                        sub_solver.add_clause([-out_l[i], -out_l[j]])
                            else:
                                for lit in out_l: sub_solver.add_clause([-lit])
                                
                        # In degree: 1 for all except s_node (which has 0 in path)
                        for u in sub_nodes:
                            in_l = [s_arc_var[(v, u)] for v in in_adj[u] if v in sub_set]
                            target_in = 0 if u == s_node else 1
                            if len(in_l) < target_in:
                                continue
                            if target_in == 1:
                                sub_solver.add_clause(in_l)
                                for i in range(len(in_l)):
                                    for j in range(i + 1, len(in_l)):
                                        sub_solver.add_clause([-in_l[i], -in_l[j]])
                            else:
                                for lit in in_l: sub_solver.add_clause([-lit])
                                
                        # Solve with CEGAR for connectedness
                        while sub_solver.solve():
                            m_sub = set(sub_solver.get_model())
                            act_sub = {u: v for u, v in sub_arcs if s_arc_var[(u, v)] in m_sub}
                            
                            # Trace path from s_node
                            path = [s_node]
                            curr = s_node
                            while curr in act_sub:
                                curr = act_sub[curr]
                                path.append(curr)
                                if curr == e_node: break
                                
                            if len(path) == len(sub_nodes):
                                # Full path found! Splice into c_giant:
                                # c_giant up to idx_entry + path + c_giant from idx_exit
                                if idx_exit > idx_entry:
                                    new_giant = c_giant[:idx_entry+1] + path + c_giant[idx_exit:]
                                else:
                                    new_giant = c_giant[idx_exit:idx_entry+1] + path
                                return new_giant
                            else:
                                # Subtour cut
                                p_set = set(path)
                                for node in sub_nodes:
                                    if node not in p_set:
                                        # find cycle containing node
                                        cyc = []; c_curr = node
                                        while c_curr in act_sub and c_curr not in cyc:
                                            cyc.append(c_curr)
                                            c_curr = act_sub[c_curr]
                                        if cyc and c_curr == node:
                                            sub_solver.add_clause([-s_arc_var[(cyc[i], cyc[(i+1)%len(cyc)])] for i in range(len(cyc))])
                                            break
        return None

    # Test on the small cycles
    print("Testing local SAT splicer on first 10 small cycles:")
    absorbed = 0
    t0 = time.time()
    for idx, sc in enumerate(cycles[1:11]):
        res = local_sat_splice(giant, sc, max_window=6)
        if res is not None:
            giant = res
            absorbed += 1
            print(f"  [+] Small cycle {idx} (len {len(sc)}) ABSORBED! Giant now: {len(giant)}/{n_dir}")
        else:
            print(f"  [-] Small cycle {idx} (len {len(sc)}) could not be absorbed with window <= 6")
    print(f"Absorbed {absorbed}/10 in {time.time()-t0:.2f}s!")

if __name__ == "__main__":
    test_local_sat_absorption()
