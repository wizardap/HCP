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

# Find degree-2 vertices
deg2 = sorted([u for u in G if len(G[u]) == 2])
v_partner = {}
for v in deg2:
    u, w = list(G[v])
    v_partner[u] = w
    v_partner[w] = u

endpoints = set(v_partner.keys())
print(f"Contracted endpoints: {len(endpoints)}")

# In Flinders graphs (Class 1), each module is connected to its neighbors.
# Let's inspect the degree distribution of endpoints:
deg_map = {u: len(G[u]) for u in endpoints}
# In endpoints: degree 3, 4, 5
# Notice: which edges connect DIFFERENT modules?
# In Flinders graphs, modules are connected via "ring" or "interface" edges.
# Can we identify the 42 modules by looking at the BFS / distance structure or the degree 2 vertices?
# Let's inspect the indices of deg2 vertices:
print(f"Sample deg2 vertices: {deg2[:20]}")
# Are deg2 vertices clustered?
diffs = [deg2[i+1] - deg2[i] for i in range(len(deg2)-1)]
print(f"Differences between consecutive deg2 vertices: {collections.Counter(diffs)}")

