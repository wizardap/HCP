import os, pytest
from scratch.graph677.decomposer import load_graph, load_and_decompose_graph677

def test_load_and_decompose_graph677():
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
    col_path = os.path.join(repo_root, "FHCPCS-col/graph677.col")
    assert os.path.exists(col_path)

    G, mod_nodes, comp0_nodes, (port1, port2), (p1_ext, p2_ext) = load_and_decompose_graph677(col_path)

    assert len(G) == 3868
    assert len(mod_nodes) == 169
    assert len(comp0_nodes) == 3699
    assert len(mod_nodes & comp0_nodes) == 0
    assert len(mod_nodes | comp0_nodes) == 3868

    assert port1 in mod_nodes and port2 in mod_nodes
    assert p1_ext in comp0_nodes and p2_ext in comp0_nodes
    assert p1_ext in G[port1]
    assert p2_ext in G[port2]
