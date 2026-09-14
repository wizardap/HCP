import collections, itertools, json, os, sys, time
from typing import Dict, List, Set, Tuple
from pysat.solvers import Cadical153

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

def solve_subgraph_path(G: Dict[int, Set[int]], nodes: Set[int], src: int, dst: int) -> List[int]:
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
    raise RuntimeError(f"No Hamiltonian path found in subgraph from {src} to {dst}")

def solve_chains(G: Dict[int, Set[int]], mod_nodes: Set[int], block2_nodes: Set[int]) -> Tuple[List[int], List[int]]:
    cache_path = os.path.join(os.path.dirname(__file__), "chain_path.json")
    if os.path.exists(cache_path):
        with open(cache_path, "r") as f:
            data = json.load(f)
            if isinstance(data, dict) and "chain1" in data and "chain2" in data:
                c1, c2 = data["chain1"], data["chain2"]
                if len(c1) == 170 and len(c2) == 15:
                    return c1, c2

    t0 = time.time()
    # Chain 1: Module 169 HP from 5200 to 3195 + hub 4509
    p_mod = solve_subgraph_path(G, mod_nodes, src=5200, dst=3195)
    assert len(p_mod) == 169
    assert 4509 in G[3195]
    chain1 = p_mod + [4509]
    assert len(chain1) == 170
    assert chain1[0] == 5200 and chain1[-1] == 4509

    # Chain 2: Comp 2 (15 vertices) HP from 4779 to 893
    comp2_nodes = block2_nodes - {4509}
    assert len(comp2_nodes) == 15
    chain2 = solve_subgraph_path(G, comp2_nodes, src=4779, dst=893)
    assert len(chain2) == 15
    assert chain2[0] == 4779 and chain2[-1] == 893

    assert set(chain1).isdisjoint(set(chain2))
    assert len(set(chain1) | set(chain2)) == 185
    print(f"[*] Solved Chain 1 (170v) and Chain 2 (15v) in {time.time() - t0:.2f}s!")

    with open(cache_path, "w") as f:
        json.dump({"chain1": chain1, "chain2": chain2}, f)

    return chain1, chain2

if __name__ == "__main__":
    from scratch.graph882.decomposer import load_and_decompose_graph882
    col_path = os.path.join(repo_root, "FHCPCS-col/graph882.col")
    G, mod_nodes, block2_nodes, _, _ = load_and_decompose_graph882(col_path)
    solve_chains(G, mod_nodes, block2_nodes)
