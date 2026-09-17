import collections, time, pickle
from solve_class1_dp_bitmask import load_graph, setup_stage1_contraction, acquire_giant_backbone, decompose_subcycle_components, get_cycles_from_edges
from pysat.solvers import Cadical195

G, degs = load_graph('FHCPCS-col/graph868.col')
blocks, node_to_block_end = setup_stage1_contraction(G, degs)
with open('scratch/graph868_giant_1686.pkl', 'rb') as f:
    edges = set(tuple(sorted(e)) for e in pickle.load(f)['edges'])
edges, cycs, port_nbr, giant_idx = acquire_giant_backbone(blocks, node_to_block_end, edges)
comps = decompose_subcycle_components(G, blocks, node_to_block_end, cycs, giant_idx)
rem_subs = [cycs[i] for i in range(len(cycs)) if i != giant_idx]
giant_ports = set(cycs[giant_idx])
giant_blocks = set(p[0] for p in giant_ports)

print("Testing Step 3 logic on Comp 0 and Comp 6...")
for c_id in [0, 6]:
    comp = comps[c_id]
    comp_blocks = set()
    comp_ports = set()
    for sid in comp:
        sc = rem_subs[sid - 1]
        for p in sc:
            comp_blocks.add(p[0])
            comp_ports.add(p)
    
    giant_nbr_blocks = set()
    for b in comp_blocks:
        for end in ['u', 'w']:
            node = blocks[b][1] if end == 'u' else blocks[b][3]
            for v in G[node]:
                if v in node_to_block_end:
                    gb = node_to_block_end[v][0]
                    if gb in giant_blocks:
                        giant_nbr_blocks.add(gb)
                        
    local_blocks = comp_blocks | giant_nbr_blocks
    print(f"Comp {c_id}: {len(comp_blocks)} comp blocks, {len(giant_nbr_blocks)} giant nbr blocks, total {len(local_blocks)} local blocks")

    edge_to_var = {}
    var_to_edge = {}
    v_cnt = 0
    port_vars = collections.defaultdict(list)
    
    for b1 in local_blocks:
        for end1 in ['u', 'w']:
            p1 = (b1, end1)
            node1 = blocks[b1][1] if end1 == 'u' else blocks[b1][3]
            for node2 in G[node1]:
                if node2 in node_to_block_end and node2 > node1:
                    b2, end2 = node_to_block_end[node2]
                    if b2 in local_blocks:
                        p2 = (b2, end2)
                        e = tuple(sorted((node1, node2)))
                        v_cnt += 1
                        edge_to_var[e] = v_cnt
                        var_to_edge[v_cnt] = e
                        port_vars[p1].append(v_cnt)
                        port_vars[p2].append(v_cnt)
                        
    old_local_edges = set()
    for p in comp_ports:
        partner_p, e = port_nbr[p]
        old_local_edges.add(e)
    for gb in giant_nbr_blocks:
        p = (gb, 'u')
        partner_p, e = port_nbr[p]
        if partner_p[0] in giant_nbr_blocks:
            old_local_edges.add(e)

    solver = Cadical195()
    for p in comp_ports:
        evars = port_vars[p]
        if evars:
            solver.add_clause(evars)
            for i in range(len(evars)):
                for j in range(i + 1, len(evars)):
                    solver.add_clause([-evars[i], -evars[j]])
                    
    for gb in giant_nbr_blocks:
        for end in ['u', 'w']:
            p = (gb, end)
            evars = port_vars[p]
            for i in range(len(evars)):
                for j in range(i + 1, len(evars)):
                    solver.add_clause([-evars[i], -evars[j]])
                    
    block_pair_evars = collections.defaultdict(list)
    for e, var in edge_to_var.items():
        b1 = node_to_block_end[e[0]][0]
        b2 = node_to_block_end[e[1]][0]
        bp = tuple(sorted((b1, b2)))
        block_pair_evars[bp].append(var)
    for bp, vlist in block_pair_evars.items():
        if len(vlist) > 1:
            for i in range(len(vlist)):
                for j in range(i + 1, len(vlist)):
                    solver.add_clause([-vlist[i], -vlist[j]])
                    
    routes = []
    attempts = 0
    while len(routes) < 3 and solver.solve() and attempts < 10:
        attempts += 1
        model = set(solver.get_model())
        active_local = set(var_to_edge[v] for v in model if v > 0 and v in var_to_edge)
        added = active_local - old_local_edges
        removed = old_local_edges - active_local
        
        giant_ports_used = set()
        for (u, v) in (added | removed):
            p_u = node_to_block_end[u]
            p_v = node_to_block_end[v]
            if p_u[0] in giant_blocks: giant_ports_used.add(p_u)
            if p_v[0] in giant_blocks: giant_ports_used.add(p_v)
            
        print(f"  Attempt {attempts}: |added|={len(added)}, |removed|={len(removed)}, giant_ports_used={len(giant_ports_used)}")
        if added and removed and len(giant_ports_used) > 0:
            test_edges = (edges - removed) | added
            new_cycs, _ = get_cycles_from_edges(test_edges, blocks, node_to_block_end)
            print(f"  Cycle count: {len(cycs)} -> {len(new_cycs)}")
            if len(new_cycs) < len(cycs):
                routes.append({
                    'c_id': c_id,
                    'added': added,
                    'removed': removed,
                    'ports_used': giant_ports_used
                })
            solver.add_clause([-edge_to_var[e] for e in added])
        else:
            break
    print(f"Comp {c_id}: found {len(routes)} valid routes")
