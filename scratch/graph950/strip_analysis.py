#!/usr/bin/env python3
"""Deep structural analysis of ladder-family strip construction."""
import collections, sys

def load(path):
    adj = collections.defaultdict(set)
    for l in open(path):
        t = l.split()
        if t and t[0] == 'e':
            u, v = int(t[1]), int(t[2])
            adj[u].add(v); adj[v].add(u)
    return adj

def analyze(gname):
    adj = load(f'/home/ubuntu/HCP/FHCPCS-col/{gname}.col')
    deg = {v: len(a) for v, a in adj.items()}
    print(f'===== {gname}: n={len(adj)}, m={sum(deg.values())//2} =====')
    # tier classification by exact degree
    tiers = collections.defaultdict(list)
    for v, d in deg.items():
        tiers[d].append(v)
    print('degree tiers:', {d: len(vs) for d, vs in sorted(tiers.items()) if len(vs) <= 1000})
    print('big tiers:', {d: len(vs) for d, vs in sorted(tiers.items()) if len(vs) > 1000})

    # hub set = all vertices with degree >= some cutoff (mid tiers)
    hub_deg_cutoff = 20
    hubs = sorted([v for v, d in deg.items() if d >= hub_deg_cutoff])
    hset = set(hubs)
    bulk = [v for v in adj if v not in hset]
    print(f'hubs (deg>={hub_deg_cutoff}): {len(hubs)}; bulk: {len(bulk)}')

    # bulk hub-attachment pairs
    pairs = collections.defaultdict(list)
    n_attach_not_hub = 0
    for v in bulk:
        att = tuple(sorted(u for u in adj[v] if u in hset))
        if len(att) == 2:
            pairs[att].append(v)
        else:
            n_attach_not_hub += 1
            pairs[('other', len(att))].append(v)
    print(f'bulk with !=2 hub attachments: {n_attach_not_hub}')
    pair_sizes = sorted((len(v) for v in pairs.values()), reverse=True)
    print(f'distinct strip pairs: {len(pairs)}; size histogram head: {collections.Counter(pair_sizes).most_common(10)}')

    # decompose pairs by hub-tier combination
    tier_of = {v: d for v, d in deg.items()}
    combo = collections.Counter()
    for (a, b), vs in pairs.items():
        if a == 'other': continue
        combo[(tier_of[a], tier_of[b])] += len(vs)
    print('bulk vertices by (hub-degree, hub-degree) pair:', dict(combo))

    # which hubs are involved
    hub_in_strips = collections.Counter()
    for (a, b) in pairs.keys():
        if a == 'other': continue
        hub_in_strips[a] += 1
        hub_in_strips[b] += 1
    print('strips per hub: min={} max={} | hubs with 0 strips: {}'.format(
        min(hub_in_strips.values()), max(hub_in_strips.values()),
        len(set(hubs) - set(hub_in_strips))))

    # strip internal structure of the largest strips
    def strip_stats(vs):
        bset = set(vs)
        internal = collections.Counter()
        hub_att = collections.Counter()
        for v in vs:
            for u in adj[v]:
                if u in bset:
                    internal[u if u > v else v] += 1  # count unordered
                elif u in hset:
                    hub_att[u] += 1
        # degree within strip
        dstrip = {v: sum(1 for u in adj[v] if u in bset) for v in vs}
        maxd = max(dstrip.values())
        # components within strip
        seen = set(); comps = []
        for v in vs:
            if v in seen: continue
            st = [v]; seen.add(v); c = 0
            while st:
                x = st.pop(); c += 1
                for u in adj[x]:
                    if u in bset and u not in seen:
                        seen.add(u); st.append(u)
            comps.append(c)
        return len(internal), maxd, sorted(comps, reverse=True), sorted(hub_att.items())

    print('\nstrip internal structure (top 6 by size):')
    for (a, b), vs in sorted(pairs.items(), key=lambda kv: -len(kv[1]))[:6]:
        if a == 'other': continue
        int_edges, maxd, comps, hatt = strip_stats(vs)
        # 2*int_edges = sum of internal degrees
        print(f'  pair ({a},{b}) tiers ({tier_of[a]},{tier_of[b]}): size={len(vs)}, internal_edges={int_edges}, max_internal_degree={maxd}, components={comps}, hub_edges={hatt}')

    # hub-hub edges
    hh = collections.Counter()
    for u in hset:
        for v in adj[u]:
            if v in hset and u < v:
                hh[(tier_of[u], tier_of[v])] += 1
    print('hub-hub edges by (deg,deg):', dict(hh))

    # bulk internal-degree distribution (per vertex: edges not to hubs)
    d_in = collections.Counter()
    for v in bulk:
        d_in[sum(1 for u in adj[v] if u not in hset)] += 1
    print('bulk internal-degree distribution (non-hub edges):', dict(sorted(d_in.items())))
    print()

for g in ['graph746', 'graph950', 'graph990']:
    analyze(g)