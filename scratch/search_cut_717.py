import collections, itertools, time

col_path = "FHCPCS-col/graph717.col"
G = collections.defaultdict(set)
with open(col_path) as f:
    for line in f:
        if line.startswith("e "):
            p = line.split()
            u, v = int(p[1]), int(p[2])
            G[u].add(v)
            G[v].add(u)

N = len(G)
M = sum(len(v) for v in G.values()) // 2
print(f"Loaded graph717: N={N}, M={M}")

# Find articulation points (1-vertex cuts)
visited = set()
tin, low = {}, {}
timer = 0
art_points = set()

def dfs_art(u, p=-1):
    global timer
    visited.add(u)
    tin[u] = low[u] = timer
    timer += 1
    children = 0
    for to in G[u]:
        if to == p:
            continue
        if to in visited:
            low[u] = min(low[u], tin[to])
        else:
            dfs_art(to, u)
            low[u] = min(low[u], low[to])
            if low[to] >= tin[u] and p != -1:
                art_points.add(u)
            children += 1
    if p == -1 and children > 1:
        art_points.add(u)

import sys
sys.setrecursionlimit(20000)
dfs_art(1)
print(f"Articulation points (1-vertex cuts): {len(art_points)}")

nodes_sorted_deg = sorted(G.keys(), key=lambda u: len(G[u]), reverse=True)
top_hubs = [u for u in nodes_sorted_deg if len(G[u]) >= 4]
print(f"Nodes with degree >= 4: {len(top_hubs)}")

found_cuts = []
t0 = time.time()
for u, v in itertools.combinations(top_hubs[:150], 2):
    rem = set(G.keys()) - {u, v}
    start = next(iter(rem))
    vis = {start}
    q = [start]
    for x in q:
        for nbr in G[x]:
            if nbr in rem and nbr not in vis:
                vis.add(nbr)
                q.append(nbr)
    if len(vis) < len(rem):
        rem_unvis = len(rem) - len(vis)
        print(f"FOUND 2-VERTEX CUT: ({u}, {v})! Component sizes: {len(vis)} and {rem_unvis}")
        found_cuts.append((u, v, len(vis), rem_unvis))
        if len(found_cuts) >= 5:
            break

print(f"Search time: {time.time()-t0:.2f}s, found {len(found_cuts)} cuts.")
