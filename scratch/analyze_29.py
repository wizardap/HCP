import os, sys, collections
from scratch.engine.graph_loader import load_dimacs
from scratch.engine.decomposer import detect_bridge_corridors

graphs = [832, 868, 937, 950, 951, 954, 959, 960, 965, 966, 971, 974, 976, 981, 983, 987, 993, 994, 998]

print(f"{'Graph':<10} | {'|V|':<6} | {'|E|':<7} | {'Deg-2':<6} | {'Deg-14':<6} | {'Corridors':<12} | {'Comp0':<6} | {'Contracted':<10}")
print("-" * 80)

for gid in graphs:
    col_path = f"FHCPCS-col/graph{gid}.col"
    if not os.path.exists(col_path):
        print(f"graph{gid:<5} | NOT FOUND")
        continue
    G = load_dimacs(col_path)
    N = len(G)
    M = sum(len(adj) for adj in G.values()) // 2
    d2 = sum(1 for u in G if len(G[u]) == 2)
    d14 = sum(1 for u in G if len(G[u]) == 14)
    corridors, comp0 = detect_bridge_corridors(G)
    
    # Contract degree-2 in comp0
    adj_c0 = collections.defaultdict(set)
    for u in comp0:
        for v in G[u]:
            if v in comp0:
                adj_c0[u].add(v)
    for corr in corridors:
        ext_u, ext_v = corr['ext_ports']
        adj_c0[ext_u].add(ext_v)
        adj_c0[ext_v].add(ext_u)
    
    ports = set()
    for corr in corridors:
        ports.update(corr['ext_ports'])
        
    rem = set(comp0)
    while True:
        cand = [u for u in rem if u not in ports and len(adj_c0[u]) == 2]
        if not cand:
            break
        v = cand[0]
        nbrs = list(adj_c0[v])
        adj_c0[nbrs[0]].remove(v)
        adj_c0[nbrs[1]].remove(v)
        del adj_c0[v]
        rem.remove(v)
        adj_c0[nbrs[0]].add(nbrs[1])
        adj_c0[nbrs[1]].add(nbrs[0])
        
    corr_str = f"{len(corridors)} (" + ",".join(str(len(c['nodes'])) for c in corridors) + ")" if corridors else "0"
    print(f"graph{gid:<5} | {N:<6} | {M:<7} | {d2:<6} | {d14:<6} | {corr_str:<12} | {len(comp0):<6} | {len(rem):<10}")
