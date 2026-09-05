import collections

# Let's inspect graph479.col directed representation!
adj = collections.defaultdict(set)
with open('FHCPCS-col/graph479.col') as f:
    for line in f:
        if line.startswith('e '):
            p = line.split()
            u, v = int(p[1]), int(p[2])
            adj[u].add(v)
            adj[v].add(u)

deg2 = {v for v, nbrs in adj.items() if len(nbrs) == 2}
c_nodes = set(adj.keys()) - deg2
c_adj = collections.defaultdict(set)
for u in c_nodes:
    for v in adj[u]:
        if v in c_nodes: c_adj[u].add(v)

chain_map = {}
for v in deg2:
    nbrs = list(adj[v])
    if len(nbrs) == 2:
        u, w = min(nbrs), max(nbrs)
        chain_map[(u, w)] = v
        c_adj[u].add(w)
        c_adj[w].add(u)

v_partner = {}
for (u, w) in chain_map.keys():
    v_partner[u] = w
    v_partner[w] = u

print(f"Graph 479: Raw N={len(adj)}, Deg2={len(deg2)}, Contracted N={len(c_nodes)}, Virtual pairs={len(chain_map)}")

# Check 2-coloring bipartite
color = {}
pairs = list(chain_map.keys())
u0, w0 = pairs[0]
color[u0] = 0
color[w0] = 1
q = [u0, w0]
bip = True
while q:
    curr = q.pop(0)
    curr_c = color[curr]
    vp = v_partner[curr]
    if vp not in color:
        color[vp] = 1 - curr_c
        q.append(vp)
    for nxt in c_adj[curr]:
        if nxt != vp:
            if nxt not in color:
                color[nxt] = 1 - curr_c
                q.append(nxt)
            elif color[nxt] == curr_c:
                bip = False
                break

print(f"Contracted Graph 479 is 100% strictly bipartite: {bip}")
