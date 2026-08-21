import pytest
from scratch.graph950.two_tier_decomposer import decompose_graph, load_graph

def test_decomposer_graph950():
    G, degs = load_graph('FHCPCS-col/graph950.col')
    assert len(G) == 6620
    
    decomp = decompose_graph(G, degs)
    assert len(decomp.all_hubs) == 310
    assert len(decomp.s_hubs) == 10
    assert len(decomp.b_hubs) == 50
    assert len(decomp.m_hubs) == 250
    assert len(decomp.hh_edges) == 650
    assert len(decomp.strips) == 74
    
    # 50 large strips of 125, 12 of 3, 12 of 2
    lens = [len(s) for s in decomp.strips]
    assert lens.count(125) == 50
    assert lens.count(3) == 12
    assert lens.count(2) == 12
    
    # Check every large strip connects to 1 S, 1 B, 5 M
    for si, s in enumerate(decomp.strips):
        if len(s) == 125:
            adj_h = decomp.strip_adj_hubs[si]
            assert len(adj_h & set(decomp.s_hubs)) == 1
            assert len(adj_h & set(decomp.b_hubs)) == 1
            assert len(adj_h & set(decomp.m_hubs)) == 5
