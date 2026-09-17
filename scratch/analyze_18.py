import glob, collections

targets = [832, 868, 937, 951, 954, 959, 960, 965, 966, 971, 974, 976, 981, 983, 987, 993, 994, 998]

def analyze(gid):
    path = f"FHCPCS-col/graph{gid}.col"
    adj = collections.defaultdict(set)
    with open(path) as f:
        for line in f:
            if line.startswith("e "):
                _, u, v = line.split()
                u, v = int(u), int(v)
                adj[u].add(v); adj[v].add(u)
    nv = len(adj)
    ne = sum(len(x) for x in adj.values()) // 2
    d2 = sum(1 for v, nbrs in adj.items() if len(nbrs) == 2)
    max_d = max(len(nbrs) for nbrs in adj.values())
    
    contracted_v = nv - d2
    
    # Check triangles
    tri_sample = 0
    verts = list(adj.keys())[:200]
    for u in verts:
        for v in adj[u]:
            if v > u:
                tri_sample += len(adj[u] & adj[v])
                
    # Hub and non-hub components
    hubs = {v for v, nbrs in adj.items() if len(nbrs) == max_d and max_d >= 10}
    if hubs:
        visited = set(hubs)
        comps = []
        for v in adj:
            if v not in visited:
                comp = []
                q = [v]
                visited.add(v)
                for curr in q:
                    comp.append(curr)
                    for nxt in adj[curr]:
                        if nxt not in visited:
                            visited.add(nxt)
                            q.append(nxt)
                comps.append(len(comp))
        comps.sort(reverse=True)
        small_corridors = [c for c in comps if c < 600]
        corridor_info = f"Hubs={len(hubs)} (d={max_d}), comps={comps[:4]}, small_corridors={len(small_corridors)} (total {sum(small_corridors)}v)"
    else:
        corridor_info = f"Max deg={max_d} (Dense/Regular)"
        
    return nv, ne, d2, contracted_v, tri_sample, corridor_info

header = f"{'Graph':<9} {'|V|':<6} {'|E|':<7} {'Deg2':<6} {'|Vc|':<6} {'Tri':<5} {'Structure'}"
print(header)
print("-" * len(header) + "-" * 40)
for gid in targets:
    nv, ne, d2, vc, tri, info = analyze(gid)
    print(f"graph{gid:<4} {nv:<6} {ne:<7} {d2:<6} {vc:<6} {tri:<5} {info}")
