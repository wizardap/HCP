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

# Let us verify the two chains and Comp 0
# Chain 1 nodes:
# [255] -edge- [1389] -Comp2- [2677] -edge- [1213] -Comp4- [2681] -edge- [773] -Comp6- [702] -edge- [1955]
# Chain 2 nodes:
# [3358] -edge- [3986] -Comp1- [1016] -edge- [1177] -Comp5- [2467] -edge- [577] -Comp3- [540] -edge- [2609]

print("Verifying induced edges:")
print("(255, 1389) in G:", 1389 in G[255])
print("(2677, 1213) in G:", 1213 in G[2677])
print("(2681, 773) in G:", 773 in G[2681])
print("(702, 1955) in G:", 1955 in G[702])

print("(3358, 3986) in G:", 3986 in G[3358])
print("(1016, 1177) in G:", 1177 in G[1016])
print("(2467, 577) in G:", 577 in G[2467])
print("(540, 2609) in G:", 2609 in G[540])

# Check degrees of ports into each 167-vertex module
cut_nodes = {255, 540, 577, 702, 773, 1016, 1177, 1213, 1389, 1955, 2467, 2609, 2677, 2681, 3358, 3986}
rem_16 = set(G.keys()) - cut_nodes

# Re-extract components
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
        comps.append(set(c))

comp_map = {}
for c in comps:
    if len(c) == 3104:
        comp_0 = c
    else:
        # find the 2 cut nodes
        adj_cuts = tuple(sorted([nbr for u in c for nbr in G[u] if nbr in cut_nodes]))
        adj_cuts_set = tuple(sorted(list(set(adj_cuts))))
        comp_map[adj_cuts_set] = c

print("\nFor each 167-node module, check boundary port edges:")
for (u, v), mod in comp_map.items():
    u_edges = [nbr for nbr in G[u] if nbr in mod]
    v_edges = [nbr for nbr in G[v] if nbr in mod]
    print(f"Module between {u} and {v}: size={len(mod)}, edges from {u} into mod={len(u_edges)}, from {v} into mod={len(v_edges)}")

print("\nFor Comp 0 (size 3104), check edges to the 4 interface cut nodes (255, 1955, 2609, 3358):")
for u in [255, 1955, 2609, 3358]:
    edges_into_c0 = [nbr for nbr in G[u] if nbr in comp_0]
    print(f"Node {u}: degree in G={len(G[u])}, edges into Comp 0={len(edges_into_c0)}")
