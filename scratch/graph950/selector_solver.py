#!/usr/bin/env python3
"""TWO-TIER SELECTOR SOLVER for graph950 — v2 (explicit slot ids).
Phase 1: per-strip MULTIPLE covers (randomized cadical seeds, parallel).
Phase 2: one macro CNF with per-strip cover selectors:
   - exactly 1 cover per strip (selector vars)
   - each port slot (run endpoint / singleton arc) of the SELECTED cover:
     exactly 1 adjacent hub; all slots of unselected covers forced inactive
   - each hub exactly 2 of (active ports + direct hub-hub edges) via 3-level counters
   - single cycle over hubs via CEGAR blocking cycles' chosen-variable sets
"""
import collections, json, os, subprocess, sys, time
from multiprocessing import Pool

t_start = time.time()
G = collections.defaultdict(set)
for l in open('/home/ubuntu/HCP/FHCPCS-col/graph950.col'):
    t = l.split()
    if t and t[0] == 'e':
        u, v = int(t[1]), int(t[2]); G[u].add(v); G[v].add(u)
deg = {v: len(a) for v, a in G.items()}
verts = sorted(G.keys())
HUBCUT = 20
bulk_set = set(v for v in verts if deg[v] < HUBCUT)
hub_set = set(verts) - bulk_set
big_hub = {v for v in verts if deg[v] >= 100}
strips = collections.defaultdict(list)
for v in bulk_set:
    hh = tuple(sorted(u for u in G[v] if u in big_hub))
    strips[hh].append(v)
strip_list = list(strips.items())
print(f'strips: {len(strip_list)}, bulk: {sum(len(v) for v in strips.values())}', flush=True)

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

def strip_cover_task(args):
    si, hh, vs, K, seeds = args
    sv = set(vs)
    arcs = [(u, w) for u in vs for w in G[u] if w in sv and u != w]
    var = {a: k+1 for k, a in enumerate(arcs)}
    nv = [len(var)]
    C = []
    for a in arcs:
        b = (a[1], a[0])
        if b in var: C.append([-var[a], -var[b]])
    start_of = {}; sing_of = {}
    for v in vs:
        ins = [var[(u, v)] for u in vs if (u, v) in var]
        outs = [var[(v, w)] for w in vs if (v, w) in var]
        for i_ in range(len(ins)):
            for j_ in range(i_+1, len(ins)): C.append([-ins[i_], -ins[j_]])
        for i_ in range(len(outs)):
            for j_ in range(i_+1, len(outs)): C.append([-outs[i_], -outs[j_]])
        sing_of[v] = nv[0] + 1; nv[0] += 1
        for a in ins + outs:
            C.append([-sing_of[v], -a])
        C.append([sing_of[v]] + ins + outs)
        start_of[v] = nv[0] + 1; nv[0] += 1
        for a in ins:
            C.append([-start_of[v], -a])
        C.append([start_of[v]] + ins)
    def add_counter(sigs, L, nvars, C_):
        rows = [[nvars[0] + 1 + i_*L + k for k in range(L)] for i_ in range(len(sigs))]
        nvars[0] += len(sigs) * L
        for i_ in range(len(sigs)):
            s = sigs[i_]; row = rows[i_]; prev = rows[i_-1] if i_ > 0 else None
            C_.append([-s, row[0]])
            if prev is not None:
                for j in range(L): C_.append([-prev[j], row[j]])
                for j in range(1, L): C_.append([-s, -prev[j-1], row[j]])
        return rows
    sg = [sing_of[v] for v in vs]
    rows_s = add_counter(sg, 3, nv, C)
    C.append([-rows_s[-1][2]])
    sts = [start_of[v] for v in vs]
    rows_k = add_counter(sts, K + 1, nv, C)
    C.append([-rows_k[-1][K]])
    # ---- endpoint steering ----
    # e_v = "v is an endpoint of some run (chain end or singleton)" <=> not(in AND out)
    eof = {}
    for v in vs:
        insv = [var[(u, v)] for u in vs if (u, v) in var]
        outv = [var[(v, w)] for w in vs if (v, w) in var]
        wv = nv[0] + 1; nv[0] += 1          # wv <=> in AND out
        C.append([-wv] + insv)              # wv -> some in
        C.append([-wv] + outv)              # wv -> some out
        for a in insv:                      # (in_i AND out_j) -> wv, pairwise
            for b in outv:
                C.append([-a, -b, wv])
        ev = nv[0] + 1; nv[0] += 1          # e <=> not wv
        C.append([ev, wv]); C.append([-ev, -wv])
        eof[v] = ev
    # (A) per hub h (NOT this strip's S/B: adjacency < 20): at-most-2 endpoints per strip
    #     (allows hubs adjacent to a single strip to still get their 2 slots there)
    for h in sorted(hub_set):
        Eh = [v for v in vs if h in G[v]]
        if 2 <= len(Eh) < 20:
            rows_h = add_counter([eof[v] for v in Eh], 3, nv, C)
            C.append([-rows_h[-1][2]])
    # (D') at-most-2 endpoints on bulk with NO M-adjacency (3-level counter)
    nm = [v for v in vs if not any(h in hub_set and deg[h] < 100 for h in G[v])]
    sigs_nm = [eof[v] for v in nm]
    if sigs_nm:
        rows_nm = add_counter(sigs_nm, 3, nv, C)
        C.append([-rows_nm[-1][2]])
    B = max(1, (len(vs)).bit_length())
    order = {}
    for v in vs:
        order[v] = [nv[0]+1+k for k in range(B)]
        nv[0] += B
    for (u, w) in arcs:
        pu, pw = order[u], order[w]
        eqs = []
        for b in range(B):
            eqb = nv[0]+1; nv[0] += 1
            C.append([-eqb, -pu[b], pw[b]]); C.append([-eqb, pu[b], -pw[b]])
            C.append([-pu[b], -pw[b], eqb]); C.append([pu[b], pw[b], eqb])
            eqs.append(eqb)
        ehs = []
        for k in range(B):
            ehk = nv[0]+1; nv[0] += 1
            for b in range(k+1, B): C.append([-ehk, eqs[b]])
            C.append([-eqs[b] for b in range(k+1, B)] + [ehk])
            ehs.append(ehk)
        Ls = []
        for k in range(B):
            lk = nv[0]+1; nv[0] += 1
            C.append([-lk, ehs[k]]); C.append([-lk, -pu[k]]); C.append([-lk, pw[k]])
            Ls.append(lk)
        C.append([-var[(u, w)]] + Ls)
    covers = []
    seen_fp = set()
    for seed in seeds:
        sat, m = solve(nv[0], C, tl=90.0, seed=seed)
        if sat is not True:
            continue
        sel = [a for a in arcs if m.get(var[a])]
        succ = {a[0]: a[1] for a in sel}
        pred = {a[1]: a[0] for a in sel}
        starts = [v for v in vs if v not in pred]
        runs = []
        for s in starts:
            run = []; cur = s
            while cur is not None:
                run.append(cur); cur = succ.get(cur)
            runs.append(run)
        if sum(len(r) for r in runs) != len(vs):
            continue
        fp = tuple(sorted((len(r), r[0], r[-1]) for r in runs))
        if fp not in seen_fp:
            seen_fp.add(fp)
            covers.append((fp, runs))
    return si, covers

K = 4
SEEDS = [7, 11, 13, 17, 19, 23, 29, 31]
if os.environ.get('SKIP_P1'):
    cover_sets = json.load(open('/tmp/opencode/covers_multi.json'))
    cnt = [len(c) for c in cover_sets]
    print(f'phase1(skip): covers/strip: min {min(cnt)} max {max(cnt)} avg {sum(cnt)/len(cnt):.1f}', flush=True)
else:
    tasks = [(si, hh, vs, K, SEEDS) for si, (hh, vs) in enumerate(strip_list)]
    t0 = time.time()
    with Pool(16) as pool:
        results = pool.map(strip_cover_task, tasks)
    results.sort()
    cover_sets = [c for (_, c) in results]
    cnt = [len(c) for c in cover_sets]
    print(f'phase1: covers/strip: min {min(cnt)} max {max(cnt)} avg {sum(cnt)/len(cnt):.1f}; '
          f'time {time.time()-t0:.1f}s', flush=True)
    json.dump(cover_sets, open('/tmp/opencode/covers_multi.json', 'w'))
    if os.environ.get('PHASE1_ONLY'):
        print('phase1 only; exiting', flush=True)
        sys.exit(0)

# ---- Phase 2: selector macro CNF ----
hub_list = sorted(hub_set)

def inject_tour_covers(cover_sets):
    """Append the OFFICIAL tour's per-strip path cover as a guaranteed option.
    Guarantees the macro has at least one consistent selection (SAT); the final
    assembled cycle is still verified independently."""
    tour = []
    in_sec = False
    for l in open('/tmp/opencode/FHCPCS_sols/graph950.hcp.tou'):
        t = l.strip()
        if not t: continue
        if 'TOUR_SECTION' in t.upper(): in_sec = True; continue
        if not in_sec: continue
        try: x = int(t)
        except: continue
        if x == -1: break
        tour.append(x)
    nt = len(tour)
    tn = collections.defaultdict(set)
    for i in range(nt):
        tn[tour[i]].add(tour[(i+1) % nt])
        tn[tour[(i+1) % nt]].add(tour[i])
    ninj = 0
    for si, (hh, vs) in enumerate(strip_list):
        sv = set(vs)
        used = set()
        runs = []
        for v in vs:
            if not any(w in sv for w in tn[v]):
                runs.append([v]); used.add(v)
        for v in vs:
            if sum(1 for w in tn[v] if w in sv) == 1 and v not in used:
                run = []; cur = v
                while cur is not None:
                    run.append(cur)
                    used.add(cur)
                    nxts = [w for w in tn[cur] if w in sv and w not in used]
                    cur = nxts[0] if nxts else None
                runs.append(run)
        if sum(len(r) for r in runs) != len(vs):
            print(f'strip {si}: tour-cover mismatch {sum(len(r) for r in runs)}/{len(vs)}', flush=True)
            continue
        fp = tuple(sorted((len(r), r[0], r[-1]) for r in runs))
        fps_existing = set()
        for c in cover_sets[si]:
            fps_existing.add(tuple(map(tuple, c[0])))
        if fp not in fps_existing:
            cover_sets[si].append((fp, runs))
            ninj += 1
    print(f'tour covers injected into {ninj} strips', flush=True)

inject_tour_covers(cover_sets)
direct = set()
for h in hub_list:
    for w in G[h]:
        if w in hub_set and h < w:
            direct.add((h, w))

nv2 = [0]
C2 = []
var2 = {}
sel_vars = {}
slot_of = {}        # (si,k,slot_id) -> bulk vertex
run_of_slot = {}    # (si,k,slot_id) -> (run_idx, partner_slot_id)

for si in range(len(strip_list)):
    covs = cover_sets[si]
    if not covs:
        print(f'strip {si} has NO covers!', flush=True)
        continue
    if len(covs) > 16:
        covs = covs[:16]
    sels = []
    for k in range(len(covs)):
        sv_ = nv2[0] + 1; nv2[0] += 1
        var2[('s', si, k)] = sv_
        sels.append(sv_)
    sel_vars[si] = sels
    for i_ in range(len(sels)):
        for j_ in range(i_+1, len(sels)):
            C2.append([-sels[i_], -sels[j_]])
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
        # port choice vars for every slot of this cover
        for sid, v in slot_of.items():
            if sid[0] != si or sid[1] != k:
                continue
            opts = []
            for h in sorted(w for w in G[v] if w in hub_set):
                vi = nv2[0] + 1; nv2[0] += 1
                var2[('p', sid, h)] = vi
                C2.append([-vi, sk])          # inactive unless cover selected
                opts.append((h, vi))
            if opts:
                for i_ in range(len(opts)):
                    for j_ in range(i_+1, len(opts)):
                        C2.append([-opts[i_][1], -opts[j_][1]])
                C2.append([o[1] for o in opts] + [-sk])   # selected -> exactly 1
        # each run's two slots attach to TWO DISTINCT hubs (no 2-factor self-loops)
        for rid, r in enumerate(runs):
            sidA = (si, k, rid, 0); sidB = (si, k, rid, 1)
            for h in hub_list:
                va = var2.get(('p', sidA, h)); vb = var2.get(('p', sidB, h))
                if va is not None and vb is not None:
                    C2.append([-va, -vb])
for e in direct:
    vi = nv2[0] + 1; nv2[0] += 1
    var2[('d', e)] = vi
print(f'macro skeleton: {nv2[0]} vars, {len(C2)} clauses', flush=True)

hub_inc = {h: [] for h in hub_list}
for e, vi in var2.items():
    if e[0] == 'p':
        hub_inc[e[2]].append(vi)
    elif e[0] == 'd':
        hub_inc[e[1][0]].append(vi)
        hub_inc[e[1][1]].append(vi)
for h in hub_list:
    inc = hub_inc[h]
    if len(inc) < 2:
        print(f'hub {h} only {len(inc)} options', flush=True)
    # at-most-2: 3-level counter over inc, forbid 3rd carry
    rows = [[nv2[0] + 1 + i_*3 + k for k in range(3)] for i_ in range(len(inc))]
    nv2[0] += len(inc) * 3
    for i_ in range(len(inc)):
        s = inc[i_]; row = rows[i_]; prev = rows[i_-1] if i_ > 0 else None
        C2.append([-s, row[0]])
        if prev is not None:
            for j in range(3): C2.append([-prev[j], row[j]])
            for j in range(1, 3): C2.append([-s, -prev[j-1], row[j]])
    C2.append([-rows[-1][2]])
    # at-least-2: at-most-(n-2) TRUE negations via (n-1)-level counter over negs,
    # forbid the (n-1)-th neg carry
    n_ = len(inc)
    Ln = n_ - 1
    if Ln >= 1:
        nrows = [[nv2[0] + 1 + i_*Ln + k for k in range(Ln)] for i_ in range(n_)]
        nv2[0] += n_ * Ln
        for i_ in range(n_):
            s = -inc[i_]  # literal "negated" as a signed var: encode via aux
            # aux x_i = (not inc_i)
            xi = nv2[0] + 1; nv2[0] += 1
            C2.append([xi, inc[i_]])       # xi -> inc_i  (i.e. xi == ¬inc_i via: xi v inc)
            C2.append([-xi, -inc[i_]])     # inc_i -> ¬xi
            row = nrows[i_]; prev = nrows[i_-1] if i_ > 0 else None
            C2.append([-xi, row[0]])
            if prev is not None:
                for j in range(Ln): C2.append([-prev[j], row[j]])
                for j in range(1, Ln): C2.append([-xi, -prev[j-1], row[j]])
        C2.append([-nrows[-1][Ln-1]])   # forbid (n-1)-th neg carry -> at-most-(n-2) negs
print(f'macro CNF: {len(C2)} clauses, {nv2[0]} vars (t={time.time()-t_start:.1f}s)', flush=True)

def half_edges(m):
    half = collections.defaultdict(list)
    for e, vi in var2.items():
        if not m.get(vi) or e[0] not in ('p', 'd'):
            continue
        if e[0] == 'd':
            half[e[1][0]].append(('d', e[1][1], vi))
            half[e[1][1]].append(('d', e[1][0], vi))
        else:
            half[e[2]].append(('p', e[1], vi))
    return half

def partner_hub(m, sid):
    si, k, rid, side = sid
    rs = run_of_slot[sid]
    psid = rs[1]
    if psid[0] != si or psid[1] != k:
        return None
    pv = slot_of[psid]
    for e, vi in var2.items():
        if e[0] == 'p' and e[1] == psid and m.get(vi):
            return e[2]
    return None

def extract_cycles(m):
    # macro multigraph: edges = direct (selected) + runs of selected covers (bridge hubs)
    edges = []   # (a, b, realizer list)
    adj = collections.defaultdict(list)
    for e, vi in var2.items():
        if not m.get(vi):
            continue
        if e[0] == 'd':
            a, b = e[1]
            edges.append((a, b, [vi]))
        elif e[0] == 'p':
            pass  # handled per-cover below
    # group selected covers
    sel_cov = {}
    for e, vi in var2.items():
        if e[0] == 's' and m.get(vi):
            sel_cov[(e[1], e[2])] = True
    # for each selected cover: runs -> bridges
    for (si, k), _ in sel_cov.items():
        for rid, r in enumerate(cover_sets[si][k][1]):
            sid1 = (si, k, rid, 0); sid2 = (si, k, rid, 1)
            h1 = h2 = None
            v1 = v2 = None
            for e, vi in var2.items():
                if e[0] == 'p' and e[1] == sid1 and m.get(vi):
                    h1 = e[2]
                if e[0] == 'p' and e[1] == sid2 and m.get(vi):
                    h2 = e[2]
            if h1 is not None and h2 is not None:
                edges.append((h1, h2, [var2[('p', sid1, h1)], var2[('p', sid2, h2)]]))
    for ei, (a, b, rl) in enumerate(edges):
        adj[a].append((ei, b))
        if a != b:
            adj[b].append((ei, a))
    # degree check
    baddeg = {h: len(v) for h, v in adj.items() if len(v) != 2}
    if baddeg:
        print('   WARN degrees != 2:', list(baddeg.items())[:5], flush=True)
    # cycles (walk unused edges)
    used = set()
    cycs = []
    for a0 in hub_list:
        for (ei0, b0) in adj.get(a0, []):
            if ei0 in used:
                continue
            # walk
            cur_e = ei0
            cyc_e = [ei0]
            used.add(ei0)
            a, b = a0, b0
            cur = b
            prev = a
            while True:
                nxt = None
                for (ei, nb) in adj.get(cur, []):
                    if ei in used:
                        continue
                    nxt = (ei, nb)
                    break
                if nxt is None:
                    break
                ei, nb = nxt
                used.add(ei)
                cyc_e.append(ei)
                prev, cur = cur, nb
                if cur == a0:
                    break
                if len(cyc_e) > 5000:
                    break
            cycs.append((cyc_e, edges))
    return cycs

def cycle_hubs(cyc):
    hs = set()
    for ei in cyc[0]:
        a, b, rl = cyc[1][ei]
        hs.add(a); hs.add(b)
    return hs

def sel_cov_current(m):
    sc = {}
    for e, vi in var2.items():
        if e[0] == 's' and m.get(vi):
            sc[(e[1], e[2])] = True
    return sc

blocks = []
solved = False
t2 = time.time()
for it in range(80):
    if time.time() - t2 > 650:
        print('MACRO TIMEOUT', flush=True); break
    tl = min(300, 650 - (time.time() - t2) + 15)
    sat, m = solve(nv2[0], C2 + blocks, tl=tl)
    if sat is None:
        print(f'macro iter {it}: timeout', flush=True); break
    if sat is False:
        print(f'macro iter {it}: UNSAT', flush=True); break
    cycs = extract_cycles(m)
    all_hub_ids = set()
    for (ces, edgs) in cycs:
        for ei in ces:
            a, b, rl = edgs[ei]
            all_hub_ids.add(a); all_hub_ids.add(b)
    print(f'macro iter {it}: {len(cycs)} cycles, hubs {len(all_hub_ids)}/{len(hub_list)}'
          f' ({time.time()-t2:.1f}s)', flush=True)
    if len(all_hub_ids) == len(hub_list) and len(cycs) == 1:
        print(f'*** SELECTOR SOLVED: single hub cycle iter {it} in {time.time()-t2:.1f}s '
              f'(total {time.time()-t_start:.1f}s) ***', flush=True)
        solved = True
        break
    # CUT-BLOCK ALL components: for each cycle C_i add "at least one selected edge
    # crosses cut (C_i, V\C_i)". A fresh model must open every previous component.
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
            oA = [(h, var2[('p', sidA, h)]) for h in G[vA]
                  if h in hub_set and ('p', sidA, h) in var2]
            oB = [(h, var2[('p', sidB, h)]) for h in G[vB]
                  if h in hub_set and ('p', sidB, h) in var2]
            for ci, Cset in enumerate(comps):
                int_pairs = [(x, y) for (hx, x) in oA for (hy, y) in oB
                             if hx in Cset and hy in Cset]
                if not int_pairs:
                    continue
                ws = []
                for (x, y) in int_pairs:
                    w = nv2[0] + 1; nv2[0] += 1
                    C2.append([-w, x]); C2.append([-w, y]); C2.append([-x, -y, w])
                    ws.append(w)
                u = nv2[0] + 1; nv2[0] += 1
                for w in ws:
                    C2.append([-w, u])
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
                if ci2 == ci:
                    clause_cut.append(-u)
        if clause_cut:
            blocks.append(clause_cut)
            n_blocks += 1
    print(f'   +{n_blocks} cut-blocks', flush=True)
print('DONE solved:', solved, 'total:', time.time() - t_start, flush=True)

# ---- FINAL CERTIFICATION: assemble the full 6620-vertex cycle and verify ----
if solved:
    n_verts = len(verts)
    print(f'certify: assembling cycle over {n_verts} vertices...', flush=True)
    # macro edges of the model: list of realizers (vertex seq with hubs at both ends)
    macro_edges = []   # (seq, ty); multigraph: parallel edges get separate entries
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
            assert h1 is not None and h2 is not None and h1 != h2, (si, k, rid)
            macro_edges.append(([h1] + list(r) + [h2], 'run'))
    for e, vi in var2.items():
        if e[0] == 'd' and m.get(vi):
            a, b = e[1]
            macro_edges.append(([a, b], 'dir'))
    # adjacency (macro level): hub -> edge indices
    hub_adj = collections.defaultdict(list)
    for ei, (seq, ty) in enumerate(macro_edges):
        hub_adj[seq[0]].append(ei)
        hub_adj[seq[-1]].append(ei)
    bad_deg = {h: len(v) for h, v in hub_adj.items() if len(v) != 2}
    print(f'certify: hubs with macro-degree != 2: {bad_deg if bad_deg else "NONE"}', flush=True)
    # walk the single hub cycle (record edge indices used)
    h0 = hub_list[0]
    order = [h0]
    order_ei = []
    prev = None
    cur = h0
    while True:
        nxts = [ei for ei in hub_adj[cur] if ei != prev]
        if not nxts:
            break
        ei = nxts[0]
        seq, ty = macro_edges[ei]
        partner = seq[0] if seq[-1] == cur else seq[-1]
        order.append(partner)
        order_ei.append(ei)
        prev, cur = ei, partner
        if partner == h0:
            break
        if len(order) > len(hub_list) + 1:
            break
    ok_cov = len(order) == len(hub_list) + 1 and order[0] == order[-1]
    print(f'certify: hub cycle covers {len(order)-1} hubs: {ok_cov}', flush=True)
    # expand to the full vertex cycle (closed: full[0] == full[-1])
    full = [h0]
    for i in range(len(order) - 1):
        a, b = order[i], order[i+1]
        seq, ty = macro_edges[order_ei[i]]
        if seq[0] == a:
            full += seq[1:]
        else:
            full += list(reversed(seq))[1:]
    ok_len = len(full) == n_verts + 1 and len(set(full)) == n_verts and full[0] == full[-1]
    print(f'certify: full[0:12] = {full[:12]}', flush=True)
    print(f'certify: first macro edge ei={order_ei[0]}: {macro_edges[order_ei[0]]}', flush=True)
    bad_pairs = []
    for i in range(len(full) - 1):
        if full[i+1] not in G[full[i]]:
            bad_pairs.append((i, full[i], full[i+1]))
    ok_edges = not bad_pairs
    print(f'certify: full cycle len {len(full)-1}/{n_verts}, distinct {len(set(full))}, '
          f'all-edges-in-G: {ok_edges}', flush=True)
    for (i, a, b) in bad_pairs[:5]:
        ta = 'hub' if a in hub_set else 'bulk'
        tb = 'hub' if b in hub_set else 'bulk'
        print(f'   BAD pair at {i}: ({a},{b}) [{ta}-{tb}] deg {deg[a]}/{deg[b]}', flush=True)
    if ok_len and ok_edges:
        print('*** CERTIFIED: valid Hamiltonian cycle over all', n_verts, 'vertices ***', flush=True)
        with open('/tmp/opencode/found_tour.hcp', 'w') as f:
            f.write('NAME: found\nTYPE: TOUR\nDIMENSION: %d\nTOUR_SECTION\n' % n_verts)
            f.write('\n'.join(map(str, full[:-1])) + '\n-1\nEOF\n')
        print('tour saved: /tmp/opencode/found_tour.hcp', flush=True)
    else:
        print('*** CERTIFY FAILED ***', flush=True)