import collections, os
from typing import Dict, Set, Tuple

def load_graph(col_path: str) -> Dict[int, Set[int]]:
    G = collections.defaultdict(set)
    with open(col_path, "r") as f:
        for line in f:
            if line.startswith("e "):
                parts = line.split()
                u, v = int(parts[1]), int(parts[2])
                G[u].add(v)
                G[v].add(u)
    return dict(G)

def load_and_decompose_graph717(col_path: str) -> Tuple[Dict[int, Set[int]], Dict[Tuple[int, int], Set[int]], Set[int], Set[int], Set[int]]:
    G = load_graph(col_path)
    assert len(G) == 4122

    cut_nodes = {255, 540, 577, 702, 773, 1016, 1177, 1213, 1389, 1955, 2467, 2609, 2677, 2681, 3358, 3986}
    rem_16 = set(G.keys()) - cut_nodes

    visited = set()
    comps = []
    for u in rem_16:
        if u not in visited:
            c = []
            q = [u]
            visited.add(u)
            for x in q:
                c.append(x)
                for nbr in G[x]:
                    if nbr in rem_16 and nbr not in visited:
                        visited.add(nbr)
                        q.append(nbr)
            comps.append(set(c))

    assert len(comps) == 7

    modules = {}
    comp0_internal = None
    for c in comps:
        if len(c) == 3104:
            comp0_internal = c
        elif len(c) == 167:
            ports = tuple(sorted([nbr for u in c for nbr in G[u] if nbr in cut_nodes]))
            ports_set = tuple(sorted(list(set(ports))))
            assert len(ports_set) == 2
            modules[ports_set] = c

    assert len(modules) == 6
    assert comp0_internal is not None

    ports_c0 = {255, 1955, 2609, 3358}
    comp0_nodes = comp0_internal | ports_c0

    # Chain 1: modules (1389, 2677), (1213, 2681), (702, 773) + intermediate cut nodes + ports 255, 1955
    mod2 = modules[tuple(sorted([1389, 2677]))]
    mod4 = modules[tuple(sorted([1213, 2681]))]
    mod6 = modules[tuple(sorted([702, 773]))]
    chain1_nodes = mod2 | mod4 | mod6 | {255, 1389, 2677, 1213, 2681, 773, 702, 1955}
    assert len(chain1_nodes) == 509

    # Chain 2: modules (1016, 3986), (1177, 2467), (540, 577) + intermediate cut nodes + ports 3358, 2609
    mod1 = modules[tuple(sorted([1016, 3986]))]
    mod5 = modules[tuple(sorted([1177, 2467]))]
    mod3 = modules[tuple(sorted([540, 577]))]
    chain2_nodes = mod1 | mod5 | mod3 | {3358, 3986, 1016, 1177, 2467, 577, 540, 2609}
    assert len(chain2_nodes) == 509

    return G, modules, chain1_nodes, chain2_nodes, comp0_nodes
