import collections
from typing import Dict, Set

def load_dimacs(col_path: str) -> Dict[int, Set[int]]:
    """
    Loads an undirected graph from a DIMACS .col file.
    Vertices are 1-indexed integers.
    """
    G = collections.defaultdict(set)
    with open(col_path, "r") as f:
        for line in f:
            if line.startswith("e "):
                parts = line.split()
                u, v = int(parts[1]), int(parts[2])
                G[u].add(v)
                G[v].add(u)
    return dict(G)
