import os, sys, pytest
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "../..")))
from scratch.graph882.decomposer import load_and_decompose_graph882

def test_decomposer_invariants():
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
    col_path = os.path.join(repo_root, "FHCPCS-col/graph882.col")
    G, mod_nodes, block2_nodes, chain_nodes, comp0_nodes = load_and_decompose_graph882(col_path)

    # 1. Graph sizes
    assert len(G) == 5686
    num_edges = sum(len(nbrs) for nbrs in G.values()) // 2
    assert num_edges == 9306

    # 2. Module count and sizes
    assert len(mod_nodes) == 169
    assert {3195, 5200}.issubset(mod_nodes)
    internal_mod = mod_nodes - {3195, 5200}
    assert len(internal_mod) == 167
    assert len(G[3195] & internal_mod) == 13
    assert len(G[5200] & internal_mod) == 13

    # 3. Buffer block sizes
    assert len(block2_nodes) == 16
    assert 4509 in block2_nodes
    assert 4779 in block2_nodes

    # 4. Chain nodes and bridge edge
    assert chain_nodes == (mod_nodes | block2_nodes)
    assert len(chain_nodes) == 185
    assert 4509 in G[3195]  # Bridge edge connecting Module to Buffer

    # 5. Comp 0 size and disjointness
    assert len(comp0_nodes) == 5501
    assert chain_nodes & comp0_nodes == set()
    assert (chain_nodes | comp0_nodes) == set(G.keys())

    # 6. Terminal connections to Comp 0
    assert 2080 in comp0_nodes
    assert 5066 in comp0_nodes
    assert 2080 in G[5200]
    assert 5066 in G[4779]
