import collections

with open('FHCPCS-col/graph868.col') as f:
    edges = []
    n = 0
    for line in f:
        if line.startswith('p '):
            n = int(line.split()[2])
        elif line.startswith('e '):
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

chains = sorted(list(set([(min(u, w), max(u, w)) for u, w in v_partner.items()])))
node_to_ve = {}
for idx, (u, w) in enumerate(chains):
    node_to_ve[u] = idx
    node_to_ve[w] = idx

# Let's count common neighbors between VEs
# A VE pair (i, j) that are inside the SAME module will have strong connection
ve_weight = collections.defaultdict(int)
for idx, (u, w) in enumerate(chains):
    for end in [u, w]:
        for nxt in G[end]:
            if nxt in node_to_ve:
                other = node_to_ve[nxt]
                if other > idx:
                    ve_weight[(idx, other)] += 1

# Let's see: what are the weights?
print(f"Weight distribution between VE pairs: {collections.Counter(ve_weight.values())}")

# If we only keep edges with weight >= 2 or higher:
adj_heavy = collections.defaultdict(set)
for (i, j), w in ve_weight.items():
    if w >= 2:
        adj_heavy[i].add(j)
        adj_heavy[j].add(i)

# Connected components of heavy edges:
visited = set()
comps = []
for i in range(len(chains)):
    if i not in visited:
        comp = []
        q = [i]
        visited.add(i)
        while q:
            curr = q.pop()
            comp.append(curr)
            for nxt in adj_heavy[curr]:
                if nxt not in visited:
                    visited.add(nxt)
                    q.append(nxt)
        comps.append(comp)

comp_lens = [len(c) for c in comps]
print(f"Connected components with weight >= 2: {len(comps)}, sizes: {collections.Counter(comp_lens)}")

