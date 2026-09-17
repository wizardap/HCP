import collections, time, pickle
from pysat.solvers import Cadical195
from solve_class1_dp_bitmask import load_graph, setup_stage1_contraction, acquire_giant_backbone, decompose_subcycle_components

print("Loading graph868...")
G, degs = load_graph('FHCPCS-col/graph868.col')
blocks, node_to_block_end = setup_stage1_contraction(G, degs)
with open('scratch/graph868_giant_1686.pkl', 'rb') as f:
    edges = set(tuple(sorted(e)) for e in pickle.load(f)['edges'])

color = {}
color[blocks[0][1]] = 0
color[blocks[0][3]] = 1
q = [blocks[0][1], blocks[0][3]]
while q:
    curr = q.pop(0)
    c = color[curr]
    b_id, end_type = node_to_block_end[curr]
    other_node = blocks[b_id][3] if end_type == 'u' else blocks[b_id][1]
    if other_node not in color:
        color[other_node] = 1 - c
        q.append(other_node)
    for nbr in G[curr]:
        if nbr in node_to_block_end:
            if nbr not in color:
                color[nbr] = 1 - c
                q.append(nbr)

# Build all directed arcs:
arc_to_var = {}
var_to_arc = {}
v_cnt = 0
dir_out = collections.defaultdict(list)
dir_in = collections.defaultdict(list)

for u in color:
    if color[u] == 1:
        b1, _ = node_to_block_end[u]
        for v in G[u]:
            if v in node_to_block_end and color[v] == 0:
                b2, _ = node_to_block_end[v]
                arc = (b1, b2)
                if arc not in arc_to_var:
                    v_cnt += 1
                    arc_to_var[arc] = v_cnt
                    var_to_arc[v_cnt] = arc
                    dir_out[b1].append(v_cnt)
                    dir_in[b2].append(v_cnt)

print(f"Total directed arcs: {len(arc_to_var)}")

# Map initial edges to directed arcs
init_arcs = set()
init_succ = {}
for (u, v) in edges:
    b1, _ = node_to_block_end[u]
    b2, _ = node_to_block_end[v]
    if color[u] == 1 and color[v] == 0:
        arc = (b1, b2)
    else:
        arc = (b2, b1)
    init_arcs.add(arc)
    init_succ[arc[0]] = arc[1]

# Base solver clauses
base_clauses = []
for b in range(len(blocks)):
    outs = dir_out[b]
    base_clauses.append(outs)
    for i in range(len(outs)):
        for j in range(i + 1, len(outs)):
            base_clauses.append([-outs[i], -outs[j]])
            
    ins = dir_in[b]
    base_clauses.append(ins)
    for i in range(len(ins)):
        for j in range(i + 1, len(ins)):
            base_clauses.append([-ins[i], -ins[j]])

# 2-cycle mutex
for (u, v), var in arc_to_var.items():
    if u < v and (v, u) in arc_to_var:
        base_clauses.append([-var, -arc_to_var[(v, u)]])

print(f"Base CNF clauses: {len(base_clauses)}")

edges_dummy, cycs, port_nbr, giant_idx = acquire_giant_backbone(blocks, node_to_block_end, edges)
comps = decompose_subcycle_components(G, blocks, node_to_block_end, cycs, giant_idx)
rem_subs = [cycs[i] for i in range(len(cycs)) if i != giant_idx]

# Map subcycle to directed arcs
sub_arcs = []
for sc in rem_subs:
    sc_arcs = []
    for p in sc:
        b = p[0]
        nxt_b = init_succ[b]
        sc_arcs.append((b, nxt_b))
    sub_arcs.append(sc_arcs)

print(f"Testing route generation on all 20 components...")
routes_found = {}

for c_id in range(len(comps)):
    t0 = time.time()
    comp = comps[c_id]
    
    comp_blocks = set()
    for sid in comp:
        for arc in sub_arcs[sid - 1]:
            comp_blocks.add(arc[0])
            comp_blocks.add(arc[1])
            
    other_comp_blocks = set()
    for other_cid, other_c in enumerate(comps):
        if other_cid != c_id:
            for sid in other_c:
                for arc in sub_arcs[sid - 1]:
                    other_comp_blocks.add(arc[0])
                    
    solver = Cadical195()
    for cl in base_clauses:
        solver.add_clause(cl)
        
    for b in other_comp_blocks:
        solver.add_clause([arc_to_var[(b, init_succ[b])]])
        
    for sid in comp:
        solver.add_clause([-arc_to_var[arc] for arc in sub_arcs[sid - 1]])
        
    hints = [arc_to_var[arc] for arc in init_arcs]
    solver.set_phases(hints)
    
    cegar_it = 0
    found_route = None
    while cegar_it < 30:
        cegar_it += 1
        if not solver.solve():
            break
            
        model = set(solver.get_model())
        active_arcs = set(var_to_arc[v] for v in model if v > 0 and v in var_to_arc)
        
        succ = {u: v for (u, v) in active_arcs}
        
        vis = set()
        sol_cycs = []
        for start in range(len(blocks)):
            if start not in vis:
                c = []
                curr = start
                while curr not in vis:
                    vis.add(curr)
                    c.append(curr)
                    curr = succ[curr]
                sol_cycs.append(c)
                
        added_arcs = active_arcs - init_arcs
        removed_arcs = init_arcs - active_arcs
        
        spurious_cycs = []
        for c in sol_cycs:
            c_arcs = set((c[i], c[(i+1)%len(c)]) for i in range(len(c)))
            if c_arcs & added_arcs:
                if cycs[giant_idx][0][0] not in c:
                    spurious_cycs.append(c)
                    
        if not spurious_cycs:
            dt = time.time() - t0
            print(f"Comp {c_id:2d} ({len(comp_blocks):2d} blocks): SUCCESS in {dt:.3f}s (CEGAR iters={cegar_it}, added={len(added_arcs)}, removed={len(removed_arcs)}, cycles={len(sol_cycs)})")
            found_route = {
                'c_id': c_id,
                'added': added_arcs,
                'removed': removed_arcs,
                'cegar_iters': cegar_it
            }
            break
        else:
            for c in spurious_cycs:
                bad_added = [(c[i], c[(i+1)%len(c)]) for i in range(len(c)) if (c[i], c[(i+1)%len(c)]) in added_arcs]
                solver.add_clause([-arc_to_var[arc] for arc in bad_added])
                
    if found_route is None:
        print(f"Comp {c_id:2d} ({len(comp_blocks):2d} blocks): FAILED (UNSAT or CEGAR limit)")
    else:
        routes_found[c_id] = found_route

print(f"\nSummary: Found routes for {len(routes_found)} / {len(comps)} components.")
