import os
import pytest
from scratch.engine.graph_loader import load_dimacs
from scratch.engine.decomposer import detect_bridge_corridors
from scratch.engine.modular_comp0_solver import solve_modular_comp0


def test_modular_comp0_empty():
    cycle = solve_modular_comp0({}, set(), [])
    assert cycle == []


def test_modular_comp0_triangle():
    # Simple triangle graph: 1-2-3-1
    G = {
        1: {2, 3},
        2: {1, 3},
        3: {1, 2},
    }
    c0 = {1, 2, 3}
    cycle = solve_modular_comp0(G, c0, [], time_limit=30)
    assert len(cycle) == 3
    assert set(cycle) == {1, 2, 3}


def test_modular_comp0_solve_graph882():
    col_path = "FHCPCS-col/graph882.col"
    if not os.path.exists(col_path):
        pytest.skip("graph882.col not found")
    G = load_dimacs(col_path)
    corridors, c0_nodes = detect_bridge_corridors(G)
    ve = [c['ext_ports'] for c in corridors]
    cycle = solve_modular_comp0(G, set(c0_nodes), ve, time_limit=300)
    assert len(cycle) == len(c0_nodes), f"Expected {len(c0_nodes)}, got {len(cycle)}"
    assert len(set(cycle)) == len(c0_nodes), "Duplicate vertices in cycle"
