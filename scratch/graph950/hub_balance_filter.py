#!/usr/bin/env python3
"""Phase 1.5: Global Hub Reachability & Balance Filter for graph950.

Validates that all 310 hubs across all tiers (S, B, M) have sufficient candidate
connections (endpoints from strip path covers + direct hub-hub edges) for the
Macro Selector CNF. If any hub is undercovered (< 2 candidate options), optionally
triggers targeted fallback steering on the relevant strips.
"""

import collections
import json
import os
import sys


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


def classify_hubs(G, hubcut=20, s_cut=500, b_cut=100):
    """Classify hubs in graph into S-hubs, B-hubs, and M-hubs.

    Args:
        G: graph adjacency dict
        hubcut: minimum degree for hub (default 20)
        s_cut: minimum degree for Super-hubs (default 500)
        b_cut: minimum degree for Big-hubs (default 100)

    Returns:
        s_hubs: set of Super-hubs (deg >= 500)
        b_hubs: set of Big-hubs (100 <= deg < 500)
        m_hubs: set of Medium-hubs (20 <= deg < 100)
        hub_set: set of all hubs (deg >= 20)
    """
    deg = {v: len(a) for v, a in G.items()}
    hub_set = set(v for v, d in deg.items() if d >= hubcut)
    s_hubs = set(v for v in hub_set if deg[v] >= s_cut)
    b_hubs = set(v for v in hub_set if b_cut <= deg[v] < s_cut)
    m_hubs = set(v for v in hub_set if hubcut <= deg[v] < b_cut)
    return s_hubs, b_hubs, m_hubs, hub_set


def extract_cover_endpoints(cover_sets):
    """Extract bulk endpoint vertices and slot counts from strip cover sets.

    Args:
        cover_sets: list of cover lists per strip, where each cover is (fp, runs)

    Returns:
        all_endpoints: set of all bulk vertices appearing as an endpoint in any cover
        strip_endpoints: dict mapping strip_idx -> set of endpoint vertices
        endpoint_slot_counts: dict mapping vertex -> total slot occurrences across covers
    """
    all_endpoints = set()
    strip_endpoints = collections.defaultdict(set)
    endpoint_slot_counts = collections.defaultdict(int)

    for si, covers in enumerate(cover_sets):
        for cover in covers:
            # Each cover is either (fp, runs) tuple or [fp, runs] list
            if isinstance(cover, (tuple, list)) and len(cover) == 2:
                runs = cover[1]
            else:
                continue
            for r in runs:
                if not r:
                    continue
                v1, v2 = r[0], r[-1]
                all_endpoints.add(v1)
                all_endpoints.add(v2)
                strip_endpoints[si].add(v1)
                strip_endpoints[si].add(v2)
                endpoint_slot_counts[v1] += 1
                endpoint_slot_counts[v2] += 1

    return all_endpoints, strip_endpoints, endpoint_slot_counts


def check_hub_candidate_coverage(
    cover_sets,
    G,
    hub_set=None,
    include_direct=True,
    min_required=2,
    hubcut=20,
    s_cut=500,
    b_cut=100,
):
    """Check candidate endpoints and direct hub-hub edges for all hubs.

    Args:
        cover_sets: list of covers per strip
        G: graph adjacency dict
        hub_set: optional set of hub vertices; if None, computed from G
        include_direct: whether to count direct hub-hub edges as candidates
        min_required: minimum candidate count per hub (default 2)
        hubcut: degree cutoff for hubs
        s_cut: degree cutoff for S-hubs
        b_cut: degree cutoff for B-hubs

    Returns:
        all_ok: bool, True if all hubs have >= min_required candidate connections
        stats: dict containing comprehensive balance metrics per tier and overall
    """
    deg = {v: len(a) for v, a in G.items()}
    if hub_set is None:
        s_hubs, b_hubs, m_hubs, hub_set = classify_hubs(G, hubcut=hubcut, s_cut=s_cut, b_cut=b_cut)
    else:
        s_hubs = set(v for v in hub_set if deg[v] >= s_cut)
        b_hubs = set(v for v in hub_set if b_cut <= deg[v] < s_cut)
        m_hubs = set(v for v in hub_set if hubcut <= deg[v] < b_cut)

    all_endpoints, strip_endpoints, endpoint_slot_counts = extract_cover_endpoints(cover_sets)

    direct_counts = {}
    endpoint_counts = {}
    total_candidates = {}
    slot_counts = {}
    undercovered_hubs = []

    for h in sorted(hub_set):
        direct = sum(1 for w in G[h] if w in hub_set and w != h)
        eps = sum(1 for v in G[h] if v in all_endpoints)
        slots = sum(endpoint_slot_counts[v] for v in G[h] if v not in hub_set)
        cand = (direct if include_direct else 0) + eps

        direct_counts[h] = direct
        endpoint_counts[h] = eps
        slot_counts[h] = slots
        total_candidates[h] = cand

        if cand < min_required:
            undercovered_hubs.append(h)

    def _calc_summary(vals):
        if not vals:
            return {'min': 0, 'max': 0, 'avg': 0.0}
        return {
            'min': min(vals),
            'max': max(vals),
            'avg': round(sum(vals) / len(vals), 2),
        }

    def _tier_summary(tier_set):
        t_cands = [total_candidates[h] for h in tier_set]
        t_directs = [direct_counts[h] for h in tier_set]
        t_eps = [endpoint_counts[h] for h in tier_set]
        t_slots = [slot_counts[h] for h in tier_set]
        t_und = [h for h in tier_set if total_candidates[h] < min_required]
        return {
            'count': len(tier_set),
            'min_candidates': min(t_cands) if t_cands else 0,
            'max_candidates': max(t_cands) if t_cands else 0,
            'avg_candidates': round(sum(t_cands) / len(t_cands), 2) if t_cands else 0.0,
            'min_direct': min(t_directs) if t_directs else 0,
            'max_direct': max(t_directs) if t_directs else 0,
            'avg_direct': round(sum(t_directs) / len(t_directs), 2) if t_directs else 0.0,
            'min_endpoints': min(t_eps) if t_eps else 0,
            'max_endpoints': max(t_eps) if t_eps else 0,
            'avg_endpoints': round(sum(t_eps) / len(t_eps), 2) if t_eps else 0.0,
            'min_slots': min(t_slots) if t_slots else 0,
            'max_slots': max(t_slots) if t_slots else 0,
            'avg_slots': round(sum(t_slots) / len(t_slots), 2) if t_slots else 0.0,
            'undercovered_count': len(t_und),
            'undercovered_hubs': t_und,
        }

    all_cands = list(total_candidates.values())
    stats = {
        'num_hubs': len(hub_set),
        'min_candidates': min(all_cands) if all_cands else 0,
        'max_candidates': max(all_cands) if all_cands else 0,
        'avg_candidates': round(sum(all_cands) / len(all_cands), 2) if all_cands else 0.0,
        'num_undercovered': len(undercovered_hubs),
        'undercovered_hubs': undercovered_hubs,
        'direct_stats': _calc_summary(list(direct_counts.values())),
        'endpoint_stats': _calc_summary(list(endpoint_counts.values())),
        'slot_stats': _calc_summary(list(slot_counts.values())),
        'tier_stats': {
            'S': _tier_summary(s_hubs),
            'B': _tier_summary(b_hubs),
            'M': _tier_summary(m_hubs),
        },
        'hub_candidates': total_candidates,
        'hub_directs': direct_counts,
        'hub_endpoints': endpoint_counts,
    }

    all_ok = len(undercovered_hubs) == 0
    return all_ok, stats


def verify_global_balance(cover_sets, G, hub_set=None, min_candidates=2, include_direct=True):
    """Verify that all hubs satisfy global reachability and balance constraints.

    Interface conforming to Phase 1.5 pipeline specification.

    Args:
        cover_sets: list of cover lists per strip
        G: graph adjacency dict
        hub_set: optional set of hub vertices
        min_candidates: minimum candidates per hub required for balance (default 2)
        include_direct: whether direct hub-hub edges are included

    Returns:
        (is_balanced: bool, stats: dict)
    """
    return check_hub_candidate_coverage(
        cover_sets=cover_sets,
        G=G,
        hub_set=hub_set,
        include_direct=include_direct,
        min_required=min_candidates,
    )


def get_undercovered_hubs(cover_sets, G, hub_set=None, min_required=2, include_direct=True):
    """Return list of undercovered hub vertex IDs."""
    _, stats = check_hub_candidate_coverage(
        cover_sets=cover_sets,
        G=G,
        hub_set=hub_set,
        include_direct=include_direct,
        min_required=min_required,
    )
    return stats['undercovered_hubs']


def fallback_steer_undercovered_hubs(
    cover_sets,
    G,
    undercovered_hubs,
    strip_list,
    hubcut=20,
    K=4,
    seeds=(7, 11, 13, 17, 19, 23),
    timeout=30.0,
):
    """Trigger targeted fallback steering for undercovered hubs.

    Args:
        cover_sets: mutable list of covers per strip
        G: graph adjacency dict
        undercovered_hubs: iterable of undercovered hub vertex IDs
        strip_list: list of (hh, vs) tuples
        hubcut: degree cutoff for hubs
        K: max paths per strip cover
        seeds: random seeds for solving
        timeout: per-solve timeout in seconds

    Returns:
        updated cover_sets
    """
    if not undercovered_hubs:
        return cover_sets

    try:
        from steered_p1_generator import solve_strip_targeted
    except ImportError:
        sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
        from steered_p1_generator import solve_strip_targeted

    deg = {v: len(a) for v, a in G.items()}
    und_set = set(undercovered_hubs)

    for si, (hh, vs) in enumerate(strip_list):
        # Check if strip contains bulk vertices adjacent to any undercovered hub
        adj_und = [h for h in und_set if any(v in G[h] for v in vs)]
        if not adj_und:
            continue

        print(f"Fallback steering on strip {si} for undercovered hubs: {adj_und}", flush=True)
        new_covers = solve_strip_targeted(
            si=si,
            hh=hh,
            vs=vs,
            G=G,
            deg=deg,
            K=K,
            seeds=seeds,
            timeout=timeout,
        )
        # Deduplicate and append new covers
        existing_fps = set(c[0] if isinstance(c, (tuple, list)) else None for c in cover_sets[si])
        for cov in new_covers:
            fp = cov[0]
            if fp not in existing_fps:
                cover_sets[si].append(cov)
                existing_fps.add(fp)

    return cover_sets


def print_balance_report(stats):
    """Print formatted summary report of hub balance statistics."""
    print("=" * 60)
    print("GLOBAL HUB REACHABILITY & BALANCE REPORT (Phase 1.5)")
    print("=" * 60)
    print(f"Total Hubs:          {stats['num_hubs']}")
    print(f"Candidate Range:     [{stats['min_candidates']}, {stats['max_candidates']}] (avg: {stats['avg_candidates']})")
    print(f"Direct Edges Range:  [{stats['direct_stats']['min']}, {stats['direct_stats']['max']}] (avg: {stats['direct_stats']['avg']})")
    print(f"Bulk Endpoint Range: [{stats['endpoint_stats']['min']}, {stats['endpoint_stats']['max']}] (avg: {stats['endpoint_stats']['avg']})")
    print(f"Undercovered Hubs:   {stats['num_undercovered']}")
    print("-" * 60)
    print("Tier Breakdown:")
    for tier, name in [('S', 'Super-hubs (deg >= 500)'), ('B', 'Big-hubs (100 <= deg < 500)'), ('M', 'Medium-hubs (20 <= deg < 100)')]:
        ts = stats['tier_stats'][tier]
        print(f"  {name}:")
        print(f"    Count:        {ts['count']}")
        print(f"    Candidates:   min={ts['min_candidates']}, max={ts['max_candidates']}, avg={ts['avg_candidates']}")
        print(f"    Directs:      min={ts['min_direct']}, max={ts['max_direct']}, avg={ts['avg_direct']}")
        print(f"    Endpoints:    min={ts['min_endpoints']}, max={ts['max_endpoints']}, avg={ts['avg_endpoints']}")
        print(f"    Undercovered: {ts['undercovered_count']}")
    print("=" * 60)
    if stats['num_undercovered'] == 0:
        print("RESULT: ALL HUBS BALANCED AND REACHABLE (>= 2 candidates each).")
    else:
        print(f"RESULT: FAILED ({stats['num_undercovered']} undercovered hubs: {stats['undercovered_hubs']}).")
    print("=" * 60)


if __name__ == '__main__':
    graph_file = sys.argv[1] if len(sys.argv) > 1 else '/home/ubuntu/HCP/FHCPCS-col/graph950.col'
    covers_file = sys.argv[2] if len(sys.argv) > 2 else '/home/ubuntu/HCP/scratch/graph950/covers_multi.json'

    G_ = load_graph(graph_file)
    with open(covers_file, 'r') as f_:
        cover_sets_ = json.load(f_)

    all_ok_, stats_ = check_hub_candidate_coverage(cover_sets_, G_)
    print_balance_report(stats_)

    sys.exit(0 if all_ok_ else 1)
