import os, sys, pytest

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from scratch.graph882.decomposer import load_and_decompose_graph882
from scratch.graph882.comp0_solver import solve_comp0

def test_comp0_solver():
    col_path = os.path.join(repo_root, "FHCPCS-col/graph882.col")
    G, _, _, _, comp0_nodes = load_and_decompose_graph882(col_path)

    cycle = solve_comp0(G, comp0_nodes)
    assert len(cycle) == 5501
    assert len(set(cycle)) == 5501
    assert set(cycle) == comp0_nodes

    # Check that both virtual edges (2080, 2117) and (3811, 5066) are present
    has_virt1 = False
    has_virt2 = False
    for i in range(len(cycle)):
        u, v = cycle[i], cycle[(i + 1) % len(cycle)]
        if {u, v} == {2080, 2117}:
            has_virt1 = True
        elif {u, v} == {3811, 5066}:
            has_virt2 = True
        else:
            assert v in G[u], f"Phantom edge in comp0 cycle: ({u}, {v})"
    assert has_virt1, "Virtual edge (2080, 2117) not found in comp0 cycle!"
    assert has_virt2, "Virtual edge (3811, 5066) not found in comp0 cycle!"
