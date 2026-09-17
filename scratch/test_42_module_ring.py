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
chains = []
v_partner = {}
for v in deg2:
    u, w = list(G[v])
    chains.append((min(u, w), max(u, w)))
    v_partner[u] = w
    v_partner[w] = u

endpoints = set(v_partner.keys())
print(f"Contracted endpoints: {len(endpoints)}")

# Let's inspect the graph of virtual edges (VE graph):
ve_list = sorted(list(set(chains)))
node_to_ve = {}
for idx, (u, w) in enumerate(ve_list):
    node_to_ve[u] = idx
    node_to_ve[w] = idx

# Count connections between virtual edges
ve_adj = collections.defaultdict(collections.Counter)
for idx, (u, w) in enumerate(ve_list):
    for end in [u, w]:
        for nxt in G[end]:
            if nxt in node_to_ve:
                other = node_to_ve[nxt]
                if other != idx:
                    ve_adj[idx][other] += 1

print(f"Virtual edges: {len(ve_list)}")

# How is the 42 ring structured?
# If each module has 44 VEs (88 endpoints):
# Let's find strongly connected components or biconnected components or cut vertices
# Let's see which external edges connect between different modules.
# Are there specific bottleneck edges or interface ports?
# Let's check the degrees of nodes in the contracted graph:
deg_map = {u: len(G[u]) for u in endpoints}
print(f"Degrees: {collections.Counter(deg_map.values())}")

# Let's check how many external edges each VE has:
ext_counts = []
for idx in range(len(ve_list)):
    u, w = ve_list[idx]
    ext = (len(G[u]) - 1) + (len(G[w]) - 1)
    ext_counts.append(ext)
print(f"VE external edge count distribution: {collections.Counter(ext_counts)}")

