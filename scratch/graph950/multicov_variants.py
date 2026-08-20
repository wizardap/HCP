#!/usr/bin/env python3
"""Isolate multi-cover macro UNSAT: rebuild from covers_multi.json, try variants."""
import collections, json, subprocess, sys, time

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
hub_list = sorted(hub_set)
print('hubs:', len(hub_list))

cov_sets = json.load(open('/tmp/opencode/covers_multi.json'))
nstrips = len(cov_sets)
print('strips:', nstrips, 'covers:', [len(c) for c in cov_sets])

direct = set()
for h in hub_list:
    for w in G[h]:
        if w in hub_set and h < w:
            direct.add((h, w))
print('direct hub-hub edges:', len(direct))

def build(use_direct, exact2, distinct_hub):
    nv = [0]; C = []; var2 = {}
    sel_vars = {}
    slot_of = {}; run_of_slot = {}
    for si in range(nstrips):
        covs = cov_sets[si]
        if len(covs) > 8:
            covs = covs[:8]
        sels = []
        for k in range(len(covs)):
            sv_ = nv[0] + 1; nv[0] += 1
            var2[('s', si, k)] = sv_
            sels.append(sv_)
        sel_vars[si] = sels
        for i_ in range(len(sels)):
            for j_ in range(i_+1, len(sels)):
                C.append([-sels[i_], -sels[j_]])
        C.append(sels)
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
                    vi = nv[0] + 1; nv[0] += 1
                    var2[('p', sid, h)] = vi
                    C.append([-vi, sk])
                    opts.append((h, vi))
                if opts:
                    for i_ in range(len(opts)):
                        for j_ in range(i_+1, len(opts)):
                            C.append([-opts[i_][1], -opts[j_][1]])
                    C.append([o[1] for o in opts] + [-sk])
            if distinct_hub:
                for rid, r in enumerate(runs):
                    sidA = (si, k, rid, 0); sidB = (si, k, rid, 1)
                    for h in hub_list:
                        va = var2.get(('p', sidA, h)); vb = var2.get(('p', sidB, h))
                        if va is not None and vb is not None:
                            C.append([-va, -vb])
    if use_direct:
        for e in direct:
            vi = nv[0] + 1; nv[0] += 1
            var2[('d', e)] = vi
    hub_inc = {h: [] for h in hub_list}
    for e, vi in var2.items():
        if e[0] == 'p':
            hub_inc[e[2]].append(vi)
        elif e[0] == 'd':
            hub_inc[e[1][0]].append(vi)
            hub_inc[e[1][1]].append(vi)
    for h in hub_list:
        inc = hub_inc[h]
        if len(inc) == 0:
            return None, f'hub {h} zero options'
        # at-most-2
        rows = [[nv[0] + 1 + i_*3 + k for k in range(3)] for i_ in range(len(inc))]
        nv[0] += len(inc) * 3
        for i_ in range(len(inc)):
            s = inc[i_]; row = rows[i_]; prev = rows[i_-1] if i_ > 0 else None
            C.append([-s, row[0]])
            if prev is not None:
                for j in range(3): C.append([-prev[j], row[j]])
                for j in range(1, 3): C.append([-s, -prev[j-1], row[j]])
        C.append([-rows[-1][2]])
        if exact2:
            n_ = len(inc)
            Ln = n_ - 1
            if Ln < 1:
                return None, f'hub {h} contraint'
            nrows = [[nv[0] + 1 + i_*Ln + k for k in range(Ln)] for i_ in range(n_)]
            nv[0] += n_ * Ln
            for i_ in range(n_):
                xi = nv[0] + 1; nv[0] += 1
                C.append([xi, inc[i_]]); C.append([-xi, -inc[i_]])
                row = nrows[i_]; prev = nrows[i_-1] if i_ > 0 else None
                C.append([-xi, row[0]])
                if prev is not None:
                    for j in range(Ln): C.append([-prev[j], row[j]])
                    for j in range(1, Ln): C.append([-xi, -prev[j-1], row[j]])
            C.append([-nrows[-1][Ln-1]])
    return nv[0], C

def run(nv, C, tl=120.0):
    lines = [f'p cnf {nv} {len(C)}'] + [' '.join(map(str, c+[0])) for c in C]
    try:
        r = subprocess.run(['/usr/local/bin/cadical', '--quiet', '/dev/stdin'],
                           input='\n'.join(lines) + '\n', capture_output=True, text=True, timeout=tl)
    except subprocess.TimeoutExpired:
        return 'TIMEOUT'
    out = r.stdout
    if 's UNSATISFIABLE' in out: return 'UNSAT'
    if 's SATISFIABLE' in out: return 'SAT'
    return '??'

t0 = time.time()
for use_direct, exact2, dh, nm in [(False, False, True, 'ports atmost2'),
                                   (False, True, True, 'ports exactly2'),
                                   (True, True, True, 'ports+directs exactly2'),
                                   (True, False, True, 'ports+directs atmost2')]:
    t1 = time.time()
    r = build(use_direct, exact2, dh)
    if r[0] is None:
        print(f'{nm}: build error {r[1]}', flush=True)
        continue
    nv_, C_ = r
    print(f'{nm}: {len(C_)} cls, {nv_} vars (build {time.time()-t1:.0f}s) -> ', end='', flush=True)
    st = run(nv_, C_)
    print(st, flush=True)
print('total', time.time()-t0)