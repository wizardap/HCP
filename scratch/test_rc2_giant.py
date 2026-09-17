import sys; sys.path.insert(0, '.')
import time, collections
from pysat.formula import WCNF
from pysat.examples.rc2 import RC2
from scratch.solve_graph788_dp_bitmask import load_graph, setup_stage1, acquire_giant_backbone

print("Loading graph...")
t0 = time.time()
G, degs = load_graph('FHCPCS-col/graph788.col')
blocks, node_to_block_end, _, _, _, _, _, _, _ = setup_stage1(G, degs)

# Exact 2-coloring for orientation
v_partner = {blocks[b][1]: blocks[b][3] for b in range(len(blocks))}
for b in range(len(blocks)): v_partner[blocks[b][3]] = blocks[b][1]
color = {blocks[0][1]: 0, v_partner[blocks[0][1]]: 1}
q = [blocks[0][1], v_partner[blocks[0][1]]]
while q:
    curr = q.pop(); curr_c = color[curr]; vp = v_partner[curr]
    if vp not in color: color[vp] = 1 - curr_c; q.append(vp)
    for nxt in G[curr]:
        if nxt in v_partner and nxt != vp and nxt not in color:
            color[nxt] = 1 - curr_c; q.append(nxt)

node_to_bid = {blocks[b][1]: b for b in range(len(blocks))}
for b in range(len(blocks)): node_to_bid[blocks[b][3]] = b

block_in = {b: blocks[b][1] if color[blocks[b][1]] == 0 else blocks[b][3] for b in range(len(blocks))}
block_out = {b: blocks[b][3] if color[blocks[b][1]] == 0 else blocks[b][1] for b in range(len(blocks))}

dir_arcs = []
arc_to_var = {}
var_to_arc = {}
var_id = 0
out_arcs = collections.defaultdict(list)
in_arcs = collections.defaultdict(list)

for b1 in range(len(blocks)):
    u_out = block_out[b1]
    for v_in in G[u_out]:
        if v_in in node_to_bid and v_in != block_in[b1]:
            b2 = node_to_bid[v_in]
            var_id += 1
            arc = (b1, b2)
            dir_arcs.append(arc)
            arc_to_var[arc] = var_id
            var_to_arc[var_id] = arc
            out_arcs[b1].append(var_id)
            in_arcs[b2].append(var_id)

print(f"Directed block graph: {len(blocks)} blocks, {len(dir_arcs)} arcs, built in {time.time()-t0:.2f}s")

wcnf = WCNF()

# Hard constraints: Degree-1 in and out
for b in range(len(blocks)):
    o_vars = out_arcs[b]
    wcnf.append(o_vars)
    for i in range(len(o_vars)):
        for j in range(i+1, len(o_vars)):
            wcnf.append([-o_vars[i], -o_vars[j]])
    i_vars = in_arcs[b]
    wcnf.append(i_vars)
    for i in range(len(i_vars)):
        for j in range(i+1, len(i_vars)):
            wcnf.append([-i_vars[i], -i_vars[j]])

# Hard constraints: 2-cycle mutexes
m2_count = 0
for (b1, b2) in dir_arcs:
    if b1 < b2 and (b2, b1) in arc_to_var:
        wcnf.append([-arc_to_var[(b1, b2)], -arc_to_var[(b2, b1)]])
        m2_count += 1

# Hard constraints: 3-cycle mutexes
m3_count = 0
for (b1, b2) in dir_arcs:
    v12 = arc_to_var[(b1, b2)]
    for v23 in out_arcs[b2]:
        b3 = var_to_arc[v23][1]
        if b3 != b1 and (b3, b1) in arc_to_var:
            if b1 < b2 and b1 < b3:
                v31 = arc_to_var[(b3, b1)]
                wcnf.append([-v12, -v23, -v31])
                m3_count += 1

# Soft constraints: Backbone arcs (weight = 10)
edges_init, cycs_init, _, giant_idx = acquire_giant_backbone(blocks, node_to_block_end)
giant_ports = cycs_init[giant_idx]
giant_order = [giant_ports[i][0] for i in range(0, len(giant_ports), 2)]
giant_arcs = set()
for (u, v) in edges_init:
    b1 = node_to_bid[u]; b2 = node_to_bid[v]
    if (b1, b2) in arc_to_var: giant_arcs.add((b1, b2))
    elif (b2, b1) in arc_to_var: giant_arcs.add((b2, b1))

for arc in giant_arcs:
    wcnf.append([arc_to_var[arc]], weight=10)

print(f"Built WCNF: {len(wcnf.hard)} hard clauses, {len(wcnf.soft)} soft clauses.")

print("Starting Incremental RC2 CEGAR...")
t_start = time.time()
with RC2(wcnf, solver='cadical195') as rc2:
    for round in range(1, 100):
        t_r = time.time()
        model = rc2.compute()
        cost = rc2.cost
        dt = time.time() - t_r
        if model is None:
            print("UNSAT in MaxSAT!")
            break
        model_set = set(model)
        
        succ = {}
        for arc, vid in arc_to_var.items():
            if vid in model_set:
                succ[arc[0]] = arc[1]
                
        visited = set()
        cycles = []
        for b in range(len(blocks)):
            if b not in visited:
                c = []
                curr = b
                while curr not in visited:
                    visited.add(curr)
                    c.append(curr)
                    curr = succ[curr]
                cycles.append(c)
        cycles.sort(key=len, reverse=True)
        print(f"Round {round:2d} ({dt:.2f}s, cost={cost}): {len(cycles)} cycles. Lens: {[len(c) for c in cycles[:10]]}")
        
        if len(cycles) == 1:
            print(f"\n*** 100% HAMILTONIAN TOUR FOUND IN {round} ROUNDS ({time.time()-t_start:.2f}s)! ***")
            break
            
        N = len(blocks)
        for c in cycles:
            if len(c) < N:
                # Negative cycle clause
                rc2.add_clause([-arc_to_var[(c[i], c[(i+1)%len(c)])] for i in range(len(c))])
                # Dual boundary cuts
                c_small = c if len(c) <= N // 2 else [x for x in range(N) if x not in set(c)]
                c_set = set(c_small)
                out_cut = [arc_to_var[(u, v)] for u in c_small for v in [var_to_arc[vid][1] for vid in out_arcs[u]] if v not in c_set]
                if out_cut: rc2.add_clause(out_cut)
                in_cut = [arc_to_var[(u, v)] for v in c_small for u in [var_to_arc[vid][0] for vid in in_arcs[v]] if u not in c_set]
                if in_cut: rc2.add_clause(in_cut)
