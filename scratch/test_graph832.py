import sys, time
sys.path.insert(0, ".")
from scratch.engine.graph_loader import load_dimacs
from scratch.engine.comp0_solver import solve_comp0_core

col_path = "FHCPCS-col/graph832.col"
t0 = time.time()
print(f"[*] Loading {col_path}...")
G = load_dimacs(col_path)
print(f"[*] Loaded: {len(G)} nodes. Running solve_comp0_core...")
try:
    cycle = solve_comp0_core(G, set(G.keys()), [])
    print(f"[*] Graph 832 solved successfully in {time.time()-t0:.2f}s! Cycle length: {len(cycle)}")
    import os
    from scratch.engine.assembler import export_hcp_tour
    out_path = "scratch/graph832/found_tour_graph832.hcp"
    export_hcp_tour(cycle, "graph832.col", out_path)
    print(f"[✓] Successfully wrote verified tour to {out_path}!")
except Exception as e:
    import traceback
    traceback.print_exc()
    print(f"[!] Error on graph832: {e}")
