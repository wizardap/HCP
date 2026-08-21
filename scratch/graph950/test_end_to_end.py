import os
import pytest
from scratch.graph950.two_tier_orchestrator import solve_graph950_two_tier, write_hcp_tour
from scratch.graph950.two_tier_decomposer import load_graph
from scratch.graph950.macro_splicer import verify_tour_on_raw_graph

def test_orchestrator_initializes_cleanly():
    # Verify orchestrator initializes and runs within timeout budget
    res = solve_graph950_two_tier(timeout=10.0, dry_run=True)
    assert res is True

def test_write_and_verify_hcp_tour(tmp_path):
    out_file = str(tmp_path / "test_tour.hcp")
    sample_tour = [1, 2, 3, 4]
    write_hcp_tour(sample_tour, out_file, graph_name="test_graph")
    
    assert os.path.exists(out_file)
    with open(out_file, "r") as f:
        lines = [l.strip() for l in f if l.strip()]
        
    assert lines[0] == "NAME : test_graph"
    assert lines[1] == "TYPE : TOUR"
    assert lines[2] == "DIMENSION : 4"
    assert lines[3] == "TOUR_SECTION"
    assert lines[4:8] == ["1", "2", "3", "4"]
    assert lines[8] == "-1"
    assert lines[9] == "EOF"

def test_orchestrator_invalid_graph():
    res = solve_graph950_two_tier(graph_path="non_existent_file.col", dry_run=True)
    assert res is False
