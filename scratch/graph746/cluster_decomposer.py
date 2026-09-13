import collections
from typing import Dict, List, Set, Tuple, Any
from scratch.graph950.two_half_two_tier_solver import load_graph

def load_and_decompose_graph746(col_path: str) -> Tuple[
    Dict[int, Set[int]],
    Dict[int, int],
    List[Tuple[int, int, int, Dict[str, List[int]]]],
    Set[int],
    Dict[int, Set[int]]
]:
    G, degs = load_graph(col_path)
    super_hubs = sorted([u for u in degs if degs[u] >= 500])
    assert super_hubs == [1430, 3641, 3735, 3790, 3960]

    hubs = set(u for u in degs if degs[u] >= 16)
    bulk = set(G.keys()) - hubs

    adj_bulk = {u: G[u] & bulk for u in bulk}
    visited = set()
    strips = []
    for u in bulk:
        if u not in visited:
            c = []
            q = [u]
            visited.add(u)
            for x in q:
                c.append(x)
                for y in adj_bulk[x]:
                    if y not in visited:
                        visited.add(y)
                        q.append(y)
            strips.append(c)
    strips.sort(key=len, reverse=True)

    sh_cfgs = {
        1430: {"large": [3, 8, 9, 13, 16], "med": [27, 29, 30, 34, 46], "tiny": [50, 57]},
        3641: {"large": [5, 11, 12, 15, 24], "med": [25, 32, 37, 44, 45], "tiny": [53, 56]},
        3735: {"large": [0, 2, 20, 21, 22], "med": [26, 28, 35, 36, 41], "tiny": [52, 58]},
        3790: {"large": [4, 10, 14, 17, 18], "med": [31, 33, 39, 43, 47], "tiny": [51, 59]},
        3960: {"large": [1, 6, 7, 19, 23], "med": [38, 40, 42, 48, 49], "tiny": [54, 61]}
    }

    strip_adj_hubs = collections.defaultdict(set)
    for si, s in enumerate(strips):
        for u in s:
            for nbr in G[u]:
                if nbr in hubs:
                    strip_adj_hubs[si].add(nbr)

    bulks = {}
    for sh, cfg in sh_cfgs.items():
        v = set()
        for si in cfg["large"] + cfg["med"]:
            v.update(strips[si])
            for h in strip_adj_hubs[si]:
                if degs[h] < 500:
                    v.add(h)
        for ti in cfg["tiny"]:
            v.update(strips[ti])
        bulks[sh] = v

    macro_nodes = set(G.keys()) - set().union(*bulks.values())
    assert len(macro_nodes) == 11

    group_targets = [
        (1430, 3003, 2623, sh_cfgs[1430]),
        (3790, 2165, 1264, sh_cfgs[3790]),
        (3960, 1025, 3498, sh_cfgs[3960]),
        (3641, 3146, 2397, sh_cfgs[3641]),
        (3735, 46, 3547, sh_cfgs[3735]),
    ]

    return G, degs, group_targets, macro_nodes, bulks
