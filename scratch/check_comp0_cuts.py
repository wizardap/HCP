import collections

col_path = "FHCPCS-col/graph717.col"
G = collections.defaultdict(set)
with open(col_path) as f:
    for line in f:
        if line.startswith("e "):
            p = line.split()
            u, v = int(p[1]), int(p[2])
            G[u].add(v); G[v].add(u)

cut_nodes = {255, 540, 577, 702, 773, 1016, 1177, 1213, 1389, 1955, 2467, 2609, 2677, 2681, 3358, 3986}
rem_16 = set(G.keys()) - cut_nodes

# Extract Comp 0
vis = set()
for x in rem_16:
    if x not in vis:
        c = []
        q = [x]
        vis.add(x)
        for y in q:
            c.append(y)
            for nbr in G[y]:
                if nbr in rem_16 and nbr not in vis:
                    vis.add(nbr); q.append(nbr)
        if len(c) > 200:
            comp_0 = set(c)

ports = {255, 1955, 2609, 3358}
V_c0 = comp_0 | ports

# Check internal 2-vertex cuts in V_c0
# We look for pairs (u, v) in V_c0 such that V_c0 \ {u, v} is disconnected
print(f"Checking internal 2-vertex cuts in Comp 0 (|V_c0| = {len(V_c0)})...")
import sys
sys.setrecursionlimit(50000)

adj_c0 = collections.defaultdict(set)
for u in V_c0:
    for v in G[u]:
        if v in V_c0:
            adj_c0[u].add(v)

# Check articulation points in V_c0
visited = set()
tin, low = {}, {}
timer = 0
art = set()
def dfs(u, p=-1):
    global timer
    visited.add(u); tin[u] = low[u] = timer; timer += 1
    children = 0
    for to in adj_c0[u]:
        if to == p: continue
        if to in visited: low[u] = min(low[u], tin[to])
        else:
            dfs(to, u); low[u] = min(low[u], low[to])
            if low[to] >= tin[u] and p != -1: art.add(u)
            children += 1
    if p == -1 and children > 1: art.add(u)

dfs(next(iter(V_c0)))
print(f"Articulation points in Comp 0: {len(art)}")

# Check 2-vertex cuts involving highest degree nodes in Comp 0
degs_c0 = {u: len(adj_c0[u]) for u in V_c0}
high_deg = [u for u in degs_c0 if degs_c0[u] >= 10]
print(f"Nodes in Comp 0 with degree >= 10: {len(high_deg)}")
c0_cuts = []
for h in high_deg:
    rem_h = V_c0 - {h}
    vis_h = set()
    tin_h, low_h = {}, {}
    timer_h = 0
    art_h = set()
    def dfs_h(u, p=-1):
        global timer_h
        vis_h.add(u); tin_h[u] = low_h[u] = timer_h; timer_h += 1
        children = 0
        for to in adj_c0[u]:
            if to == p or to == h: continue
            if to in vis_h: low_h[u] = min(low_h[u], tin_h[to])
            else:
                dfs_h(to, u); low_h[u] = min(low_h[u], low_h[to])
                if low_h[to] >= tin_h[u] and p != -1: art_h.add(u)
                children += 1
        if p == -1 and children > 1: art_h.add(u)
    dfs_h(next(iter(rem_h)))
    for v in art_h:
        c0_cuts.append((h, v))

print(f"Internal 2-vertex cuts in Comp 0 involving degree>=10 nodes: {len(c0_cuts)}")
for u, v in c0_cuts[:5]:
    rem = V_c0 - {u, v}
    vis_c = set()
    comps = []
    for x in rem:
        if x not in vis_c:
            c = []
            q = [x]
            vis_c.add(x)
            for y in q:
                c.append(y)
                for nbr in adj_c0[y]:
                    if nbr in rem and nbr not in vis_c:
                        vis_c.add(nbr); q.append(nbr)
            comps.append(len(c))
    comps.sort()
    print(f"Comp 0 Cut ({u}, {v}): component sizes = {comps}")
