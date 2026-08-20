#!/usr/bin/env python3
"""Which hubs cannot reach degree >= 2 in ANY (at-most-2) solution?
For each hub: at-most-2 CNF + 'h has >= 2 of its options selected' clause.
Hubs where this is UNSAT can never balance -> the exact-2 blocker(s)."""
import collections, subprocess, sys, time

sys.path.insert(0, '/tmp/opencode')
src = open('/tmp/opencode/multicov_variants.py').read()
cut = src.find('t0 = time.time()')
exec(src[:cut])
print('hubs:', len(hub_list), flush=True)

# cache: option vars per hub
hub_opts = collections.defaultdict(list)
# rebuild var mapping is inside build(); do a lightweight rebuild of option lists:
# (repeat the port option enumeration quickly)
cov_sets2 = json.load(open('/tmp/opencode/covers_multi.json'))
G2 = G

def run_cnf(nv_, C_, tl=60.0):
    lines = [f'p cnf {nv_} {len(C_)}'] + [' '.join(map(str, c+[0])) for c in C_]
    try:
        r = subprocess.run(['/usr/local/bin/cadical', '--quiet', '/dev/stdin'],
                           input='\n'.join(lines) + '\n', capture_output=True, text=True, timeout=tl)
    except subprocess.TimeoutExpired:
        return None
    out = r.stdout
    if 's UNSATISFIABLE' in out: return False
    if 's SATISFIABLE' in out: return True
    return None

# enumerate options per hub as (var, cover-sel...) — reuse build's var2: rebuild via exec of build() internals
# simplest: rebuild the SAME build with a returned var map: patch multicov build to return var2 — do inline copy
def build2(use_direct=False, exact2=False, distinct_hub=True):
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
            continue   # forced 0 under ports-only: fine for at-most-2
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
    return nv[0], C, var2, hub_inc

def htype(h):
    d = deg[h]
    return 'S' if d >= 500 else ('B' if d >= 100 else 'M')

nv_, C_, var2, hub_inc = build2()
print(f'base CNF {len(C_)} cls, {nv_} vars', flush=True)
nb = len(C_)
print('all hubs option counts: min', min(len(v) for v in hub_inc.values()),
      'max', max(len(v) for v in hub_inc.values()), flush=True)

unreach = []
reach = 0
t0 = time.time()
for idx, h in enumerate(hub_list):
    inc = hub_inc[h]
    if len(inc) < 2:
        unreach.append((h, 'fewer-than-2-options'))
        continue
    # "h reaches >= 2": OR over pairs (o_i & o_j)
    add = []
    nvx = [nv_]
    pairs_w = []
    ws = []
    for i_ in range(len(inc)):
        for j_ in range(i_+1, len(inc)):
            w = nvx[0] + 1; nvx[0] += 1
            add.append([-w, inc[i_]]); add.append([-w, inc[j_]]); add.append([-inc[i_], -inc[j_], w])
            ws.append(w)
    add.append(ws)
    st = run_cnf(nvx[0], C_ + add, tl=40.0)
    if st is False:
        unreach.append((h, len(inc)))
    else:
        reach += 1
    if (idx+1) % 50 == 0:
        print(f'  {idx+1}/310 done ({time.time()-t0:.0f}s)', flush=True)
print('hubs that CAN reach >=2 (ports only):', reach, '/', len(hub_list))
print('unreachable:', len(unreach))
from collections import Counter
print('  by type:', Counter(htype(h) for h, _ in unreach))
for h, info in unreach[:25]:
    print(f'   hub {h} ({htype(h)}, deg {deg[h]}): {info} options, '
          f'ports-adj {len(hub_inc[h])}')