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
for si in comp0:
    for p in cycs[si]:
        comp0_blocks.add(p[0])

giant_nbrs = set()
for b in comp0_blocks:
    for u in [blocks[b][1], blocks[b][3]]:
        for v in G[u]:
            if v in node_to_block:
                b2 = node_to_block[v][0]
                if b2 in giant_pos:
                    giant_nbrs.add(b2)

positions = sorted(giant_pos[b] for b in giant_nbrs)
print(f"Comp 0 has {len(comp0_blocks)} blocks, {len(giant_nbrs)} giant neighbors.")
print(f"Giant neighbor positions: {positions}")

# Check close pairs on Giant
close_pairs = []
for i in range(len(positions)):
    p1 = positions[i]
    p2 = positions[(i + 1) % len(positions)]
    dist = (p2 - p1) % n_g
    if dist <= 10:
        close_pairs.append((p1, p2, dist))

print(f"Close pairs (dist <= 10): {close_pairs}")
