import json, os, pytest
from scratch.graph717.decomposer import load_graph

def test_assembled_tour_soundness():
    base_dir = os.path.dirname(os.path.abspath(__file__))
    tour_file = os.path.join(base_dir, "found_tour_graph717.hcp")
    assert os.path.exists(tour_file), "found_tour_graph717.hcp missing, run solve_graph717.py first"

    repo_root = os.path.abspath(os.path.join(base_dir, "../.."))
    col_path = os.path.join(repo_root, "FHCPCS-col/graph717.col")
    G = load_graph(col_path)

    # Parse tour file
    nodes = []
    in_tour = False
    with open(tour_file, "r") as f:
        for line in f:
            line = line.strip()
            if line == "TOUR_SECTION":
                in_tour = True
                continue
            if in_tour:
                if line in ("-1", "EOF"):
                    break
                nodes.append(int(line))

    assert len(nodes) == 4122, f"Expected 4122 nodes, got {len(nodes)}"
    assert len(set(nodes)) == 4122, f"Expected 4122 unique nodes, got {len(set(nodes))}"
    assert set(nodes) == set(G.keys()), "Tour does not visit all vertices of graph717"

    # Verify every edge exists in raw G
    n = len(nodes)
    for i in range(n):
        u = nodes[i]
        v = nodes[(i + 1) % n]
        assert v in G[u], f"Phantom edge: ({u}, {v})"
