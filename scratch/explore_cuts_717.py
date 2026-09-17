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

# What is the degree of 3358?
print(f"Degree of 3358: {len(G[3358])}")

# Remove 3358 and find articulation points in G \ {3358}
rem_3358 = set(G.keys()) - {3358}
# Every articulation point in G \ {3358} forms a 2-vertex cut with 3358!
visited = set()
tin, low = {}, {}
timer = 0
art_points_in_rem = set()

def dfs_art_rem(u, p=-1):
    global timer
    visited.add(u)
    tin[u] = low[u] = timer
    timer += 1
    children = 0
    for to in G[u]:
        if to == p or to == 3358:
            continue
        if to in visited:
            low[u] = min(low[u], tin[to])
        else:
            dfs_art_rem(to, u)
            low[u] = min(low[u], low[to])
            if low[to] >= tin[u] and p != -1:
                art_points_in_rem.add(u)
            children += 1
    if p == -1 and children > 1:
        art_points_in_rem.add(u)

import sys
sys.setrecursionlimit(20000)
start_node = next(iter(rem_3358))
dfs_art_rem(start_node)
print(f"Number of articulation points in G \\ {{3358}}: {len(art_points_in_rem)}")

# For each articulation point v, what are the component sizes when {3358, v} is removed?
for v in sorted(list(art_points_in_rem))[:15]:
    cut = {3358, v}
    rem = set(G.keys()) - cut
    vis = set()
    comps = []
    for u in rem:
        if u not in vis:
            c = []
            q = [u]
            vis.add(u)
            for x in q:
                c.append(x)
                for nbr in G[x]:
                    if nbr in rem and nbr not in vis:
                        vis.add(nbr)
                        q.append(nbr)
            comps.append(len(c))
    comps.sort()
    print(f"Cut {{3358, {v}}}: deg(3358)={len(G[3358])}, deg({v})={len(G[v])}, components: {comps}")
