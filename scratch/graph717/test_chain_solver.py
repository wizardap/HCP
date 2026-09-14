import json, os, pytest
from scratch.graph717.decomposer import load_and_decompose_graph717

def test_cached_chains_valid():
    base_dir = os.path.dirname(os.path.abspath(__file__))
    cache_file = os.path.join(base_dir, "chains.json")
    assert os.path.exists(cache_file), "chains.json missing, run chain_solver.py first"

    repo_root = os.path.abspath(os.path.join(base_dir, "../.."))
    col_path = os.path.join(repo_root, "FHCPCS-col/graph717.col")
    G, _, chain1_nodes, chain2_nodes, _ = load_and_decompose_graph717(col_path)

    with open(cache_file, "r") as f:
        data = json.load(f)

    assert "chain1" in data and "chain2" in data
    c1 = data["chain1"]
    c2 = data["chain2"]

    # Chain 1 assertions
    assert len(c1) == 509
    assert len(set(c1)) == 509
    assert set(c1) == chain1_nodes
    assert c1[0] == 255 and c1[-1] == 1955
    for i in range(len(c1) - 1):
        assert c1[i+1] in G[c1[i]], f"Phantom edge in c1: ({c1[i]}, {c1[i+1]})"

    # Chain 2 assertions
    assert len(c2) == 509
    assert len(set(c2)) == 509
    assert set(c2) == chain2_nodes
    assert c2[0] == 3358 and c2[-1] == 2609
    for i in range(len(c2) - 1):
        assert c2[i+1] in G[c2[i]], f"Phantom edge in c2: ({c2[i]}, {c2[i+1]})"

def test_solve_module_path_logic():
    # Verify CaDiCaL CEGAR path solving algorithm on a synthetic module
    from scratch.graph717.chain_solver import solve_module_path
    G = {
        0: {1, 2},
        1: {0, 2, 3},
        2: {0, 1, 4},
        3: {1, 4, 5},
        4: {2, 3, 5},
        5: {3, 4}
    }
    path = solve_module_path(G, {1, 2, 3, 4}, 0, 5)
    assert len(path) == 6
    assert path[0] == 0 and path[-1] == 5
    assert set(path) == {0, 1, 2, 3, 4, 5}
    for i in range(5):
        assert path[i+1] in G[path[i]]
