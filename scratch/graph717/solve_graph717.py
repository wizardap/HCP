import collections, json, os, sys, time
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "../..")))
from scratch.graph717.decomposer import load_and_decompose_graph717

def assemble_and_verify_tour():
    t0 = time.time()
    base_dir = os.path.dirname(os.path.abspath(__file__))
    repo_root = os.path.abspath(os.path.join(base_dir, "../.."))
    col_path = os.path.join(repo_root, "FHCPCS-col/graph717.col")
    chains_cache = os.path.join(base_dir, "chains.json")
    comp0_cache = os.path.join(base_dir, "comp0_cycle.json")
    tour_path = os.path.join(base_dir, "found_tour_graph717.hcp")

    G, modules, chain1_nodes, chain2_nodes, comp0_nodes = load_and_decompose_graph717(col_path)

    with open(chains_cache, "r") as f:
        chains = json.load(f)
    chain1 = chains["chain1"]  # 255 -> ... -> 1955 (509 vertices)
    chain2 = chains["chain2"]  # 3358 -> ... -> 2609 (509 vertices)

    with open(comp0_cache, "r") as f:
        comp0_cyc = json.load(f)  # 3108 vertices

    assert len(chain1) == 509 and chain1[0] == 255 and chain1[-1] == 1955
    assert len(chain2) == 509 and chain2[0] == 3358 and chain2[-1] == 2609
    assert len(comp0_cyc) == 3108 and len(set(comp0_cyc)) == 3108

    # comp0_cyc contains virtual edges (255, 1955) and (3358, 2609).
    # We splice chain1 into edge (255, 1955) and chain2 into edge (3358, 2609).
    n_c0 = len(comp0_cyc)
    idx_1955 = comp0_cyc.index(1955)
    idx_255 = comp0_cyc.index(255)
    assert (idx_1955 - idx_255) % n_c0 == 1 or (idx_255 - idx_1955) % n_c0 == 1, "Edge (255, 1955) not in comp0_cyc"

    idx_2609 = comp0_cyc.index(2609)
    idx_3358 = comp0_cyc.index(3358)
    assert (idx_2609 - idx_3358) % n_c0 == 1 or (idx_3358 - idx_2609) % n_c0 == 1, "Edge (3358, 2609) not in comp0_cyc"

    assembled = []
    for i in range(n_c0):
        u = comp0_cyc[i]
        v = comp0_cyc[(i + 1) % n_c0]
        assembled.append(u)
        if (u, v) == (255, 1955):
            assembled.extend(chain1[1:-1])
        elif (u, v) == (1955, 255):
            assembled.extend(list(reversed(chain1))[1:-1])
        elif (u, v) == (3358, 2609):
            assembled.extend(chain2[1:-1])
        elif (u, v) == (2609, 3358):
            assembled.extend(list(reversed(chain2))[1:-1])

    assert len(assembled) == 4122
    assert len(set(assembled)) == 4122
    assert set(assembled) == set(G.keys())

    # Verify every edge in assembled exists in raw DIMACS graph G
    n_all = len(assembled)
    for i in range(n_all):
        u = assembled[i]
        v = assembled[(i + 1) % n_all]
        assert v in G[u], f"Phantom edge in assembled tour: ({u}, {v})"

    print(f"Tour assembly verified sound: {n_all} unique vertices, {n_all} valid DIMACS edges!")

    # Write HCP file
    with open(tour_path, "w") as f:
        f.write("NAME : graph717.col\n")
        f.write("TYPE : TOUR\n")
        f.write(f"DIMENSION : {len(assembled)}\n")
        f.write("TOUR_SECTION\n")
        for node in assembled:
            f.write(f"{node}\n")
        f.write("-1\n")
        f.write("EOF\n")

    print(f"Tour written to {tour_path} in {time.time()-t0:.2f}s.")
    return tour_path

if __name__ == "__main__":
    assemble_and_verify_tour()
