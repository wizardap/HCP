import collections
from typing import Dict, List, Set, Tuple

def decompose_modular_graph(G: Dict[int, Set[int]]) -> Tuple[List[Dict], Set[int]]:
    """
    Fast structural decomposition for modular graphs with degree-14 hubs.
    Isolates buffer blocks (15v) and multi-module linear corridor chains,
    leaving a clean Comp 0.
    """
    hubs = {v for v, nbrs in G.items() if len(nbrs) == 14}
    if len(hubs) < 20:
        return [], set(G.keys())

    rem_hubs = set(G.keys()) - hubs
    vis = set()
    comps = []
    for u in rem_hubs:
        if u not in vis:
            q = [u]
            vis.add(u)
            c = []
            for x in q:
                c.append(x)
                for nxt in G[x]:
                    if nxt in rem_hubs and nxt not in vis:
                        vis.add(nxt)
                        q.append(nxt)
            comps.append(set(c))

    big_comps = [c for c in comps if len(c) > 500]
    if len(big_comps) != 1:
        return [], set(G.keys())
    big_c = big_comps[0]

    comp0_hubs = {h for h in hubs if len(G[h] & big_c) > 0}
    corridor_hubs = hubs - comp0_hubs

    small_comps = [c for c in comps if len(c) <= 500]
    v_corr = set()
    for c in small_comps:
        v_corr.update(c)
    v_corr.update(corridor_hubs)
    v_comp0 = set(G.keys()) - v_corr

    cross_edges = []
    for u in v_corr:
        for v in G[u]:
            if v in v_comp0:
                cross_edges.append((u, v))

    corridors = []
    remaining_corr = set(v_corr)
    for c in small_comps:
        c_ce = [e for e in cross_edges if e[0] in c]
        if len(c_ce) == 2:
            corridors.append({
                'nodes': set(c),
                'ports': (c_ce[0][0], c_ce[1][0]),
                'ext_ports': (c_ce[0][1], c_ce[1][1]),
                'size': len(c)
            })
            remaining_corr -= c

    vis2 = set()
    for u in remaining_corr:
        if u not in vis2:
            q = [u]
            vis2.add(u)
            c = []
            for x in q:
                c.append(x)
                for nxt in G[x]:
                    if nxt in remaining_corr and nxt not in vis2:
                        vis2.add(nxt)
                        q.append(nxt)
            c_set = set(c)
            c_ce = [e for e in cross_edges if e[0] in c_set]
            if len(c_ce) == 2:
                corridors.append({
                    'nodes': c_set,
                    'ports': (c_ce[0][0], c_ce[1][0]),
                    'ext_ports': (c_ce[0][1], c_ce[1][1]),
                    'size': len(c_set)
                })

    if corridors:
        v_comp0 = set(G.keys()) - set().union(*[c['nodes'] for c in corridors])
    else:
        v_comp0 = set(G.keys())

    return corridors, v_comp0

def detect_bridge_corridors(G: Dict[int, Set[int]], min_size: int = 50, max_size: int = 2000) -> Tuple[List[Dict], Set[int]]:
    """
    Detects bridge corridors in graph G.
    First checks for structural modular chains. If not applicable,
    checks 2-vertex cuts on candidate hubs.
    """
    mod_corridors, mod_comp0 = decompose_modular_graph(G)
    if mod_corridors:
        return mod_corridors, mod_comp0

    hubs = [u for u in G if len(G[u]) == 14]
    if not hubs:
        return [], set(G.keys())

    all_nodes = set(G.keys())
    candidates = []

    for i in range(len(hubs)):
        for j in range(i + 1, len(hubs)):
            u, v = hubs[i], hubs[j]
            rem = all_nodes - {u, v}
            visited = set()
            comps = []
            for w in rem:
                if w not in visited:
                    c = []
                    q = [w]
                    visited.add(w)
                    for x in q:
                        c.append(x)
                        for nxt in G[x]:
                            if nxt in rem and nxt not in visited:
                                visited.add(nxt)
                                q.append(nxt)
                    comps.append(set(c))
            for c in comps:
                if min_size <= len(c) <= max_size:
                    if any(u in G[w] for w in c) and any(v in G[w] for w in c):
                        rest = rem - c
                        ext_u = list(G[u] & rest)
                        ext_v = list(G[v] & rest)
                        if len(ext_u) == 1 and len(ext_v) == 1:
                            candidates.append({
                                'nodes': set(c) | {u, v},
                                'ports': (u, v),
                                'ext_ports': (ext_u[0], ext_v[0]),
                                'size': len(c) + 2
                            })

    candidates.sort(key=lambda x: x['size'], reverse=True)
    allocated = set()
    corridors = []
    for cand in candidates:
        if not (cand['nodes'] & allocated):
            if cand['ext_ports'][0] not in allocated and cand['ext_ports'][1] not in allocated:
                allocated.update(cand['nodes'])
                corridors.append(cand)

    comp0_nodes = all_nodes - allocated
    return corridors, comp0_nodes
