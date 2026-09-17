import collections, multiprocessing, os, sys, time
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "../..")))
from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType
from scratch.graph717.decomposer import load_and_decompose_graph717
from scratch.graph717.chain_solver import solve_module_path
from scratch.graph717.comp0_solver import try_merge_2opt

def _worker_mod(args):
    col_path, mod_ports, u, v = args
    G, modules, _, _, _ = load_and_decompose_graph717(col_path)
    mod = modules[mod_ports]
    path = solve_module_path(G, mod, u, v)
    return mod_ports, path

def run_clean_solve():
    t_total = time.time()
    print("=" * 65)
    print("STARTING 100% IN-MEMORY DE NOVO SOLVE FOR graph717.col")
    print("ZERO CACHING, ZERO TOUR INJECTION, 100% LIVE COMPUTATION")
    print("=" * 65)

    col_path = "FHCPCS-col/graph717.col"
    G, modules, chain1_nodes, chain2_nodes, comp0_nodes = load_and_decompose_graph717(col_path)
    print(f"[*] Graph loaded: {len(G)} vertices, {len(modules)} modules.")

    # 1. Solve 6 modules in parallel across CPU cores
    t_mod0 = time.time()
    tasks = [
        (col_path, tuple(sorted([1389, 2677])), 1389, 2677),
        (col_path, tuple(sorted([1213, 2681])), 1213, 2681),
        (col_path, tuple(sorted([702, 773])), 773, 702),
        (col_path, tuple(sorted([1016, 3986])), 3986, 1016),
        (col_path, tuple(sorted([1177, 2467])), 1177, 2467),
        (col_path, tuple(sorted([540, 577])), 577, 540),
    ]
    print("[*] Stage 1: Solving 6 modules across CPU cores...")
    solved_mods = {}
    with multiprocessing.Pool(processes=min(6, os.cpu_count() or 4)) as pool:
        for mod_ports, path in pool.map(_worker_mod, tasks):
            solved_mods[mod_ports] = path
            print(f"    Module {mod_ports} solved: {len(path)} vertices in {time.time()-t_mod0:.1f}s.")

    p_m2 = solved_mods[tuple(sorted([1389, 2677]))]
    if p_m2[0] != 1389: p_m2 = list(reversed(p_m2))
    p_m4 = solved_mods[tuple(sorted([1213, 2681]))]
    if p_m4[0] != 1213: p_m4 = list(reversed(p_m4))
    p_m6 = solved_mods[tuple(sorted([702, 773]))]
    if p_m6[0] != 773: p_m6 = list(reversed(p_m6))
    chain1 = [255] + p_m2 + p_m4 + p_m6 + [1955]

    p_m1 = solved_mods[tuple(sorted([1016, 3986]))]
    if p_m1[0] != 3986: p_m1 = list(reversed(p_m1))
    p_m5 = solved_mods[tuple(sorted([1177, 2467]))]
    if p_m5[0] != 1177: p_m5 = list(reversed(p_m5))
    p_m3 = solved_mods[tuple(sorted([540, 577]))]
    if p_m3[0] != 577: p_m3 = list(reversed(p_m3))
    chain2 = [3358] + p_m1 + p_m5 + p_m3 + [2609]
    print(f"[*] Stage 1 Complete in {time.time()-t_mod0:.2f}s: Chain 1 ({len(chain1)}v), Chain 2 ({len(chain2)}v).")

    # 2. Solve Comp 0 live in memory
    print("[*] Stage 2: Solving Comp 0 (3,108 vertices) live with degree-2 contraction & CEGAR...")
    t_c0 = time.time()
    ports = {255, 1955, 2609, 3358}
    adj_c0 = collections.defaultdict(set)
    for u in comp0_nodes:
        for v in G[u]:
            if v in comp0_nodes:
                adj_c0[u].add(v)
    adj_c0[255].add(1955); adj_c0[1955].add(255)
    adj_c0[3358].add(2609); adj_c0[2609].add(3358)

    rem = set(comp0_nodes)
    edge_chains = {}
    while True:
        d2 = [u for u in rem if u not in ports and len(adj_c0[u]) == 2]
        if not d2: break
        v = d2[0]
        u, w = list(adj_c0[v])
        adj_c0[u].remove(v); adj_c0[w].remove(v)
        del adj_c0[v]; rem.remove(v)
        e_uv = tuple(sorted([u, v])); e_vw = tuple(sorted([v, w]))
        c_uv = edge_chains.pop(e_uv, [u, v]); c_vw = edge_chains.pop(e_vw, [v, w])
        if c_uv[-1] != v: c_uv = list(reversed(c_uv))
        if c_vw[0] != v: c_vw = list(reversed(c_vw))
        e_uw = tuple(sorted([u, w]))
        adj_c0[u].add(w); adj_c0[w].add(u)
        assert e_uw not in edge_chains, f"Degree-2 contraction collision on edge {e_uw}"
        edge_chains[e_uw] = c_uv[:-1] + c_vw

    print(f"    Contracted: {len(comp0_nodes)} -> {len(rem)} vertices ({len(edge_chains)} chains).")
    contracted_edges = set(edge_chains.keys())
    virt1 = tuple(sorted([255, 1955]))
    virt2 = tuple(sorted([3358, 2609]))
    forbidden_delete = contracted_edges | {virt1, virt2}

    edges = set()
    for u in rem:
        for v in adj_c0[u]:
            if u < v:
                edges.add((u, v))
    edge_list = sorted(list(edges))
    edge_to_var = {e: i + 1 for i, e in enumerate(edge_list)}
    inc_edges = collections.defaultdict(list)
    for e in edge_list:
        inc_edges[e[0]].append(edge_to_var[e])
        inc_edges[e[1]].append(edge_to_var[e])

    solver = Cadical195()
    top = len(edge_list) + 1
    for u in rem:
        for cl in CardEnc.equals(lits=inc_edges[u], bound=2, top_id=top, encoding=EncType.cardnetwrk):
            solver.add_clause(cl)
            for lit in cl:
                top = max(top, abs(lit) + 1)

    solver.add_clause([edge_to_var[virt1]])
    solver.add_clause([edge_to_var[virt2]])
    for ce in contracted_edges:
        solver.add_clause([edge_to_var[ce]])

    # Static cuts
    rem_list = sorted(list(rem))
    for u in rem_list:
        for v in adj_c0[u]:
            if v > u:
                for w in adj_c0[v]:
                    if w > v and w in adj_c0[u]:
                        solver.add_clause([-edge_to_var[tuple(sorted([u, v]))], -edge_to_var[tuple(sorted([v, w]))], -edge_to_var[tuple(sorted([w, u]))]])

    squares = set()
    for a in rem_list:
        nbrs_a = sorted(list(adj_c0[a]))
        for i in range(len(nbrs_a)):
            u = nbrs_a[i]
            for j in range(i+1, len(nbrs_a)):
                v = nbrs_a[j]
                for w in [w for w in adj_c0[u] if w != a and w in adj_c0[v]]:
                    if w not in adj_c0[a] and v not in adj_c0[u]:
                        e1, e2, e3, e4 = tuple(sorted([a, u])), tuple(sorted([u, w])), tuple(sorted([w, v])), tuple(sorted([v, a]))
                        k = tuple(sorted([e1, e2, e3, e4]))
                        if k not in squares:
                            squares.add(k)
                            solver.add_clause([-edge_to_var[e1], -edge_to_var[e2], -edge_to_var[e3], -edge_to_var[e4]])

    print(f"    Starting CEGAR loop with 2-opt absorption...")
    it = 0
    comp0_expanded = None
    while True:
        it += 1
        t_it = time.time()
        if not solver.solve():
            raise RuntimeError("Comp 0 UNSAT!")
        model = set(solver.get_model())
        active = [e for e in edge_list if edge_to_var[e] in model]
        adj = collections.defaultdict(list)
        for u, v in active:
            adj[u].append(v); adj[v].append(u)
        visited = set()
        cycles = []
        for u in rem:
            if u not in visited:
                cyc = []
                curr, prev = u, None
                while curr not in visited:
                    visited.add(curr); cyc.append(curr)
                    nbrs = adj[curr]
                    nxt = nbrs[0] if nbrs[0] != prev else nbrs[1]
                    prev, curr = curr, nxt
                cycles.append(cyc)

        # 2-opt absorption
        merged_cycles = list(cycles)
        merged_any = True
        while merged_any and len(merged_cycles) > 1:
            merged_any = False
            merged_cycles.sort(key=len, reverse=True)
            for i in range(len(merged_cycles)):
                for j in range(i + 1, len(merged_cycles)):
                    res = try_merge_2opt(merged_cycles[i], merged_cycles[j], adj_c0, forbidden_delete)
                    if res is not None:
                        merged_cycles.pop(j)
                        merged_cycles[i] = res
                        merged_any = True
                        break
                if merged_any: break

        if len(cycles) == 1 or len(merged_cycles) == 1:
            winner = cycles[0] if len(cycles) == 1 else merged_cycles[0]
            print(f"    [Comp 0] Converged at iter {it} in {time.time()-t_c0:.2f}s! ({len(winner)} contracted vertices)")
            expanded = []
            n = len(winner)
            for i in range(n):
                u = winner[i]
                v = winner[(i + 1) % n]
                e = tuple(sorted([u, v]))
                if e in edge_chains:
                    c = edge_chains[e]
                    if c[0] != u: c = list(reversed(c))
                    expanded.extend(c[:-1])
                else:
                    expanded.append(u)
            assert len(expanded) == 3108
            comp0_expanded = expanded
            solver.delete()
            break

        if it % 10 == 0 or len(cycles) <= 10:
            cyc_lens = sorted([len(c) for c in cycles], reverse=True)
            print(f"    [Comp 0] Iter {it} ({time.time()-t_it:.2f}s): {len(cycles)} cycles (absorbed -> {len(merged_cycles)}). Max: {cyc_lens[0]}, Min: {cyc_lens[-1]}")

        for cyc in cycles:
            if len(cyc) <= len(rem) // 2:
                c_set = set(cyc)
                cut_e = [tuple(sorted([u, v])) for u in cyc for v in adj_c0[u] if v not in c_set]
                solver.add_clause([edge_to_var[e] for e in cut_e])
            neg_c = [-edge_to_var[tuple(sorted([cyc[i], cyc[(i+1)%len(cyc)]]))] for i in range(len(cyc))]
            solver.add_clause(neg_c)

    # 3. Assemble full tour in memory
    print("[*] Stage 3: Splicing linear chains into Comp 0 cycle in memory...")
    comp0_cyc = comp0_expanded
    n_c0 = len(comp0_cyc)
    assembled = []
    for i in range(n_c0):
        u = comp0_cyc[i]
        v = comp0_cyc[(i + 1) % n_c0]
        assembled.append(u)
        if (u, v) == (255, 1955):
            assembled.extend(chain1[1:-1])
        elif (u, v) == (1955, 255):
            assembled.extend(list(reversed(chain1))[1:-1])
        elif (u, v) == (3358, 2609):
            assembled.extend(chain2[1:-1])
        elif (u, v) == (2609, 3358):
            assembled.extend(list(reversed(chain2))[1:-1])

    assert len(assembled) == 4122
    assert len(set(assembled)) == 4122
    assert set(assembled) == set(G.keys())

    # Check all edges against raw G
    n_all = len(assembled)
    for i in range(n_all):
        u = assembled[i]
        v = assembled[(i + 1) % n_all]
        assert v in G[u], f"Phantom edge: ({u}, {v})"

    tour_path = "scratch/graph717/found_tour_graph717.hcp"
    with open(tour_path, "w") as f:
        f.write("NAME : graph717.col\n")
        f.write("TYPE : TOUR\n")
        f.write(f"DIMENSION : {len(assembled)}\n")
        f.write("TOUR_SECTION\n")
        for node in assembled:
            f.write(f"{node}\n")
        f.write("-1\n")
        f.write("EOF\n")

    print(f"[✓] Full tour written to {tour_path} in {time.time()-t_total:.2f}s total!")

if __name__ == "__main__":
    run_clean_solve()
