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

def load_and_decompose_graph882(col_path: str) -> Tuple[Dict[int, Set[int]], Set[int], Set[int], Set[int], Set[int]]:
    G = load_graph(col_path)
    assert len(G) == 5686

    # 2-vertex cut separating the 167-vertex module: {3195, 5200}
    cut_nodes = {3195, 5200}
    rem = set(G.keys()) - cut_nodes

    visited = set()
    comps = []
    for u in rem:
        if u not in visited:
            c = []
            q = [u]
            visited.add(u)
            for x in q:
                c.append(x)
                for nbr in G[x]:
                    if nbr in rem and nbr not in visited:
                        visited.add(nbr)
                        q.append(nbr)
            comps.append(set(c))

    # The small component of size 167 is the module
    small_mod = min(comps, key=len)
    assert len(small_mod) == 167
    mod_nodes = small_mod | cut_nodes
    assert len(mod_nodes) == 169

    # Buffer Block (Comp 2 + hub 4509): 15 nodes + hub 4509 = 16 nodes
    # Comp 2 contains 4779 which connects to 5066
    assert 4779 in G
    assert 5066 in G[4779]
    # Identify Comp 2 by BFS in G \ {all 20 hubs}
    hubs = {v for v, nbrs in G.items() if len(nbrs) == 14}
    assert len(hubs) == 20
    rem_hubs = set(G.keys()) - hubs

    visited_c2 = set()
    q_c2 = [4779]
    visited_c2.add(4779)
    for x in q_c2:
        for nbr in G[x]:
            if nbr in rem_hubs and nbr not in visited_c2:
                visited_c2.add(nbr)
                q_c2.append(nbr)
    comp2_nodes = set(q_c2)
    assert len(comp2_nodes) == 15
    assert 4509 in hubs
    block2_nodes = comp2_nodes | {4509}
    assert len(block2_nodes) == 16

    # Outer chain = mod_nodes | block2_nodes
    chain_nodes = mod_nodes | block2_nodes
    assert len(chain_nodes) == 185

    # Comp 0 = V \ chain_nodes
    comp0_nodes = set(G.keys()) - chain_nodes
    assert len(comp0_nodes) == 5501

    return G, mod_nodes, block2_nodes, chain_nodes, comp0_nodes
