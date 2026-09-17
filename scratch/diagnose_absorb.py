import collections, sys
from pysat.solvers import Cadical195

def diagnose(col_path="FHCPCS-col/graph868.col"):
    G = collections.defaultdict(set)
    with open(col_path) as f:
        for line in f:
            if line.startswith("e "):
                _, u, v = line.split()
                u, v = int(u), int(v)
                G[u].add(v); G[v].add(u)
    
    deg2 = [u for u in G if len(G[u]) == 2]
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
                
    solver = Cadical195(bootstrap_with=static_clauses)
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
    
    print(f"Iter 1 cycles: {len(cycles)}, lengths: {[len(c) for c in cycles[:10]]}")
    
    # Check 2-opt pairs
    giant = cycles[0]
    giant_set = set(giant)
    giant_pos = {v: i for i, v in enumerate(giant)}
    print(f"Giant has {len(giant)} vertices.")
    
    # How many arcs leave giant to small cycles?
    out_to_small = []
    for u in giant:
        for v in dir_adj[u]:
            if v not in giant_set:
                out_to_small.append((u, v))
    print(f"Total arcs from giant to small cycles: {len(out_to_small)}")
    
    # How many arcs enter giant from small cycles?
    in_from_small = []
    for u in giant:
        for v in in_adj[u]:
            if v not in giant_set:
                in_from_small.append((v, u))
    print(f"Total arcs from small cycles to giant: {len(in_from_small)}")
    
    # For each small cycle, check all possible connections to giant
    absorbable_count = 0
    for s_idx, small in enumerate(cycles[1:]):
        s_set = set(small)
        # Check if 2-opt is possible:
        # We need: u1 -> v2 (u1 in giant, v2 in small)
        # AND: u2 -> v1 (u2 in small, v1 in giant)
        # In small: does small have arc from some w2 to v2?
        # Yes, predecessor of v2 in small is u2!
        # If u2 is predecessor of v2, does arc u2 -> v1 exist?
        possible_2opts = []
        for i1, u1 in enumerate(giant):
            v1 = giant[(i1 + 1) % len(giant)]
            for j2, v2 in enumerate(small):
                if v2 in dir_adj[u1]:
                    u2 = small[(j2 - 1 + len(small)) % len(small)]
                    if v1 in dir_adj[u2]:
                        possible_2opts.append((u1, v1, u2, v2))
        if possible_2opts:
            absorbable_count += 1
            print(f"  Small cycle {s_idx} (len {len(small)}) HAS {len(possible_2opts)} 2-opt moves!")
        else:
            # Check 3-opt or path: how close is u2 to giant?
            pass
    print(f"Total small cycles absorbable via 2-opt: {absorbable_count}/{len(cycles)-1}")

if __name__ == "__main__":
    diagnose()
