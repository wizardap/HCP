import collections, time, os, sys, pickle
from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType

def load_graph(path):
    G = collections.defaultdict(set)
    with open(path, 'r') as f:
        for line in f:
            if line.startswith('e '):
                p = line.split()
                u, v = int(p[1]), int(p[2])
                G[u].add(v); G[v].add(u)
    degs = {u: len(G[u]) for u in G}
    return G, degs

def setup_stage1(G, degs):
    deg2 = sorted([u for u, d in degs.items() if d == 2])
    blocks = []
    node_to_block_end = {}
    for b_id, v in enumerate(deg2):
        u, w = list(G[v])
        blocks.append((b_id, u, v, w))
        node_to_block_end[u] = (b_id, 'u')
        node_to_block_end[w] = (b_id, 'w')
        
    edge_to_var = {}
    var_to_edge = {}
    var_cnt = 0
    adj_external = collections.defaultdict(list)
    block_adj_all = collections.defaultdict(list)
    block_pair_vars = collections.defaultdict(list)
    
    for u in node_to_block_end:
        b1, e1 = node_to_block_end[u]
        for v in G[u]:
            if v in node_to_block_end and v > u:
                b2, e2 = node_to_block_end[v]
                var_cnt += 1
                e = tuple(sorted((u, v)))
                edge_to_var[e] = var_cnt
                var_to_edge[var_cnt] = e
                adj_external[(b1, e1)].append(var_cnt)
                adj_external[(b2, e2)].append(var_cnt)
                block_adj_all[b1].append((b2, var_cnt, e))
                block_adj_all[b2].append((b1, var_cnt, e))
                bp = tuple(sorted((b1, b2)))
                block_pair_vars[bp].append(var_cnt)
                
    two_cycle_mutexes = []
    for bp, vlist in block_pair_vars.items():
        if len(vlist) > 1:
            for i in range(len(vlist)):
                for j in range(i + 1, len(vlist)):
                    two_cycle_mutexes.append([-vlist[i], -vlist[j]])
                    
    block_G = collections.defaultdict(set)
    for (b1, b2) in block_pair_vars:
        block_G[b1].add(b2); block_G[b2].add(b1)
    tri_cycle_mutexes = []
    for b1 in block_G:
        for b2 in block_G[b1]:
            if b2 > b1:
                for b3 in block_G[b1] & block_G[b2]:
                    if b3 > b2:
                        e12_list = block_pair_vars[tuple(sorted((b1, b2)))]
                        e23_list = block_pair_vars[tuple(sorted((b2, b3)))]
                        e31_list = block_pair_vars[tuple(sorted((b3, b1)))]
                        for e12 in e12_list:
                            for e23 in e23_list:
                                for e31 in e31_list:
                                    tri_cycle_mutexes.append([-e12, -e23, -e31])
                                    
    return blocks, node_to_block_end, edge_to_var, var_to_edge, var_cnt, adj_external, block_adj_all, two_cycle_mutexes, tri_cycle_mutexes

def get_cycles_from_edges(edges, blocks, node_to_block_end):
    port_nbr = {}
    for (u, v) in edges:
        p1 = node_to_block_end[u]; p2 = node_to_block_end[v]
        port_nbr[p1] = (p2, (u, v)); port_nbr[p2] = (p1, (u, v))
    visited = set(); cycs = []
    for b in range(len(blocks)):
        p = (b, 'u')
        if p not in visited:
            c_ports = []; curr = p
            while curr not in visited:
                visited.add(curr); c_ports.append(curr)
                nxt_p, e = port_nbr[curr]
                visited.add(nxt_p); c_ports.append(nxt_p)
                curr = (nxt_p[0], 'w' if nxt_p[1] == 'u' else 'u')
            cycs.append(c_ports)
    return cycs, port_nbr

def acquire_giant_backbone(blocks, node_to_block_end, cache_path='scratch/graph788_model_it15.pkl'):
    with open(cache_path, 'rb') as f:
        data = pickle.load(f)
    edges = set(tuple(sorted(e)) for e in data['active_edges'])
    
    # Step 1 alternating 4-opt flip:
    # Absorbs Subcycle 3 (218 blocks) and Subcycle 11 (2 blocks) into Giant
    added = [(517, 2614), (798, 3487), (1051, 3597), (3317, 3940)]
    removed = [(517, 798), (1051, 3487), (3597, 3940), (2614, 3317)]
    edges = (edges - set(removed)) | set(added)
    
    cycs, port_nbr = get_cycles_from_edges(edges, blocks, node_to_block_end)
    giant_idx = max(range(len(cycs)), key=lambda i: len(cycs[i]))
    giant_len = len(cycs[giant_idx]) // 2
    assert giant_len == 1432, f"Expected Giant to have 1432 blocks, got {giant_len}"
    assert len(cycs) == 19, f"Expected 19 cycles, got {len(cycs)}"
    return edges, cycs, port_nbr, giant_idx

