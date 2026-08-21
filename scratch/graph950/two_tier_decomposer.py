import collections
from dataclasses import dataclass
from typing import Dict, List, Set, Tuple

@dataclass
class DecompositionResult:
    all_hubs: Set[int]
    s_hubs: List[int]
    b_hubs: List[int]
    m_hubs: List[int]
    hh_edges: List[Tuple[int, int]]
    strips: List[List[int]]
    strip_adj_hubs: Dict[int, Set[int]]
    hub_adj_strips: Dict[int, Set[int]]
    strip_hub_ports: Dict[Tuple[int, int], List[int]]

def load_graph(path: str) -> Tuple[Dict[int, Set[int]], Dict[int, int]]:
    G = collections.defaultdict(set)
    with open(path, 'r') as f:
        for line in f:
            if line.startswith('e '):
                parts = line.split()
                u, v = int(parts[1]), int(parts[2])
                G[u].add(v)
                G[v].add(u)
    degs = {u: len(G[u]) for u in G}
    return G, degs

def decompose_graph(G: Dict[int, Set[int]], degs: Dict[int, int], hub_threshold: int = 20) -> DecompositionResult:
    all_hubs = {u for u, d in degs.items() if d >= hub_threshold}
    s_hubs = sorted([u for u in all_hubs if degs[u] > 300])
    b_hubs = sorted([u for u in all_hubs if 100 <= degs[u] <= 300])
    m_hubs = sorted([u for u in all_hubs if degs[u] < 100])
    
    bulk = set(G.keys()) - all_hubs
    visited = set()
    strips = []
    for u in sorted(bulk):
        if u not in visited:
            comp = []
            q = [u]
            visited.add(u)
            for curr in q:
                comp.append(curr)
                for nbr in G[curr]:
                    if nbr in bulk and nbr not in visited:
                        visited.add(nbr)
                        q.append(nbr)
            strips.append(sorted(comp))
    
    # Sort strips descending by size
    strips.sort(key=len, reverse=True)
    
    hh_edges = []
    for u in all_hubs:
        for v in G[u]:
            if v in all_hubs and u < v:
                hh_edges.append((u, v))
                
    strip_adj_hubs = collections.defaultdict(set)
    hub_adj_strips = collections.defaultdict(set)
    strip_hub_ports = collections.defaultdict(list)
    
    for si, s in enumerate(strips):
        for u in s:
            for nbr in G[u]:
                if nbr in all_hubs:
                    strip_adj_hubs[si].add(nbr)
                    hub_adj_strips[nbr].add(si)
                    strip_hub_ports[(si, nbr)].append(u)
                    
    return DecompositionResult(
        all_hubs=all_hubs,
        s_hubs=s_hubs,
        b_hubs=b_hubs,
        m_hubs=m_hubs,
        hh_edges=hh_edges,
        strips=strips,
        strip_adj_hubs=dict(strip_adj_hubs),
        hub_adj_strips=dict(hub_adj_strips),
        strip_hub_ports=dict(strip_hub_ports)
    )
