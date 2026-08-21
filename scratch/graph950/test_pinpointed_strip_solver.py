import pytest
from scratch.graph950.two_tier_decomposer import load_graph, decompose_graph
from scratch.graph950.pinpointed_strip_solver import PinpointedStripSolver, solve_strip_pinpointed


def test_strip_solver_sat_and_unsat_core():
    G, degs = load_graph('FHCPCS-col/graph950.col')
    decomp = decompose_graph(G, degs)

    solver = PinpointedStripSolver(G, decomp)

    # Strip 0 has 5 M-hubs
    s0 = decomp.strips[0]
    s0_m = sorted([h for h in decomp.strip_adj_hubs[0] if h in decomp.m_hubs])
    s_hub = list(decomp.strip_adj_hubs[0] & set(decomp.s_hubs))[0]
    b_hub = list(decomp.strip_adj_hubs[0] & set(decomp.b_hubs))[0]

    # Case 1: Feasible demand: only 1 M-hub requests 1 port, K=4
    m_demands_sat = {s0_m[0]: 1, s0_m[1]: 0, s0_m[2]: 0, s0_m[3]: 0, s0_m[4]: 0}
    is_sat, res = solver.solve_strip(0, m_demands_sat, s_hub, b_hub, K=4)
    assert is_sat is True
    assert len(res) == 4  # exactly 4 paths covering 125 vertices

    # Verify full coverage and path validity
    covered = set()
    endpoints = set()
    for p in res:
        for i in range(len(p) - 1):
            assert p[i + 1] in G[p[i]]
        endpoints.add(p[0])
        endpoints.add(p[-1])
        for v in p:
            assert v not in covered  # vertex-disjoint
            covered.add(v)
    assert covered == set(s0)

    # Verify requested port is used as an endpoint
    ports_m0 = set(decomp.strip_hub_ports.get((0, s0_m[0]), []))
    assert len(endpoints & ports_m0) >= 1

    # Case 2: Impossible demand: request impossible number of endpoints (> 2K)
    m_demands_unsat = {s0_m[0]: 2, s0_m[1]: 2, s0_m[2]: 2, s0_m[3]: 2, s0_m[4]: 2}  # sum=10, K=2 -> max=4
    is_sat, core = solver.solve_strip(0, m_demands_unsat, s_hub, b_hub, K=2)
    assert is_sat is False
    assert len(core) > 0  # minimal core returned
    assert all(h in m_demands_unsat for h in core)


def test_solve_strip_pinpointed_helper():
    G, degs = load_graph('FHCPCS-col/graph950.col')
    decomp = decompose_graph(G, degs)

    s0 = decomp.strips[0]
    s0_m = sorted([h for h in decomp.strip_adj_hubs[0] if h in decomp.m_hubs])
    s_hub = list(decomp.strip_adj_hubs[0] & set(decomp.s_hubs))[0]
    b_hub = list(decomp.strip_adj_hubs[0] & set(decomp.b_hubs))[0]

    m_demands = {s0_m[0]: 1}
    is_sat, paths = solve_strip_pinpointed(0, s0, m_demands, G, s_hub, b_hub, K=2)
    assert is_sat is True
    assert len(paths) == 2
    covered = {v for p in paths for v in p}
    assert covered == set(s0)
