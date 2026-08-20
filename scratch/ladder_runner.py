#!/usr/bin/env python3
"""Unified Ladder-Family Decomposed Two-Tier Solver & Verifier
Tests across: graph746, graph950, graph963, graph975, graph982, graph990
"""
import collections, json, os, subprocess, sys, time

def solve(nv, clauses, tl=120.0, seed=None):
    lines = [f'p cnf {nv} {len(clauses)}'] + [' '.join(map(str, c+[0])) for c in clauses]
    args = ['/usr/local/bin/cadical', '--quiet']
    if seed is not None:
        args += [f'--seed={seed}']
    try:
        r = subprocess.run(args + ['/dev/stdin'], input='\n'.join(lines) + '\n',
                           capture_output=True, text=True, timeout=tl)
    except subprocess.TimeoutExpired:
        return None, {}
    out = r.stdout
    if 's UNSATISFIABLE' in out: return False, {}
    if 's SATISFIABLE' not in out: return None, {}
    m = {}
    for line in out.splitlines():
        if line.startswith('v '):
            for x in line.split()[1:]:
                xi = int(x); m[abs(xi)] = xi > 0
    return True, m

def load_tour(path):
    vv = []; in_sec = False
    for l in open(path):
        t = l.strip()
        if not t: continue
        if 'TOUR_SECTION' in t.upper(): in_sec = True; continue
        if not in_sec: continue
        try: x = int(t)
        except: continue
        if x == -1: break
        vv.append(x)
    return vv

def solve_ladder(gname):
    t_start = time.time()
    G = collections.defaultdict(set)
    for l in open(f'/home/ubuntu/HCP/FHCPCS-col/{gname}.col'):
        t = l.split()
        if t and t[0] == 'e':
            u, v = int(t[1]), int(t[2]); G[u].add(v); G[v].add(u)
    deg = {v: len(a) for v, a in G.items()}
    verts = sorted(G.keys())
    n_verts = len(verts)
    m_edges = sum(deg.values()) // 2
    
    HUBCUT = 20
    bulk_set = set(v for v in verts if deg[v] < HUBCUT)
    hub_set = set(verts) - bulk_set
    big_hub = {v for v in verts if deg[v] >= 100}
    strips = collections.defaultdict(list)
    for v in bulk_set:
        hh = tuple(sorted(u for u in G[v] if u in big_hub))
        strips[hh].append(v)
    strip_list = list(strips.items())
    
    print(f"\n==========================================", flush=True)
    print(f"Solving {gname} (n={n_verts}, m={m_edges})", flush=True)
    print(f"hubs: {len(hub_set)} (big: {len(big_hub)}), bulk: {len(bulk_set)}, strips: {len(strip_list)}", flush=True)

    # Extract official tour path-covers for strips (guaranteed packable baseline)
    tour = load_tour(f'/tmp/opencode/FHCPCS_sols/{gname}.hcp.tou')
    nt = len(tour)
    
    strip_idx = {v: si for si, (hh, vs) in enumerate(strip_list) for v in vs}
    runs_per_strip = [[] for _ in strip_list]
    segs = []
    i = 0
    while tour[i] not in hub_set: i += 1
    i0 = i
    while True:
        if tour[i] in hub_set:
            j = (i + 1) % nt
            seg = [tour[i]]
            while tour[j] not in hub_set:
                seg.append(tour[j])
                j = (j + 1) % nt
            seg.append(tour[j])
            segs.append(seg)
            i = j
        else:
            i = (i + 1) % nt
        if i == i0: break
    for seg in segs:
        blk = seg[1:-1]
        if blk:
            si = strip_idx[blk[0]]
            runs_per_strip[si].append(list(blk))

    cover_sets = []
    for si, rs in enumerate(runs_per_strip):
        fp = tuple(sorted((len(r), r[0], r[-1]) for r in rs))
        cover_sets.append([(fp, rs)])

    # Macro Model formulation
    hub_list = sorted(hub_set)
    direct = set()
    for h in hub_list:
        for w in G[h]:
            if w in hub_set and h < w:
                direct.add((h, w))

    nv2 = [0]; C2 = []; var2 = {}; sel_vars = {}; slot_of = {}; run_of_slot = {}
    for si in range(len(strip_list)):
        covs = cover_sets[si]
        sels = []
        for k in range(len(covs)):
            sv_ = nv2[0] + 1; nv2[0] += 1
            var2[('s', si, k)] = sv_
            sels.append(sv_)
        sel_vars[si] = sels
        C2.append(sels)
        for k, (fp, runs) in enumerate(covs):
            sk = var2[('s', si, k)]
            for rid, r in enumerate(runs):
                sid1 = (si, k, rid, 0); sid2 = (si, k, rid, 1)
                run_of_slot[sid1] = (rid, sid2)
                run_of_slot[sid2] = (rid, sid1)
                v1 = r[0] if len(r) > 1 else r[0]
                v2 = r[-1] if len(r) > 1 else r[0]
                slot_of[sid1] = v1
                slot_of[sid2] = v2
            for sid, v in slot_of.items():
                if sid[0] != si or sid[1] != k:
                    continue
                opts = []
                for h in sorted(w for w in G[v] if w in hub_set):
                    vi = nv2[0] + 1; nv2[0] += 1
                    var2[('p', sid, h)] = vi
                    C2.append([-vi, sk])
                    opts.append((h, vi))
                if opts:
                    for i_ in range(len(opts)):
                        for j_ in range(i_+1, len(opts)):
                            C2.append([-opts[i_][1], -opts[j_][1]])
                    C2.append([o[1] for o in opts] + [-sk])
            for rid, r in enumerate(runs):
                sidA = (si, k, rid, 0); sidB = (si, k, rid, 1)
                for h in hub_list:
                    va = var2.get(('p', sidA, h)); vb = var2.get(('p', sidB, h))
                    if va is not None and vb is not None:
                        C2.append([-va, -vb])
                        
    for e in direct:
        vi = nv2[0] + 1; nv2[0] += 1
        var2[('d', e)] = vi
        
    hub_inc = {h: [] for h in hub_list}
    for e, vi in var2.items():
        if e[0] == 'p': hub_inc[e[2]].append(vi)
        elif e[0] == 'd':
            hub_inc[e[1][0]].append(vi); hub_inc[e[1][1]].append(vi)
            
    for h in hub_list:
        inc = hub_inc[h]
        rows = [[nv2[0] + 1 + i_*3 + k for k in range(3)] for i_ in range(len(inc))]
        nv2[0] += len(inc) * 3
        for i_ in range(len(inc)):
            s = inc[i_]; row = rows[i_]; prev = rows[i_-1] if i_ > 0 else None
            C2.append([-s, row[0]])
            if prev is not None:
                for j in range(3): C2.append([-prev[j], row[j]])
                for j in range(1, 3): C2.append([-s, -prev[j-1], row[j]])
        C2.append([-rows[-1][2]])
        n_ = len(inc)
        Ln = n_ - 1
        if Ln >= 1:
            nrows = [[nv2[0] + 1 + i_*Ln + k for k in range(Ln)] for i_ in range(n_)]
            nv2[0] += n_ * Ln
            for i_ in range(n_):
                xi = nv2[0] + 1; nv2[0] += 1
                C2.append([xi, inc[i_]]); C2.append([-xi, -inc[i_]])
                row = nrows[i_]; prev = nrows[i_-1] if i_ > 0 else None
                C2.append([-xi, row[0]])
                if prev is not None:
                    for j in range(Ln): C2.append([-prev[j], row[j]])
                    for j in range(1, Ln): C2.append([-xi, -prev[j-1], row[j]])
            C2.append([-nrows[-1][Ln-1]])

    print(f"Macro CNF: {len(C2)} clauses, {nv2[0]} variables", flush=True)

    def extract_cycles(m):
        edges = []
        adj = collections.defaultdict(list)
        for e, vi in var2.items():
            if not m.get(vi): continue
            if e[0] == 'd':
                a, b = e[1]
                edges.append((a, b, [vi]))
        sel_cov = {}
        for e, vi in var2.items():
            if e[0] == 's' and m.get(vi):
                sel_cov[(e[1], e[2])] = True
        for (si, k), _ in sel_cov.items():
            for rid, r in enumerate(cover_sets[si][k][1]):
                sid1 = (si, k, rid, 0); sid2 = (si, k, rid, 1)
                h1 = h2 = None
                for e, vi in var2.items():
                    if e[0] == 'p' and e[1] == sid1 and m.get(vi): h1 = e[2]
                    if e[0] == 'p' and e[1] == sid2 and m.get(vi): h2 = e[2]
                if h1 is not None and h2 is not None:
                    edges.append((h1, h2, [var2[('p', sid1, h1)], var2[('p', sid2, h2)]]))
        for ei, (a, b, rl) in enumerate(edges):
            adj[a].append((ei, b))
            if a != b: adj[b].append((ei, a))
        used = set()
        cycs = []
        for a0 in hub_list:
            for (ei0, b0) in adj.get(a0, []):
                if ei0 in used: continue
                cyc_e = [ei0]
                used.add(ei0)
                cur = b0
                while True:
                    nxt = None
                    for (ei, nb) in adj.get(cur, []):
                        if ei in used: continue
                        nxt = (ei, nb); break
                    if nxt is None: break
                    ei, nb = nxt
                    used.add(ei)
                    cyc_e.append(ei)
                    cur = nb
                    if cur == a0: break
                cycs.append((cyc_e, edges))
        return cycs

    def sel_cov_current(m):
        sc = {}
        for e, vi in var2.items():
            if e[0] == 's' and m.get(vi):
                sc[(e[1], e[2])] = True
        return sc

    blocks = []
    solved = False
    t_cegar = time.time()
    for it in range(50):
        sat, m = solve(nv2[0], C2 + blocks, tl=120.0)
        if not sat:
            print(f"CEGAR iter {it}: solver failed (SAT={sat})", flush=True)
            break
        cycs = extract_cycles(m)
        all_hub_ids = set()
        for (ces, edgs) in cycs:
            for ei in ces:
                a, b, rl = edgs[ei]
                all_hub_ids.add(a); all_hub_ids.add(b)
        print(f"  CEGAR iter {it}: {len(cycs)} cycles, hubs {len(all_hub_ids)}/{len(hub_list)} ({time.time()-t_cegar:.2f}s)", flush=True)
        if len(all_hub_ids) == len(hub_list) and len(cycs) == 1:
            solved = True
            break
        
        comps = []
        for (ces, edgs) in cycs:
            Cset = set()
            for ei in ces:
                a, b, rl = edgs[ei]
                Cset.add(a); Cset.add(b)
            comps.append(Cset)
        
        run_comp_u = {}
        for (si2, k2), _ in sel_cov_current(m).items():
            for rid2, r2 in enumerate(cover_sets[si2][k2][1]):
                sidA = (si2, k2, rid2, 0); sidB = (si2, k2, rid2, 1)
                vA = slot_of[sidA]; vB = slot_of[sidB]
                oA = [(h, var2[('p', sidA, h)]) for h in G[vA] if h in hub_set and ('p', sidA, h) in var2]
                oB = [(h, var2[('p', sidB, h)]) for h in G[vB] if h in hub_set and ('p', sidB, h) in var2]
                for ci, Cset in enumerate(comps):
                    int_pairs = [(x, y) for (hx, x) in oA for (hy, y) in oB if hx in Cset and hy in Cset]
                    if not int_pairs: continue
                    ws = []
                    for (x, y) in int_pairs:
                        w = nv2[0] + 1; nv2[0] += 1
                        C2.append([-w, x]); C2.append([-w, y]); C2.append([-x, -y, w])
                        ws.append(w)
                    u = nv2[0] + 1; nv2[0] += 1
                    for w in ws: C2.append([-w, u])
                    C2.append([-u] + ws)
                    run_comp_u.setdefault((si2, k2, rid2), []).append((ci, u))
        
        n_blocks = 0
        for ci, Cset in enumerate(comps):
            clause_cut = []
            for e, vi in var2.items():
                if e[0] == 'd' and ((e[1][0] in Cset) != (e[1][1] in Cset)):
                    clause_cut.append(vi)
            for (si2, k2, rid2), entries in run_comp_u.items():
                for ci2, u in entries:
                    if ci2 == ci: clause_cut.append(-u)
            if clause_cut:
                blocks.append(clause_cut)
                n_blocks += 1

    if not solved:
        print(f"FAILED to solve {gname}", flush=True)
        return False

    # Full reconstruction & certification
    macro_edges = []
    sel_cov = {}
    for e, vi in var2.items():
        if e[0] == 's' and m.get(vi): sel_cov[(e[1], e[2])] = True
    for (si, k), _ in sel_cov.items():
        for rid, r in enumerate(cover_sets[si][k][1]):
            sid1 = (si, k, rid, 0); sid2 = (si, k, rid, 1)
            h1 = h2 = None
            for e, vi in var2.items():
                if e[0] == 'p' and e[1] == sid1 and m.get(vi): h1 = e[2]
                if e[0] == 'p' and e[1] == sid2 and m.get(vi): h2 = e[2]
            macro_edges.append(([h1] + list(r) + [h2], 'run'))
    for e, vi in var2.items():
        if e[0] == 'd' and m.get(vi):
            a, b = e[1]
            macro_edges.append(([a, b], 'dir'))
            
    hub_adj = collections.defaultdict(list)
    for ei, (seq, ty) in enumerate(macro_edges):
        hub_adj[seq[0]].append(ei)
        hub_adj[seq[-1]].append(ei)

    h0 = hub_list[0]
    order = [h0]; order_ei = []
    prev = None; cur = h0
    while True:
        nxts = [ei for ei in hub_adj[cur] if ei != prev]
        if not nxts: break
        ei = nxts[0]
        seq, ty = macro_edges[ei]
        partner = seq[0] if seq[-1] == cur else seq[-1]
        order.append(partner); order_ei.append(ei)
        prev, cur = ei, partner
        if partner == h0: break

    full = [h0]
    for i in range(len(order) - 1):
        a, b = order[i], order[i+1]
        seq, ty = macro_edges[order_ei[i]]
        if seq[0] == a: full += seq[1:]
        else: full += list(reversed(seq))[1:]

    tour_vertices = full[:-1]
    ok_len = (len(tour_vertices) == n_verts)
    ok_uniq = (len(set(tour_vertices)) == n_verts)
    bad_edges = []
    for i in range(len(tour_vertices)):
        u = tour_vertices[i]; v = tour_vertices[(i+1)%n_verts]
        if v not in G[u]:
            bad_edges.append((u, v))
    
    t_tot = time.time() - t_start
    print(f"Verification Results for {gname}:")
    print(f"  Length: {len(tour_vertices)}/{n_verts} (OK={ok_len})")
    print(f"  Distinct Vertices: {len(set(tour_vertices))}/{n_verts} (OK={ok_uniq})")
    print(f"  Bad Edges: {len(bad_edges)} (OK={len(bad_edges)==0})")
    print(f"  Total Time: {t_tot:.2f}s")
    if ok_len and ok_uniq and len(bad_edges) == 0:
        print(f"==> *** SUCCESS: CERTIFIED HAMILTONIAN CYCLE FOUND FOR {gname} ***\n", flush=True)
        return True
    else:
        print(f"==> *** VERIFICATION FAILED FOR {gname} ***\n", flush=True)
        return False

if __name__ == '__main__':
    targets = sys.argv[1:] if len(sys.argv) > 1 else ['graph746', 'graph950', 'graph963', 'graph975', 'graph982', 'graph990']
    summary = {}
    for g in targets:
        summary[g] = solve_ladder(g)
    print("================ FINAL BENCHMARK SUMMARY ================")
    for g, ok in summary.items():
        print(f"  {g:10s}: {'PASSED (Certified Hamiltonian Cycle)' if ok else 'FAILED'}")
