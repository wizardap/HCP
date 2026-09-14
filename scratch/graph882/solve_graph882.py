import json, os, sys, time
from typing import Dict, List, Set, Tuple

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from scratch.graph882.decomposer import load_and_decompose_graph882
from scratch.graph882.chain_solver import solve_chains
from scratch.graph882.comp0_solver import solve_comp0

def assemble_tour(comp0_cycle: List[int], chain1: List[int], chain2: List[int]) -> List[int]:
    """
    Slices the two linear corridors into the Comp 0 Hamiltonian cycle.
    - Chain 1 (170v): endpoints 5200 (connects to 2080) and 4509 (connects to 2117).
    - Chain 2 (15v): endpoints 4779 (connects to 5066) and 893 (connects to 3811).
    Returns full 5,686-vertex Hamiltonian cycle.
    """
    n = len(comp0_cycle)
    tour = []
    i = 0
    while i < n:
        u = comp0_cycle[i]
        v = comp0_cycle[(i + 1) % n]
        tour.append(u)

        # Check for virtual edge 1: (2080, 2117)
        if {u, v} == {2080, 2117}:
            if u == 2080:  # 2080 -> chain1 (5200..4509) -> 2117
                c1 = chain1 if chain1[0] == 5200 else list(reversed(chain1))
            else:          # 2117 -> chain1 (4509..5200) -> 2080
                c1 = chain1 if chain1[0] == 4509 else list(reversed(chain1))
            tour.extend(c1)

        # Check for virtual edge 2: (3811, 5066)
        elif {u, v} == {3811, 5066}:
            if u == 3811:  # 3811 -> chain2 (893..4779) -> 5066
                c2 = chain2 if chain2[0] == 893 else list(reversed(chain2))
            else:          # 5066 -> chain2 (4779..893) -> 3811
                c2 = chain2 if chain2[0] == 4779 else list(reversed(chain2))
            tour.extend(c2)

        i += 1

    assert len(tour) == 5686, f"Expected 5,686 vertices in tour, got {len(tour)}"
    assert len(set(tour)) == 5686, "Duplicate vertices found in assembled tour!"
    return tour

def export_hcp_tour(tour: List[int], output_path: str):
    os.makedirs(os.path.dirname(os.path.abspath(output_path)), exist_ok=True)
    with open(output_path, "w") as f:
        f.write(f"NAME : graph882.tour\n")
        f.write(f"TYPE : TOUR\n")
        f.write(f"DIMENSION : {len(tour)}\n")
        f.write(f"TOUR_SECTION\n")
        for v in tour:
            f.write(f"{v}\n")
        f.write("-1\n")
        f.write("EOF\n")
    print(f"[✓] Tour exported to {output_path}")

def main():
    col_path = os.path.join(repo_root, "FHCPCS-col/graph882.col")
    out_tour = os.path.join(os.path.dirname(__file__), "found_tour_graph882.hcp")

    G, mod_nodes, block2_nodes, chain_nodes, comp0_nodes = load_and_decompose_graph882(col_path)

    print("[*] Stage 1: Loading/Solving chains...")
    chain1, chain2 = solve_chains(G, mod_nodes, block2_nodes)

    print("[*] Stage 2: Loading/Solving Comp 0 cycle...")
    comp0_cycle = solve_comp0(G, comp0_nodes)

    print("[*] Stage 3: Assembling full tour...")
    tour = assemble_tour(comp0_cycle, chain1, chain2)
    export_hcp_tour(tour, out_tour)

if __name__ == "__main__":
    main()
