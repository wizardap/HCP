import collections

with open('FHCPCS-col/graph868.col') as f:
    edges = []
    n = 0
    for line in f:
        if line.startswith('p '):
            p = line.split()
            n = int(p[2])
        elif line.startswith('e '):
            p = line.split()
            edges.append((int(p[1]), int(p[2])))

G = collections.defaultdict(set)
for u, v in edges:
    G[u].add(v)
    G[v].add(u)

print(f"Raw graph: N={n}, M={len(edges)}")

# Degree-2 contraction
deg2 = [u for u in G if len(G[u]) == 2]
print(f"Degree-2 vertices: {len(deg2)}")

# Contract degree-2 vertices
contracted_G = collections.defaultdict(set)
chains = []
for v in deg2:
    nbrs = list(G[v])
    u, w = nbrs[0], nbrs[1]
    chains.append((min(u, w), max(u, w)))

print(f"Total degree-2 chains (virtual edges): {len(chains)}")
# 1848 virtual edges * 2 = 3696 endpoints
contracted_nodes = set()
for u, w in chains:
    contracted_nodes.add(u)
    contracted_nodes.add(w)
print(f"Contracted nodes count: {len(contracted_nodes)}")

# Let's check 42:
# 1848 / 42 = 44 virtual edges per module!
# 3696 / 42 = 88 nodes per module!
print(f"1848 / 42 = {1848 / 42} virtual edges per module")
print(f"3696 / 42 = {3696 / 42} contracted vertices per module")
