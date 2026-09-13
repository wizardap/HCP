import json, os, pytest
from scratch.graph710.decomposer import load_and_decompose_graph710

def test_cached_block_paths_valid():
    base_dir = os.path.dirname(os.path.abspath(__file__))
    cache_file = os.path.join(base_dir, "block_paths.json")
    assert os.path.exists(cache_file), "block_paths.json missing, run solve_blocks_parallel.py first"

    col_path = "FHCPCS-col/graph710.col"
    G, V_A, V_B, port_u, port_v = load_and_decompose_graph710(col_path)

    with open(cache_file, "r") as f:
        paths = json.load(f)

    assert "A" in paths and "B" in paths
    path_a = paths["A"]
    path_b = paths["B"]

    # Block A assertions
    assert len(path_a) == 887
    assert len(set(path_a)) == 887
    assert set(path_a) == V_A
    assert path_a[0] == port_u and path_a[-1] == port_v
    for i in range(len(path_a) - 1):
        assert path_a[i+1] in G[path_a[i]], f"Phantom edge in path_a: ({path_a[i]}, {path_a[i+1]})"

    # Block B assertions
    assert len(path_b) == 3179
    assert len(set(path_b)) == 3179
    assert set(path_b) == V_B
    assert path_b[0] == port_v and path_b[-1] == port_u
    for i in range(len(path_b) - 1):
        assert path_b[i+1] in G[path_b[i]], f"Phantom edge in path_b: ({path_b[i]}, {path_b[i+1]})"
