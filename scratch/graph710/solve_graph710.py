import json, os, sys, time
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "../..")))
from scratch.graph710.decomposer import load_and_decompose_graph710
from scratch.graph710.solve_blocks_parallel import solve_both_blocks_parallel

def assemble_and_verify_tour():
    t0 = time.time()
    base_dir = os.path.dirname(os.path.abspath(__file__))
    repo_root = os.path.abspath(os.path.join(base_dir, "../.."))
    col_path = os.path.join(repo_root, "FHCPCS-col/graph710.col")
    if not os.path.exists(col_path):
        col_path = "FHCPCS-col/graph710.col"
    cache_path = os.path.join(base_dir, "block_paths.json")
    tour_path = os.path.join(base_dir, "found_tour_graph710.hcp")

    G, V_A, V_B, port_u, port_v = load_and_decompose_graph710(col_path)
    paths = solve_both_blocks_parallel(col_path, cache_path)

    P_A = paths["A"]  # 1876 -> ... -> 2491 (887 vertices)
    P_B = paths["B"]  # 2491 -> ... -> 1876 (3179 vertices)

    assert P_A[0] == port_u and P_A[-1] == port_v
    assert P_B[0] == port_v and P_B[-1] == port_u

    # Assemble tour: omit last element of each path to avoid duplicating cut vertices
    tour = P_A[:-1] + P_B[:-1]
    assert len(tour) == 4064
    assert len(set(tour)) == 4064
    assert set(tour) == set(G.keys())

    # Verify all edges along the cycle exist in raw G
    for i in range(len(tour)):
        u = tour[i]
        v = tour[(i + 1) % len(tour)]
        assert v in G[u], f"Phantom edge in assembled tour: ({u}, {v})"

    print(f"Tour assembly verified sound: 4064 unique vertices, 4064 valid DIMACS edges!")

    # Write HCP file
    with open(tour_path, "w") as f:
        f.write("NAME : graph710.col\n")
        f.write("TYPE : TOUR\n")
        f.write(f"DIMENSION : {len(tour)}\n")
        f.write("TOUR_SECTION\n")
        for node in tour:
            f.write(f"{node}\n")
        f.write("-1\n")
        f.write("EOF\n")

    print(f"Tour written to {tour_path} in {time.time()-t0:.2f}s.")
    return tour_path

if __name__ == "__main__":
    assemble_and_verify_tour()
