import collections, sys
sys.setrecursionlimit(50000)

col_path = "FHCPCS-col/graph717.col"
G = collections.defaultdict(set)
with open(col_path) as f:
    for line in f:
        if line.startswith("e "):
            p = line.split()
            u, v = int(p[1]), int(p[2])
            G[u].add(v)
            G[v].add(u)

degs = {u: len(G[u]) for u in G}
high_deg = [u for u in degs if degs[u] >= 10]
print(f"Nodes with degree >= 10: {len(high_deg)}")

all_cuts = set()
for h in high_deg:
    rem_h = set(G.keys()) - {h}
    visited = set()
    tin, low = {}, {}
    timer = 0
    art = set()

    def dfs(u, p=-1):
        global timer
        visited.add(u)
        tin[u] = low[u] = timer
        timer += 1
        children = 0
        for to in G[u]:
            if to == p or to == h:
                continue
            if to in visited:
                low[u] = min(low[u], tin[to])
            else:
                dfs(to, u)
                low[u] = min(low[u], low[to])
                if low[to] >= tin[u] and p != -1:
                    art.add(u)
                children += 1
        if p == -1 and children > 1:
            art.add(u)

    start = next(iter(rem_h))
    dfs(start)
    for v in art:
        cut = tuple(sorted([h, v]))
        all_cuts.add(cut)

print(f"Total unique 2-vertex cuts involving at least one degree >= 10 node: {len(all_cuts)}")
for u, v in sorted(list(all_cuts)):
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
                        vis.add(nbr)
                        q.append(nbr)
            comps.append(len(c))
    comps.sort()
    print(f"Cut ({u}, {v}) [deg: {degs[u]}, {degs[v]}]: comps = {comps}")
