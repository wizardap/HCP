import os, pytest
from scratch.graph717.decomposer import load_and_decompose_graph717

def test_decomposer_invariants():
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
    col_path = os.path.join(repo_root, "FHCPCS-col/graph717.col")
    G, modules, chain1_nodes, chain2_nodes, comp0_nodes = load_and_decompose_graph717(col_path)

    # 1. Graph sizes
    assert len(G) == 4122
    num_edges = sum(len(nbrs) for nbrs in G.values()) // 2
    assert num_edges == 7638

    # 2. Module count and sizes
    assert len(modules) == 6
    for ports, mod in modules.items():
        assert len(mod) == 167
        u, v = ports
        assert u in G and v in G
        assert len(G[u] & mod) == 13
        assert len(G[v] & mod) == 13

    # 3. Chain and Comp 0 sizes
    assert len(chain1_nodes) == 509
    assert len(chain2_nodes) == 509
    assert len(comp0_nodes) == 3108

    # 4. Overlaps and union
    assert chain1_nodes & chain2_nodes == set()
    ports_c0 = {255, 1955, 2609, 3358}
    assert (chain1_nodes | chain2_nodes) & comp0_nodes == ports_c0
    assert (chain1_nodes | chain2_nodes | comp0_nodes) == set(G.keys())
    assert len(chain1_nodes) + len(chain2_nodes) + len(comp0_nodes) - 4 == 4122

    # 5. Bridge edges
    assert 1389 in G[255]
    assert 1213 in G[2677]
    assert 773 in G[2681]
    assert 1955 in G[702]

    assert 3986 in G[3358]
    assert 1177 in G[1016]
    assert 577 in G[2467]
    assert 2609 in G[540]
