import collections
from typing import Dict, List, Set, Tuple, Any
from scratch.graph950.two_half_two_tier_solver import load_graph, decompose_half

def load_and_partition_graph982(col_path: str):
    G, degs = load_graph(col_path)
    h1_roots = {1714, 5022, 5575, 5852, 6696}
    h2_roots = {2907, 4740, 5378, 6335, 6956}
    s_hubs = h1_roots | h2_roots

    dist, owner = {}, {}
    q = collections.deque()
    for s in s_hubs:
        owner[s] = s
        dist[s] = 0
        q.append(s)
    while q:
        u = q.popleft()
        for v in G[u]:
            if v not in dist:
                dist[v] = dist[u] + 1
                owner[v] = owner[u]
                q.append(v)

    grp1 = set(u for u in G if owner[u] in h1_roots)
    grp2 = set(u for u in G if owner[u] in h2_roots)

    # Detect exact cut edges
    bridges = []
    for u in grp1:
        for v in G[u]:
            if v in grp2 and u < v:
                bridges.append((u, v))

    all_hubs1, strips1, hh1, strip_adj_hubs1, _ = decompose_half(G, degs, grp1)
    all_hubs2, strips2, hh2, strip_adj_hubs2, _ = decompose_half(G, degs, grp2)

    # Group configurations for Half 1 (each group = 5 large strips + 2 tiny strips = 760v)
    # Tiny strips: 6 of len 3 (25..30), 6 of len 2 (31..36)
    h1_targets = [
        (1714, 462, 2351, {"clusters": [8, 10, 11, 14, 20], "tiny": [28, 35]}),
        (5022, 1495, 6670, {"clusters": [0, 3, 4, 7, 16], "tiny": [26, 31]}),
        (5575, 4740, 3450, {"clusters": [5, 6, 9, 22, 24], "tiny": [30, 32]}),
        (5852, 1481, 5852, {"clusters": [12, 13, 18, 19, 21], "tiny": [27, 34]}),
        (6696, 2673, 4420, {"clusters": [1, 2, 15, 17, 23], "tiny": [29, 33]}),
    ]

    # Group configurations for Half 2
    h2_targets = [
        (2907, 2400, 7123, {"clusters": [0, 6, 7, 8, 20], "tiny": [26, 34]}),
        (4740, 5575, 1209, {"clusters": [5, 10, 12, 23, 24], "tiny": [30, 31]}),
        (5378, 7311, 3120, {"clusters": [4, 15, 17, 18, 22], "tiny": [29, 35]}),
        (6335, 6335, 1495, {"clusters": [2, 9, 11, 14, 19], "tiny": [25, 33]}),
        (6956, 4209, 6804, {"clusters": [1, 3, 13, 16, 21], "tiny": [28, 32]}),
    ]

    return G, degs, grp1, grp2, h1_targets, h2_targets, bridges
