import sys, pickle, collections
sys.path.append("scratch")
from solve_class1_dp_bitmask import load_graph, setup_stage1_contraction, acquire_giant_backbone, decompose_subcycle_components, get_cycles_from_edges
from pysat.solvers import Cadical195

with open("scratch/graph868_giant_1686.pkl", "rb") as f:
    data = pickle.load(f)
edges = set(tuple(sorted(e)) for e in data["edges"])

G, degs = load_graph("FHCPCS-col/graph868.col")
blocks, node_to_block = setup_stage1_contraction(G, degs)
edges, cycs, port_nbr, giant_idx = acquire_giant_backbone(blocks, node_to_block, edges)

comps = decompose_subcycle_components(G, blocks, node_to_block, cycs, giant_idx)
giant = cycs[giant_idx]
giant_blocks_seq = [p[0] for p in giant[::2]]
n_g = len(giant_blocks_seq)
giant_pos = {b: i for i, b in enumerate(giant_blocks_seq)}

def get_comp_route(comp_idx, current_edges, current_cycs, current_giant_idx):
    comp = comps[comp_idx]
    comp_blocks = set()
    comp_ports = set()
    for si in comp:
        for p in cycs[si]:
            comp_blocks.add(p[0])
            comp_ports.add(p)

    c_giant = current_cycs[current_giant_idx]
    c_giant_blocks_seq = [p[0] for p in c_giant[::2]]
    c_n_g = len(c_giant_blocks_seq)
    c_giant_pos = {b: i for i, b in enumerate(c_giant_blocks_seq)}

    giant_nbrs = set()
    for b in comp_blocks:
        for u in [blocks[b][1], blocks[b][3]]:
            for v in G[u]:
                if v in node_to_block:
                    b2 = node_to_block[v][0]
                    if b2 in c_giant_pos:
                        giant_nbrs.add(b2)

    positions = sorted(c_giant_pos[b] for b in giant_nbrs)
    
    for i in range(len(positions)):
        p1 = positions[i]
        p2 = positions[(i + 1) % len(positions)]
        dist = (p2 - p1) % c_n_g
        if dist <= 8:
            w_start = (p1 - 1) % c_n_g
            w_end = (p2 + 1) % c_n_g
            w_pos = []
            curr = w_start
            while True:
                w_pos.append(curr)
                if curr == w_end: break
                curr = (curr + 1) % c_n_g
            
            cluster_blocks = set(c_giant_blocks_seq[p] for p in w_pos)
            local_blocks = comp_blocks | cluster_blocks
            
            edge_to_var = {}
            var_to_edge = {}
            v_cnt = 0
            port_vars = collections.defaultdict(list)

            for b1 in local_blocks:
                for end1 in ['u', 'w']:
                    p1_port = (b1, end1)
                    node1 = blocks[b1][1] if end1 == 'u' else blocks[b1][3]
                    for node2 in G[node1]:
                        if node2 in node_to_block and node2 > node1:
                            b2, end2 = node_to_block[node2]
                            if b2 in local_blocks:
                                p2_port = (b2, end2)
                                e = tuple(sorted((node1, node2)))
                                v_cnt += 1
                                edge_to_var[e] = v_cnt
                                var_to_edge[v_cnt] = e
                                port_vars[p1_port].append(v_cnt)
                                port_vars[p2_port].append(v_cnt)

            old_local_edges = set()
            for p in comp_ports:
                partner_p, e = port_nbr[p]
                old_local_edges.add(e)
            for gb in cluster_blocks:
                for end in ['u', 'w']:
                    p = (gb, end)
                    partner_p, e = port_nbr[p]
                    if partner_p[0] in cluster_blocks:
                        old_local_edges.add(e)

            solver = Cadical195()
            for p in comp_ports:
                evars = port_vars[p]
                solver.add_clause(evars)
                for i_e in range(len(evars)):
                    for j_e in range(i_e + 1, len(evars)):
                        solver.add_clause([-evars[i_e], -evars[j_e]])

            for pos_idx in w_pos[1:-1]:
                gb = c_giant_blocks_seq[pos_idx]
                for end in ['u', 'w']:
                    evars = port_vars[(gb, end)]
                    solver.add_clause(evars)
                    for i_e in range(len(evars)):
                        for j_e in range(i_e + 1, len(evars)):
                            solver.add_clause([-evars[i_e], -evars[j_e]])

            p_first = (c_giant_blocks_seq[w_pos[0]], 'u')
            p_first_w = (c_giant_blocks_seq[w_pos[0]], 'w')
            for p_b in [p_first, p_first_w]:
                partner_p, e = port_nbr[p_b]
                if partner_p[0] == c_giant_blocks_seq[w_pos[1]]:
                    evars = port_vars[p_b]
                    solver.add_clause(evars)
                    for i_e in range(len(evars)):
                        for j_e in range(i_e + 1, len(evars)):
                            solver.add_clause([-evars[i_e], -evars[j_e]])

            p_last = (c_giant_blocks_seq[w_pos[-1]], 'u')
            p_last_w = (c_giant_blocks_seq[w_pos[-1]], 'w')
            for p_b in [p_last, p_last_w]:
                partner_p, e = port_nbr[p_b]
                if partner_p[0] == c_giant_blocks_seq[w_pos[-2]]:
                    evars = port_vars[p_b]
                    solver.add_clause(evars)
                    for i_e in range(len(evars)):
                        for j_e in range(i_e + 1, len(evars)):
                            solver.add_clause([-evars[i_e], -evars[j_e]])

            comp_old_edges = [edge_to_var[e] for e in old_local_edges if e in edge_to_var and (node_to_block[e[0]][0] in comp_blocks or node_to_block[e[1]][0] in comp_blocks)]
            solver.add_clause([-v for v in comp_old_edges])

            if solver.solve():
                model = set(solver.get_model())
                active_edges = set(var_to_edge[v] for v in model if v > 0 and v in var_to_edge)
                added = active_edges - old_local_edges
                removed = old_local_edges - active_edges
                return added, removed
    return None, None

current_edges = set(edges)
current_cycs = cycs
current_giant_idx = giant_idx

print(f"Initial: {len(current_cycs)} cycles, Giant: {len(current_cycs[current_giant_idx])//2} blocks")

for target_comp in [1, 4, 5, 8]:
    added, removed = get_comp_route(target_comp, current_edges, current_cycs, current_giant_idx)
    if added and removed:
        current_edges = (current_edges - removed) | added
        current_cycs, port_nbr = get_cycles_from_edges(current_edges, blocks, node_to_block)
        current_giant_idx = max(range(len(current_cycs)), key=lambda i: len(current_cycs[i]))
        print(f"Absorbed Comp {target_comp}: {len(current_cycs)} cycles remaining, Giant: {len(current_cycs[current_giant_idx])//2} blocks")
    else:
        print(f"Comp {target_comp} route not found in current configuration")
