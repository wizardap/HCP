#!/usr/bin/env python3
import pickle, collections, time

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

with open('scratch/graph788_model_it15.pkl', 'rb') as f:
    data = pickle.load(f)
current_edges = set(tuple(sorted(e)) for e in data['active_edges'])

def get_cycles(edges):
    port_nbr = {}
    for (u, v) in edges:
        p1 = node_to_block_end[u]
        p2 = node_to_block_end[v]
        port_nbr[p1] = (p2, (u, v))
        port_nbr[p2] = (p1, (u, v))
    visited = set()
    cycs = []
    for b in range(len(blocks)):
        p = (b, 'u')
        if p not in visited:
            c_ports = []
            curr = p
            while curr not in visited:
                visited.add(curr)
                c_ports.append(curr)
                nxt_p, e = port_nbr[curr]
                visited.add(nxt_p)
                c_ports.append(nxt_p)
                curr = (nxt_p[0], 'w' if nxt_p[1] == 'u' else 'u')
            cycs.append(c_ports)
    return cycs, port_nbr

cycs, port_nbr = get_cycles(current_edges)
print(f"Step 0: {len(cycs)} cycles, Giant={max(len(c)//2 for c in cycs)} blocks")

# Greedy / Splicing loop
for step in range(1, 30):
    cycs, port_nbr = get_cycles(current_edges)
    giant_idx = max(range(len(cycs)), key=lambda i: len(cycs[i]))
    giant_ports = set(cycs[giant_idx])
    giant_len = len(giant_ports)//2
    if len(cycs) == 1:
        print(f"*** FOUND HAMILTONIAN CYCLE! 1 cycle of {giant_len} blocks! ***")
        break
        
    subcycles = [cycs[i] for i in range(len(cycs)) if i != giant_idx]
    port_to_cid = {}
    for p in giant_ports: port_to_cid[p] = 0
    for idx, sc in enumerate(subcycles):
        for p in sc: port_to_cid[p] = idx + 1

    # Auxiliary graph on ports
    aux_edges = collections.defaultdict(list)
    for u, p1 in node_to_block_end.items():
        f_p2, f_e = port_nbr[p1]
        for v in G[u]:
            if v in node_to_block_end:
                p2 = node_to_block_end[v]
                if p2 != f_p2:
                    next_port = port_nbr[p2][0]
                    removed_edge = port_nbr[p2][1]
                    added_edge = tuple(sorted((u, v)))
                    aux_edges[p1].append((next_port, added_edge, removed_edge))

    # Find a move that strictly reduces the number of cycles and increases/maintains Giant
    best_flip = None
    best_metric = (0, 0) # (cycle_reduction, new_giant_len)

    # Search starting from non-giant ports
    non_giant_ports = [p for p in port_nbr if port_to_cid[p] != 0]
    # Sort non_giant_ports by degree or cycle size
    found = False
    for max_depth in [4, 6, 8, 10]:
        for start_p in non_giant_ports:
            q = collections.deque([(start_p, [start_p], [], [])])
            while q:
                curr, path_nodes, added_edges, removed_edges = q.popleft()
                if len(path_nodes) > max_depth: continue
                for nxt, a_e, r_e in aux_edges[curr]:
                    if a_e in added_edges or r_e in removed_edges: continue
                    new_added = added_edges + [a_e]
                    new_removed = removed_edges + [r_e]
                    if nxt == start_p:
                        c_cids = set(port_to_cid[x] for x in path_nodes)
                        if 0 in c_cids: # touches Giant
                            test_edges = (current_edges - set(new_removed)) | set(new_added)
                            if len(test_edges) == len(current_edges):
                                new_cycs, _ = get_cycles(test_edges)
                                if len(new_cycs) < len(cycs):
                                    new_giant = max(len(c)//2 for c in new_cycs)
                                    # We found a reducing move!
                                    best_flip = (test_edges, len(cycs) - len(new_cycs), new_giant, len(new_added), c_cids)
                                    found = True
                                    break
                    elif nxt not in path_nodes and len(path_nodes) < max_depth:
                        q.append((nxt, path_nodes + [nxt], new_added, new_removed))
                if found: break
            if found: break
        if found: break

    if best_flip:
        current_edges, red, new_g, flen, cids = best_flip
        print(f"Step {step:2d}: Merged {red} cycle(s)! New cycles={len(cycs)-red}, Giant={new_g} blocks (depth {flen}, cids={cids})")
    else:
        print(f"Step {step:2d}: No reducing move found at depth <= 10.")
        break
