import collections
from typing import Dict, List, Set, Tuple

def detect_bridge_corridors(G: Dict[int, Set[int]], min_size: int = 50, max_size: int = 1500) -> Tuple[List[Dict], Set[int]]:
    """
    Detects bridge corridors in graph G.
    A bridge corridor is a subgraph C attached via two port vertices (u, v)
    such that each port connects to exactly 1 external vertex in the rest of G (Comp 0).
    Returns:
      (corridors, comp0_nodes)
    where each corridor in corridors has:
      - 'nodes': Set[int] of vertices in the corridor (including internal ports)
      - 'ports': (u, v) internal ports
      - 'ext_ports': (e_u, e_v) external attachment ports in Comp 0
    """
    cand_nodes = [u for u in G if len(G[u]) >= 3]
    hubs = [u for u in G if len(G[u]) == 14]
    if not hubs:
        hubs = sorted(cand_nodes, key=lambda u: len(G[u]), reverse=True)[:60]

    all_nodes = set(G.keys())
    allocated_corridor_nodes = set()
    corridors = []

    for i in range(len(hubs)):
        for j in range(i + 1, len(hubs)):
            u, v = hubs[i], hubs[j]
            if u in allocated_corridor_nodes or v in allocated_corridor_nodes:
                continue
            rem = all_nodes - allocated_corridor_nodes - {u, v}
            visited = set()
            comps = []
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
                    comps.append(set(c))
            for c in comps:
                if min_size <= len(c) <= max_size and not (c & allocated_corridor_nodes):
                    # Check internal connection
                    if any(u in G[w] for w in c) and any(v in G[w] for w in c):
                        rest = rem - c
                        ext_u = list(G[u] & rest)
                        ext_v = list(G[v] & rest)
                        if len(ext_u) == 1 and len(ext_v) == 1:
                            corr_nodes = set(c) | {u, v}
                            allocated_corridor_nodes.update(corr_nodes)
                            corridors.append({
                                'nodes': corr_nodes,
                                'ports': (u, v),
                                'ext_ports': (ext_u[0], ext_v[0])
                            })
                            break

    comp0_nodes = all_nodes - allocated_corridor_nodes
    return corridors, comp0_nodes
