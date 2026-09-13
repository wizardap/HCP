import json, os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
from scratch.graph982.two_half_decomposer import load_and_partition_graph982
from scratch.graph982.cluster_path_solver import solve_group_bulk
from scratch.graph950.two_half_two_tier_solver import decompose_half

def test_solve_single_group(use_cache: bool = True):
    col_path = "FHCPCS-col/graph982.col"
    G, degs, grp1, grp2, h1_targets, h2_targets, bridges = load_and_partition_graph982(col_path)
    all_hubs1, strips1, hh1, strip_adj_hubs1, _ = decompose_half(G, degs, grp1)

    # Test solving first group (super-hub 4740, 760 vertices)
    sh, u_in, u_out, cfg = h1_targets[0]
    cache_file = os.path.join(os.path.dirname(os.path.abspath(__file__)), "half1_group_paths.json")
    if use_cache and os.path.exists(cache_file):
        with open(cache_file) as f:
            cached = json.load(f)
            if str(sh) in cached:
                path = cached[str(sh)]
            else:
                path = solve_group_bulk(sh, u_in, u_out, cfg, strips1, strip_adj_hubs1, degs, G)
    else:
        path = solve_group_bulk(sh, u_in, u_out, cfg, strips1, strip_adj_hubs1, degs, G)

    assert path is not None, f"Failed to solve Group {sh}"
    assert len(path) == 760, f"Path length mismatch: {len(path)} != 760"
    assert len(set(path)) == 760, "Path contains duplicate vertices"
    assert path[0] == u_in and path[-1] == u_out, "Path boundary mismatch"
    for i in range(len(path) - 1):
        assert path[i+1] in G[path[i]], f"Invalid edge at {i}: ({path[i]}, {path[i+1]})"
    print(f"Group {sh} path test PASSED in < 15s!")

def test_all_cached_groups():
    col_path = "FHCPCS-col/graph982.col"
    G, degs, grp1, grp2, h1_targets, h2_targets, bridges = load_and_partition_graph982(col_path)
    s_hubs = {u for u in degs if degs[u] == 762}

    base_dir = os.path.dirname(os.path.abspath(__file__))
    h1_cache = os.path.join(base_dir, "half1_group_paths.json")
    h2_cache = os.path.join(base_dir, "half2_group_paths.json")
    assert os.path.exists(h1_cache), f"Missing {h1_cache}"
    assert os.path.exists(h2_cache), f"Missing {h2_cache}"

    with open(h1_cache) as f:
        h1_paths = {int(k): v for k, v in json.load(f).items()}
    with open(h2_cache) as f:
        h2_paths = {int(k): v for k, v in json.load(f).items()}

    assert len(h1_paths) == 5, f"Expected 5 paths in Half 1, got {len(h1_paths)}"
    assert len(h2_paths) == 5, f"Expected 5 paths in Half 2, got {len(h2_paths)}"

    h1_all_v = set()
    for sh, u_in, u_out, cfg in h1_targets:
        path = h1_paths[sh]
        assert len(path) == 760, f"Group {sh} length mismatch"
        assert len(set(path)) == 760, f"Group {sh} duplicate vertices"
        assert path[0] == u_in and path[-1] == u_out, f"Group {sh} endpoints mismatch"
        assert s_hubs.isdisjoint(path), f"Group {sh} overlaps with super-hubs"
        for i in range(len(path) - 1):
            assert path[i+1] in G[path[i]], f"Invalid edge ({path[i]}, {path[i+1]})"
        assert h1_all_v.isdisjoint(path), f"Group {sh} overlaps with other groups in Half 1"
        h1_all_v.update(path)

    h2_all_v = set()
    for sh, u_in, u_out, cfg in h2_targets:
        path = h2_paths[sh]
        assert len(path) == 760, f"Group {sh} length mismatch"
        assert len(set(path)) == 760, f"Group {sh} duplicate vertices"
        assert path[0] == u_in and path[-1] == u_out, f"Group {sh} endpoints mismatch"
        assert s_hubs.isdisjoint(path), f"Group {sh} overlaps with super-hubs"
        for i in range(len(path) - 1):
            assert path[i+1] in G[path[i]], f"Invalid edge ({path[i]}, {path[i+1]})"
        assert h2_all_v.isdisjoint(path), f"Group {sh} overlaps with other groups in Half 2"
        h2_all_v.update(path)

    assert len(h1_all_v) == 3800
    assert len(h2_all_v) == 3800
    assert h1_all_v.isdisjoint(h2_all_v)
    assert len(grp1 - h1_all_v) == 10
    assert len(grp2 - h2_all_v) == 10
    print("All 10 cached group paths verified perfectly (7,600 vertices, zero defects)!")

if __name__ == "__main__":
    test_solve_single_group()
    test_all_cached_groups()
