import os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
from scratch.graph982.two_half_decomposer import load_and_partition_graph982
from scratch.graph950.two_half_two_tier_solver import decompose_half

def test_partition_invariants():
    col_path = "FHCPCS-col/graph982.col"
    assert os.path.exists(col_path), f"File not found: {col_path}"
    G, degs, grp1, grp2, h1_targets, h2_targets, bridges = load_and_partition_graph982(col_path)

    # Validate 10 super-hubs
    s_hubs = [u for u in degs if degs[u] == 762]
    assert len(s_hubs) == 10, f"Expected 10 super-hubs, got {len(s_hubs)}"

    # Validate 2 equal halves of 3,810 vertices
    assert len(grp1) == 3810, f"Half 1 size mismatch: {len(grp1)}"
    assert len(grp2) == 3810, f"Half 2 size mismatch: {len(grp2)}"
    assert grp1.isdisjoint(grp2), "Halves must be disjoint"
    assert len(grp1 | grp2) == 7620, "Union must cover all 7,620 vertices"

    # Validate 2-edge cut
    assert len(bridges) == 2, f"Expected 2 bridge edges, got {len(bridges)}"
    assert (4740, 6956) in bridges or (6956, 4740) in bridges
    assert (5022, 5852) in bridges or (5852, 5022) in bridges

    # Validate 5 targets per half
    assert len(h1_targets) == 5, f"Expected 5 groups in H1, got {len(h1_targets)}"
    assert len(h2_targets) == 5, f"Expected 5 groups in H2, got {len(h2_targets)}"

    # Validate group sizes and ports strictly in v_bulk (Defect B)
    all_hubs1, strips1, _, strip_adj_hubs1, _ = decompose_half(G, degs, grp1)
    for sh, u_in, u_out, cfg in h1_targets:
        v_bulk = set()
        for ci in cfg["clusters"]:
            v_bulk.update(strips1[ci])
            for h in strip_adj_hubs1[ci]:
                if degs[h] != 762:
                    v_bulk.add(h)
        for ti in cfg["tiny"]:
            v_bulk.update(strips1[ti])
        assert len(v_bulk) == 760, f"Group {sh} size {len(v_bulk)} != 760"
        assert u_in in v_bulk and u_out in v_bulk, f"Group {sh} ports not in v_bulk!"
        assert u_in != u_out

    all_hubs2, strips2, _, strip_adj_hubs2, _ = decompose_half(G, degs, grp2)
    for sh, u_in, u_out, cfg in h2_targets:
        v_bulk = set()
        for ci in cfg["clusters"]:
            v_bulk.update(strips2[ci])
            for h in strip_adj_hubs2[ci]:
                if degs[h] != 762:
                    v_bulk.add(h)
        for ti in cfg["tiny"]:
            v_bulk.update(strips2[ti])
        assert len(v_bulk) == 760, f"Group {sh} size {len(v_bulk)} != 760"
        assert u_in in v_bulk and u_out in v_bulk, f"Group {sh} ports not in v_bulk!"
        assert u_in != u_out

    print("All partition invariants PASSED successfully!")

if __name__ == "__main__":
    test_partition_invariants()
