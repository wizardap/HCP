import collections

with open('FHCPCS-col/graph868.col') as f:
    edges = []
    for line in f:
        if line.startswith('e '):
            p = line.split()
            edges.append((int(p[1]), int(p[2])))

G = collections.defaultdict(set)
for u, v in edges:
    G[u].add(v)
    G[v].add(u)

deg2 = sorted([u for u in G if len(G[u]) == 2])
v_partner = {}
for v in deg2:
    u, w = list(G[v])
    v_partner[u] = w
    v_partner[w] = u

# Contracted vertices
endpoints = set(v_partner.keys())
G_c = collections.defaultdict(set)
for u in endpoints:
    for v in G[u]:
        if v in endpoints:
            G_c[u].add(v)

# Degree 2 in G_c: len(G_c[u]) == 2
d2 = [u for u in endpoints if len(G_c[u]) == 2]

# Gadget connectivity:
gadget_adj = collections.defaultdict(set)
for u in d2:
    for v in G_c[u]:
        gadget_adj[u].add(v)
        gadget_adj[v].add(u)
for u, w in v_partner.items():
    gadget_adj[u].add(w)
    gadget_adj[w].add(u)

visited = set()
gadgets = []
for u in sorted(endpoints):
    if u not in visited:
        comp = []
        q = [u]
        visited.add(u)
        for curr in q:
            comp.append(curr)
            for nxt in gadget_adj[curr]:
                if nxt not in visited:
                    visited.add(nxt)
                    q.append(nxt)
        gadgets.append(sorted(comp))

print(f"Extracted {len(gadgets)} gadgets, all size 22: {all(len(g)==22 for g in gadgets)}")

# Map node to gadget
node_to_g = {}
for gid, g in enumerate(gadgets):
    for u in g:
        node_to_g[u] = gid

# Inter-gadget edge weights:
inter_g_edges = collections.defaultdict(int)
for u in endpoints:
    g1 = node_to_g[u]
    for v in G_c[u]:
        g2 = node_to_g[v]
        if g1 < g2:
            inter_g_edges[(g1, g2)] += 1

weights = collections.Counter(inter_g_edges.values())
print(f"Inter-gadget edge weights: {weights}")

# Look at the inter-gadget graph with edge weight >= 2 or highest weights
# What are the weights?
for w in sorted(weights.keys(), reverse=True):
    print(f"Weight {w}: count={weights[w]}")
