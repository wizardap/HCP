import pytest
from scratch.graph746.cluster_decomposer import load_and_decompose_graph746

def test_partition_invariants():
    col_path = "FHCPCS-col/graph746.col"
    G, degs, group_targets, macro_nodes, bulks = load_and_decompose_graph746(col_path)

    assert len(G) == 4286
    assert len(group_targets) == 5
    assert len(macro_nodes) == 11

    # Super-hubs
    super_hubs = {1430, 3641, 3735, 3790, 3960}
    assert super_hubs.issubset(macro_nodes)

    # Check each group bulk
    all_bulk_nodes = set()
    for sh, u_in, u_out, cfg in group_targets:
        b = bulks[sh]
        assert len(b) == 855
        assert u_in in b and u_out in b and u_in != u_out
        assert not (b & macro_nodes), f"Group {sh} overlaps with macro nodes"
        assert not (b & all_bulk_nodes), f"Group {sh} overlaps with other bulks"
        all_bulk_nodes.update(b)

    assert len(all_bulk_nodes) == 5 * 855  # 4275
    assert all_bulk_nodes | macro_nodes == set(G.keys())
    assert len(all_bulk_nodes & macro_nodes) == 0

    # Check external connections of each group bulk
    port_pairs = {
        1430: (3003, 2623),
        3790: (2165, 1264),
        3960: (1025, 3498),
        3641: (3146, 2397),
        3735: (46, 3547),
    }
    for sh, u_in, u_out, _ in group_targets:
        expected_ports = set(port_pairs[sh])
        assert {u_in, u_out} == expected_ports

def test_graph_dimensions_and_connectors():
    col_path = "FHCPCS-col/graph746.col"
    G, degs, group_targets, macro_nodes, bulks = load_and_decompose_graph746(col_path)

    # Verify edge count (18,286 edges)
    total_edges = sum(len(adj) for adj in G.values()) // 2
    assert total_edges == 18286

    super_hubs = {1430, 3641, 3735, 3790, 3960}
    connectors = macro_nodes - super_hubs
    expected_connectors = {1321, 2361, 2433, 3106, 3566, 3692}
    assert connectors == expected_connectors

    # Check that each super hub has degree >= 500
    for sh in super_hubs:
        assert degs[sh] >= 500

    # Check port external connectivity
    for sh, u_in, u_out, _ in group_targets:
        b = bulks[sh]
        ext_in = G[u_in] - b
        ext_out = G[u_out] - b
        assert len(ext_in) >= 1
        assert len(ext_out) >= 1
        # Each port connects to its super hub
        assert sh in ext_in or sh in ext_out
