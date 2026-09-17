import sys, pickle, collections
sys.path.append("scratch")
from solve_class1_dp_bitmask import load_graph, setup_stage1_contraction, acquire_giant_backbone, decompose_subcycle_components

with open("scratch/graph868_giant_1686.pkl", "rb") as f:
    data = pickle.load(f)
edges = data["edges"]

G, degs = load_graph("FHCPCS-col/graph868.col")
blocks, node_to_block = setup_stage1_contraction(G, degs)
edges, cycs, port_nbr, giant_idx = acquire_giant_backbone(blocks, node_to_block, edges)

comps = decompose_subcycle_components(G, blocks, node_to_block, cycs, giant_idx)
giant = cycs[giant_idx]
giant_blocks = set(p[0] for p in giant)
giant_pos = {}
for i, p in enumerate(giant):
    if p[0] not in giant_pos:
        giant_pos[p[0]] = i

print(f"Loaded {len(comps)} components.")

# For each component, find which giant blocks it connects to
for c_idx, comp in enumerate(comps):
    c_blocks = set()
    for si in comp:
        for p in cycs[si]:
            c_blocks.add(p[0])
    
    giant_nbrs = set()
    for b in c_blocks:
        for u in [blocks[b][1], blocks[b][3]]:
            for v in G[u]:
                if v in node_to_block:
                    b2 = node_to_block[v][0]
                    if b2 in giant_blocks:
                        giant_nbrs.add(b2)
    
    print(f"Comp {c_idx:2d} ({len(c_blocks):2d} blocks, {len(comp)} cycs): {len(giant_nbrs)} giant neighbors")
