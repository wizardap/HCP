import collections, time, sys
from pysat.solvers import Cadical195

def build_base_instance():
    # Load graph868.col and contract
    with open('/home/ubuntu/HCP/FHCPCS-col/graph868.col') as f:
        edges_raw = []
        n_raw = 0
        for line in f:
            if line.startswith('p '):
                n_raw = int(line.split()[2])
            elif line.startswith('e '):
                p = line.split()
                edges_raw.append((int(p[1]), int(p[2])))
    
    G = collections.defaultdict(list)
    for u, v in edges_raw:
        G[u].append(v)
        G[v].append(u)

    deg2 = sorted([u for u in G if len(G[u]) == 2])
    n_dir = len(deg2)
    node_to_id = {}
    id_to_pair = {}
    pairs = []
    v_partner = {}
    for idx, v in enumerate(deg2):
        u, w = G[v]
        pairs.append((u, w))
        v_partner[u] = w
        v_partner[w] = u

    # 2-coloring
    color = {}
    u0, w0 = pairs[0]
    color[u0] = 0
    color[w0] = 1
    q = [u0, w0]
    while q:
        curr = q.pop()
        curr_c = color[curr]
        vp = v_partner[curr]
        if vp not in color:
            color[vp] = 1 - curr_c
            q.append(vp)
        for nxt in G[curr]:
            if nxt != vp and nxt not in color:
                color[nxt] = 1 - curr_c
                q.append(nxt)

    for idx, (u, w) in enumerate(pairs):
        node_to_id[u] = idx
        node_to_id[w] = idx
        in_v = u if color[u] == 0 else w
        out_v = u if color[u] == 1 else w
        id_to_pair[idx] = (in_v, out_v)

    dir_adj = collections.defaultdict(list)
    arc_set = set()
    for u, c in color.items():
        if c == 1:
            u_id = node_to_id[u]
            for v in sorted(G[u]):
                v_id = node_to_id[v]
                if u_id != v_id:
                    dir_adj[u_id].append(v_id)
                    arc_set.add((u_id, v_id))

    sorted_arcs = sorted(list(arc_set))
    arc_lit_map = {arc: idx + 1 for idx, arc in enumerate(sorted_arcs)}
    in_arcs = collections.defaultdict(list)
    for u, v in arc_set:
        in_arcs[v].append(u)

    clauses = []
    # Degree 1 out
    for u in range(n_dir):
        out_lits = [arc_lit_map[(u, v)] for v in dir_adj[u]]
        clauses.append(out_lits)
        for i in range(len(out_lits)):
            for j in range(i + 1, len(out_lits)):
                clauses.append([-out_lits[i], -out_lits[j]])

    # Degree 1 in
    for v in range(n_dir):
        in_lits = [arc_lit_map[(u, v)] for u in in_arcs[v]]
        clauses.append(in_lits)
        for i in range(len(in_lits)):
            for j in range(i + 1, len(in_lits)):
                clauses.append([-in_lits[i], -in_lits[j]])

    # No 2-cycles
    for u, v in arc_set:
        if u < v and (v, u) in arc_set:
            clauses.append([-arc_lit_map[(u, v)], -arc_lit_map[(v, u)]])

    # Static 3-cycles
    for u in range(n_dir):
        for v in dir_adj[u]:
            for w in dir_adj[v]:
                if w != u and u in dir_adj[w] and u < v and u < w:
                    clauses.append([-arc_lit_map[(u, v)], -arc_lit_map[(v, w)], -arc_lit_map[(w, u)]])

    # Static 4-cycles
    for u in range(n_dir):
        for v in dir_adj[u]:
            for w in dir_adj[v]:
                if w != u:
                    for x in dir_adj[w]:
                        if x != u and x != v and u in dir_adj[x] and u < v and u < w and u < x:
                            clauses.append([-arc_lit_map[(u, v)], -arc_lit_map[(v, w)], -arc_lit_map[(w, x)], -arc_lit_map[(x, u)]])

    return n_dir, dir_adj, in_arcs, arc_lit_map, clauses, id_to_pair

print("Building base instance...")
t0 = time.time()
n_dir, dir_adj, in_arcs, arc_lit_map, base_clauses, id_to_pair = build_base_instance()
print(f"Base instance ready in {time.time() - t0:.2f}s: {len(arc_lit_map)} vars, {len(base_clauses)} clauses.")

# Test solving base instance with standard CaDiCaL
s = Cadical195()
for c in base_clauses: s.add_clause(c)
t1 = time.time()
res = s.solve()
print(f"Solve base instance: res={res} in {time.time() - t1:.3f}s")
s.delete()
