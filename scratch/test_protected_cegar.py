#!/usr/bin/env python3
import sys
sys.path.insert(0, '.')
import collections, time, pickle
from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType

from scratch.solve_graph788_dp_bitmask import load_graph, setup_stage1, acquire_giant_backbone, get_cycles_from_edges
from scratch.explore_splicer import find_step_move, get_aux_edges

G, degs = load_graph('FHCPCS-col/graph788.col')
blocks, node_to_block_end, _, _, _, _, _, _, _ = setup_stage1(G, degs)
edges, cycs, port_nbr, giant_idx = acquire_giant_backbone(blocks, node_to_block_end)

# Advance to 1456
for step in range(4):
    cycs, port_nbr = get_cycles_from_edges(edges, blocks, node_to_block_end)
    giant_idx = max(range(len(cycs)), key=lambda i: len(cycs[i]))
    giant_len = len(cycs[giant_idx]) // 2
    giant_ports = set(cycs[giant_idx])
    subcycles = [cycs[i] for i in range(len(cycs)) if i != giant_idx]
    port_to_cid = {p: 0 for p in giant_ports}
    for idx, sc in enumerate(subcycles):
        for p in sc: port_to_cid[p] = idx + 1
    port_nbr_dict, aux = get_aux_edges(edges)
    non_giant_ports = [p for p in port_nbr_dict if port_to_cid[p] != 0]
    move = find_step_move(edges, giant_len, len(cycs), non_giant_ports, aux)
    edges, red, n_g, d, mtype = move

print("Reached 1456 state.")

# Build SAT variables for external edges
edge_to_var = {}
var_to_edge = {}
var_cnt = 0
adj_external = collections.defaultdict(list)
block_adj_all = collections.defaultdict(list)

for u in node_to_block_end:
    b1, e1 = node_to_block_end[u]
    for v in G[u]:
        if v in node_to_block_end and v > u:
            b2, e2 = node_to_block_end[v]
            var_cnt += 1
            edge_to_var[(u, v)] = var_cnt
            var_to_edge[var_cnt] = (u, v)
            adj_external[(b1, e1)].append(var_cnt)
            adj_external[(b2, e2)].append(var_cnt)
            block_adj_all[b1].append((b2, var_cnt, (u, v)))
            block_adj_all[b2].append((b1, var_cnt, (u, v)))

solver = Cadical195()
for (b_id, end_type), var_list in adj_external.items():
    clauses = CardEnc.equals(lits=var_list, bound=1, top_id=var_cnt, encoding=EncType.seqcounter)
    for cl in clauses:
        solver.add_clause(cl)
        for lit in cl:
            if abs(lit) > var_cnt: var_cnt = abs(lit)

# Invariant mutexes (2-cycles and 3-cycles)
# 2-cycle mutex
for (u, v), var1 in edge_to_var.items():
    b1, _ = node_to_block_end[u]
    b2, _ = node_to_block_end[v]
    # check if there is another edge between b1 and b2
    for b_other, var2, _ in block_adj_all[b1]:
        if b_other == b2 and var2 > var1:
            solver.add_clause([-var1, -var2])

print(f"Base SAT initialized with {var_cnt} variables.")

# Set phase saving based on current 1456 edges!
phases = []
for (u, v), var in edge_to_var.items():
    e_sorted = tuple(sorted((u, v)))
    if e_sorted in edges:
        phases.append(var)
    else:
        phases.append(-var)

solver.set_phases(phases)

# Run Protected CEGAR
t0 = time.time()
print("Starting Protected CEGAR from 1456 state...")
for it in range(1, 100):
    t_start = time.time()
    sat = solver.solve()
    t_sat = time.time() - t_start
    if not sat:
        print(f"UNSAT at iteration {it}!")
        break
    model = set(solver.get_model())
    
    # Extract current edges
    curr_sol_edges = set()
    for (u, v), var in edge_to_var.items():
        if var in model:
            curr_sol_edges.add(tuple(sorted((u, v))))
            
    c_list, _ = get_cycles_from_edges(curr_sol_edges, blocks, node_to_block_end)
    lens = [len(c)//2 for c in c_list]
    giant_c_idx = max(range(len(c_list)), key=lambda i: len(c_list[i]))
    giant_sz = lens[giant_c_idx]
    print(f"It {it:2d} ({t_sat:.3f}s): {len(c_list)} cycles, Giant={giant_sz} ({100*giant_sz/1540:.1f}%), sizes={sorted(lens, reverse=True)[:5]}")
    
    if len(c_list) == 1:
        print(f"*** SOLVED! Single Hamiltonian cycle of {giant_sz} blocks found in {time.time()-t0:.2f}s! ***")
        with open("scratch/graph788_hamiltonian_solution.pkl", "wb") as f:
            pickle.dump({"edges": list(curr_sol_edges)}, f)
        break
        
    # Cut ONLY subcycles, NOT giant!
    cuts_added = 0
    for idx, c in enumerate(c_list):
        if idx == giant_c_idx:
            continue
        c_blocks = set(p[0] for p in c)
        cut_vars = set()
        for b in c_blocks:
            for nxt_b, evar, _ in block_adj_all[b]:
                if nxt_b not in c_blocks:
                    cut_vars.add(evar)
        if cut_vars:
            solver.add_clause(list(cut_vars))
            cuts_added += 1
            
    # Also update phases towards current giant edges
    # solver.set_phases(phases)
