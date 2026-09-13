import os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
import pytest
from scratch.graph990.two_half_decomposer import load_and_partition_graph990
from scratch.graph950.two_half_two_tier_solver import decompose_half

def test_partition_invariants():
    col_path = "FHCPCS-col/graph990.col"
    G, degs, grp1, grp2, h1_targets, h2_targets, bridges = load_and_partition_graph990(col_path)

    assert len(G) == 8020
    assert len(grp1) == 4010
    assert len(grp2) == 4010
    assert len(grp1 & grp2) == 0
    assert len(grp1 | grp2) == 8020

    # Bridges
    assert sorted(bridges) == [(3517, 7858), (3728, 4178)]

    # Half 1 Decomposition & Targets
    all_hubs1, strips1, _, strip_adj_hubs1, _ = decompose_half(G, degs, grp1)
    assert len(strips1) == 37
    for sh, u_in, u_out, cfg in h1_targets:
        v_bulk = set()
        for ci in cfg["clusters"]:
            v_bulk.update(strips1[ci])
            for h in strip_adj_hubs1[ci]:
                if degs[h] != 802:
                    v_bulk.add(h)
        for ti in cfg["tiny"]:
            v_bulk.update(strips1[ti])
        assert len(v_bulk) == 800
        assert u_in in v_bulk and u_out in v_bulk and u_in != u_out

    # Half 2 Decomposition & Targets
    all_hubs2, strips2, _, strip_adj_hubs2, _ = decompose_half(G, degs, grp2)
    assert len(strips2) == 37
    for sh, u_in, u_out, cfg in h2_targets:
        v_bulk = set()
        for ci in cfg["clusters"]:
            v_bulk.update(strips2[ci])
            for h in strip_adj_hubs2[ci]:
                if degs[h] != 802:
                    v_bulk.add(h)
        for ti in cfg["tiny"]:
            v_bulk.update(strips2[ti])
        assert len(v_bulk) == 800
        assert u_in in v_bulk and u_out in v_bulk and u_in != u_out

if __name__ == "__main__":
    test_partition_invariants()
    print("All partition invariants PASSED successfully!")
