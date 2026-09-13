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
 
def test_assembled_tour_soundness():
    base_dir = os.path.dirname(os.path.abspath(__file__))
    tour_file = os.path.join(base_dir, "found_tour_graph746.hcp")
    assert os.path.exists(tour_file), "found_tour_graph746.hcp missing"

    col_path = "FHCPCS-col/graph746.col"
    G, _, _, _, _ = load_and_decompose_graph746(col_path)

    # Read tour file
    with open(tour_file, "r") as f:
        lines = [l.strip() for l in f if l.strip()]

    tour_idx = lines.index("TOUR_SECTION")
    tour = []
    for l in lines[tour_idx + 1:]:
        if l in ("-1", "EOF"):
            break
        tour.append(int(l))

    assert len(tour) == 4286
    assert len(set(tour)) == 4286
    assert set(tour) == set(G.keys())
    for i in range(len(tour)):
        u = tour[i]
        v = tour[(i + 1) % len(tour)]
        assert v in G[u], f"Phantom edge ({u}, {v})"
