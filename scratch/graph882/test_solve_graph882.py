import os, pytest, sys

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from scratch.graph882.decomposer import load_and_decompose_graph882
from scratch.graph882.chain_solver import solve_chains
from scratch.graph882.comp0_solver import solve_comp0
from scratch.graph882.solve_graph882 import assemble_tour

def test_assemble_tour_and_verify():
    col_path = os.path.join(repo_root, "FHCPCS-col/graph882.col")
    G, mod_nodes, block2_nodes, chain_nodes, comp0_nodes = load_and_decompose_graph882(col_path)

    chain1, chain2 = solve_chains(G, mod_nodes, block2_nodes)
    comp0_cycle = solve_comp0(G, comp0_nodes)

    tour = assemble_tour(comp0_cycle, chain1, chain2)

    assert len(tour) == 5686
    assert len(set(tour)) == 5686
    assert set(tour) == set(G.keys())

    # Verify all edges exist in raw graph G (zero phantom edges)
    n = len(tour)
    for i in range(n):
        u = tour[i]
        v = tour[(i + 1) % n]
        assert v in G[u], f"Phantom edge in assembled tour: ({u}, {v})"
