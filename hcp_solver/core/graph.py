"""
Graph representation and fast DIMACS .col reader.
"""

from collections import defaultdict
from typing import Dict, Set, Tuple, List

class Graph:
    def __init__(self, adj: Dict[int, Set[int]], name: str = ""):
        self.adj = adj
        self.name = name
        self.num_vertices = len(adj)
        self.num_edges = sum(len(v) for v in adj.values()) // 2
        self.degrees = {u: len(nbrs) for u, nbrs in adj.items()}

    def __len__(self) -> int:
        return self.num_vertices

    def __getitem__(self, u: int) -> Set[int]:
        return self.adj.get(u, set())

    def __contains__(self, u: int) -> bool:
        return u in self.adj

    def neighbors(self, u: int) -> Set[int]:
        return self.adj.get(u, set())

    def degree(self, u: int) -> int:
        return self.degrees.get(u, 0)

    def has_edge(self, u: int, v: int) -> bool:
        return v in self.adj.get(u, set())


def load_graph(col_path: str) -> Dict[int, Set[int]]:
    """
    Parse a standard DIMACS .col file and return an undirected adjacency dictionary.
    """
    adj: Dict[int, Set[int]] = defaultdict(set)
    with open(col_path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("c"):
                continue
            if line.startswith("p "):
                continue
            if line.startswith("e "):
                parts = line.split()
                if len(parts) >= 3:
                    u, v = int(parts[1]), int(parts[2])
                    adj[u].add(v)
                    adj[v].add(u)
    return dict(adj)
