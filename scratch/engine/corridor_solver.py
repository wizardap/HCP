import collections, itertools, time
from typing import Dict, List, Set, Tuple
from pysat.solvers import Cadical153

def solve_corridor_path(G: Dict[int, Set[int]], nodes: Set[int], src: int, dst: int) -> List[int]:
    """
    Solves Hamiltonian path on the subgraph induced by `nodes` starting at `src` and ending at `dst`.
    Uses exact degree-2 / degree-1 SAT constraints with subcycle elimination.
    """
    nodes_list = sorted(list(nodes))
    sub_adj = {u: sorted([v for v in G[u] if v in nodes]) for u in nodes_list}
    edges = []
    for u in nodes_list:
        for v in sub_adj[u]:
            if u < v:
                edges.append((u, v))

    e2v = {e: idx + 1 for idx, e in enumerate(edges)}
    solver = Cadical153()

    for u in nodes_list:
        inc = [e2v[tuple(sorted((u, v)))] for v in sub_adj[u]]
        target_deg = 1 if u in (src, dst) else 2
        if target_deg == 1:
            solver.add_clause(inc)
            for a, b in itertools.combinations(inc, 2):
                solver.add_clause([-a, -b])
        else:
            solver.add_clause(inc)
            for c in itertools.combinations(inc, len(inc) - 1):
                solver.add_clause(list(c))
            for c in itertools.combinations(inc, 3):
                solver.add_clause([-c[0], -c[1], -c[2]])

    while solver.solve():
        model = set(solver.get_model())
        active_edges = [e for e in edges if e2v[e] in model]
        adj_m = collections.defaultdict(list)
        for u, v in active_edges:
            adj_m[u].append(v)
            adj_m[v].append(u)

        visited = set()
        comps = []
        for u in nodes_list:
            if u not in visited:
                c = []
                q = collections.deque([u])
                visited.add(u)
                while q:
                    curr = q.popleft()
                    c.append(curr)
                    for to in adj_m[curr]:
                        if to not in visited:
                            visited.add(to)
                            q.append(to)
                comps.append(c)

        if len(comps) == 1:
            path = [src]
            curr = src
            prev = None
            while len(path) < len(nodes):
                nxts = [to for to in adj_m[curr] if to != prev]
                assert nxts, "Broken path in reconstruction"
                nxt = nxts[0]
                path.append(nxt)
                prev, curr = curr, nxt
            assert path[-1] == dst
            solver.delete()
            return path

        for c in comps:
            if src not in c and dst not in c:
                c_set = set(c)
                cut = [e2v[e] for e in edges if (e[0] in c_set) != (e[1] in c_set)]
                solver.add_clause(cut)

    solver.delete()
    raise RuntimeError(f"Corridor Hamiltonian path UNSAT between {src} and {dst} on {len(nodes)} vertices!")
