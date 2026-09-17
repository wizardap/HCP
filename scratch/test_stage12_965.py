import sys, time
sys.path.insert(0, ".")
from scratch.engine.graph_loader import load_dimacs
from scratch.engine.decomposer import detect_bridge_corridors
from scratch.engine.corridor_solver import solve_corridor_path

col_path = "FHCPCS-col/graph965.col"
t0 = time.time()
print(f"[*] Loading {col_path}...")
G = load_dimacs(col_path)
corridors, comp0 = detect_bridge_corridors(G)
print(f"[*] Decomposed in {time.time()-t0:.2f}s: {len(corridors)} corridors, Comp 0 ({len(comp0)}v).")
for idx, c in enumerate(corridors):
    u, v = c['ports']
    eu, ev = c['ext_ports']
    print(f"    Corridor {idx+1}: {len(c['nodes'])}v, internal ports ({u}, {v}) -> Comp 0 ports ({eu}, {ev})")
    t_c = time.time()
    path = solve_corridor_path(G, c['nodes'], src=u, dst=v)
    print(f"    Corridor {idx+1} ({len(path)}v) solved in {time.time()-t_c:.2f}s!")
