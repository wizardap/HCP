import json, os, sys, pytest

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
from scratch.graph990.two_half_decomposer import load_and_partition_graph990

def test_cached_paths_valid():
    base_dir = os.path.dirname(os.path.abspath(__file__))
    h1_cache = os.path.join(base_dir, "half1_group_paths.json")
    h2_cache = os.path.join(base_dir, "half2_group_paths.json")

    assert os.path.exists(h1_cache), "half1_group_paths.json missing"
    assert os.path.exists(h2_cache), "half2_group_paths.json missing"

    col_path = "FHCPCS-col/graph990.col"
    G, degs, grp1, grp2, h1_targets, h2_targets, _ = load_and_partition_graph990(col_path)
    s_hubs = {u for u in degs if degs[u] == 802}

    with open(h1_cache, "r") as f:
        p1 = {int(k): v for k, v in json.load(f).items()}
    assert len(p1) == 5

    h1_all_v = set()
    for sh, u_in, u_out, _ in h1_targets:
        assert sh in p1
        path = p1[sh]
        assert len(path) == 800
        assert len(set(path)) == 800
        assert path[0] == u_in
        assert path[-1] == u_out
        assert s_hubs.isdisjoint(path)
        for i in range(len(path) - 1):
            assert path[i+1] in G[path[i]]
        assert h1_all_v.isdisjoint(path)
        h1_all_v.update(path)

    with open(h2_cache, "r") as f:
        p2 = {int(k): v for k, v in json.load(f).items()}
    assert len(p2) == 5

    h2_all_v = set()
    for sh, u_in, u_out, _ in h2_targets:
        assert sh in p2
        path = p2[sh]
        assert len(path) == 800
        assert len(set(path)) == 800
        assert path[0] == u_in
        assert path[-1] == u_out
        assert s_hubs.isdisjoint(path)
        for i in range(len(path) - 1):
            assert path[i+1] in G[path[i]]
        assert h2_all_v.isdisjoint(path)
        h2_all_v.update(path)

    assert len(h1_all_v) == 4000
    assert len(h2_all_v) == 4000
    assert h1_all_v.isdisjoint(h2_all_v)
    assert len(grp1 - h1_all_v) == 10
    assert len(grp2 - h2_all_v) == 10
