import os, pytest
from scratch.engine.graph_loader import load_dimacs
from scratch.engine.decomposer import detect_bridge_corridors
from scratch.engine.bipartite_module_detector import detect_variable_modules

def test_detect_modules_empty():
    modules = detect_variable_modules({}, set(), [])
    assert modules == []

def test_detect_modules_graph868():
    col_path = "FHCPCS-col/graph868.col"
    if not os.path.exists(col_path):
        pytest.skip("graph868.col not found")
    G = load_dimacs(col_path)
    corridors, c0_nodes = detect_bridge_corridors(G)
    modules = detect_variable_modules(G, set(c0_nodes), [])
    assert len(modules) == 42, f"Expected 42 modules for graph868, got {len(modules)}"
    for m in modules:
        assert len(m['nodes']) == 88 or len(m['nodes']) == 44, f"Unexpected module size: {len(m['nodes'])}"
        assert len(m['ports']) == 2, f"Module must have exactly 2 interface ports"

def test_detect_modules_graph965():
    col_path = "FHCPCS-col/graph965.col"
    if not os.path.exists(col_path):
        pytest.skip("graph965.col not found")
    G = load_dimacs(col_path)
    corridors, c0_nodes = detect_bridge_corridors(G)
    ve = [c['ext_ports'] for c in corridors]
    modules = detect_variable_modules(G, set(c0_nodes), ve)
    assert len(modules) >= 30, f"Expected >= 30 modules for graph965, got {len(modules)}"
    for m in modules:
        assert len(m['ports']) == 2
