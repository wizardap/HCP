import os, pytest
from scratch.engine.graph_loader import load_dimacs
from scratch.engine.decomposer import detect_bridge_corridors
from scratch.engine.corridor_solver import solve_corridor_path
from scratch.engine.assembler import assemble_full_tour

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))

def test_loader_and_detector():
    col_path = os.path.join(repo_root, "FHCPCS-col/graph677.col")
    G = load_dimacs(col_path)
    assert len(G) == 3868

    corridors, comp0_nodes = detect_bridge_corridors(G)
    assert len(corridors) == 1
    assert len(corridors[0]['nodes']) == 169
    assert len(comp0_nodes) == 3699
    assert corridors[0]['ports'] == (2696, 493)
    assert set(corridors[0]['ext_ports']) == {2291, 2042}

def test_corridor_path_solve():
    col_path = os.path.join(repo_root, "FHCPCS-col/graph677.col")
    G = load_dimacs(col_path)
    corridors, _ = detect_bridge_corridors(G)
    c = corridors[0]
    u, v = c['ports']
    path = solve_corridor_path(G, c['nodes'], src=u, dst=v)
    assert len(path) == 169
    assert path[0] == u and path[-1] == v
    assert len(set(path)) == 169

def test_assembler_toy():
    comp0_cycle = [1, 2, 3, 4]
    corridors = [{
        'ports': (10, 30),
        'ext_ports': (2, 3),
        'path': [10, 20, 30]
    }]
    tour = assemble_full_tour(comp0_cycle, corridors)
    assert len(tour) == 7
    assert tour == [1, 2, 10, 20, 30, 3, 4]
