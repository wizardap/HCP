import collections
from typing import Dict, List, Set, Tuple, Any
from scratch.graph950.two_half_two_tier_solver import load_graph

def load_and_partition_graph982(col_path: str):
    G, degs = load_graph(col_path)
    h1_roots = {4740, 5575, 1714, 6696, 5852}
    h2_roots = {6956, 2907, 5022, 6335, 5378}
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
            if v in grp2:
                bridges.append((u, v) if u < v else (v, u))
    bridges = sorted(list(set(bridges)))

    # Group configurations for Half 1 (each group = 5 large strips + 2 tiny strips = 760v)
    h1_targets = [
        (4740, 186, 6791, {"clusters": [2, 5, 11, 20, 23], "tiny": [30, 31]}),
        (5575, 2162, 4687, {"clusters": [3, 4, 7, 21, 24], "tiny": [29, 32]}),
        (1714, 1612, 6848, {"clusters": [6, 8, 9, 13, 18], "tiny": [27, 35]}),
        (6696, 1267, 7180, {"clusters": [0, 1, 14, 15, 22], "tiny": [28, 36]}),
        (5852, 184, 7199, {"clusters": [10, 12, 16, 17, 19], "tiny": [26, 34]}),
    ]

    # Group configurations for Half 2
    h2_targets = [
        (6956, 2067, 5172, {"clusters": [2, 4, 14, 17, 23], "tiny": [29, 31]}),
        (2907, 382, 6633, {"clusters": [1, 8, 9, 10, 22], "tiny": [26, 34]}),
        (5022, 1495, 6670, {"clusters": [0, 5, 6, 12, 20], "tiny": [27, 33]}),
        (6335, 1686, 6614, {"clusters": [3, 11, 13, 15, 21], "tiny": [25, 32]}),
        (5378, 2164, 7310, {"clusters": [7, 16, 18, 19, 24], "tiny": [30, 35]}),
    ]

    return G, degs, grp1, grp2, h1_targets, h2_targets, bridges
