import os, pytest
from scratch.graph677.decomposer import load_and_decompose_graph677
from scratch.graph677.chain_solver import solve_module_corridor

def test_module_corridor_path():
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
    col_path = os.path.join(repo_root, "FHCPCS-col/graph677.col")

    G, mod_nodes, comp0_nodes, (port1, port2), _ = load_and_decompose_graph677(col_path)
    path = solve_module_corridor(G, mod_nodes, port1, port2)

    assert len(path) == 169
    assert path[0] == port1
    assert path[-1] == port2
    assert len(set(path)) == 169
    assert set(path) == mod_nodes

    # Check path validity
    for i in range(len(path) - 1):
        assert path[i+1] in G[path[i]], f"Invalid step {path[i]} -> {path[i+1]}"
