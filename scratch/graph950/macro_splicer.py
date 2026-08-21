import collections
from typing import Dict, List, Set, Tuple, Any, Optional
from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType

def verify_tour_on_raw_graph(tour: List[int], G: Dict[int, Set[int]]) -> bool:
    """Independently verifies that tour is a valid Hamiltonian cycle on raw graph G."""
    n = len(G)
    if len(tour) != n:
        return False
    if len(set(tour)) != n:
        return False
    for i in range(n):
        u = tour[i]
        v = tour[(i + 1) % n]
        if v not in G[u]:
            return False
    return True

def _find_boundary_matching(
    G: Dict[int, Set[int]],
    all_hubs: Set[int],
    hh_edges: List[Tuple[int, int]],
    endpoint_slots: List[Tuple[int, int]]
) -> Optional[List[Tuple[int, int]]]:
    """
    Finds a valid degree-preserving matching between strip path endpoints and Hubs.
    Each endpoint slot must connect to exactly 1 hub in G[u] & all_hubs.
    Each hub h must connect to exactly (2 - deg_hh(h)) endpoint slots.
    """
    # 1. Compute hub capacities
    hub_deg_hh = collections.defaultdict(int)
    for u, v in hh_edges:
        if u in all_hubs:
            hub_deg_hh[u] += 1
        if v in all_hubs:
            hub_deg_hh[v] += 1
            
    hub_cap = {}
    for h in all_hubs:
        cap = 2 - hub_deg_hh[h]
        if cap < 0:
            return None
        hub_cap[h] = cap
        
    total_needed_cap = sum(hub_cap.values())
    if total_needed_cap != len(endpoint_slots):
        return None
        
    # 2. Build SAT matching problem
    solver = Cadical195()
    nv = 0
    var_map = {} # (slot_idx, hub) -> var_id
    slot_vars = collections.defaultdict(list)
    hub_vars = collections.defaultdict(list)
    
    for slot_idx, (ep_u, slot_sub_id) in enumerate(endpoint_slots):
        adj_hubs = [h for h in G[ep_u] if h in all_hubs]
        if not adj_hubs:
            return None
        for h in adj_hubs:
            nv += 1
            var_map[(slot_idx, h)] = nv
            slot_vars[slot_idx].append(nv)
            hub_vars[h].append(nv)
            
    # Each slot connects to exactly 1 hub
    for slot_idx in range(len(endpoint_slots)):
        lits = slot_vars[slot_idx]
        if not lits:
            return None
        if len(lits) == 1:
            solver.add_clause([lits[0]])
        else:
            cnf = CardEnc.equals(lits=lits, bound=1, top_id=nv, encoding=EncType.seqcounter)
            nv = max(nv, cnf.nv)
            for cl in cnf.clauses:
                solver.add_clause(cl)
                
    # If the same endpoint vertex has 2 slots, prevent connecting to the same hub twice
    slots_by_vertex = collections.defaultdict(list)
    for slot_idx, (ep_u, slot_sub_id) in enumerate(endpoint_slots):
        slots_by_vertex[ep_u].append(slot_idx)
    for ep_u, s_indices in slots_by_vertex.items():
        if len(s_indices) == 2:
            s1, s2 = s_indices[0], s_indices[1]
            for h in G[ep_u]:
                if h in all_hubs:
                    v1 = var_map.get((s1, h))
                    v2 = var_map.get((s2, h))
                    if v1 and v2:
                        solver.add_clause([-v1, -v2])
                        
    # Each hub h connects to exactly hub_cap[h] slots
    for h, cap in hub_cap.items():
        lits = hub_vars[h]
        if len(lits) < cap:
            return None
        if cap == 0:
            for lit in lits:
                solver.add_clause([-lit])
        elif cap == len(lits):
            for lit in lits:
                solver.add_clause([lit])
        else:
            cnf = CardEnc.equals(lits=lits, bound=cap, top_id=nv, encoding=EncType.seqcounter)
            nv = max(nv, cnf.nv)
            for cl in cnf.clauses:
                solver.add_clause(cl)
                
    if not solver.solve():
        return None
        
    model = solver.get_model()
    m_bool = {abs(x): x > 0 for x in model}
    
    boundary_edges = []
    for (slot_idx, h), var_id in var_map.items():
        if m_bool.get(var_id, False):
            ep_u = endpoint_slots[slot_idx][0]
            boundary_edges.append((ep_u, h))
            
    return boundary_edges

def splice_macro_tour(
    G: Dict[int, Set[int]],
    decomp: Any,
    hh_edges: List[Tuple[int, int]],
    strip_paths: Any,
    boundary_edges: Optional[List[Tuple[int, int]]] = None
) -> Tuple[bool, Any]:
    """
    Robustly connects all strip internal paths with Hub-Hub edges and Hub-strip boundary edges.
    Returns (True, tour_vertex_list) if a single Hamiltonian cycle is formed.
    Returns (False, list_of_cycles) if disconnected subtours or invalid degrees occur.
    """
    internal_edges = []
    endpoint_slots = [] # list of (endpoint_vertex, slot_id)
    
    # Standardize strip_paths iteration
    if isinstance(strip_paths, dict):
        strip_items = sorted(strip_paths.items())
    elif isinstance(strip_paths, list):
        strip_items = list(enumerate(strip_paths))
    else:
        return False, []
        
    for si, paths in strip_items:
        for p in paths:
            if not p:
                continue
            if len(p) == 1:
                # Single-vertex path needs 2 boundary connections
                endpoint_slots.append((p[0], 0))
                endpoint_slots.append((p[0], 1))
            else:
                for i in range(len(p) - 1):
                    internal_edges.append((p[i], p[i + 1]))
                endpoint_slots.append((p[0], 0))
                endpoint_slots.append((p[-1], 0))
                
    all_hubs = set(decomp.all_hubs)
    
    # Determine boundary edges if not provided
    if boundary_edges is None:
        boundary_edges = _find_boundary_matching(G, all_hubs, hh_edges, endpoint_slots)
        if boundary_edges is None:
            return False, []
            
    # Build 2-factor adjacency graph
    adj_2f = collections.defaultdict(list)
    
    for u, v in internal_edges:
        adj_2f[u].append(v)
        adj_2f[v].append(u)
        
    for u, v in hh_edges:
        adj_2f[u].append(v)
        adj_2f[v].append(u)
        
    for u, v in boundary_edges:
        adj_2f[u].append(v)
        adj_2f[v].append(u)
        
    # Check degree == 2 for all vertices in G
    for u in G:
        if len(adj_2f[u]) != 2:
            return False, []
            
    # Extract disjoint cycles
    visited = set()
    cycles = []
    
    for v0 in sorted(G.keys()):
        if v0 not in visited:
            cyc = [v0]
            visited.add(v0)
            curr = v0
            prev = None
            while True:
                nxts = [w for w in adj_2f[curr] if w != prev]
                if not nxts or nxts[0] == v0:
                    break
                nxt = nxts[0]
                cyc.append(nxt)
                visited.add(nxt)
                prev, curr = curr, nxt
            cycles.append(cyc)
            
    if len(cycles) == 1 and len(cycles[0]) == len(G):
        tour = cycles[0]
        if verify_tour_on_raw_graph(tour, G):
            return True, tour
        return False, tour
        
    return False, cycles

def splice_and_verify_tour(
    G: Dict[int, Set[int]],
    decomp: Any,
    hh_edges: List[Tuple[int, int]],
    strip_paths: Any,
    boundary_edges: Optional[List[Tuple[int, int]]] = None
) -> Tuple[bool, List[int]]:
    """
    High-level interface: splices macro tour and returns (is_valid, tour_vertex_list).
    """
    is_valid, res = splice_macro_tour(G, decomp, hh_edges, strip_paths, boundary_edges)
    if is_valid:
        return True, res
    return False, []
