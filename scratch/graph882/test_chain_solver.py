import os, sys, pytest

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from scratch.graph882.decomposer import load_and_decompose_graph882
from scratch.graph882.chain_solver import solve_chains

def test_two_corridor_chain_solver():
    col_path = os.path.join(repo_root, "FHCPCS-col/graph882.col")
    G, mod_nodes, block2_nodes, chain_nodes, comp0_nodes = load_and_decompose_graph882(col_path)

    chain1, chain2 = solve_chains(G, mod_nodes, block2_nodes)

    # Invariants for Chain 1 (170 vertices)
    assert len(chain1) == 170
    assert len(set(chain1)) == 170
    assert chain1[0] == 5200
    assert chain1[-1] == 4509
    for i in range(len(chain1) - 1):
        assert chain1[i+1] in G[chain1[i]], f"Edge missing: ({chain1[i]}, {chain1[i+1]})"

    # Invariants for Chain 2 (15 vertices)
    assert len(chain2) == 15
    assert len(set(chain2)) == 15
    assert chain2[0] == 4779
    assert chain2[-1] == 893
    for i in range(len(chain2) - 1):
        assert chain2[i+1] in G[chain2[i]], f"Edge missing: ({chain2[i]}, {chain2[i+1]})"

    # Disjointness and partition coverage
    assert set(chain1).isdisjoint(set(chain2))
    assert set(chain1) | set(chain2) == chain_nodes
    assert (set(chain1) | set(chain2)).isdisjoint(comp0_nodes)

    # External interface connections into Comp 0
    assert 2080 in G[chain1[0]], "5200 must connect to 2080 in Comp 0"
    assert 2117 in G[chain1[-1]], "4509 must connect to 2117 in Comp 0"
    assert 5066 in G[chain2[0]], "4779 must connect to 5066 in Comp 0"
    assert 3811 in G[chain2[-1]], "893 must connect to 3811 in Comp 0"
