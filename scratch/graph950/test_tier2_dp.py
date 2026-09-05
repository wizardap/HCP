
from scratch.graph950.two_half_two_tier_solver import load_graph, partition_halves, decompose_half
import collections, json, time

G, degs = load_graph("FHCPCS-col/graph950.col")
grp1, grp2 = partition_halves(G, degs)
all_hubs, strips, hh_edges, strip_adj_hubs, hub_adj_strips = decompose_half(G, degs, grp1)

with open("scratch/graph950/feasible_cluster_pairs.json") as f:
    cluster_pairs = {int(k): [tuple(p) for p in v] for k, v in json.load(f).items()}

group_configs = {
    164: {"clusters": [18, 2, 3, 15, 6], "tiny": [27, 31]},
    4785: {"clusters": [1, 8, 7, 9, 16], "tiny": [30, 33]},
    5787: {"clusters": [0, 21, 17, 5, 24], "tiny": [28, 35]},
    4000: {"clusters": [10, 4, 23, 19, 11], "tiny": [25, 32]},
    5835: {"clusters": [12, 13, 20, 14, 22], "tiny": [26, 36]},
}

print("Testing Bitmask DP inside each of the 5 groups:")
for sh, cfg in group_configs.items():
    t0 = time.time()
    cl_list = cfg["clusters"]
    # 5 clusters: 0 to 4 in local index
    # Total local entities: 5 clusters + 2 tiny strips = 7 entities
    # 2^7 = 128 states!
    # Let us check all simple paths through all 7 entities:
    local_ents = [("C", ci) for ci in cl_list] + [("T", ti) for ti in cfg["tiny"]]
    N_loc = len(local_ents) # 7
    
    ent_map = {}
    for ci in cl_list:
        for u in strips[ci]: ent_map[u] = ("C", ci)
        for h in strip_adj_hubs[ci]:
            if degs[h] != 662: ent_map[h] = ("C", ci)
    for ti in cfg["tiny"]:
        for u in strips[ti]: ent_map[u] = ("T", ti)
        
    # Feasible pairs:
    feas = {}
    for ci in cl_list:
        prs = []
        for p in cluster_pairs[ci]:
            prs.append((p[0], p[1])); prs.append((p[1], p[0]))
        feas[("C", ci)] = prs
    for ti in cfg["tiny"]:
        s_v = strips[ti]
        if len(s_v) == 2:
            feas[("T", ti)] = [(s_v[0], s_v[1]), (s_v[1], s_v[0])]
        else:
            feas[("T", ti)] = [(s_v[0], s_v[1]), (s_v[1], s_v[0]), (s_v[0], s_v[2]), (s_v[2], s_v[0]), (s_v[1], s_v[2]), (s_v[2], s_v[1])]
            
    # Edges between local entities:
    local_adj = collections.defaultdict(list)
    for u in ent_map:
        eu = ent_map[u]
        for v in G[u]:
            if v in ent_map and ent_map[v] != eu:
                local_adj[u].append(v)
                
    # Bitmask DP:
    # State: (mask, curr_exit_port, curr_ent_idx)
    # Target: mask == (1 << N_loc) - 1
    # Find all (entry_port, exit_port) pairs for the group:
    valid_bulk_paths = collections.defaultdict(list)
    
    for start_idx, start_ent in enumerate(local_ents):
        for p_in, p_out in feas[start_ent]:
            # Run DP or DFS from (1 << start_idx, p_out)
            memo = set()
            def dfs_loc(mask, curr_p, trace):
                if mask == (1 << N_loc) - 1:
                    valid_bulk_paths[(p_in, curr_p)].append(trace)
                    return
                state = (mask, curr_p)
                if state in memo: return
                memo.add(state)
                for nxt_p in local_adj[curr_p]:
                    nxt_ent = ent_map[nxt_p]
                    nxt_idx = local_ents.index(nxt_ent)
                    if not (mask & (1 << nxt_idx)):
                        for np_in, np_out in feas[nxt_ent]:
                            if np_in == nxt_p:
                                dfs_loc(mask | (1 << nxt_idx), np_out, trace + [(curr_p, nxt_p, nxt_ent)])
            dfs_loc(1 << start_idx, p_out, [(p_in, start_ent)])
            
    print(f"Group {sh}: Bitmask DP found {len(valid_bulk_paths)} feasible (entry, exit) pairs in {time.time()-t0:.3f}s:")
    for (p1, p2), paths in sorted(valid_bulk_paths.items()):
        print(f"   {p1} -> {p2}: {len(paths)} valid sequence(s)")
