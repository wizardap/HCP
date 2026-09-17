#!/usr/bin/env python3
import sys
sys.path.insert(0, '.')
import collections, time

from scratch.solve_graph788_dp_bitmask import load_graph, setup_stage1, acquire_giant_backbone, get_cycles_from_edges
from scratch.explore_splicer import find_step_move, get_aux_edges

G, degs = load_graph('FHCPCS-col/graph788.col')
blocks, node_to_block_end, _, _, _, _, _, _, _ = setup_stage1(G, degs)
edges, cycs, port_nbr, giant_idx = acquire_giant_backbone(blocks, node_to_block_end)

# Advance 4 steps to 1456
for step in range(4):
    cycs, port_nbr = get_cycles_from_edges(edges, blocks, node_to_block_end)
    giant_idx = max(range(len(cycs)), key=lambda i: len(cycs[i]))
    giant_len = len(cycs[giant_idx]) // 2
    giant_ports = set(cycs[giant_idx])
    subcycles = [cycs[i] for i in range(len(cycs)) if i != giant_idx]
    port_to_cid = {p: 0 for p in giant_ports}
    for idx, sc in enumerate(subcycles):
        for p in sc: port_to_cid[p] = idx + 1
    port_nbr_dict, aux = get_aux_edges(edges)
    non_giant_ports = [p for p in port_nbr_dict if port_to_cid[p] != 0]
    move = find_step_move(edges, giant_len, len(cycs), non_giant_ports, aux)
    edges, red, n_g, d, mtype = move

cycs, port_nbr = get_cycles_from_edges(edges, blocks, node_to_block_end)
giant_idx = max(range(len(cycs)), key=lambda i: len(cycs[i]))
giant_len = len(cycs[giant_idx]) // 2
print(f"Current state: {len(cycs)} cycles, Giant={giant_len} blocks.")

# Build forward and backward aux graphs
port_nbr_dict, aux_fwd = get_aux_edges(edges)
aux_bwd = collections.defaultdict(list)
for p1, elist in aux_fwd.items():
    for p2, a_e, r_e in elist:
        aux_bwd[p2].append((p1, a_e, r_e))

print(f"Forward aux graph: {len(aux_fwd)} ports, Backward aux graph: {len(aux_bwd)} ports.")

# Test bidirectional search for a given start_p
giant_ports = set(cycs[giant_idx])
subcycles = [cycs[i] for i in range(len(cycs)) if i != giant_idx]
port_to_cid = {p: 0 for p in giant_ports}
for idx, sc in enumerate(subcycles):
    for p in sc: port_to_cid[p] = idx + 1

non_giant_ports = [p for p in port_nbr_dict if port_to_cid[p] != 0]

# For each half depth h1=3, h2=3 (total depth 6), h1=4, h2=4 (total depth 8), h1=5, h2=5 (total depth 10)
for half in [3, 4, 5]:
    total_depth = half * 2
    t0 = time.time()
    found_reducing = []
    for start_p in non_giant_ports:
        # Forward BFS from start_p up to depth half
        # Store: fwd_visited[mid_p] = (path, added, removed)
        fwd = {start_p: ([], [], [])}
        fwd_layer = {start_p: ([], [], [])}
        for d in range(half):
            next_layer = {}
            for curr, (path, added, removed) in fwd_layer.items():
                for nxt, a_e, r_e in aux_fwd[curr]:
                    if a_e not in added and r_e not in removed and nxt not in path:
                        if nxt not in next_layer:
                            next_layer[nxt] = (path + [nxt], added + [a_e], removed + [r_e])
            fwd_layer = next_layer
            for k, v in fwd_layer.items():
                if k not in fwd: fwd[k] = v

        # Backward BFS from start_p up to depth half
        bwd = {start_p: ([], [], [])}
        bwd_layer = {start_p: ([], [], [])}
        for d in range(half):
            next_layer = {}
            for curr, (path, added, removed) in bwd_layer.items():
                for prev, a_e, r_e in aux_bwd[curr]:
                    if a_e not in added and r_e not in removed and prev not in path:
                        if prev not in next_layer:
                            next_layer[prev] = (path + [prev], added + [a_e], removed + [r_e])
            bwd_layer = next_layer
            for k, v in bwd_layer.items():
                if k not in bwd: bwd[k] = v

        # Check intersection between fwd and bwd
        common = set(fwd.keys()) & set(bwd.keys()) - {start_p}
        for mid in common:
            p_f, a_f, r_f = fwd[mid]
            p_b, a_b, r_b = bwd[mid]
            # Check edge disjointness
            if set(a_f) & set(a_b) or set(r_f) & set(r_b):
                continue
            all_a = a_f + a_b
            all_r = r_f + r_b
            t_edges = (edges - set(all_r)) | set(all_a)
            if len(t_edges) == len(edges):
                n_cycs, _ = get_cycles_from_edges(t_edges, blocks, node_to_block_end)
                n_g = max(len(c)//2 for c in n_cycs)
                if len(n_cycs) < len(cycs):
                    found_reducing.append((len(n_cycs), n_g, len(all_a), t_edges))
                    break
        if found_reducing:
            break

    print(f"Total depth {total_depth} (search {time.time()-t0:.2f}s): found {len(found_reducing)} reducing moves.")
    if found_reducing:
        best = found_reducing[0]
        print(f"  Best move: cycles {len(cycs)} -> {best[0]}, Giant {giant_len} -> {best[1]} blocks (depth {best[2]})")
