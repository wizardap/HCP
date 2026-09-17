import sys
sys.path.insert(0, ".")
#!/usr/bin/env python3
import collections, time, pickle

def load_graph(path):
    G = collections.defaultdict(set)
    with open(path, 'r') as f:
        for line in f:
            if line.startswith('e '):
                parts = line.split()
                u, v = int(parts[1]), int(parts[2])
                G[u].add(v); G[v].add(u)
    degs = {u: len(G[u]) for u in G}
    return G, degs

col_path = "FHCPCS-col/graph788.col"
G, degs = load_graph(col_path)
deg2 = sorted([u for u, d in degs.items() if d == 2])
blocks = []
node_to_block_end = {}
for b_id, v in enumerate(deg2):
    u, w = list(G[v])
    blocks.append((b_id, u, v, w))
    node_to_block_end[u] = (b_id, 'u')
    node_to_block_end[w] = (b_id, 'w')

from scratch.solve_graph788_dp_bitmask import acquire_giant_backbone, get_cycles_from_edges

current_edges, cycs, port_nbr, giant_idx = acquire_giant_backbone(blocks, node_to_block_end)
print(f"Initial Backbone: {len(cycs)} cycles, Giant={len(cycs[giant_idx])//2} blocks")
subcycle_lens = [len(c)//2 for i, c in enumerate(cycs) if i != giant_idx]
print(f"Subcycle sizes ({len(subcycle_lens)}): {sorted(subcycle_lens)}")

def get_aux_edges(current_edges):
    port_nbr = {}
    for (u, v) in current_edges:
        p1 = node_to_block_end[u]
        p2 = node_to_block_end[v]
        port_nbr[p1] = (p2, (u, v))
        port_nbr[p2] = (p1, (u, v))
    aux = collections.defaultdict(list)
    for u, p1 in node_to_block_end.items():
        f_p2, f_e = port_nbr[p1]
        for v in G[u]:
            if v in node_to_block_end:
                p2 = node_to_block_end[v]
                if p2 != f_p2:
                    aux[p1].append((port_nbr[p2][0], tuple(sorted((u, v))), port_nbr[p2][1]))
    return port_nbr, aux

def find_step_move(current_edges, giant_len, cycs_count, non_giant_ports, aux):
    for max_d in [3, 4, 5, 6]:
        best_candidate = None
        for start_p in non_giant_ports:
            # iterative DFS with stack
            stack = [(start_p, [start_p], [], [])]
            while stack:
                curr, path, added_e, removed_e = stack.pop()
                if len(path) == max_d:
                    for nxt, a_e, r_e in aux[curr]:
                        if nxt == start_p and a_e not in added_e and r_e not in removed_e:
                            all_a = added_e + [a_e]
                            all_r = removed_e + [r_e]
                            t_edges = (current_edges - set(all_r)) | set(all_a)
                            if len(t_edges) == len(current_edges):
                                n_cycs, _ = get_cycles_from_edges(t_edges, blocks, node_to_block_end)
                                n_g = max(len(c)//2 for c in n_cycs)
                                if len(n_cycs) < cycs_count and n_g >= giant_len:
                                    return (t_edges, cycs_count - len(n_cycs), n_g, max_d, "reduce+grow")
                                elif len(n_cycs) < cycs_count and best_candidate is None:
                                    best_candidate = (t_edges, cycs_count - len(n_cycs), n_g, max_d, "reduce")
                                elif n_g > giant_len and len(n_cycs) <= cycs_count and best_candidate is None:
                                    best_candidate = (t_edges, cycs_count - len(n_cycs), n_g, max_d, "grow")
                    continue
                for nxt, a_e, r_e in aux[curr]:
                    if nxt not in path and a_e not in added_e and r_e not in removed_e:
                        stack.append((nxt, path + [nxt], added_e + [a_e], removed_e + [r_e]))
        if best_candidate is not None:
            return best_candidate
    return None

step = 0
while True:
    step += 1
    cycs, port_nbr = get_cycles_from_edges(current_edges, blocks, node_to_block_end)
    giant_idx = max(range(len(cycs)), key=lambda i: len(cycs[i]))
    giant_len = len(cycs[giant_idx]) // 2
    if len(cycs) == 1:
        print(f"\n*** REACHED SINGLE CYCLE OF {giant_len} BLOCKS! ***")
        break
    
    giant_ports = set(cycs[giant_idx])
    subcycles = [cycs[i] for i in range(len(cycs)) if i != giant_idx]
    port_to_cid = {p: 0 for p in giant_ports}
    for idx, sc in enumerate(subcycles):
        for p in sc:
            port_to_cid[p] = idx + 1
            
    port_nbr_dict, aux = get_aux_edges(current_edges)
    non_giant_ports = [p for p in port_nbr_dict if port_to_cid[p] != 0]
    
    move = find_step_move(current_edges, giant_len, len(cycs), non_giant_ports, aux)
    if move:
        current_edges, red, n_g, d, mtype = move
        print(f"Step {step:2d} ({mtype}, depth {d}): Cycles {len(cycs)} -> {len(cycs)-red}, Giant {giant_len} -> {n_g}")
    else:
        print(f"Step {step:2d}: No move found at depth <= 6. Stopping.")
        break
