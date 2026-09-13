import collections
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

def load_and_decompose_graph710(col_path: str) -> Tuple[Dict[int, Set[int]], Set[int], Set[int], int, int]:
    G = load_graph(col_path)
    assert len(G) == 4064

    port_u = 1876
    port_v = 2491
    cut = {port_u, port_v}
    rem_nodes = set(G.keys()) - cut

    # Find connected components in G \ cut
    visited = set()
    comps = []
    for u in rem_nodes:
        if u not in visited:
            c = []
            q = [u]
            visited.add(u)
            for x in q:
                c.append(x)
                for nbr in G[x]:
                    if nbr in rem_nodes and nbr not in visited:
                        visited.add(nbr)
                        q.append(nbr)
            comps.append(set(c))

    assert len(comps) == 2, f"Expected 2 components, found {len(comps)}"
    comps.sort(key=len)
    comp_a, comp_b = comps[0], comps[1]

    assert len(comp_a) == 885
    assert len(comp_b) == 3177

    V_A = comp_a | cut
    V_B = comp_b | cut

    return G, V_A, V_B, port_u, port_v
