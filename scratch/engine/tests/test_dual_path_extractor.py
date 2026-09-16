import os
import pytest
import collections
from scratch.engine.graph_loader import load_dimacs
from scratch.engine.decomposer import detect_bridge_corridors
from scratch.engine.bipartite_module_detector import detect_variable_modules
from scratch.engine.dual_path_extractor import extract_module_dual_paths


def test_dual_path_extraction_empty():
    assert extract_module_dual_paths({}, {}) == (set(), set())
    assert extract_module_dual_paths({}, {'nodes': set(), 'ports': ()}) == (set(), set())
    assert extract_module_dual_paths({}, {'nodes': {1, 2}, 'ports': (1,)}) == (set(), set())


def test_dual_path_extraction_synthetic():
    # Small module of 6 vertices: 1, 2, 3, 4, 5, 6
    # Virtual edges: (1, 2), (3, 4), (5, 6)
    # Ports: (1, 6)
    # Path 1: 1-2 (VE), 2-3 (real), 3-4 (VE), 4-5 (real), 5-6 (VE)
    # Path 2: 1-4 (real), 4-3 (VE), 3-2 (real), 2-5 (real), 5-6 (VE) - or another alternative
    G_adj = {
        1: {2, 4},
        2: {1, 3, 5},
        3: {4, 2, 6},
        4: {3, 1, 5},
        5: {6, 2, 4},
        6: {5, 3},
    }
    module = {
        'id': 0,
        'nodes': {1, 2, 3, 4, 5, 6},
        'ports': (1, 6),
        'virtual_edges': {(1, 2), (3, 4), (5, 6)},
        'internal_nodes': {2, 3, 4, 5},
    }
    e_true, e_false = extract_module_dual_paths(G_adj, module)
    assert len(e_true) == 5, f"Expected 5 edges, got {len(e_true)}"
    assert len(e_false) == 5, f"Expected 5 edges, got {len(e_false)}"
    assert e_true != e_false, "True and False paths must be distinct"

    # Verify vertex coverage
    v_true = {v for e in e_true for v in e}
    v_false = {v for e in e_false for v in e}
    assert v_true == module['nodes']
    assert v_false == module['nodes']


def test_dual_path_extraction_graph868():
    col_path = "FHCPCS-col/graph868.col"
    if not os.path.exists(col_path):
        pytest.skip("graph868.col not found")
    G = load_dimacs(col_path)
    modules = detect_variable_modules(G, set(G.keys()), [])
    assert len(modules) > 0
    m0 = modules[0]
    e_true, e_false = extract_module_dual_paths(G, m0)
    assert len(e_true) > 0, "True path must not be empty"
    assert len(e_false) > 0, "False path must not be empty"
    assert e_true != e_false, "True and False paths must be distinct"

    # Verify coverage and degrees
    m_nodes = set(m0['nodes'])
    p_in, p_out = m0['ports']
    assert len(e_true) == len(m_nodes) - 1
    assert len(e_false) == len(m_nodes) - 1

    # 100% vertex coverage
    assert {v for e in e_true for v in e} == m_nodes
    assert {v for e in e_false for v in e} == m_nodes

    # True path must visit 100% of internal virtual edges
    for ve in m0['virtual_edges']:
        assert tuple(sorted(ve)) in e_true

    # Path degrees check
    for p in (e_true, e_false):
        deg = collections.defaultdict(int)
        for u, v in p:
            deg[u] += 1
            deg[v] += 1
        assert deg[p_in] == 1
        assert deg[p_out] == 1
        for u in m_nodes:
            if u not in (p_in, p_out):
                assert deg[u] == 2


def test_dual_path_extraction_graph965():
    col_path = "FHCPCS-col/graph965.col"
    if not os.path.exists(col_path):
        pytest.skip("graph965.col not found")
    G = load_dimacs(col_path)
    corridors, c0_nodes = detect_bridge_corridors(G)
    ve = [c['ext_ports'] for c in corridors]
    modules = detect_variable_modules(G, set(c0_nodes), ve)
    assert len(modules) > 0
    m0 = modules[0]
    e_true, e_false = extract_module_dual_paths(G, m0)
    assert len(e_true) > 0
    assert len(e_false) > 0
    assert e_true != e_false
    assert {v for e in e_true for v in e} == set(m0['nodes'])
    assert {v for e in e_false for v in e} == set(m0['nodes'])
