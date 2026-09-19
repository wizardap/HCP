import os, sys, collections, itertools, time
from typing import Dict, List, Optional, Set, Tuple

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType
from scratch.engine.sat_merger import sat_merge_cycles, solve_local_hp_sat

def try_merge_2opt(cyc1: List[int], cyc2: List[int], adj: Dict[int, Set[int]], forbidden_edges: Set[Tuple[int, int]]):
    n1 = len(cyc1)
    n2 = len(cyc2)
    pos2 = {u: i for i, u in enumerate(cyc2)}
    for i in range(n1):
        u1 = cyc1[i]
        v1 = cyc1[(i + 1) % n1]
        e1 = tuple(sorted([u1, v1]))
        if e1 in forbidden_edges:
            continue
        nbrs_u1 = [u2 for u2 in adj[u1] if u2 in pos2]
        for u2 in nbrs_u1:
            idx2 = pos2[u2]
            for v2, rev in [(cyc2[(idx2 + 1) % n2], False), (cyc2[(idx2 - 1) % n2], True)]:
                e2 = tuple(sorted([u2, v2]))
                if e2 in forbidden_edges:
                    continue
                if v2 in adj[v1]:
                    p1 = cyc1[i+1:] + cyc1[:i+1]
                    p2 = [cyc2[(idx2 - k) % n2] for k in range(n2)] if not rev else [cyc2[(idx2 + k) % n2] for k in range(n2)]
                    return p1 + p2
    return None

def try_merge_3opt(cyc1: List[int], cyc2: List[int], adj: Dict[int, Set[int]], forbidden_edges: Set[Tuple[int, int]]):
    """
    3-opt cycle merge:
    Splits C1 at (B, C) and (D, A), with chord (C, A) in C1,
    and splits C2 at (x, y) with cross-edges (B, x) and (y, D).
    Inserts C2 into C1 and reverses one of C1's segments using chord (C, A).
    """
    n1 = len(cyc1)
    n2 = len(cyc2)
    pos1 = {u: i for i, u in enumerate(cyc1)}

    for idx2, x in enumerate(cyc2):
        for y, q_path in [(cyc2[(idx2 + 1) % n2], [cyc2[(idx2 - k) % n2] for k in range(n2)]),
                          (cyc2[(idx2 - 1) % n2], [cyc2[(idx2 + k) % n2] for k in range(n2)])]:
            e_xy = tuple(sorted([x, y]))
            if e_xy in forbidden_edges:
                continue
            nbrs_x = [b for b in adj[x] if b in pos1]
            nbrs_y = [d for d in adj[y] if d in pos1]
            if not nbrs_x or not nbrs_y:
                continue
            for B in nbrs_x:
                idx_B = pos1[B]
                C = cyc1[(idx_B + 1) % n1]
                e_BC = tuple(sorted([B, C]))
                if e_BC in forbidden_edges:
                    continue
                for D in nbrs_y:
                    if D == B or D == C:
                        continue
                    idx_D = pos1[D]
                    A = cyc1[(idx_D + 1) % n1]
                    if A == B or A == C or A == D:
                        continue
                    e_DA = tuple(sorted([D, A]))
                    if e_DA in forbidden_edges:
                        continue
                    if A in adj[C]:
                        p1 = []
                        curr = (idx_D + 1) % n1
                        while True:
                            p1.append(cyc1[curr])
                            if cyc1[curr] == B:
                                break
                            curr = (curr + 1) % n1
                        p2 = []
                        curr = (idx_B + 1) % n1
                        while True:
                            p2.append(cyc1[curr])
                            if cyc1[curr] == D:
                                break
                            curr = (curr + 1) % n1
                        if len(p1) + len(p2) == n1:
                            new_cyc = p1 + q_path + list(reversed(p2))
                            return new_cyc
def try_patch_merge(cyc1: List[int], cyc2: List[int], adj: Dict[int, Set[int]], forbidden_edges: Set[Tuple[int, int]], max_window: int = 10) -> Optional[List[int]]:
    """
    Attempt to merge a small cycle (cyc2) into a larger cycle (cyc1) by finding a Hamiltonian
    path through cyc2 and a small contiguous window of cyc1.
    Soundness guaranteed: checks that no forbidden edges in cyc1 or cyc2 are violated.
    """
    if len(cyc2) > 40:
        return None
    n1 = len(cyc1)
    n2 = len(cyc2)
    s1 = set(cyc1)
    s2 = set(cyc2)
    pos1 = {u: i for i, u in enumerate(cyc1)}

    ce_in_cyc2 = [e for e in forbidden_edges if e[0] in s2 and e[1] in s2]
    touching_cyc1 = {v for u in cyc2 for v in adj[u] if v in pos1}
    if not touching_cyc1:
        return None

    touch_indices = sorted([pos1[v] for v in touching_cyc1])

    for t_idx in touch_indices:
        for w_len in range(1, max_window + 1):
            for offset in range(-w_len + 1, 1):
                idx_start = (t_idx + offset) % n1
                idx_prev = (idx_start - 1) % n1
                idx_end = (idx_start + w_len) % n1

                u_start = cyc1[idx_prev]
                u_end = cyc1[idx_end]

                # Check removed edges from cyc1
                removed_edges = [
                    tuple(sorted([cyc1[(idx_prev + k) % n1], cyc1[(idx_prev + k + 1) % n1]]))
                    for k in range(w_len + 1)
                ]
                if any(e in forbidden_edges for e in removed_edges):
                    continue

                win_nodes = [cyc1[(idx_start + k) % n1] for k in range(w_len)]
                sub_nodes = win_nodes + cyc2
                sub_set = set(sub_nodes)
                all_set = sub_set | {u_start, u_end}

                if len(adj[u_start] & sub_set) < 1 or len(adj[u_end] & sub_set) < 1:
                    continue
                if any(len(adj[u] & all_set) < 2 for u in sub_nodes):
                    continue

                visited = set()
                path = [u_start]
                total_target = len(sub_nodes) + 1

                step_limit = [2000]
                def dfs(curr):
                    step_limit[0] -= 1
                    if step_limit[0] <= 0:
                        return False
                    if len(path) == total_target:
                        if u_end in adj[curr]:
                            path.append(u_end)
                            return True
                        return False
                    nbrs = [nxt for nxt in adj[curr] if nxt in sub_set and nxt not in visited]
                    nbrs.sort(key=lambda x: len(adj[x] & (sub_set - visited)))
                    for nxt in nbrs:
                        visited.add(nxt)
                        path.append(nxt)
                        if dfs(nxt):
                            return True
                        path.pop()
                        visited.remove(nxt)
                    return False

                if dfs(u_start):
                    if ce_in_cyc2:
                        p_edges = {tuple(sorted([path[i], path[i+1]])) for i in range(len(path)-1)}
                        if not all(ce in p_edges for ce in ce_in_cyc2):
                            continue
                    if idx_end <= idx_prev:
                        cyc_rest = cyc1[idx_end : idx_prev + 1]
                    else:
                        cyc_rest = cyc1[idx_end:] + cyc1[: idx_prev + 1]
                    merged = cyc_rest + path[1:-1]
                    if len(merged) != n1 + n2:
                        continue
                    return merged
    return None

def solve_comp0_core(G: Dict[int, Set[int]], comp0_nodes: Set[int], virtual_edges: List[Tuple[int, int]]) -> List[int]:
    t0 = time.time()
    virt_set = {tuple(sorted(e)) for e in virtual_edges}
    ports = set()
    for e in virt_set:
        ports.update(e)

    adj_c0 = collections.defaultdict(set)
    for u in comp0_nodes:
        for v in G[u]:
            if v in comp0_nodes:
                adj_c0[u].add(v)
    for u, v in virt_set:
        adj_c0[u].add(v)
        adj_c0[v].add(u)

    # Recursive degree-2 contraction (ports untouched)
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

    print(f"[*] Comp 0 Contracted: {len(comp0_nodes)} -> {len(rem)} vertices ({len(comp0_nodes) - len(rem)} contracted chains).", flush=True)

    contracted_edges = set(edge_chains.keys())

    # Map edges to SAT variables
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
    # Degree-2 constraint for each vertex (pure direct combinatoric, 0 auxiliary variables)
    for u in sorted(rem):
        lits = inc_edges[u]
        d = len(lits)
        for comb in itertools.combinations(lits, 3):
            static_clauses.append([-x for x in comb])
        for comb in itertools.combinations(lits, d - 1):
            static_clauses.append(list(comb))

    # Force virtual edges and contracted macro-edges
    for ve in virt_set:
        static_clauses.append([edge_to_var[ve]])
    for ce in contracted_edges:
        static_clauses.append([edge_to_var[ce]])

    rem_list = sorted(list(rem))
    # Static chordless triangle cuts in contracted graph
    for u in rem_list:
        for v in adj_c0[u]:
            if v > u:
                for w in adj_c0[v]:
                    if w > v and w in adj_c0[u]:
                        static_clauses.append([-edge_to_var[tuple(sorted([u, v]))], -edge_to_var[tuple(sorted([v, w]))], -edge_to_var[tuple(sorted([w, u]))]])

    # Static chordless square cuts in contracted graph
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

    solver = Cadical195(bootstrap_with=static_clauses)
    seen_cuts = {tuple(sorted(cl)) for cl in static_clauses}
    it = 0
    winner_cycle = None
    forbidden_delete = contracted_edges | set(virt_set)
    accumulated_cuts = []
    active_edges = []

    while True:
        it += 1
        t_it = time.time()
        if not solver.solve():
            solver.delete()
            raise RuntimeError(f"Comp 0 UNSAT at iteration {it}!")

        model = set(solver.get_model())
        active_edges = [e for e in edge_list if edge_to_var[e] in model]
        adj = collections.defaultdict(list)
        for u, v in active_edges:
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
            print(f"[*] Comp 0 SAT converged at iter {it} in {time.time()-t0:.2f}s!", flush=True)
            winner_cycle = cycles[0]
            break

        # 2-opt and 3-opt cycle absorption
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
            print(f"[*] Comp 0 2-opt/3-opt converged at iter {it} in {time.time()-t0:.2f}s!", flush=True)
            winner_cycle = merged[0]
            break

        # Non-intrusive multi-cycle closing operator for small subcycles (<= 120v):
        if 2 <= len(merged) <= 60:
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
            if len(cand) < len(merged):
                merged = list(cand)
            if len(merged) == 1:
                print(f"[*] Comp 0 closing operator (SAT-HP/patch/3-opt) converged at iter {it} in {time.time()-t0:.2f}s!", flush=True)
                winner_cycle = merged[0]
                break

        # 2-cycle boundary HP splicer (merges 2 remaining macro-cycles across direct boundary edges)
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
                                p_c1 = c1[start_idx:] + c1[:end_idx+1]
                            else:
                                p_c1 = c1[start_idx:end_idx+1]
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
                print(f"[*] Comp 0 2-cycle boundary HP splicer converged at iter {it} in {time.time()-t0:.2f}s!", flush=True)
                winner_cycle = merged_cyc
                break

        round_time = time.time() - t_it
        abs_lens = sorted([len(c) for c in merged], reverse=True)
        print(f"    Iter {it} ({round_time:.2f}s, total {time.time()-t0:.1f}s): {len(cycles)} raw -> {len(merged)} absorbed (max {abs_lens[0]}, min {abs_lens[-1]})", flush=True)

        if time.time() - t0 > 1800:
            solver.delete()
            raise TimeoutError(f"Comp 0 reached 1800s timeout at iteration {it}!")

        # Cuts:
        # Cocycle cuts and negative cuts for raw cycles
        for cyc in cycles:
            if len(cyc) <= len(rem) // 2:
                c_set = set(cyc)
                cut_e = [tuple(sorted([u, v])) for u in cyc for v in adj_c0[u] if v not in c_set]
                c_clause = [edge_to_var[e] for e in cut_e]
                t_cut = tuple(sorted(c_clause))
                if t_cut not in seen_cuts:
                    seen_cuts.add(t_cut)
                    solver.add_clause(c_clause)
                    accumulated_cuts.append(c_clause)

            neg_c = [-edge_to_var[tuple(sorted([cyc[i], cyc[(i + 1) % len(cyc)]]))] for i in range(len(cyc))]
            t_neg = tuple(sorted(neg_c))
            if t_neg not in seen_cuts:
                seen_cuts.add(t_neg)
                solver.add_clause(neg_c)
                accumulated_cuts.append(neg_c)

        # Cocycle cuts for absorbed macro-cycles
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

    # Expand contracted Comp 0 cycle
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
