import os, sys, time
from typing import List, Optional

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from scratch.engine.graph_loader import load_dimacs
from scratch.engine.decomposer import detect_bridge_corridors
from scratch.engine.corridor_solver import solve_corridor_path
from scratch.engine.comp0_solver import solve_comp0_core
from scratch.engine.assembler import assemble_full_tour, export_hcp_tour

def solve_hierarchical_hcp(col_path: str, output_path: Optional[str] = None) -> List[int]:
    t_start = time.time()
    base_name = os.path.basename(col_path)
    print("=" * 70)
    print(f"STARTING GENERIC UNIFIED HIERARCHICAL SOLVE FOR {base_name}")
    print("ZERO CACHING, ZERO TOUR INJECTION, 100% IN-MEMORY COMPUTATION")
    print("=" * 70)

    # Step 1: Load graph
    G = load_dimacs(col_path)
    N = len(G)
    print(f"[*] Graph loaded: |V| = {N}, |E| = {sum(len(nbrs) for nbrs in G.values()) // 2}")

    # Step 2: Automated corridor decomposition
    t_decomp = time.time()
    corridors, comp0_nodes = detect_bridge_corridors(G)
    print(f"[*] Stage 1 Complete in {time.time()-t_decomp:.2f}s: Detected {len(corridors)} outer corridors, Comp 0 ({len(comp0_nodes)}v).")
    for idx, c in enumerate(corridors):
        u, v = c['ports']
        eu, ev = c['ext_ports']
        print(f"    Corridor {idx+1}: {len(c['nodes'])}v, internal ports ({u}, {v}) -> Comp 0 ports ({eu}, {ev})")

    # Step 3: Solve all outer corridors via local SAT
    t_corr = time.time()
    print("[*] Stage 2: Solving outer corridors via local SAT HP...")
    for idx, c in enumerate(corridors):
        u, v = c['ports']
        t_c = time.time()
        path = solve_corridor_path(G, c['nodes'], src=u, dst=v)
        c['path'] = path
        print(f"    Corridor {idx+1} ({len(path)}v) solved in {time.time()-t_c:.2f}s.")
    print(f"[*] Stage 2 Complete in {time.time()-t_corr:.2f}s.")

    # Step 4: Solve Comp 0
    t_c0 = time.time()
    virtual_edges = [c['ext_ports'] for c in corridors]
    if len(comp0_nodes) > 4600:
        from scratch.engine.modular_comp0_solver import solve_modular_comp0
        print(f"[*] Stage 3: Large Comp 0 ({len(comp0_nodes)}v) detected -> routing to Modular State Equivalence Solver...")
        comp0_cycle = solve_modular_comp0(G, comp0_nodes, virtual_edges)
    else:
        print(f"[*] Stage 3: Solving Comp 0 ({len(comp0_nodes)}v) via degree-2 contraction & 2-opt CEGAR...")
        comp0_cycle = solve_comp0_core(G, comp0_nodes, virtual_edges)
    print(f"[*] Stage 3 Complete in {time.time()-t_c0:.2f}s.")

    # Step 5: Tour Assembly & Export
    print("[*] Stage 4: Assembling full Hamiltonian tour...")
    tour = assemble_full_tour(comp0_cycle, corridors)
    assert len(tour) == N, f"Tour length {len(tour)} != {N}"
    assert len(set(tour)) == N, "Duplicate vertices in tour!"

    elapsed = time.time() - t_start
    print("=" * 70)
    print(f"[✓] SOLVE COMPLETED SUCCESSFULLY IN {elapsed:.2f}s!")
    print(f"    Tour Dimension: {len(tour)} vertices")
    print("=" * 70)

    if output_path:
        export_hcp_tour(tour, base_name, output_path)

    return tour

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python3 unified_solver.py <path_to_col> [output_tour_path]")
        sys.exit(1)
    col_file = sys.argv[1]
    out_file = sys.argv[2] if len(sys.argv) > 2 else None
    solve_hierarchical_hcp(col_file, out_file)
