import collections, time, os, sys
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
