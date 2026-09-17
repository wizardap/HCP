import sys; sys.path.insert(0, '.')
import time, collections
from pysat.solvers import Cadical195
from scratch.solve_graph788_dp_bitmask import load_graph, setup_stage1, acquire_giant_backbone, get_cycles_from_edges

print("Loading graph...")
t0 = time.time()
G, degs = load_graph('FHCPCS-col/graph788.col')
blocks, node_to_block_end, edge_to_var, var_to_edge, var_cnt, adj_external, block_adj_all, m2, m3 = setup_stage1(G, degs)

# Check 2-coloring
v_partner = {}
for b_id, u, v, w in blocks:
    v_partner[u] = w
    v_partner[w] = u

color = {}
u0 = blocks[0][1]
color[u0] = 0
color[v_partner[u0]] = 1
q = [u0, v_partner[u0]]
while q:
    curr = q.pop()
    curr_c = color[curr]
    vp = v_partner[curr]
    if vp not in color:
        color[vp] = 1 - curr_c
        q.append(vp)
    for nxt in G[curr]:
        if nxt in v_partner and nxt != vp and nxt not in color:
            color[nxt] = 1 - curr_c
            q.append(nxt)

assert len(color) == len(blocks) * 2

node_to_bid = {blocks[b][1]: b for b in range(len(blocks))}
for b in range(len(blocks)):
    node_to_bid[blocks[b][3]] = b

block_in = {}
block_out = {}
for b in range(len(blocks)):
    u, w = blocks[b][1], blocks[b][3]
    if color[u] == 0:
        block_in[b] = u
        block_out[b] = w
    else:
        block_in[b] = w
        block_out[b] = u

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

solver = Cadical195()
# Degree constraints
for b in range(len(blocks)):
    o_vars = out_arcs[b]
    solver.add_clause(o_vars)
    for i in range(len(o_vars)):
        for j in range(i+1, len(o_vars)):
            solver.add_clause([-o_vars[i], -o_vars[j]])
    i_vars = in_arcs[b]
    solver.add_clause(i_vars)
    for i in range(len(i_vars)):
        for j in range(i+1, len(i_vars)):
            solver.add_clause([-i_vars[i], -i_vars[j]])

# 2-cycles mutex
m2_count = 0
for (b1, b2) in dir_arcs:
    if b1 < b2 and (b2, b1) in arc_to_var:
        solver.add_clause([-arc_to_var[(b1, b2)], -arc_to_var[(b2, b1)]])
        m2_count += 1
print(f"Added degree constraints and {m2_count} 2-cycle mutexes.")

# Pre-inject 3-cycles
m3_count = 0
for (b1, b2) in dir_arcs:
    v12 = arc_to_var[(b1, b2)]
    for v23 in out_arcs[b2]:
        b3 = var_to_arc[v23][1]
        if b3 != b1 and (b3, b1) in arc_to_var:
            if b1 < b2 and b1 < b3:
                v31 = arc_to_var[(b3, b1)]
                solver.add_clause([-v12, -v23, -v31])
                m3_count += 1
print(f"Pre-injected {m3_count} 3-cycle mutexes.")

# Warm start phases from backbone
edges_init, _, _, _ = acquire_giant_backbone(blocks, node_to_block_end)
phase_lits = []
for (u, v) in edges_init:
    b1 = node_to_bid[u]
    b2 = node_to_bid[v]
    if (b1, b2) in arc_to_var:
        phase_lits.append(arc_to_var[(b1, b2)])
    elif (b2, b1) in arc_to_var:
        phase_lits.append(arc_to_var[(b2, b1)])

solver.set_phases(phase_lits)
print(f"Set phases for {len(phase_lits)} arcs from backbone.")

for round in range(1, 15):
    t_r = time.time()
    sat = solver.solve()
    assert sat
    model = set(solver.get_model())
    
    succ = {}
    for arc, vid in arc_to_var.items():
        if vid in model:
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
    dt = time.time() - t_r
    print(f"Round {round:2d} ({dt:.3f}s): {len(cycles)} cycles (largest={len(cycles[0])}, smallest={len(cycles[-1])})")
    if len(cycles) == 1:
        print("FOUND HAMILTONIAN CYCLE!")
        break
        
    for c in cycles:
        if len(c) < len(blocks):
            solver.add_clause([-arc_to_var[(c[i], c[(i+1)%len(c)])] for i in range(len(c))])
            if len(c) <= 64:
                c_set = set(c)
                out_cut = [arc_to_var[(u, v)] for u in c for v in [var_to_arc[vid][1] for vid in out_arcs[u]] if v not in c_set]
                if out_cut: solver.add_clause(out_cut)
                in_cut = [arc_to_var[(u, v)] for v in c for u in [var_to_arc[vid][0] for vid in in_arcs[v]] if u not in c_set]
                if in_cut: solver.add_clause(in_cut)
