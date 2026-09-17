import collections

bench_graphs = [
    710, 717, 746, 788, 832, 868, 882, 937, 944, 950,
    951, 954, 959, 960, 963, 965, 966, 971, 974, 975,
    976, 981, 982, 983, 987, 990, 993, 994, 998
]

for g_id in bench_graphs:
    path = f"FHCPCS-col/graph{g_id}.col"
    G = collections.defaultdict(set)
    try:
        with open(path) as f:
            for line in f:
                if line.startswith("e "):
                    _, u, v = line.split()
                    u, v = int(u), int(v)
                    G[u].add(v); G[v].add(u)
    except Exception as e:
        print(f"graph{g_id}: error opening: {e}")
        continue
    
    nv = len(G)
    deg2 = [u for u in G if len(G[u]) == 2]
    deg3 = [u for u in G if len(G[u]) == 3]
    other = [u for u in G if len(G[u]) not in (2, 3)]
    
    is_3block = (len(deg2) * 3 == nv and len(other) == 0)
    
    # Check if bipartite 3-block
    bipartite_status = "N/A"
    if is_3block:
        v_partner = {}
        for v in deg2:
            u, w = list(G[v])
            v_partner[u] = w
            v_partner[w] = u
        color = {}
        q = [list(deg2)[0]] # start from first deg2 neighbor
        u0, w0 = list(G[list(deg2)[0]])
        color[u0] = 0; color[w0] = 1
        q = [u0, w0]
        bip = True
        while q:
            curr = q.pop()
            c = color[curr]
            vp = v_partner[curr]
            if vp not in color:
                color[vp] = 1 - c
                q.append(vp)
            elif color[vp] == c:
                bip = False
                break
            for nxt in G[curr]:
                if nxt != vp and len(G[nxt]) == 3:
                    if nxt not in color:
                        color[nxt] = 1 - c
                        q.append(nxt)
                    elif color[nxt] == c:
                        bip = False
                        break
            if not bip:
                break
        bipartite_status = "Bipartite" if bip else "Non-Bipartite"
        
    print(f"graph{g_id:3d}: |V|={nv:5d}, deg2={len(deg2):4d}, deg3={len(deg3):4d}, other={len(other):3d}, 3-block={is_3block} ({bipartite_status})")
