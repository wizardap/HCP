import collections

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
comps = []
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
print(f"Comp 0 total vertices (including 4 ports): {len(V_c0)}")

# Check induced degrees inside V_c0
adj_c0 = collections.defaultdict(set)
for u in V_c0:
    for v in G[u]:
        if v in V_c0:
            adj_c0[u].add(v)

# Add virtual edges
adj_c0[255].add(1955); adj_c0[1955].add(255)
adj_c0[3358].add(2609); adj_c0[2609].add(3358)

degs_c0 = [len(adj_c0[u]) for u in V_c0]
counter = collections.Counter(degs_c0)
print(f"Degree distribution in Comp 0 + virtual edges: {sorted(counter.items())}")

# Degree-2 contraction in Comp 0
rem = set(V_c0)
edge_chains = {}
while True:
    d2 = [u for u in rem if u not in ports and len(adj_c0[u]) == 2]
    if not d2:
        break
    v = d2[0]
    u, w = list(adj_c0[v])
    adj_c0[u].remove(v)
    adj_c0[w].remove(v)
    del adj_c0[v]
    rem.remove(v)
    e_uv = tuple(sorted([u, v]))
    e_vw = tuple(sorted([v, w]))
    chain_uv = edge_chains.pop(e_uv, [u, v])
    chain_vw = edge_chains.pop(e_vw, [v, w])
    if chain_uv[-1] != v:
        chain_uv = list(reversed(chain_uv))
    if chain_vw[0] != v:
        chain_vw = list(reversed(chain_vw))
    merged_chain = chain_uv[:-1] + chain_vw
    e_uw = tuple(sorted([u, w]))
    adj_c0[u].add(w)
    adj_c0[w].add(u)
    edge_chains[e_uw] = merged_chain

print(f"After degree-2 contraction: {len(V_c0)} -> {len(rem)} vertices ({len(edge_chains)} contracted chains)!")
degs_rem = [len(adj_c0[u]) for u in rem]
print(f"Contracted Comp 0 degree distribution: {sorted(collections.Counter(degs_rem).items())}")

# Check chordless triangles in contracted Comp 0
triangles = []
rem_list = sorted(list(rem))
for u in rem_list:
    for v in adj_c0[u]:
        if v > u:
            for w in adj_c0[v]:
                if w > v and w in adj_c0[u]:
                    triangles.append((u, v, w))
print(f"Chordless triangles in contracted Comp 0: {len(triangles)}")
