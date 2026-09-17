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

def main():
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
    active_edges = set(tuple(sorted(e)) for e in data['active_edges'])
    print(f"Loaded {len(active_edges)} active external edges.")

    def get_cycles(current_edges):
        port_nbr = {}
        for (u, v) in current_edges:
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

    cycs, port_nbr = get_cycles(active_edges)
    lens = [len(c)//2 for c in cycs]
    print(f"Initial 2-factor: {len(cycs)} cycles, lens = {sorted(lens, reverse=True)}")

    # Auxiliary graph on ports
    aux_edges = collections.defaultdict(list)
    for u, p1 in node_to_block_end.items():
        f_p2, f_e = port_nbr[p1]
        for v in G[u]:
            if v in node_to_block_end:
                p2 = node_to_block_end[v]
                if p2 != f_p2:
                    removed_p = p2
                    next_port = port_nbr[p2][0]
                    removed_edge = port_nbr[p2][1]
                    added_edge = tuple(sorted((u, v)))
                    aux_edges[p1].append((next_port, added_edge, removed_edge))

    print(f"Auxiliary graph built with {len(aux_edges)} ports.")

    giant_idx = max(range(len(cycs)), key=lambda i: len(cycs[i]))
    giant_ports = set(cycs[giant_idx])
    subcycles = [cycs[i] for i in range(len(cycs)) if i != giant_idx]

    port_to_cid = {}
    for p in giant_ports: port_to_cid[p] = 0
    for idx, sc in enumerate(subcycles):
        for p in sc: port_to_cid[p] = idx + 1

    successful_flips = 0
    for sc_id in range(1, len(subcycles) + 1):
        sc_ports = [p for p, cid in port_to_cid.items() if cid == sc_id]
        print(f"\nAnalyzing Subcycle {sc_id} (len={len(sc_ports)//2} blocks):")
        found_cycle = False
        for start_p in sc_ports:
            q = collections.deque([(start_p, [start_p], [], [])])
            while q:
                curr, path_nodes, added_edges, removed_edges = q.popleft()
                if len(path_nodes) > 7: continue
                for nxt, a_e, r_e in aux_edges[curr]:
                    if a_e in added_edges or r_e in removed_edges: continue
                    new_added = added_edges + [a_e]
                    new_removed = removed_edges + [r_e]
                    if nxt == start_p:
                        c_cids = set(port_to_cid[x] for x in path_nodes)
                        if 0 in c_cids:
                            test_edges = (active_edges - set(new_removed)) | set(new_added)
                            if len(test_edges) == len(active_edges):
                                new_cycs, _ = get_cycles(test_edges)
                                if len(new_cycs) < len(cycs):
                                    print(f"  >>> SUCCESS! Flip of length {len(new_added)} reduced cycles from {len(cycs)} to {len(new_cycs)}!")
                                    print(f"      Cids involved: {c_cids}, New Giant={max(len(c)//2 for c in new_cycs)} blocks")
                                    found_cycle = True
                                    successful_flips += 1
                                    break
                    elif nxt not in path_nodes and len(path_nodes) < 7:
                        q.append((nxt, path_nodes + [nxt], new_added, new_removed))
                if found_cycle: break
            if found_cycle: break
        if not found_cycle:
            print(f"  No short reducing cycle found.")

    print(f"\nTotal subcycles directly reducible via depth <= 6 alternating cycle: {successful_flips}/{len(subcycles)}")

if __name__ == '__main__':
    main()
