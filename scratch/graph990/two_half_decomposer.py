import collections
from typing import Dict, List, Set, Tuple, Any
from scratch.graph950.two_half_two_tier_solver import load_graph

def load_and_partition_graph990(col_path: str):
    G, degs = load_graph(col_path)
    h1_roots = {3076, 3517, 3728, 5293, 6726}
    h2_roots = {2205, 3905, 4178, 7717, 7858}
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

    # Detect cut edges
    bridges = []
    for u in grp1:
        for v in G[u]:
            if v in grp2:
                bridges.append((u, v) if u < v else (v, u))
    bridges = sorted(list(set(bridges)))

    # Group targets for Half 1 (5 groups x 800v)
    h1_targets = [
        (5293, 7644, 5381, {"clusters": [5, 6, 11, 16, 18], "tiny": [26, 35]}),
        (3728, 3862, 3071, {"clusters": [4, 7, 9, 17, 23], "tiny": [27, 33]}),
        (3076, 3494, 4316, {"clusters": [8, 10, 14, 15, 24], "tiny": [29, 34]}),
        (3517, 3455, 3729, {"clusters": [0, 2, 3, 13, 22], "tiny": [30, 31]}),
        (6726, 3391, 6248, {"clusters": [1, 12, 19, 20, 21], "tiny": [28, 36]}),
    ]

    # Group targets for Half 2 (5 groups x 800v)
    h2_targets = [
        (2205, 304, 1029, {"clusters": [2, 9, 10, 17, 20], "tiny": [29, 31]}),
        (7858, 1272, 6331, {"clusters": [1, 4, 5, 7, 12], "tiny": [28, 34]}),
        (3905, 4011, 5747, {"clusters": [8, 13, 16, 23, 24], "tiny": [25, 36]}),
        (4178, 340, 4340, {"clusters": [0, 11, 15, 19, 22], "tiny": [27, 32]}),
        (7717, 1121, 7436, {"clusters": [3, 6, 14, 18, 21], "tiny": [30, 35]}),
    ]

    return G, degs, grp1, grp2, h1_targets, h2_targets, bridges
