import json, os, sys, time
from typing import Dict, List, Set, Tuple

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from scratch.graph677.decomposer import load_and_decompose_graph677
from scratch.graph677.chain_solver import solve_module_corridor
from scratch.graph677.comp0_solver import solve_comp0

def assemble_tour(comp0_cycle: List[int], mod_path: List[int], port1: int, port2: int, p1_ext: int, p2_ext: int, expected_len: int = 3868) -> List[int]:
    """
    Slices the linear module corridor into the Comp 0 Hamiltonian cycle.
    - Module path: endpoints port1 (connects to p1_ext) and port2 (connects to p2_ext).
    - Virtual edge in Comp 0: (p1_ext, p2_ext).
    Returns full Hamiltonian cycle.
    """
    n = len(comp0_cycle)
    tour = []
    virt_found = False
    for i in range(n):
        u = comp0_cycle[i]
        v = comp0_cycle[(i + 1) % n]
        tour.append(u)

        if {u, v} == {p1_ext, p2_ext}:
            assert not virt_found, "Multiple virtual edges encountered!"
            virt_found = True
            if u == p1_ext:
                c = mod_path if mod_path[0] == port1 else list(reversed(mod_path))
            else:
                c = mod_path if mod_path[0] == port2 else list(reversed(mod_path))
            tour.extend(c)

    assert virt_found, f"Virtual edge ({p1_ext}, {p2_ext}) not found in Comp 0 cycle!"
    assert len(tour) == expected_len, f"Expected {expected_len} vertices in tour, got {len(tour)}"
    assert len(set(tour)) == expected_len, "Duplicate vertices found in assembled tour!"
    return tour

def export_hcp_tour(tour: List[int], output_path: str):
    os.makedirs(os.path.dirname(os.path.abspath(output_path)), exist_ok=True)
    with open(output_path, "w") as f:
        f.write(f"NAME : graph677.tour\n")
        f.write(f"TYPE : TOUR\n")
        f.write(f"DIMENSION : {len(tour)}\n")
        f.write(f"TOUR_SECTION\n")
        for v in tour:
            f.write(f"{v}\n")
        f.write("-1\n")
        f.write("EOF\n")
    print(f"[✓] Tour exported to {output_path}")

def main():
    col_path = os.path.join(repo_root, "FHCPCS-col/graph677.col")
    out_tour = os.path.join(os.path.dirname(__file__), "found_tour_graph677.hcp")

    G, mod_nodes, comp0_nodes, (port1, port2), (p1_ext, p2_ext) = load_and_decompose_graph677(col_path)

    print("[*] Stage 1: Solving module corridor...")
    mod_path = solve_module_corridor(G, mod_nodes, port1, port2)

    print("[*] Stage 2: Solving Comp 0 cycle...")
    comp0_cycle = solve_comp0(G, comp0_nodes, p1_ext, p2_ext)

    print("[*] Stage 3: Assembling full tour...")
    tour = assemble_tour(comp0_cycle, mod_path, port1, port2, p1_ext, p2_ext)
    export_hcp_tour(tour, out_tour)

if __name__ == "__main__":
    main()
