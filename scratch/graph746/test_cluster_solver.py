import json, os, pytest
from scratch.graph746.cluster_decomposer import load_and_decompose_graph746

def test_cached_paths_valid():
    base_dir = os.path.dirname(os.path.abspath(__file__))
    cache_file = os.path.join(base_dir, "group_paths.json")

    assert os.path.exists(cache_file), "group_paths.json missing"

    col_path = "FHCPCS-col/graph746.col"
    G, _, group_targets, macro_nodes, bulks = load_and_decompose_graph746(col_path)

    with open(cache_file, "r") as f:
        paths = {int(k): v for k, v in json.load(f).items()}

    assert len(paths) == 5
    all_path_nodes = set()
    for sh, u_in, u_out, _ in group_targets:
        assert sh in paths
        path = paths[sh]
        assert len(path) == 855
        assert len(set(path)) == 855
        assert path[0] == u_in
        assert path[-1] == u_out
        assert set(path) == bulks[sh]
        assert not (set(path) & macro_nodes)
        for i in range(len(path) - 1):
            assert path[i+1] in G[path[i]]
        all_path_nodes.update(path)

    assert len(all_path_nodes) == 5 * 855
