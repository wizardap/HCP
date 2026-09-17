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

deg2 = sorted([u for u in G if len(G[u]) == 2])
chains = []
v_partner = {}
for v in deg2:
    nbrs = list(G[v])
    u, w = nbrs[0], nbrs[1]
    chains.append((min(u, w), max(u, w)))
    v_partner[u] = w
    v_partner[w] = u

# Contracted graph G_c:
# Nodes are the 3696 endpoints.
# Edges are: the virtual edges (u, w) plus the non-degree-2 edges of G between endpoints.
endpoints = set(v_partner.keys())
contracted_adj = collections.defaultdict(set)
for u in endpoints:
    contracted_adj[u].add(v_partner[u])
    for nxt in G[u]:
        if nxt in endpoints:
            contracted_adj[u].add(nxt)

print(f"Contracted endpoints: {len(endpoints)}")

# Let's inspect the degrees in contracted graph:
degrees = collections.Counter([len(contracted_adj[u]) for u in endpoints])
print(f"Degrees in contracted graph: {dict(degrees)}")

# Now let's find the modular structure:
# Virtual edge quotient graph:
# Each virtual edge e_i = (u_i, w_i) is a super-vertex (0..1847).
# An edge exists between e_i and e_j if there is an external edge between {u_i, w_i} and {u_j, w_j}.
ve_list = sorted(list(set(chains)))
node_to_ve = {}
for idx, (u, w) in enumerate(ve_list):
    node_to_ve[u] = idx
    node_to_ve[w] = idx

ve_adj = collections.defaultdict(set)
ve_edge_count = collections.defaultdict(int)
for idx, (u, w) in enumerate(ve_list):
    for end in [u, w]:
        for nxt in G[end]:
            if nxt in node_to_ve:
                other = node_to_ve[nxt]
                if other != idx:
                    ve_adj[idx].add(other)
                    ve_edge_count[(min(idx, other), max(idx, other))] += 1

print(f"Total virtual edges: {len(ve_list)}")
print(f"Virtual edge quotient graph edges: {len(ve_edge_count)}")

# Can we detect 42 communities of 44 virtual edges each?
# Using community detection / min-cut / spectral / modularity
import networkx as nx
from networkx.algorithms import community

H = nx.Graph()
for (u, v), w in ve_edge_count.items():
    H.add_edge(u, v, weight=w)

print(f"Quotient graph H: nodes={H.number_of_nodes()}, edges={H.number_of_edges()}")

# Let's check connected components of H
comps = list(nx.connected_components(H))
print(f"Connected components of H: {len(comps)}")

# Check if Louvain or Leiden detects 42 or 84 clusters:
comms = community.louvain_communities(H, weight='weight', resolution=1.0)
print(f"Louvain communities at res=1.0: {len(comms)} communities, sizes: {[len(c) for c in comms][:10]}...")

# Try different resolution to get 42 communities
for res in [0.5, 0.8, 1.0, 1.2, 1.5, 2.0, 3.0, 4.0, 5.0]:
    c = community.louvain_communities(H, weight='weight', resolution=res, seed=42)
    sizes = sorted([len(x) for x in c])
    print(f"Res {res}: {len(c)} communities, min={sizes[0]}, max={sizes[-1]}, sample={sizes[:5]}")
