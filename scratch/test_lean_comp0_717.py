import collections, time
from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType

col_path = "FHCPCS-col/graph717.col"
G = collections.defaultdict(set)
with open(col_path) as f:
    for line in f:
        if line.startswith("e "):
            p = line.split()
            u, v = int(p[1]), int(p[2])
            G[u].add(v); G[v].add(u)

cut_nodes = {255, 540, 577, 702, 773, 1016, 1177, 1213, 1389, 1955, 2467, 2609, 2677, 2681, 3358, 3986}
rem_16 = set(G.keys()) - cut_nodes

# Extract Comp 0
vis = set()
for x in rem_16:
    if x not in vis:
        c = []
        q = [x]
        vis.add(x)
        for y in q:
            c.append(y)
            for nbr in G[y]:
                if nbr in rem_16 and nbr not in vis:
                    vis.add(nbr); q.append(nbr)
        if len(c) > 200:
            comp_0 = set(c)

ports = {255, 1955, 2609, 3358}
V_c0 = comp_0 | ports

adj_c0 = collections.defaultdict(set)
for u in V_c0:
    for v in G[u]:
        if v in V_c0:
            adj_c0[u].add(v)

# Add virtual edges
adj_c0[255].add(1955); adj_c0[1955].add(255)
adj_c0[3358].add(2609); adj_c0[2609].add(3358)

# Degree-2 contraction
rem = set(V_c0)
edge_chains = {}
while True:
    d2 = [u for u in rem if u not in ports and len(adj_c0[u]) == 2]
    if not d2:
        break
    v = d2[0]
    u, w = list(adj_c0[v])
    adj_c0[u].remove(v); adj_c0[w].remove(v)
    del adj_c0[v]; rem.remove(v)
    e_uv = tuple(sorted([u, v]))
    e_vw = tuple(sorted([v, w]))
    chain_uv = edge_chains.pop(e_uv, [u, v])
    chain_vw = edge_chains.pop(e_vw, [v, w])
    if chain_uv[-1] != v: chain_uv = list(reversed(chain_uv))
    if chain_vw[0] != v: chain_vw = list(reversed(chain_vw))
    merged_chain = chain_uv[:-1] + chain_vw
    e_uw = tuple(sorted([u, w]))
    adj_c0[u].add(w); adj_c0[w].add(u)
    edge_chains[e_uw] = merged_chain

contracted_edges = set(edge_chains.keys())
edges = set()
for u in rem:
    for v in adj_c0[u]:
        if u < v:
            edges.add((u, v))
edge_list = sorted(list(edges))
edge_to_var = {e: i + 1 for i, e in enumerate(edge_list)}
inc_edges = collections.defaultdict(list)
for e in edge_list:
    inc_edges[e[0]].append(edge_to_var[e])
    inc_edges[e[1]].append(edge_to_var[e])

solver = Cadical195()
top = len(edge_list) + 1

for u in rem:
    lits = inc_edges[u]
    clauses = CardEnc.equals(lits=lits, bound=2, top_id=top, encoding=EncType.cardnetwrk)
    for cl in clauses:
        solver.add_clause(cl)
        for lit in cl:
            top = max(top, abs(lit) + 1)

virt1 = tuple(sorted([255, 1955]))
virt2 = tuple(sorted([3358, 2609]))
solver.add_clause([edge_to_var[virt1]])
solver.add_clause([edge_to_var[virt2]])

for ce in contracted_edges:
    solver.add_clause([edge_to_var[ce]])

# 376 static triangle cuts
triangles = []
rem_list = sorted(list(rem))
for u in rem_list:
    for v in adj_c0[u]:
        if v > u:
            for w in adj_c0[v]:
                if w > v and w in adj_c0[u]:
                    triangles.append((u, v, w))
for u, v, w in triangles:
    solver.add_clause([-edge_to_var[tuple(sorted([u, v]))], -edge_to_var[tuple(sorted([v, w]))], -edge_to_var[tuple(sorted([w, u]))]])

# 517 static 4-cycle cuts
squares = set()
for a in rem_list:
    nbrs_a = sorted(list(adj_c0[a]))
    for i in range(len(nbrs_a)):
        u = nbrs_a[i]
        for j in range(i+1, len(nbrs_a)):
            v = nbrs_a[j]
            common = [w for w in adj_c0[u] if w != a and w in adj_c0[v]]
            for w in common:
                if w not in adj_c0[a] and v not in adj_c0[u]:
                    # 4-cycle is a - u - w - v - a
                    e1 = tuple(sorted([a, u]))
                    e2 = tuple(sorted([u, w]))
                    e3 = tuple(sorted([w, v]))
                    e4 = tuple(sorted([v, a]))
                    sq_key = tuple(sorted([e1, e2, e3, e4]))
                    if sq_key not in squares:
                        squares.add(sq_key)
                        solver.add_clause([-edge_to_var[e1], -edge_to_var[e2], -edge_to_var[e3], -edge_to_var[e4]])

print(f"Added {len(triangles)} triangles and {len(squares)} squares. Starting LEAN CEGAR...")
t0 = time.time()
it = 0
while True:
    it += 1
    t_start_it = time.time()
    if not solver.solve():
        print("UNSAT!")
        break
    model = set(solver.get_model())
    active = [e for e in edge_list if edge_to_var[e] in model]
    adj = collections.defaultdict(list)
    for u, v in active:
        adj[u].append(v); adj[v].append(u)

    visited = set()
    cycles = []
    for u in rem:
        if u not in visited:
            cyc = []
            curr, prev = u, None
            while curr not in visited:
                visited.add(curr); cyc.append(curr)
                nbrs = adj[curr]
                nxt = nbrs[0] if nbrs[0] != prev else nbrs[1]
                prev, curr = curr, nxt
            cycles.append(cyc)

    if len(cycles) == 1:
        print(f"CONVERGED at iter {it} in {time.time()-t0:.2f}s! ({len(cycles[0])} vertices)")
        break

    if it % 5 == 0 or len(cycles) <= 10:
        cyc_lens = sorted([len(c) for c in cycles], reverse=True)
        print(f"Iter {it} ({time.time()-t_start_it:.2f}s): {len(cycles)} cycles. Max: {cyc_lens[0]}, Min: {cyc_lens[-1]}")

    # PURE NEGATIVE CLAUSES (DFK cuts): zero auxiliary variables!
    for cyc in cycles:
        neg_clause = [-edge_to_var[tuple(sorted([cyc[i], cyc[(i+1)%len(cyc)]]))] for i in range(len(cyc))]
        solver.add_clause(neg_clause)

    if it >= 150:
        print("Iter limit 150 reached.")
        break
