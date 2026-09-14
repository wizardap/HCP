import json, os, pytest
from scratch.graph717.decomposer import load_and_decompose_graph717

def test_cached_comp0_cycle_valid():
    base_dir = os.path.dirname(os.path.abspath(__file__))
    cache_file = os.path.join(base_dir, "comp0_cycle.json")
    assert os.path.exists(cache_file), "comp0_cycle.json missing, run comp0_solver.py first"

    repo_root = os.path.abspath(os.path.join(base_dir, "../.."))
    col_path = os.path.join(repo_root, "FHCPCS-col/graph717.col")
    G, _, _, _, comp0_nodes = load_and_decompose_graph717(col_path)

    with open(cache_file, "r") as f:
        cyc = json.load(f)

    assert len(cyc) == 3108
    assert len(set(cyc)) == 3108
    assert set(cyc) == comp0_nodes

    # Check that both virtual edges (255, 1955) and (3358, 2609) exist in the cycle
    virt1 = tuple(sorted([255, 1955]))
    virt2 = tuple(sorted([3358, 2609]))
    n = len(cyc)
    cycle_edges = {tuple(sorted([cyc[i], cyc[(i+1)%n]])) for i in range(n)}
    assert virt1 in cycle_edges, "Virtual edge (255, 1955) missing from Comp 0 cycle"
    assert virt2 in cycle_edges, "Virtual edge (3358, 2609) missing from Comp 0 cycle"

    # All other edges must exist in raw G
    for e in cycle_edges - {virt1, virt2}:
        assert e[1] in G[e[0]], f"Phantom edge in comp0 cycle: {e}"
