import collections
import pytest
from scratch.graph950.two_tier_decomposer import load_graph, decompose_graph
from scratch.graph950.global_demand_coordinator import GlobalDemandCoordinator

def test_demand_coordinator_generates_valid_demands():
    G, degs = load_graph('FHCPCS-col/graph950.col')
    decomp = decompose_graph(G, degs)
    
    coord = GlobalDemandCoordinator(G, decomp)
    is_sat, hh_edges, strip_demands = coord.solve_assignment()
    assert is_sat is True
    assert len(strip_demands) == 74
    
    # Check every M-hub has sum of hh_edges + strip_demands == 2
    m_hub_totals = collections.defaultdict(int)
    for u, v in hh_edges:
        if u in set(decomp.m_hubs):
            m_hub_totals[u] += 1
        if v in set(decomp.m_hubs):
            m_hub_totals[v] += 1
            
    for si, d_map in strip_demands.items():
        for h, d in d_map.items():
            if h in set(decomp.m_hubs):
                m_hub_totals[h] += d
                
    for h in decomp.m_hubs:
        assert m_hub_totals[h] == 2

def test_demand_coordinator_conflict_clause():
    G, degs = load_graph('FHCPCS-col/graph950.col')
    decomp = decompose_graph(G, degs)
    
    coord = GlobalDemandCoordinator(G, decomp)
    # Pick strip 0 and an adjacent M-hub
    si = 0
    adj_m = list(decomp.strip_adj_hubs[si] & set(decomp.m_hubs))
    assert len(adj_m) > 0
    h = adj_m[0]
    
    # Force strip 0 to use hub h
    coord.solver.add_clause([coord.var_d1[(si, h)]])
    is_sat, hh_edges, strip_demands = coord.solve_assignment()
    assert is_sat is True
    assert strip_demands[si][h] >= 1
    
    # Add conflict clause excluding hub h from strip si
    coord.add_conflict_clause(si, [h])
    # Now it must be UNSAT because we asserted both var_d1[(si, h)] and -var_d1[(si, h)]
    is_sat_after, _, _ = coord.solve_assignment()
    assert is_sat_after is False

def test_demand_coordinator_macro_cut_clause():
    G, degs = load_graph('FHCPCS-col/graph950.col')
    decomp = decompose_graph(G, degs)
    
    coord = GlobalDemandCoordinator(G, decomp)
    cut = set(decomp.m_hubs[:5])
    coord.add_macro_cut_clause(cut)
    is_sat, hh_edges, strip_demands = coord.solve_assignment()
    assert is_sat is True
