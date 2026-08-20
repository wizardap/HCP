#!/usr/bin/env python3
"""Analyze the official FHCP tour of graph950: how the cycle threads hubs & strips."""
import collections

def load(path):
    adj = collections.defaultdict(set)
    for l in open(path):
        t = l.split()
        if t and t[0] == 'e':
            u, v = int(t[1]), int(t[2])
            adj[u].add(v); adj[v].add(u)
    return adj

def load_tour(path):
    vs = []
    for l in open(path):
        t = l.strip()
        if not t: continue
        try: x = int(t)
        except: continue
        if x == -1: break
        vs.append(x)
    return vs

gname = 'graph950'
adj = load(f'/home/ubuntu/HCP/FHCPCS-col/{gname}.col')
deg = {v: len(a) for v, a in adj.items()}
tour = load_tour(f'/tmp/opencode/FHCPCS_sols/{gname}.hcp.tou')
print(f'tour length: {len(tour)} (n={len(adj)})')
assert len(tour) == len(adj)

h1 = set(v for v, d in deg.items() if d == 662)
h2 = set(v for v, d in deg.items() if d == 133)
h3 = set(v for v, d in deg.items() if 20 <= d <= 40)
h12 = h1 | h2
bulk = set(adj) - h12 - h3
print(f'h1={len(h1)} h2={len(h2)} h3={len(h3)} bulk={len(bulk)}')

# cycle edges: tour[i] - tour[i+1] (cyclic)
cyc = set()
n = len(tour)
for i in range(n):
    a, b = tour[i], tour[(i+1) % n]
    cyc.add((min(a,b), max(a,b)))

# 1) For each hub: its 2 cycle edges + neighbor types
def hub_cycle_profile():
    print('\n=== hub cycle edges ===')
    for tier, hs in (('h1', h1), ('h2', h2), ('h3', h3)):
        typ = collections.Counter()
        for u in hs:
            nbrs = [v for (a,b) in cyc if a == u and b != u or b == u and a != u for v in [(a if b==u else b)]]
            # simpler: gather neighbors in cycle
            nbrs = set()
            for a, b in cyc:
                if a == u: nbrs.add(b)
                elif b == u: nbrs.add(a)
            t = tuple(sorted('h1' if v in h1 else 'h2' if v in h2 else 'h3' if v in h3 else 'bulk' for v in nbrs))
            typ[t] += 1
        print(f'{tier}: cycle-neighbor-type profiles: {dict(typ)}')

hub_cycle_profile()

# 2) Strip threading: for each strip (h12 pair), list the tour segments through it
# Strip def: bulk sharing the same h12 attachment pair
strips = collections.defaultdict(list)
for v in bulk:
    a12 = tuple(sorted(u for u in adj[v] if u in h12))
    if len(a12) == 2:
        strips[a12].append(v)
print(f'\nstrips: {len(strips)}')

# position of each vertex in tour
pos = {v: i for i, v in enumerate(tour)}
def next_v(v): return tour[(pos[v]+1) % n]
def prev_v(v): return tour[(pos[v]-1) % n]

# For each strip: how many maximal runs of strip-vertices appear in the tour?
print('\n=== strip threading in tour ===')
run_stats = collections.Counter()
run_lens = collections.defaultdict(list)
strip_cross = collections.Counter()  # what connects runs: hub of which tier?
for (a, b), vs in strips.items():
    sset = set(vs)
    runs = []
    cur = []
    for i in range(n):
        v = tour[i]
        if v in sset:
            cur.append(v)
        else:
            if cur: runs.append(cur); cur = []
    if cur: runs.append(cur)
    run_stats[len(runs)] += 1
    run_lens[len(runs)].append(sorted(len(r) for r in runs))
    # endpoints of each run: neighbors outside strip
    outs = []
    for r in runs:
        e1 = prev_v(r[0]); e2 = next_v(r[-1])
        outs.append(('h1' if e1 in h1 else 'h2' if e1 in h2 else 'h3' if e1 in h3 else 'bulk',
                     'h1' if e2 in h1 else 'h2' if e2 in h2 else 'h3' if e2 in h3 else 'bulk'))
    strip_cross[tuple(sorted(outs))] += 1
print(f'runs per strip: {dict(run_stats)}')
print(f'run lengths: { {k: (min(v), max(v)) for k, v in run_lens.items()} }')

# 3) The big picture: sequence of hub-visits in the tour
print('\n=== macro sequence (hub types only, compressed) ===')
seq = []
for v in tour:
    if v in h1: seq.append('S')
    elif v in h2: seq.append('B')
    elif v in h3: seq.append('M')
# compress runs
comp = []
for s in seq:
    if comp and comp[-1][0] == s: comp[-1][1] += 1
    else: comp.append([s, 1])
print('compressed hub sequence:', ''.join(f'{s}{c}' for s, c in comp))

# 4) bulk cycle-edge types: how many bulk use (internal,internal) vs (hub,*) etc
print('\n=== bulk cycle edges ===')
eused = collections.Counter()
for v in bulk:
    nb = [u for (a,b) in cyc if (a == v and b != v) or (b == v and a != v) for u in ([a] if b==v else [b])]
    nb = []
    for a, b in cyc:
        if a == v: nb.append(b)
        elif b == v: nb.append(a)
    t = tuple(sorted('h1' if u in h1 else 'h2' if u in h2 else 'h3' if u in h3 else 'int' for u in nb))
    eused[t] += 1
print(f'bulk vertex cycle-edge types: {dict(eused)}')

# 5) mid-hub cycle edges
print('\n=== mid-hub (h3) cycle edges ===')
mused = collections.Counter()
for v in h3:
    nb = []
    for a, b in cyc:
        if a == v: nb.append(b)
        elif b == v: nb.append(a)
    t = tuple(sorted('h1' if u in h1 else 'h2' if u in h2 else 'h3' if u in h3 else 'bulk' for u in nb))
    mused[t] += 1
print(f'h3 cycle-edge types: {dict(mused)}')
