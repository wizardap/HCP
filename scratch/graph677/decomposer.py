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

def find_2cut_module(G: Dict[int, Set[int]]) -> Tuple[Tuple[int, int], Set[int]]:
    """
    Search among degree-14 hubs for a 2-cut that isolates a 167-vertex module.
    """
    hubs = [u for u in G if len(G[u]) == 14]
    for i in range(len(hubs)):
        for j in range(i + 1, len(hubs)):
            u, v = hubs[i], hubs[j]
            rem = set(G.keys()) - {u, v}
            visited = set()
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
                    if len(c) == 167:
                        return (u, v), set(c)
    raise RuntimeError("Could not find 2-cut isolating 167-vertex module in graph677")

def load_and_decompose_graph677(col_path: str) -> Tuple[Dict[int, Set[int]], Set[int], Set[int], Tuple[int, int], Tuple[int, int]]:
    G = load_graph(col_path)
    assert len(G) == 3868, f"Expected 3868 vertices, got {len(G)}"

    (port1, port2), mod_int = find_2cut_module(G)
    mod_nodes = mod_int | {port1, port2}
    assert len(mod_nodes) == 169

    comp0_nodes = set(G.keys()) - mod_nodes
    assert len(comp0_nodes) == 3699

    p1_ext = list(G[port1].intersection(comp0_nodes))[0]
    p2_ext = list(G[port2].intersection(comp0_nodes))[0]

    return G, mod_nodes, comp0_nodes, (port1, port2), (p1_ext, p2_ext)
