"""
Generic degree-2 recursive chain contraction and expansion module.
Reduces graph size by replacing degree-2 paths with shortcut macro-edges,
preserving Hamiltonian cycle reachability.
"""

from collections import defaultdict
from typing import Dict, List, Set, Tuple

def contract_degree2_chains(
    adj: Dict[int, Set[int]],
    nodes: Set[int],
    protected_nodes: Set[int]
) -> Tuple[Set[int], Dict[int, Set[int]], Dict[Tuple[int, int], List[int]]]:
    """
    Recursively contracts degree-2 vertices in `nodes` that are not in `protected_nodes`.
    Returns:
    - rem_nodes: remaining active vertices
    - contracted_adj: adjacency map on remaining vertices
    - edge_chains: mapping from shortcut edge (u, w) to the expanded node list [u, v1, ..., w]
    """
    sub_adj: Dict[int, Set[int]] = defaultdict(set)
    for u in nodes:
        for v in adj[u]:
            if v in nodes:
                sub_adj[u].add(v)
                sub_adj[v].add(u)

    rem = set(nodes)
    edge_chains: Dict[Tuple[int, int], List[int]] = {}

    while True:
        d2_candidates = [u for u in rem if u not in protected_nodes and len(sub_adj[u]) == 2]
        if not d2_candidates:
            break

        v = d2_candidates[0]
        u, w = list(sub_adj[v])
        sub_adj[u].remove(v)
        sub_adj[w].remove(v)
        del sub_adj[v]
        rem.remove(v)

        e_uv = tuple(sorted([u, v]))
        e_vw = tuple(sorted([v, w]))
        c_uv = edge_chains.pop(e_uv, [u, v])
        c_vw = edge_chains.pop(e_vw, [v, w])

        if c_uv[-1] != v:
            c_uv = list(reversed(c_uv))
        if c_vw[0] != v:
            c_vw = list(reversed(c_vw))

        if w in sub_adj[u]:
            sub_adj[u].remove(w)
            sub_adj[w].remove(u)

        e_uw = tuple(sorted([u, w]))
        sub_adj[u].add(w)
        sub_adj[w].add(u)
        edge_chains[e_uw] = c_uv[:-1] + c_vw

    return rem, dict(sub_adj), edge_chains


def expand_contracted_cycle(
    contracted_cycle: List[int],
    edge_chains: Dict[Tuple[int, int], List[int]]
) -> List[int]:
    """
    Expands macro-edges in contracted_cycle back to full node sequences using edge_chains.
    """
    expanded = []
    n = len(contracted_cycle)
    for i in range(n):
        u = contracted_cycle[i]
        v = contracted_cycle[(i + 1) % n]
        e = tuple(sorted([u, v]))
        if e in edge_chains:
            chain = edge_chains[e]
            if chain[0] != u:
                chain = list(reversed(chain))
            expanded.extend(chain[:-1])
        else:
            expanded.append(u)
    return expanded
