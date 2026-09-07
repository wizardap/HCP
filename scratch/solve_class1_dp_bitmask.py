import collections, time, os, sys, pickle

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

def setup_stage1_contraction(G, degs):
    deg2 = sorted([u for u, d in degs.items() if d == 2])
    blocks = []
    node_to_block_end = {}
    for b_id, v in enumerate(deg2):
        u, w = list(G[v])
        blocks.append((b_id, u, v, w))
        node_to_block_end[u] = (b_id, 'u')
        node_to_block_end[w] = (b_id, 'w')
    return blocks, node_to_block_end

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

def acquire_giant_backbone(blocks, node_to_block_end, edges):
    cycs, port_nbr = get_cycles_from_edges(edges, blocks, node_to_block_end)
    giant_idx = max(range(len(cycs)), key=lambda i: len(cycs[i]))
    return edges, cycs, port_nbr, giant_idx
