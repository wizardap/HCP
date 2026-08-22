import collections
from typing import Dict, List, Set, Tuple, Any, Optional

def verify_tour_on_raw_graph(tour: List[int], G: Dict[int, Set[int]]) -> bool:
    """
    Independent Raw Graph Verifier.
    Checks:
    1. Tour length equals n
    2. All vertices are distinct
    3. Every consecutive pair (tour[i], tour[(i+1)%n]) is in E(G)
    """
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

def _find_boundary_matching_local(
    G: Dict[int, Set[int]],
    decomp: Any,
    hh_edges: List[Tuple[int, int]],
    strip_paths: Dict[int, List[List[int]]],
    strip_demands: Optional[Dict[int, Dict[int, int]]] = None
) -> Optional[List[Tuple[int, int]]]:
    """
    Directly connects endpoints of each strip to their assigned adjacent Hubs based on the exact demand vector.
    """
    boundary_edges = []
    s_hub_set = set(decomp.s_hubs)
    b_hub_set = set(decomp.b_hubs)
    
    for si, paths in strip_paths.items():
        endpoints = []
        for p in paths:
            if len(p) == 1:
                endpoints.append(p[0])
                endpoints.append(p[0])
            else:
                endpoints.append(p[0])
                endpoints.append(p[-1])
                
        assigned_slots = set()
        dem = strip_demands.get(si, {}) if strip_demands else {}
        
        # 1. Connect M-hubs first (restricted selective ports)
        for h, req in dem.items():
            if req > 0 and h not in s_hub_set and h not in b_hub_set:
                candidates = [idx for idx, ep in enumerate(endpoints) if idx not in assigned_slots and h in G[ep]]
                for idx in candidates[:req]:
                    boundary_edges.append((endpoints[idx], h))
                    assigned_slots.add(idx)
                    
        # 2. Connect S-hubs and B-hubs (universal ports)
        for h, req in dem.items():
            if req > 0 and (h in s_hub_set or h in b_hub_set):
                candidates = [idx for idx, ep in enumerate(endpoints) if idx not in assigned_slots and h in G[ep]]
                for idx in candidates[:req]:
                    boundary_edges.append((endpoints[idx], h))
                    assigned_slots.add(idx)
                    
        if len(assigned_slots) != len(endpoints):
            return None
            
    return boundary_edges

def patch_cycles_2opt(cycles: List[List[int]], G: Dict[int, Set[int]]) -> List[List[int]]:
    """
    Fast local 2-opt cycle patching to merge adjacent subcycles into fewer, larger cycles.
    """
    merged = True
    while merged and len(cycles) > 1:
        merged = False
        v_to_cyc = {}
        for ci, c in enumerate(cycles):
            for v in c:
                v_to_cyc[v] = ci
                
        for ci in range(len(cycles)):
            if merged:
                break
            c1 = cycles[ci]
            n1 = len(c1)
            for i in range(n1):
                if merged:
                    break
                u1 = c1[i]
                v1 = c1[(i + 1) % n1]
                for u2 in G[u1]:
                    c2_idx = v_to_cyc.get(u2)
                    if c2_idx is not None and c2_idx != ci:
                        c2 = cycles[c2_idx]
                        n2 = len(c2)
                        j = c2.index(u2)
                        v2 = c2[(j + 1) % n2]
                        if v2 in G[v1]:
                            new_c = c1[:i+1] + c2[j::-1] + c2[:j:-1] + c1[i+1:]
                            cycles[ci] = new_c
                            cycles.pop(c2_idx)
                            merged = True
                            break
                        v2_rev = c2[(j - 1) % n2]
                        if v2_rev in G[v1]:
                            new_c = c1[:i+1] + c2[j:] + c2[:j] + c1[i+1:]
                            cycles[ci] = new_c
                            cycles.pop(c2_idx)
                            merged = True
                            break
    return cycles

def splice_macro_tour(
    G: Dict[int, Set[int]],
    decomp: Any,
    hh_edges: List[Tuple[int, int]],
    strip_paths: Any,
    strip_demands: Optional[Dict[int, Dict[int, int]]] = None,
    boundary_edges: Optional[List[Tuple[int, int]]] = None,
    enable_patching: bool = True
) -> Tuple[bool, Any]:
    """
    Robustly connects all strip internal paths with Hub-Hub edges and Hub-strip boundary edges.
    Returns (True, tour_vertex_list) if a single Hamiltonian cycle is formed.
    Returns (False, list_of_cycles) if disconnected subtours or invalid degrees occur.
    """
    internal_edges = []
    
    # Standardize strip_paths
    clean_strip_paths = {}
    if isinstance(strip_paths, dict):
        for si, paths in strip_paths.items():
            if paths and isinstance(paths[0], int):
                clean_strip_paths[si] = [paths]
            else:
                clean_strip_paths[si] = paths
    elif isinstance(strip_paths, list):
        for si, paths in enumerate(strip_paths):
            if paths and isinstance(paths[0], int):
                clean_strip_paths[si] = [paths]
            else:
                clean_strip_paths[si] = paths
                
    for si, paths in clean_strip_paths.items():
        for p in paths:
            if len(p) > 1:
                for i in range(len(p) - 1):
                    internal_edges.append((p[i], p[i + 1]))
                    
    # Determine boundary edges if not provided
    if boundary_edges is None:
        boundary_edges = _find_boundary_matching_local(G, decomp, hh_edges, clean_strip_paths, strip_demands)
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
            
    if enable_patching and len(cycles) > 1:
        cycles = patch_cycles_2opt(cycles, G)
        
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
    strip_demands: Optional[Dict[int, Dict[int, int]]] = None,
    boundary_edges: Optional[List[Tuple[int, int]]] = None
) -> Tuple[bool, List[int]]:
    is_valid, res = splice_macro_tour(G, decomp, hh_edges, strip_paths, strip_demands, boundary_edges)
    if is_valid:
        return True, res
    return False, []
