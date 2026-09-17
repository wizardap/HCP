import collections, os, sys

graphs = ["graph717", "graph882", "graph937", "graph944", "graph954", "graph959", "graph965", "graph966", "graph971", "graph994", "graph998"]

print(f"{'Graph':<10} {'N':<6} {'M':<7} {'#ArtPts':<9} {'#Deg14':<8} {'#2-Cuts Found':<14} {'Sample Cut Comps'}")
print("-" * 75)

sys.setrecursionlimit(50000)

for g_name in graphs:
    col_path = f"FHCPCS-col/{g_name}.col"
    if not os.path.exists(col_path):
        continue
    G = collections.defaultdict(set)
    with open(col_path) as f:
        for line in f:
            if line.startswith("e "):
                p = line.split()
                u, v = int(p[1]), int(p[2])
                G[u].add(v); G[v].add(u)
    N = len(G)
    M = sum(len(v) for v in G.values()) // 2
    degs = {u: len(G[u]) for u in G}
    deg14_nodes = [u for u in degs if degs[u] == 14]
    
    # Check 1-vertex cuts
    visited = set()
    tin, low = {}, {}
    timer = 0
    art = set()
    def dfs(u, p=-1):
        global timer
        visited.add(u); tin[u] = low[u] = timer; timer += 1
        children = 0
        for to in G[u]:
            if to == p: continue
            if to in visited: low[u] = min(low[u], tin[to])
            else:
                dfs(to, u); low[u] = min(low[u], low[to])
                if low[to] >= tin[u] and p != -1: art.add(u)
                children += 1
        if p == -1 and children > 1: art.add(u)
    dfs(next(iter(G)))

    # Search for 2-vertex cuts among deg14 nodes
    cuts = []
    for h in deg14_nodes[:20]:
        rem_h = set(G.keys()) - {h}
        visited_h = set()
        tin_h, low_h = {}, {}
        timer_h = 0
        art_h = set()
        def dfs_h(u, p=-1):
            global timer_h
            visited_h.add(u); tin_h[u] = low_h[u] = timer_h; timer_h += 1
            children = 0
            for to in G[u]:
                if to == p or to == h: continue
                if to in visited_h: low_h[u] = min(low_h[u], tin_h[to])
                else:
                    dfs_h(to, u); low_h[u] = min(low_h[u], low_h[to])
                    if low_h[to] >= tin_h[u] and p != -1: art_h.add(u)
                    children += 1
            if p == -1 and children > 1: art_h.add(u)
        dfs_h(next(iter(rem_h)))
        for v in art_h:
            cuts.append((h, v))
            if len(cuts) >= 3:
                break
        if len(cuts) >= 3:
            break

    sample_comps = "None"
    if cuts:
        u, v = cuts[0]
        rem = set(G.keys()) - {u, v}
        vis = set()
        comps = []
        for x in rem:
            if x not in vis:
                c = []
                q = [x]
                vis.add(x)
                for y in q:
                    c.append(y)
                    for nbr in G[y]:
                        if nbr in rem and nbr not in vis:
                            vis.add(nbr); q.append(nbr)
                comps.append(len(c))
        comps.sort()
        sample_comps = f"Cut ({u},{v}): {comps}"

    print(f"{g_name:<10} {N:<6} {M:<7} {len(art):<9} {len(deg14_nodes):<8} {len(cuts):<14} {sample_comps}")
