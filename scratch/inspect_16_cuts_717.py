import collections

col_path = "FHCPCS-col/graph717.col"
G = collections.defaultdict(set)
with open(col_path) as f:
    for line in f:
        if line.startswith("e "):
            p = line.split()
            u, v = int(p[1]), int(p[2])
            G[u].add(v)
            G[v].add(u)

cut_nodes = [255, 540, 577, 702, 773, 1016, 1177, 1213, 1389, 1955, 2467, 2609, 2677, 2681, 3358, 3986]
print(f"Number of cut nodes: {len(cut_nodes)}")

# Check edges between these 16 nodes
induced_edges = []
for i, u in enumerate(cut_nodes):
    for v in cut_nodes[i+1:]:
        if v in G[u]:
            induced_edges.append((u, v))
print(f"Induced edges among the 16 cut nodes: {len(induced_edges)} edges -> {induced_edges}")

# What happens if we remove ALL 16 cut nodes from G?
rem_16 = set(G.keys()) - set(cut_nodes)
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
                    vis.add(nbr)
                    q.append(nbr)
        comps.append(c)

print(f"Removing all 16 cut nodes leaves {len(comps)} connected components!")
comp_lens = sorted([len(c) for c in comps], reverse=True)
print(f"Component lengths: {collections.Counter(comp_lens)}")

# For each component, check which of the 16 cut nodes it connects to!
print("\nComponent -> Cut Nodes connections:")
for idx, c in enumerate(sorted(comps, key=len, reverse=True)):
    adj_cuts = set()
    for u in c:
        for nbr in G[u]:
            if nbr in cut_nodes:
                adj_cuts.add(nbr)
    print(f"Comp {idx} (size {len(c)}): connects to {len(adj_cuts)} cut nodes -> {sorted(list(adj_cuts))}")
