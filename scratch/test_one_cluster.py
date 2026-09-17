import sys, pickle, collections
sys.path.append("scratch")
from solve_class1_dp_bitmask import load_graph, setup_stage1_contraction, acquire_giant_backbone, decompose_subcycle_components
from pysat.solvers import Cadical195

with open("scratch/graph868_giant_1686.pkl", "rb") as f:
    data = pickle.load(f)
edges = data["edges"]

G, degs = load_graph("FHCPCS-col/graph868.col")
blocks, node_to_block = setup_stage1_contraction(G, degs)
edges, cycs, port_nbr, giant_idx = acquire_giant_backbone(blocks, node_to_block, edges)

comps = decompose_subcycle_components(G, blocks, node_to_block, cycs, giant_idx)
giant = cycs[giant_idx]
giant_blocks_seq = [p[0] for p in giant[::2]]
n_g = len(giant_blocks_seq)
giant_pos = {b: i for i, b in enumerate(giant_blocks_seq)}

comp0 = comps[0]
comp0_blocks = set()
comp0_ports = set()
for si in comp0:
    for p in cycs[si]:
        comp0_blocks.add(p[0])
        comp0_ports.add(p)

# Test cluster at pos 1586..1588
cluster_pos = [1585, 1586, 1587, 1588, 1589]
cluster_blocks = set(giant_blocks_seq[p] for p in cluster_pos)
local_blocks = comp0_blocks | cluster_blocks

edge_to_var = {}
var_to_edge = {}
v_cnt = 0
port_vars = collections.defaultdict(list)

for b1 in local_blocks:
    for end1 in ['u', 'w']:
        p1 = (b1, end1)
        node1 = blocks[b1][1] if end1 == 'u' else blocks[b1][3]
        for node2 in G[node1]:
            if node2 in node_to_block and node2 > node1:
                b2, end2 = node_to_block[node2]
                if b2 in local_blocks:
                    p2 = (b2, end2)
                    e = tuple(sorted((node1, node2)))
                    v_cnt += 1
                    edge_to_var[e] = v_cnt
                    var_to_edge[v_cnt] = e
                    port_vars[p1].append(v_cnt)
                    port_vars[p2].append(v_cnt)

old_local_edges = set()
for p in comp0_ports:
    partner_p, e = port_nbr[p]
    old_local_edges.add(e)
for gb in cluster_blocks:
    for end in ['u', 'w']:
        p = (gb, end)
        partner_p, e = port_nbr[p]
        if partner_p[0] in cluster_blocks:
            old_local_edges.add(e)

solver = Cadical195()

# Comp0 ports: exactly 1 edge
for p in comp0_ports:
    evars = port_vars[p]
    solver.add_clause(evars)
    for i in range(len(evars)):
        for j in range(i + 1, len(evars)):
            solver.add_clause([-evars[i], -evars[j]])

# Internal cluster blocks: pos 1586, 1587, 1588: exactly 1 edge
# Boundary blocks: pos 1585, 1589: exactly 1 edge on their internal port, keep original on external
for p_idx in [1586, 1587, 1588]:
    gb = giant_blocks_seq[p_idx]
    for end in ['u', 'w']:
        evars = port_vars[(gb, end)]
        solver.add_clause(evars)
        for i in range(len(evars)):
            for j in range(i + 1, len(evars)):
                solver.add_clause([-evars[i], -evars[j]])

# Boundary pos 1585: port pointing to 1586 must have degree 1
gb_1585 = giant_blocks_seq[1585]
for end in ['u', 'w']:
    p = (gb_1585, end)
    partner_p, e = port_nbr[p]
    if partner_p[0] == giant_blocks_seq[1586]:
        evars = port_vars[p]
        solver.add_clause(evars)
        for i in range(len(evars)):
            for j in range(i + 1, len(evars)):
                solver.add_clause([-evars[i], -evars[j]])

# Boundary pos 1589: port pointing to 1588 must have degree 1
gb_1589 = giant_blocks_seq[1589]
for end in ['u', 'w']:
    p = (gb_1589, end)
    partner_p, e = port_nbr[p]
    if partner_p[0] == giant_blocks_seq[1588]:
        evars = port_vars[p]
        solver.add_clause(evars)
        for i in range(len(evars)):
            for j in range(i + 1, len(evars)):
                solver.add_clause([-evars[i], -evars[j]])

# Ban old edges in comp0
comp0_old_edges = [edge_to_var[e] for e in old_local_edges if e in edge_to_var and (node_to_block[e[0]][0] in comp0_blocks or node_to_block[e[1]][0] in comp0_blocks)]
solver.add_clause([-v for v in comp0_old_edges])

res = solver.solve()
print(f"Cluster 1586..1588 solver result: {res}")
if res:
    model = set(solver.get_model())
    active_edges = set(var_to_edge[v] for v in model if v > 0 and v in var_to_edge)
    added = active_edges - old_local_edges
    removed = old_local_edges - active_edges
    print(f"Added {len(added)}, removed {len(removed)}")
    test_edges = (edges - removed) | added
    new_cycs, _ = get_cycles_from_edges(test_edges, blocks, node_to_block)
    print(f"Cycles: {len(cycs)} -> {len(new_cycs)}, Giant len: {len(cycs[giant_idx])//2} -> {len(new_cycs[0])//2}")
