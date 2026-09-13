import os, pytest
from scratch.graph710.decomposer import load_and_decompose_graph710

def test_decomposer_invariants():
    repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
    col_path = os.path.join(repo_root, "FHCPCS-col/graph710.col")
    G, V_A, V_B, port_u, port_v = load_and_decompose_graph710(col_path)

    # 1. Graph sizes
    assert len(G) == 4064
    num_edges = sum(len(nbrs) for nbrs in G.values()) // 2
    assert num_edges == 6800

    # 2. Cut vertices
    assert port_u == 1876
    assert port_v == 2491
    assert port_v not in G[port_u], "No direct edge between cut ports in G"

    # 3. Block sizes
    assert len(V_A) == 887
    assert len(V_B) == 3179

    # 4. Overlap and union invariants
    assert V_A & V_B == {port_u, port_v}
    assert V_A | V_B == set(G.keys())
    assert len(V_A) + len(V_B) - 2 == 4064

    # 5. Forced port degrees into blocks
    interior_A = V_A - {port_u, port_v}
    interior_B = V_B - {port_u, port_v}

    # port_u (1876) has exactly 1 edge into Block A interior: (1876, 3878)
    u_nbrs_A = G[port_u] & interior_A
    assert len(u_nbrs_A) == 1
    assert 3878 in u_nbrs_A
    # port_u has 3 edges into Block B interior
    u_nbrs_B = G[port_u] & interior_B
    assert len(u_nbrs_B) == 3

    # port_v (2491) has exactly 1 edge into Block B interior: (2491, 1671)
    v_nbrs_B = G[port_v] & interior_B
    assert len(v_nbrs_B) == 1
    assert 1671 in v_nbrs_B
    # port_v has 2 edges into Block A interior
    v_nbrs_A = G[port_v] & interior_A
    assert len(v_nbrs_A) == 2
