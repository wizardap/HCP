import collections
import time
from typing import Dict, List, Set, Tuple
from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType
from scratch.engine.bipartite_module_detector import detect_variable_modules
from scratch.engine.dual_path_extractor import extract_module_dual_paths
from scratch.engine.comp0_solver import (
    try_merge_2opt,
    try_merge_3opt,
    try_patch_merge,
    sat_merge_cycles,
    solve_comp0_core,
)
from scratch.engine.sat_merger import solve_local_hp_sat


def solve_modular_comp0(
    G: Dict[int, Set[int]],
    comp0_nodes: Set[int],
    virtual_edges: List[Tuple[int, int]],
    time_limit: int = 1800
) -> List[int]:
    """
    Solves Component 0 using Modular State Equivalence encoding and Macro-CEGAR.
    Encodes module selector variables b_i, injects equivalence clauses for T_i and F_i,
    executes the Macro-CEGAR loop, merges remaining macro-cycles using 2-opt/3-opt splicers,
    and uncontracts degree-2 chains into a full Comp 0 Hamiltonian cycle.
    """
    if not comp0_nodes:
        return []
    if len(comp0_nodes) <= 3:
        nodes = list(comp0_nodes)
        return nodes

    t0 = time.time()
    virt_set = {tuple(sorted(e)) for e in virtual_edges}
    ports = set()
    for e in virt_set:
        ports.update(e)

    print(f"[*] Starting Modular State Equivalence Comp 0 Solver on {len(comp0_nodes)} vertices...", flush=True)

    # 1. Detect modules
    modules = detect_variable_modules(G, comp0_nodes, virtual_edges)
    print(f"    Detected {len(modules)} variable modules in Comp 0.", flush=True)

    # 2. Extract dual paths for each module
    module_paths = []
    all_module_nodes = set()
    for m in modules:
        t_m = extract_module_dual_paths(G, m)
        module_paths.append(t_m)
        all_module_nodes.update(m.get('nodes', ()))

    # 3. Contract degree-2 vertices in Comp 0
    adj_c0 = collections.defaultdict(set)
    for u in comp0_nodes:
        for v in G.get(u, ()):
            if v in comp0_nodes:
                adj_c0[u].add(v)
    for u, v in virt_set:
        adj_c0[u].add(v)
        adj_c0[v].add(u)

    rem = set(comp0_nodes)
    edge_chains = {}
    while True:
        d2 = [u for u in sorted(rem) if u not in ports and len(adj_c0[u]) == 2]
        if not d2:
            break
        v = d2[0]
        u, w = list(adj_c0[v])
        adj_c0[u].remove(v)
        adj_c0[w].remove(v)
        del adj_c0[v]
        rem.remove(v)
        e_uv = tuple(sorted([u, v]))
        e_vw = tuple(sorted([v, w]))
        c_uv = edge_chains.pop(e_uv, [u, v])
        c_vw = edge_chains.pop(e_vw, [v, w])
        if c_uv[-1] != v:
            c_uv = list(reversed(c_uv))
        if c_vw[0] != v:
            c_vw = list(reversed(c_vw))
        e_new = tuple(sorted([u, w]))
        edge_chains[e_new] = c_uv + c_vw[1:]
        adj_c0[u].add(w)
        adj_c0[w].add(u)

    contracted_edges = set(edge_chains.keys())
    forbidden_delete = virt_set | contracted_edges

    # 4. Map edges to variables
    edges = set()
    for u in sorted(rem):
        for v in sorted(adj_c0[u]):
            if u < v:
                edges.add((u, v))
    edge_list = sorted(list(edges))
    edge_to_var = {e: i + 1 for i, e in enumerate(edge_list)}
    inc_edges = collections.defaultdict(list)
    for e in edge_list:
        inc_edges[e[0]].append(edge_to_var[e])
        inc_edges[e[1]].append(edge_to_var[e])

    static_clauses = []
    top = len(edge_list) + 1

    # Degree-2 constraint on every node
    for u in sorted(rem):
        clauses = CardEnc.equals(
            lits=inc_edges[u],
            bound=2,
            top_id=top,
            encoding=EncType.cardnetwrk
        )
        for cl in clauses:
            static_clauses.append(cl)
            for lit in cl:
                top = max(top, abs(lit) + 1)

    # Force virtual and contracted edges
    for ve in virt_set:
        static_clauses.append([edge_to_var[ve]])
    for ce in contracted_edges:
        static_clauses.append([edge_to_var[ce]])

    # Static chordless square cuts in contracted graph
    rem_list = sorted(list(rem))
    squares = set()
    for a in rem_list:
        nbrs_a = sorted(list(adj_c0[a]))
        for i in range(len(nbrs_a)):
            u = nbrs_a[i]
            for j in range(i + 1, len(nbrs_a)):
                v = nbrs_a[j]
                common = [w for w in adj_c0[u] if w != a and w in adj_c0[v]]
                for w in common:
                    if w not in adj_c0[a] and v not in adj_c0[u]:
                        e1 = tuple(sorted([a, u]))
                        e2 = tuple(sorted([u, w]))
                        e3 = tuple(sorted([w, v]))
                        e4 = tuple(sorted([v, a]))
                        sq_key = tuple(sorted([e1, e2, e3, e4]))
                        if sq_key not in squares:
                            squares.add(sq_key)
                            static_clauses.append([-edge_to_var[e1], -edge_to_var[e2], -edge_to_var[e3], -edge_to_var[e4]])

    # 5. Inject Module State Selector Variables (b_i)
    selector_vars = {}
    for mid, (m, (t_path, f_path)) in enumerate(zip(modules, module_paths)):
        ves = {tuple(sorted(ve)) for ve in m.get('virtual_edges', ())}
        if not t_path or not f_path or t_path == f_path or not ves.issubset(t_path) or not ves.issubset(f_path):
            continue
        b_var = top
        top += 1
        selector_vars[mid] = b_var

        common = t_path & f_path
        true_only = t_path - f_path
        false_only = f_path - t_path

        for e in common:
            if e in edge_to_var:
                static_clauses.append([edge_to_var[e]])
        for e in true_only:
            if e in edge_to_var:
                var_e = edge_to_var[e]
                static_clauses.append([-b_var, var_e])
                static_clauses.append([b_var, -var_e])
        for e in false_only:
            if e in edge_to_var:
                var_e = edge_to_var[e]
                static_clauses.append([b_var, var_e])
                static_clauses.append([-b_var, -var_e])

    print(f"    Injected {len(selector_vars)} module selector variables. Top var: {top}.", flush=True)

    solver = Cadical195(bootstrap_with=static_clauses)

    # 6. Macro-CEGAR Loop
    it = 0
    winner_cycle = None
    accumulated_cuts = []
    seen_cuts = {tuple(sorted(cl)) for cl in static_clauses}
    active = []

    # Verify initial satisfiability of modular constraints
    sat_init = solver.solve()
    if not sat_init:
        print("    Module selector constraints proved UNSAT -> falling back to global solver...", flush=True)
        solver.delete()
        return solve_comp0_core(G, comp0_nodes, virtual_edges)

    while True:
        it += 1
        t_it = time.time()
        if time.time() - t0 > time_limit:
            solver.delete()
            raise TimeoutError(f"Modular Comp 0 reached {time_limit}s limit at iter {it}!")

        if not solver.solve():
            solver.delete()
            raise RuntimeError("Modular Comp 0 SAT problem proved UNSAT!")

        model = set(solver.get_model())
        active = [e for e in edge_list if edge_to_var[e] in model]
        adj = collections.defaultdict(list)
        for u, v in active:
            adj[u].append(v)
            adj[v].append(u)

        visited = set()
        cycles = []
        for u in sorted(rem):
            if u not in visited:
                cyc = []
                curr, prev = u, None
                while curr not in visited:
                    visited.add(curr)
                    cyc.append(curr)
                    nbrs = adj[curr]
                    nxt = nbrs[0] if nbrs[0] != prev else nbrs[1]
                    prev, curr = curr, nxt
                cycles.append(cyc)

        if len(cycles) == 1:
            print(f"[*] Modular Comp 0 converged at iter {it} in {time.time()-t0:.2f}s!", flush=True)
            winner_cycle = cycles[0]
            break

        # Splicer
        merged = list(cycles)
        merged_any = True
        while merged_any and len(merged) > 1:
            merged_any = False
            merged.sort(key=len, reverse=True)
            for i in range(len(merged)):
                for j in range(i + 1, len(merged)):
                    res = try_merge_2opt(merged[i], merged[j], adj_c0, forbidden_delete)
                    if res is None and len(merged) <= 12:
                        res = try_merge_3opt(merged[i], merged[j], adj_c0, forbidden_delete)
                    if res is not None:
                        merged.pop(j)
                        merged[i] = res
                        merged_any = True
                        break
                if merged_any:
                    break

        if len(merged) == 1:
            print(f"[*] Modular Comp 0 2-opt converged at iter {it} in {time.time()-t0:.2f}s!", flush=True)
            winner_cycle = merged[0]
            break

        # Multi-cycle closing operator for small subcycles (<= 120v)
        if 2 <= len(merged) <= 25:
            cand = list(merged)
            cand.sort(key=len, reverse=True)
            closed_any = True
            while closed_any and len(cand) > 1:
                closed_any = False
                for i in range(len(cand)):
                    for j in range(len(cand)):
                        if i != j and len(cand[j]) <= 120:
                            res = sat_merge_cycles(cand[i], cand[j], adj_c0, forbidden_delete, max_window=50)
                            if res is None and len(cand[j]) <= 60:
                                res = try_patch_merge(cand[i], cand[j], adj_c0, forbidden_delete, max_window=25)
                            if res is None:
                                res = try_merge_3opt(cand[i], cand[j], adj_c0, forbidden_delete)
                            if res is not None:
                                cand[i] = res
                                cand.pop(j)
                                closed_any = True
                                break
                    if closed_any:
                        break
            if len(cand) == 1:
                print(f"[*] Modular Comp 0 closing operator converged at iter {it} in {time.time()-t0:.2f}s!", flush=True)
                winner_cycle = cand[0]
                break

        # 2-cycle boundary HP splicer
        if len(merged) == 2:
            c1, c2 = merged[0], merged[1]
            s1, s2 = set(c1), set(c2)
            pos1 = {u: i for i, u in enumerate(c1)}
            cross = [(u, v) for u in s1 for v in adj_c0[u] if v in s2]
            ports_in_c2 = sorted(list({v for u, v in cross}))
            c2_nbr_in_c1 = {v: [u for u in s1 if (u, v) in cross or (v, u) in cross] for v in ports_in_c2}
            merged_cyc = None
            for p_a in ports_in_c2:
                for p_b in ports_in_c2:
                    if p_a < p_b:
                        cand_splices = []
                        for u_a in c2_nbr_in_c1[p_a]:
                            for u_b in c2_nbr_in_c1[p_b]:
                                idx_a = pos1[u_a]
                                idx_b = pos1[u_b]
                                d_fwd = (idx_b - idx_a) % len(c1)
                                d_bwd = (idx_a - idx_b) % len(c1)
                                if d_fwd == 1 and tuple(sorted([u_a, u_b])) not in forbidden_delete:
                                    cand_splices.append((idx_b, idx_a, False))
                                elif d_bwd == 1 and tuple(sorted([u_a, u_b])) not in forbidden_delete:
                                    cand_splices.append((idx_a, idx_b, True))
                        if not cand_splices:
                            continue
                        hp = solve_local_hp_sat(c2, p_a, p_b, adj_c0, forbidden_delete)
                        if hp:
                            start_idx, end_idx, rev = cand_splices[0]
                            if start_idx > end_idx:
                                p_c1 = c1[start_idx:] + c1[:end_idx + 1]
                            else:
                                p_c1 = c1[start_idx:end_idx + 1]
                            hp_seq = list(reversed(hp)) if rev else hp
                            merged_cyc = p_c1 + hp_seq
                            assert len(merged_cyc) == len(rem), f"Merged cycle length {len(merged_cyc)} != {len(rem)}"
                            assert len(set(merged_cyc)) == len(rem), "Duplicate vertices in merged cycle!"
                            for k in range(len(merged_cyc)):
                                x, y = merged_cyc[k], merged_cyc[(k+1)%len(merged_cyc)]
                                assert y in adj_c0[x], f"Invalid edge in merged cycle: ({x}, {y})"
                            break
                if merged_cyc is not None:
                    break
            if merged_cyc is not None:
                print(f"[*] Modular Comp 0 2-cycle boundary HP splicer converged at iter {it} in {time.time()-t0:.2f}s!", flush=True)
                winner_cycle = merged_cyc
                break

        round_time = time.time() - t_it
        print(f"    Iter {it} ({round_time:.2f}s, total {time.time()-t0:.1f}s): {len(cycles)} raw -> {len(merged)} macro-cycles", flush=True)

        # Cuts:
        # Negative cuts only for small cycles (<= len(rem) // 2) or when exactly 2 cycles remain
        for cyc in cycles:
            if len(cyc) <= len(rem) // 2 or len(cycles) == 2:
                neg_c = [-edge_to_var[tuple(sorted([cyc[i], cyc[(i + 1) % len(cyc)]]))] for i in range(len(cyc))]
                t_neg = tuple(sorted(neg_c))
                if t_neg not in seen_cuts:
                    seen_cuts.add(t_neg)
                    solver.add_clause(neg_c)
                    accumulated_cuts.append(neg_c)

            # Boundary cocycle cut for ALL cycles (including Giant): forces every component to connect!
            c_set = set(cyc)
            cut_e = [tuple(sorted([u, v])) for u in cyc for v in adj_c0[u] if v not in c_set]
            c_clause = [edge_to_var[e] for e in cut_e]
            t_cut = tuple(sorted(c_clause))
            if t_cut not in seen_cuts:
                seen_cuts.add(t_cut)
                solver.add_clause(c_clause)
                accumulated_cuts.append(c_clause)

        if len(merged) > 1:
            for cyc in merged:
                if len(cyc) <= len(rem) // 2:
                    c_set = set(cyc)
                    cut_e = [tuple(sorted([u, v])) for u in cyc for v in adj_c0[u] if v not in c_set]
                    c_clause = [edge_to_var[e] for e in cut_e]
                    t_cut = tuple(sorted(c_clause))
                    if t_cut not in seen_cuts:
                        seen_cuts.add(t_cut)
                        solver.add_clause(c_clause)
                        accumulated_cuts.append(c_clause)

    solver.delete()

    # Expand contracted degree-2 chains
    expanded = []
    n = len(winner_cycle)
    for i in range(n):
        u = winner_cycle[i]
        v = winner_cycle[(i + 1) % n]
        e = tuple(sorted([u, v]))
        if e in edge_chains:
            chain = edge_chains[e]
            if chain[0] != u:
                chain = list(reversed(chain))
            expanded.extend(chain[:-1])
        else:
            expanded.append(u)

    assert len(expanded) == len(comp0_nodes), f"Expected {len(comp0_nodes)}, got {len(expanded)}"
    return expanded
