import collections, json, multiprocessing, os, sys, time
from typing import Dict, List, Set, Tuple

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "../..")))
from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType
from scratch.graph717.decomposer import load_and_decompose_graph717

def solve_module_path(G: Dict[int, Set[int]], mod: Set[int], u_port: int, v_port: int) -> List[int]:
    """
    Solve Hamiltonian path in G[mod + {u, v}] between u_port and v_port.
    Returns path of 169 vertices starting at u_port and ending at v_port.
    """
    t0 = time.time()
    V_mod = mod | {u_port, v_port}
    virt = tuple(sorted([u_port, v_port]))
    edges = set()
    for u in V_mod:
        for v in G[u]:
            if v in V_mod and u < v:
                edges.add((u, v))
    edges.add(virt)
    edge_list = sorted(list(edges))
    edge_to_var = {e: i + 1 for i, e in enumerate(edge_list)}
    inc_edges = collections.defaultdict(list)
    for e in edge_list:
        inc_edges[e[0]].append(edge_to_var[e])
        inc_edges[e[1]].append(edge_to_var[e])

    solver = Cadical195()
    top = len(edge_list) + 1

    for u in V_mod:
        lits = inc_edges[u]
        clauses = CardEnc.equals(lits=lits, bound=2, top_id=top, encoding=EncType.cardnetwrk)
        for cl in clauses:
            solver.add_clause(cl)
            for lit in cl:
                top = max(top, abs(lit) + 1)

    solver.add_clause([edge_to_var[virt]])

    it = 0
    while True:
        it += 1
        if not solver.solve():
            raise RuntimeError(f"Module ({u_port}, {v_port}) UNSAT!")
        model = set(solver.get_model())
        active = [e for e in edge_list if edge_to_var[e] in model]
        adj = collections.defaultdict(list)
        for u, v in active:
            adj[u].append(v); adj[v].append(u)

        visited = set()
        cycles = []
        for u in V_mod:
            if u not in visited:
                cyc = []
                curr, prev = u, None
                while curr not in visited:
                    visited.add(curr); cyc.append(curr)
                    nbrs = adj[curr]
                    nxt = nbrs[0] if nbrs[0] != prev else nbrs[1]
                    prev, curr = curr, nxt
                cycles.append(cyc)

        if len(cycles) == 1:
            cyc = cycles[0]
            n = len(cyc)
            idx_u = cyc.index(u_port)
            if cyc[(idx_u + 1) % n] == v_port:
                cyc = list(reversed(cyc))
                idx_u = cyc.index(u_port)
            assert cyc[(idx_u - 1) % n] == v_port
            path = cyc[idx_u:] + cyc[:idx_u]
            assert path[0] == u_port and path[-1] == v_port and len(path) == 169
            solver.delete()
            return path

        for cyc in cycles:
            neg_clause = [-edge_to_var[tuple(sorted([cyc[i], cyc[(i+1)%len(cyc)]]))] for i in range(len(cyc))]
            solver.add_clause(neg_clause)

def _worker_mod(args):
    col_path, mod_ports, u_start, v_end = args
    G, modules, _, _, _ = load_and_decompose_graph717(col_path)
    mod = modules[mod_ports]
    p = solve_module_path(G, mod, u_start, v_end)
    return mod_ports, p

def solve_and_assemble_chains(col_path: str, cache_path: str) -> Dict[str, List[int]]:
    if os.path.exists(cache_path):
        with open(cache_path, "r") as f:
            data = json.load(f)
        if "chain1" in data and "chain2" in data and len(data["chain1"]) == 509 and len(data["chain2"]) == 509:
            print("Both chains already solved and cached!")
            return data

    G, modules, _, _, _ = load_and_decompose_graph717(col_path)

    tasks = [
        (col_path, tuple(sorted([1389, 2677])), 1389, 2677),
        (col_path, tuple(sorted([1213, 2681])), 1213, 2681),
        (col_path, tuple(sorted([702, 773])), 773, 702),
        (col_path, tuple(sorted([1016, 3986])), 3986, 1016),
        (col_path, tuple(sorted([1177, 2467])), 1177, 2467),
        (col_path, tuple(sorted([540, 577])), 577, 540),
    ]

    print("Solving all 6 modules in parallel using multiprocessing...")
    solved_mods = {}
    with multiprocessing.Pool(processes=min(6, os.cpu_count() or 4)) as pool:
        for mod_ports, path in pool.map(_worker_mod, tasks):
            solved_mods[mod_ports] = path
            print(f"Module {mod_ports} solved: {len(path)} vertices.")

    # Chain 1: 255 -> [1389..2677] -> [1213..2681] -> [773..702] -> 1955
    p_m2 = solved_mods[tuple(sorted([1389, 2677]))]
    if p_m2[0] != 1389: p_m2 = list(reversed(p_m2))
    p_m4 = solved_mods[tuple(sorted([1213, 2681]))]
    if p_m4[0] != 1213: p_m4 = list(reversed(p_m4))
    p_m6 = solved_mods[tuple(sorted([702, 773]))]
    if p_m6[0] != 773: p_m6 = list(reversed(p_m6))

    chain1 = [255] + p_m2 + p_m4 + p_m6 + [1955]
    assert len(chain1) == 509

    # Chain 2: 3358 -> [3986..1016] -> [1177..2467] -> [577..540] -> 2609
    p_m1 = solved_mods[tuple(sorted([1016, 3986]))]
    if p_m1[0] != 3986: p_m1 = list(reversed(p_m1))
    p_m5 = solved_mods[tuple(sorted([1177, 2467]))]
    if p_m5[0] != 1177: p_m5 = list(reversed(p_m5))
    p_m3 = solved_mods[tuple(sorted([540, 577]))]
    if p_m3[0] != 577: p_m3 = list(reversed(p_m3))

    chain2 = [3358] + p_m1 + p_m5 + p_m3 + [2609]
    assert len(chain2) == 509

    res = {"chain1": chain1, "chain2": chain2}
    os.makedirs(os.path.dirname(os.path.abspath(cache_path)), exist_ok=True)
    with open(cache_path, "w") as f:
        json.dump(res, f)
    print(f"Saved both chains to {cache_path}.")
    return res

if __name__ == "__main__":
    base_dir = os.path.dirname(os.path.abspath(__file__))
    repo_root = os.path.abspath(os.path.join(base_dir, "../.."))
    col_path = os.path.join(repo_root, "FHCPCS-col/graph717.col")
    cache_path = os.path.join(base_dir, "chains.json")
    solve_and_assemble_chains(col_path, cache_path)
