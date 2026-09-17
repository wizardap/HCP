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

giant_nbrs = set()
for b in comp0_blocks:
    for u in [blocks[b][1], blocks[b][3]]:
        for v in G[u]:
            if v in node_to_block:
                b2 = node_to_block[v][0]
                if b2 in giant_pos:
                    giant_nbrs.add(b2)

positions = sorted(giant_pos[b] for b in giant_nbrs)

# Include all intermediate giant blocks for close pairs (dist <= 6)
window_giant_blocks = set(giant_nbrs)
for i in range(len(positions)):
    p1 = positions[i]
    p2 = positions[(i + 1) % len(positions)]
    dist = (p2 - p1) % n_g
    if dist <= 6:
        for step in range(dist + 1):
            window_giant_blocks.add(giant_blocks_seq[(p1 + step) % n_g])

local_blocks = comp0_blocks | window_giant_blocks
print(f"Local blocks count: {len(local_blocks)} ({len(comp0_blocks)} comp, {len(window_giant_blocks)} giant)")

# Build variables for all edges within local_blocks
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

print(f"Variables: {v_cnt}")

# Old edges within local_blocks
old_local_edges = set()
for p in comp0_ports:
    partner_p, e = port_nbr[p]
    old_local_edges.add(e)
for gb in window_giant_blocks:
    for end in ['u', 'w']:
        p = (gb, end)
        partner_p, e = port_nbr[p]
        if partner_p[0] in window_giant_blocks:
            old_local_edges.add(e)

print(f"Old local edges: {len(old_local_edges)}")

solver = Cadical195()

# Degree constraints:
# For comp0 blocks: exactly 1 incident edge per port
for p in comp0_ports:
    evars = port_vars[p]
    if evars:
        solver.add_clause(evars)
        for i in range(len(evars)):
            for j in range(i + 1, len(evars)):
                solver.add_clause([-evars[i], -evars[j]])

# For internal window giant blocks: exactly 1 incident edge per port
# For boundary giant blocks (those with an edge outside window): at most 1 incident edge
for gb in window_giant_blocks:
    for end in ['u', 'w']:
        p = (gb, end)
        partner_p, e = port_nbr[p]
        evars = port_vars[p]
        if partner_p[0] in window_giant_blocks:
            # Internal to window: must maintain degree 1
            if evars:
                solver.add_clause(evars)
                for i in range(len(evars)):
                    for j in range(i + 1, len(evars)):
                        solver.add_clause([-evars[i], -evars[j]])
        else:
            # Boundary of window: must have at most 1 edge inside window (to match the external connection)
            # Actually, the external connection is fixed, so inside window it must keep degree 1!
            if evars:
                solver.add_clause(evars)
                for i in range(len(evars)):
                    for j in range(i + 1, len(evars)):
                        solver.add_clause([-evars[i], -evars[j]])

# Must change at least one edge in comp0
comp0_old_edges = [edge_to_var[e] for e in old_local_edges if e in edge_to_var and (node_to_block[e[0]][0] in comp0_blocks or node_to_block[e[1]][0] in comp0_blocks)]
solver.add_clause([-v for v in comp0_old_edges])

res = solver.solve()
print(f"Solver result: {res}")
if res:
    model = set(solver.get_model())
    active_edges = set(var_to_edge[v] for v in model if v > 0 and v in var_to_edge)
    added = active_edges - old_local_edges
    removed = old_local_edges - active_edges
    print(f"Added {len(added)} edges, removed {len(removed)} edges.")
    test_edges = (edges - removed) | added
    new_cycs, _ = get_cycles_from_edges(test_edges, blocks, node_to_block)
    print(f"Original cycles: {len(cycs)} -> New cycles: {len(new_cycs)}")
