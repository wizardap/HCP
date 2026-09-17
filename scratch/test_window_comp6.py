import collections, pickle, time
from scratch.solve_class1_dp_bitmask import load_graph, setup_stage1_contraction, acquire_giant_backbone, get_cycles_from_edges
from pysat.solvers import Cadical195

G, degs = load_graph("FHCPCS-col/graph868.col")
blocks, node_to_block_end = setup_stage1_contraction(G, degs)
with open("scratch/graph868_giant_1686.pkl", "rb") as f:
    edges = set(tuple(sorted(e)) for e in pickle.load(f)["edges"])
edges, cycs, port_nbr, giant_idx = acquire_giant_backbone(blocks, node_to_block_end, edges)
giant_ports = cycs[giant_idx]

# Segment from 2360 to 2570
idx_start = 2360
idx_end = 2570 # exclusive

segment_ports = set(giant_ports[idx_start:idx_end])
segment_blocks = set(p[0] for p in segment_ports)
comp_blocks = {108, 292}
W_blocks = segment_blocks | comp_blocks

W_ports = set()
for b in W_blocks:
    W_ports.add((b, "u"))
    W_ports.add((b, "w"))

p_start = (987, 'w')
p_end = (1007, 'w')

print(f"Window has {len(W_blocks)} blocks ({len(W_ports)} ports).")
print(f"p_start: {p_start} (external: {port_nbr[p_start]}), p_end: {p_end} (external: {port_nbr[p_end]})")

cand_edges = set()
for p1 in W_ports:
    u = blocks[p1[0]][1] if p1[1] == "u" else blocks[p1[0]][3]
    for v in G[u]:
        if v in node_to_block_end:
            p2 = node_to_block_end[v]
            if p2 in W_ports and p1[0] != p2[0]:
                cand_edges.add(tuple(sorted((u, v))))

edge_to_var = {}
var_to_edge = {}
for idx, e in enumerate(sorted(cand_edges), start=1):
    edge_to_var[e] = idx
    var_to_edge[idx] = e

port_vars = collections.defaultdict(list)
for e in cand_edges:
    p1 = node_to_block_end[e[0]]
    p2 = node_to_block_end[e[1]]
    port_vars[p1].append(edge_to_var[e])
    port_vars[p2].append(edge_to_var[e])

solver = Cadical195()
for p in W_ports:
    evars = port_vars[p]
    if p == p_start or p == p_end:
        for v in evars:
            solver.add_clause([-v])
    else:
        solver.add_clause(evars)
        for i in range(len(evars)):
            for j in range(i + 1, len(evars)):
                solver.add_clause([-evars[i], -evars[j]])

# Ban 2-cycles
block_pair_vars = collections.defaultdict(list)
for e in cand_edges:
    b1 = node_to_block_end[e[0]][0]
    b2 = node_to_block_end[e[1]][0]
    bp = tuple(sorted((b1, b2)))
    block_pair_vars[bp].append(edge_to_var[e])
for bp, vlist in block_pair_vars.items():
    if len(vlist) > 1:
        for i in range(len(vlist)):
            for j in range(i + 1, len(vlist)):
                solver.add_clause([-vlist[i], -vlist[j]])

old_window_edges = set()
for p in W_ports:
    if p != p_start and p != p_end:
        partner_p, e = port_nbr[p]
        if partner_p in W_ports:
            old_window_edges.add(e)

print(f"Old window edges: {len(old_window_edges)}, expected {len(segment_blocks) - 1 + len(comp_blocks)}")

t0 = time.time()
for it in range(1, 100):
    if not solver.solve():
        print(f"UNSAT at iteration {it} in {time.time() - t0:.4f}s")
        break
    model = set(solver.get_model())
    active_cand = set(var_to_edge[v] for v in model if v > 0 and v in var_to_edge)
    test_edges = (edges - old_window_edges) | active_cand
    assert len(test_edges) == len(edges), f"Edge count mismatch: {len(test_edges)} vs {len(edges)}"
    
    new_cycs, test_port_nbr = get_cycles_from_edges(test_edges, blocks, node_to_block_end)
    lens = [len(c)//2 for c in new_cycs]
    giant_len = max(lens)
    print(f"It {it:2} ({time.time() - t0:.3f}s): cycles={len(new_cycs)}, Giant={giant_len}")
    
    if len(new_cycs) < len(cycs):
        print(f"  >>> SUCCESS! Absorbed Comp 6 in iteration {it}!")
        print(f"      Cycles: {len(cycs)} -> {len(new_cycs)}, Giant: {len(giant_ports)//2} -> {giant_len}")
        break
        
    found_cut = False
    for c in new_cycs:
        c_blocks = [p[0] for p in c[::2]]
        if len(c_blocks) < len(blocks) and all(b in W_blocks for b in c_blocks):
            c_edges = []
            for i in range(len(c)//2):
                p_curr = c[2*i]
                e = test_port_nbr[p_curr][1]
                if e in edge_to_var:
                    c_edges.append(edge_to_var[e])
            if c_edges:
                solver.add_clause([-x for x in c_edges])
                found_cut = True
    if not found_cut:
        print("  No internal cut found.")
        break
