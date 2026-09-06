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

def decompose_subcycle_components(G, blocks, node_to_block_end, cycs, giant_idx):
    rem_subs = [cycs[i] for i in range(len(cycs)) if i != giant_idx]
    sub_blocks_map = {}
    for i, sc in enumerate(rem_subs):
        for p in sc: sub_blocks_map[p[0]] = i + 1

    sub_adj = collections.defaultdict(set)
    for b, s_id in sub_blocks_map.items():
        for u in [blocks[b][1], blocks[b][3]]:
            for v in G[u]:
                if v in node_to_block_end:
                    b2 = node_to_block_end[v][0]
                    if b2 in sub_blocks_map and sub_blocks_map[b2] != s_id:
                        sub_adj[s_id].add(sub_blocks_map[b2])

    vis = set(); comps = []
    for s_id in range(1, len(rem_subs) + 1):
        if s_id not in vis:
            comp = []; q = [s_id]; vis.add(s_id)
            for x in q:
                comp.append(x)
                for y in sub_adj[x]:
                    if y not in vis: vis.add(y); q.append(y)
            comps.append(comp)
    return comps

def generate_component_routes(G, blocks, node_to_block_end, port_nbr, cycs, giant_idx, comps):
    giant = cycs[giant_idx]
    giant_order = [p[0] for p in giant[::2]]
    giant_pos = {b: i for i, b in enumerate(giant_order)}
    rem_subs = [cycs[i] for i in range(len(cycs)) if i != giant_idx]
    
    comp_routes = {}
    for c_id, comp in enumerate(comps):
        comp_blocks = set()
        for s_id in comp:
            for p in rem_subs[s_id-1]: comp_blocks.add(p[0])
            
        old_comp_edges = set()
        for p_nxt, edge_raw in port_nbr.values():
            e = tuple(sorted(edge_raw))
            b1 = node_to_block_end[e[0]][0]; b2 = node_to_block_end[e[1]][0]
            if b1 in comp_blocks and b2 in comp_blocks:
                old_comp_edges.add(e)
                
        local_adj = collections.defaultdict(list)
        for b in comp_blocks:
            for u_type in ['u', 'w']:
                u = blocks[b][1] if u_type == 'u' else blocks[b][3]
                p1 = (b, u_type)
                for v in G[u]:
                    if v in node_to_block_end:
                        p2 = node_to_block_end[v]
                        if p2[0] in comp_blocks and p2[0] != b:
                            local_adj[p1].append((p2, tuple(sorted((u, v)))))
                            
        hp_pairs = []
        for start_b in comp_blocks:
            for start_port in [(start_b, 'u'), (start_b, 'w')]:
                def dfs(curr_port, visited_b, path):
                    curr_b = curr_port[0]
                    other_p = (curr_b, 'w' if curr_port[1] == 'u' else 'u')
                    if len(visited_b) == len(comp_blocks):
                        hp_pairs.append((start_port, other_p, path))
                        return
                    for nxt_p, e in local_adj[other_p]:
                        if nxt_p[0] not in visited_b:
                            dfs(nxt_p, visited_b | {nxt_p[0]}, path + [e])
                dfs(start_port, {start_b}, [])
                
        routes = []
        for p_start, p_end, path in hp_pairs:
            u_s = blocks[p_start[0]][1] if p_start[1] == 'u' else blocks[p_start[0]][3]
            u_e = blocks[p_end[0]][1] if p_end[1] == 'u' else blocks[p_end[0]][3]
            for v1 in G[u_s]:
                if v1 in node_to_block_end:
                    pg1 = node_to_block_end[v1]
                    if pg1[0] not in giant_pos: continue
                    p_prime1, rem_e1 = port_nbr[pg1]
                    for v2 in G[u_e]:
                        if v2 in node_to_block_end:
                            pg2 = node_to_block_end[v2]
                            if pg2[0] not in giant_pos or pg2 == pg1: continue
                            p_prime2, rem_e2 = port_nbr[pg2]
                            u_p1 = blocks[p_prime1[0]][1] if p_prime1[1] == 'u' else blocks[p_prime1[0]][3]
                            u_p2 = blocks[p_prime2[0]][1] if p_prime2[1] == 'u' else blocks[p_prime2[0]][3]
                            if u_p2 in G[u_p1]:
                                reconnect_e = tuple(sorted((u_p1, u_p2)))
                                e_in = tuple(sorted((u_s, v1)))
                                e_out = tuple(sorted((u_e, v2)))
                                added = set(path) | {e_in, e_out, reconnect_e}
                                removed = old_comp_edges | {rem_e1, rem_e2}
                                routes.append({
                                    'c_id': c_id,
                                    'added': added,
                                    'removed': removed,
                                    'ports_used': {pg1, pg2, p_prime1, p_prime2}
                                })
        comp_routes[c_id] = routes
    return comp_routes

def run_dp_bitmask_splicer(initial_edges, blocks, node_to_block_end, comps, comp_routes):
    dp = {0: (set(), set(), set())}  # mask -> (added_edges, removed_edges, ports_used)
    
    for c_id in range(len(comps)):
        routes = comp_routes.get(c_id, [])
        next_dp = dict(dp)
        bit = (1 << c_id)
        for mask, (added, removed, ports) in dp.items():
            if not (mask & bit):
                for r in routes:
                    if not (r['ports_used'] & ports) and not (r['removed'] & added) and not (r['added'] & removed):
                        new_mask = mask | bit
                        candidate = (added | r['added'], removed | r['removed'], ports | r['ports_used'])
                        if new_mask not in next_dp:
                            next_dp[new_mask] = candidate
        dp = next_dp
        print(f"  DP Bitmask Step {c_id+1}/{len(comps)}: {len(dp)} active mask states.")
        
    goal_mask = max(dp.keys())
    print(f"Highest bitmask reached: {bin(goal_mask)} ({goal_mask}/{(1<<len(comps))-1})")
    added_all, removed_all, _ = dp[goal_mask]
    final_edges = (initial_edges - removed_all) | added_all
    assert len(final_edges) == len(initial_edges), f"Edge count mismatch: {len(final_edges)} vs {len(initial_edges)}"
    return final_edges

def reconstruct_and_export_tour(final_edges, blocks, node_to_block_end, out_path='scratch/graph788/found_tour_graph788.hcp'):
    port_nbr = {}
    for (u, v) in final_edges:
        p1 = node_to_block_end[u]; p2 = node_to_block_end[v]
        port_nbr[p1] = p2; port_nbr[p2] = p1
        
    start_port = (0, 'u')
    visited_ports = set()
    block_seq = []
    curr = start_port
    while curr not in visited_ports:
        visited_ports.add(curr)
        b, end_type = curr
        other_end = 'w' if end_type == 'u' else 'u'
        block_seq.append((b, end_type, other_end))
        exit_port = (b, other_end)
        visited_ports.add(exit_port)
        nxt_port = port_nbr[exit_port]
        curr = nxt_port
        
    assert len(block_seq) == len(blocks), f"Expected {len(blocks)} blocks in tour, got {len(block_seq)}"
    
    raw_tour = []
    for (b_id, entry_type, exit_type) in block_seq:
        _, u, v, w = blocks[b_id]
        if entry_type == 'u':
            raw_tour.extend([u, v, w])
        else:
            raw_tour.extend([w, v, u])
            
    assert len(raw_tour) == 4620, f"Raw tour must have 4620 vertices, got {len(raw_tour)}"
    assert len(set(raw_tour)) == 4620, f"Raw tour vertices must be unique, got {len(set(raw_tour))}"
    
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    with open(out_path, 'w') as f:
        f.write(f"NAME : graph788\n")
        f.write(f"TYPE : TOUR\n")
        f.write(f"DIMENSION : {len(raw_tour)}\n")
        f.write(f"TOUR_SECTION\n")
        for v in raw_tour:
            f.write(f"{v}\n")
        f.write(f"-1\n")
        f.write(f"EOF\n")
        
    print(f"Exported certified tour to {out_path}!")
    return raw_tour

