#!/usr/bin/env python3
"""Phase 1: Targeted Hub-Demand Steered Path Cover Generator for graph950.

Decomposes graph into hubs and bulk strips. Formulates SAT with base path-cover
constraints plus targeted hub-demand steering clauses for all adjacent M-hubs
(20 <= deg < 100), ensuring all M-hubs have adequate candidate endpoints for
subsequent Macro CNF solving without any external tour injection.
"""

import collections
import json
import os
import subprocess
import sys
import time
from multiprocessing import Pool

CADICAL_BIN = '/usr/local/bin/cadical'


def load_graph(graph_path):
    """Load graph adjacency from .col file."""
    G = collections.defaultdict(set)
    with open(graph_path, 'r') as f:
        for line in f:
            tokens = line.split()
            if tokens and tokens[0] == 'e':
                u, v = int(tokens[1]), int(tokens[2])
                G[u].add(v)
                G[v].add(u)
    return G


def decompose_graph(G, hubcut=20, bighub_cut=100):
    """Decompose graph into bulk strips and hub sets.

    Returns:
        strip_list: list of (hh_tuple, vs_list)
        bulk_set: set of bulk vertices (deg < hubcut)
        hub_set: set of hub vertices (deg >= hubcut)
        big_hub: set of big hub vertices (deg >= bighub_cut)
    """
    deg = {v: len(a) for v, a in G.items()}
    verts = sorted(G.keys())
    bulk_set = set(v for v in verts if deg[v] < hubcut)
    hub_set = set(verts) - bulk_set
    big_hub = {v for v in verts if deg[v] >= bighub_cut}

    strips = collections.defaultdict(list)
    for v in bulk_set:
        hh = tuple(sorted(u for u in G[v] if u in big_hub))
        strips[hh].append(v)

    strip_list = list(strips.items())
    return strip_list, bulk_set, hub_set, big_hub


def add_sequential_counter(sigs, L, nvars, clauses):
    """Add a sequential unary counter for sum(sigs) <= L - 1.

    Returns rows of counter auxiliary variables.
    """
    rows = [[nvars[0] + 1 + i_ * L + k for k in range(L)] for i_ in range(len(sigs))]
    nvars[0] += len(sigs) * L
    for i_ in range(len(sigs)):
        s = sigs[i_]
        row = rows[i_]
        prev = rows[i_ - 1] if i_ > 0 else None
        clauses.append([-s, row[0]])
        if prev is not None:
            for j in range(L):
                clauses.append([-prev[j], row[j]])
            for j in range(1, L):
                clauses.append([-s, -prev[j - 1], row[j]])
    return rows


def solve_cnf(nv, clauses, timeout=30.0, seed=None):
    """Solve CNF using CaDiCaL SAT solver."""
    lines = [f'p cnf {nv} {len(clauses)}'] + [' '.join(map(str, c + [0])) for c in clauses]
    args = [CADICAL_BIN, '--quiet']
    if seed is not None:
        args.append(f'--seed={seed}')
    try:
        r = subprocess.run(
            args + ['/dev/stdin'],
            input='\n'.join(lines) + '\n',
            capture_output=True,
            text=True,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired:
        return None, {}

    out = r.stdout
    if 's SATISFIABLE' not in out:
        return False, {}

    m = {}
    for line in out.splitlines():
        if line.startswith('v '):
            for x in line.split()[1:]:
                xi = int(x)
                m[abs(xi)] = xi > 0
    return True, m


def solve_strip_targeted(si, hh, vs, G, deg, K=4, seeds=(7, 11, 13), timeout=30.0):
    """Solve path cover for a single strip with targeted M-hub steering.

    Args:
        si: strip index
        hh: big-hub signature tuple
        vs: list of bulk vertices in the strip
        G: graph adjacency dict
        deg: vertex degree dict
        K: maximum number of paths in the cover (default 4)
        seeds: list/tuple of solver seeds
        timeout: per-solve timeout in seconds

    Returns:
        covers: list of (fingerprint, runs)
    """
    sv = set(vs)
    arcs = [(u, w) for u in vs for w in G[u] if w in sv and u != w]
    var = {a: k + 1 for k, a in enumerate(arcs)}
    nv = [len(var)]
    C = []

    # Anti-parallel arcs
    for a in arcs:
        b = (a[1], a[0])
        if b in var:
            C.append([-var[a], -var[b]])

    # In-degree <= 1, out-degree <= 1, singletons and starts
    start_of = {}
    sing_of = {}
    for v in vs:
        ins = [var[(u, v)] for u in vs if (u, v) in var]
        outs = [var[(v, w)] for w in vs if (v, w) in var]
        for i_ in range(len(ins)):
            for j_ in range(i_ + 1, len(ins)):
                C.append([-ins[i_], -ins[j_]])
        for i_ in range(len(outs)):
            for j_ in range(i_ + 1, len(outs)):
                C.append([-outs[i_], -outs[j_]])

        sing_of[v] = nv[0] + 1
        nv[0] += 1
        for a in ins + outs:
            C.append([-sing_of[v], -a])
        C.append([sing_of[v]] + ins + outs)

        start_of[v] = nv[0] + 1
        nv[0] += 1
        for a in ins:
            C.append([-start_of[v], -a])
        C.append([start_of[v]] + ins)

    # Bound singletons <= 2
    sg = [sing_of[v] for v in vs]
    rows_s = add_sequential_counter(sg, 3, nv, C)
    C.append([-rows_s[-1][2]])

    # Bound path count <= K
    sts = [start_of[v] for v in vs]
    rows_k = add_sequential_counter(sts, K + 1, nv, C)
    C.append([-rows_k[-1][K]])

    # Endpoint indicator e_v <=> not(in_v and out_v)
    eof = {}
    for v in vs:
        insv = [var[(u, v)] for u in vs if (u, v) in var]
        outv = [var[(v, w)] for w in vs if (v, w) in var]
        wv = nv[0] + 1
        nv[0] += 1
        C.append([-wv] + insv)
        C.append([-wv] + outv)
        for a in insv:
            for b in outv:
                C.append([-a, -b, wv])
        ev = nv[0] + 1
        nv[0] += 1
        C.append([ev, wv])
        C.append([-ev, -wv])
        eof[v] = ev

    # Hub degree bound: at-most-2 endpoints per hub h with 2 <= |E_h| < 20
    hub_set = set(v for v, d in deg.items() if d >= 20)
    for h in sorted(hub_set):
        Eh = [v for v in vs if h in G[v]]
        if 2 <= len(Eh) < 20:
            rows_h = add_sequential_counter([eof[v] for v in Eh], 3, nv, C)
            C.append([-rows_h[-1][2]])

    # Acyclic order encoding (B bits)
    B = max(1, (len(vs)).bit_length())
    order = {}
    for v in vs:
        order[v] = [nv[0] + 1 + k for k in range(B)]
        nv[0] += B
    for (u, w) in arcs:
        pu, pw = order[u], order[w]
        eqs = []
        for b in range(B):
            eqb = nv[0] + 1
            nv[0] += 1
            C.append([-eqb, -pu[b], pw[b]])
            C.append([-eqb, pu[b], -pw[b]])
            C.append([-pu[b], -pw[b], eqb])
            C.append([pu[b], pw[b], eqb])
            eqs.append(eqb)
        ehs = []
        for k in range(B):
            ehk = nv[0] + 1
            nv[0] += 1
            for b in range(k + 1, B):
                C.append([-ehk, eqs[b]])
            C.append([-eqs[b] for b in range(k + 1, B)] + [ehk])
            ehs.append(ehk)
        Ls = []
        for k in range(B):
            lk = nv[0] + 1
            nv[0] += 1
            C.append([-lk, ehs[k]])
            C.append([-lk, -pu[k]])
            C.append([-lk, pw[k]])
            Ls.append(lk)
        C.append([-var[(u, w)]] + Ls)

    # Extract cover helper
    def extract_cover(m):
        sel = [a for a in arcs if m.get(var[a])]
        succ = {a[0]: a[1] for a in sel}
        pred = {a[1]: a[0] for a in sel}
        starts = [v for v in vs if v not in pred]
        runs = []
        for s in starts:
            run = []
            cur = s
            while cur is not None:
                run.append(cur)
                cur = succ.get(cur)
            runs.append(run)
        if sum(len(r) for r in runs) != len(vs):
            return None
        fp = tuple(sorted((len(r), r[0], r[-1]) for r in runs))
        return fp, runs

    mh = sorted(set(h for v in vs for h in G[v] if 20 <= deg[h] < 100))
    covers = []
    seen_fp = set()

    # Strategy 1: Base unsteered solve
    for seed in seeds:
        sat, m = solve_cnf(nv[0], C, timeout=timeout, seed=seed)
        if sat is True:
            res = extract_cover(m)
            if res and res[0] not in seen_fp:
                seen_fp.add(res[0])
                covers.append(res)

    # Strategy 2: Targeted steering for each adjacent M-hub
    for h in mh:
        Eh = [v for v in vs if h in G[v]]
        steer_clause = [eof[v] for v in Eh]
        for seed in seeds:
            sat, m = solve_cnf(nv[0], C + [steer_clause], timeout=timeout, seed=seed)
            if sat is True:
                res = extract_cover(m)
                if res and res[0] not in seen_fp:
                    seen_fp.add(res[0])
                    covers.append(res)

    # Strategy 3: Joint steering for all adjacent M-hubs
    if len(mh) > 1:
        all_m = [[eof[v] for v in vs if h in G[v]] for h in mh]
        for seed in seeds[:2]:
            sat, m = solve_cnf(nv[0], C + all_m, timeout=timeout, seed=seed)
            if sat is True:
                res = extract_cover(m)
                if res and res[0] not in seen_fp:
                    seen_fp.add(res[0])
                    covers.append(res)

    return covers


def _strip_worker(args):
    """Multiprocessing worker function."""
    si, hh, vs, G, deg, K, seeds, timeout = args
    covers = solve_strip_targeted(si, hh, vs, G, deg, K=K, seeds=seeds, timeout=timeout)
    return si, covers


def generate_steered_covers(
    graph_path='/home/ubuntu/HCP/FHCPCS-col/graph950.col',
    hubcut=20,
    K=4,
    seeds=(7, 11, 13, 17),
    num_workers=16,
    timeout=30.0,
    output_path=None,
):
    """Generate targeted steered path covers across all strips in parallel.

    Args:
        graph_path: path to .col graph file
        hubcut: minimum degree for hubs (default 20)
        K: max paths per strip cover (default 4)
        seeds: seeds to use for randomized solving
        num_workers: multiprocessing worker count
        timeout: per-solve timeout in seconds
        output_path: optional path to save JSON dumped covers

    Returns:
        cover_sets: list of covers for each strip (indexed by strip_id 0..N-1)
    """
    G = load_graph(graph_path)
    deg = {v: len(a) for v, a in G.items()}
    strip_list, bulk_set, hub_set, big_hub = decompose_graph(G, hubcut=hubcut)

    tasks = [
        (si, hh, vs, G, deg, K, seeds, timeout)
        for si, (hh, vs) in enumerate(strip_list)
    ]

    t0 = time.time()
    with Pool(num_workers) as pool:
        results = pool.map(_strip_worker, tasks)

    results.sort(key=lambda x: x[0])
    cover_sets = [c for (_, c) in results]

    elapsed = time.time() - t0
    cnt = [len(c) for c in cover_sets]
    print(
        f'generate_steered_covers: {len(strip_list)} strips in {elapsed:.1f}s; '
        f'covers/strip: min {min(cnt)} max {max(cnt)} avg {sum(cnt)/len(cnt):.1f}',
        flush=True,
    )

    if output_path:
        os.makedirs(os.path.dirname(os.path.abspath(output_path)), exist_ok=True)
        with open(output_path, 'w') as f:
            json.dump(cover_sets, f)
        print(f'Saved cover sets to {output_path}', flush=True)

    return cover_sets


if __name__ == '__main__':
    graph_file = sys.argv[1] if len(sys.argv) > 1 else '/home/ubuntu/HCP/FHCPCS-col/graph950.col'
    out_file = sys.argv[2] if len(sys.argv) > 2 else '/home/ubuntu/HCP/scratch/graph950/covers_multi.json'
    generate_steered_covers(graph_file, output_path=out_file)
