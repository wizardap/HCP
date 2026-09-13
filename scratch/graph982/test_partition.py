import os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
from scratch.graph982.two_half_decomposer import load_and_partition_graph982

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
    assert (1495, 6335) in bridges or (6335, 1495) in bridges
    assert (6670, 7311) in bridges or (7311, 6670) in bridges

    # Validate 5 targets per half
    assert len(h1_targets) == 5, f"Expected 5 groups in H1, got {len(h1_targets)}"
    assert len(h2_targets) == 5, f"Expected 5 groups in H2, got {len(h2_targets)}"

    print("All partition invariants PASSED successfully!")

if __name__ == "__main__":
    test_partition_invariants()
