import sys, collections, time, pickle
from solve_class1_dp_bitmask import load_graph, setup_stage1_contraction, acquire_giant_backbone, get_cycles_from_edges

G, degs = load_graph('FHCPCS-col/graph868.col')
blocks, node_to_block_end = setup_stage1_contraction(G, degs)

with open('scratch/graph868_giant_1686.pkl', 'rb') as f:
    edges = set(tuple(sorted(e)) for e in pickle.load(f)['edges'])

edges, cycs, port_nbr, giant_idx = acquire_giant_backbone(blocks, node_to_block_end, edges)
giant_len = len(cycs[giant_idx]) // 2
print(f"Initial: {len(cycs)} cycles, Giant={giant_len} blocks.")

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

port_nbr_dict, aux = get_aux_edges(edges)
print(f"Aux graph built: {len(aux)} ports have auxiliary transitions.")

giant_ports = set(cycs[giant_idx])
subcycles = [cycs[i] for i in range(len(cycs)) if i != giant_idx]
port_to_cid = {p: 0 for p in giant_ports}
for idx, sc in enumerate(subcycles):
    for p in sc:
        port_to_cid[p] = idx + 1

non_giant_ports = [p for p in port_nbr_dict if port_to_cid[p] != 0]
print(f"Total non-giant ports: {len(non_giant_ports)}")

t0 = time.time()
found = []
for start_p in non_giant_ports[:20]:
    q = collections.deque([(start_p, [start_p], [], [])])
    while q:
        curr, path, added_e, removed_e = q.popleft()
        if len(path) > 4:
            continue
        for nxt, a_e, r_e in aux[curr]:
            if nxt == start_p and a_e not in added_e and r_e not in removed_e:
                all_a = added_e + [a_e]
                all_r = removed_e + [r_e]
                t_edges = (edges - set(all_r)) | set(all_a)
                if len(t_edges) == len(edges):
                    n_cycs, _ = get_cycles_from_edges(t_edges, blocks, node_to_block_end)
                    if len(n_cycs) < len(cycs):
                        found.append((len(n_cycs), len(all_a), all_a, all_r))
            elif nxt not in path and a_e not in added_e and r_e not in removed_e:
                if len(path) < 4:
                    q.append((nxt, path + [nxt], added_e + [a_e], removed_e + [r_e]))

print(f"In {time.time()-t0:.2f}s, tested 20 non-giant ports, found {len(found)} reducing cycles!")
if found:
    for f in found[:5]:
        print(f"  Result: {f[0]} cycles (depth {f[1]})")
