# scratch/graph950/test_steered_p1.py
import os
import sys
import collections
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from steered_p1_generator import solve_strip_targeted, generate_steered_covers

def test_single_strip_steering():
    # Test on strip 0 of graph950
    G = collections.defaultdict(set)
    for l in open('/home/ubuntu/HCP/FHCPCS-col/graph950.col'):
        t = l.split()
        if t and t[0] == 'e':
            u, v = int(t[1]), int(t[2])
            G[u].add(v)
            G[v].add(u)
    
    # Strip 0 bulk vertices
    deg = {v: len(a) for v, a in G.items()}
    verts = sorted(G.keys())
    bulk_set = set(v for v in verts if deg[v] < 20)
    big_hub = {v for v in verts if deg[v] >= 100}
    strips = collections.defaultdict(list)
    for v in bulk_set:
        hh = tuple(sorted(u for u in G[v] if u in big_hub))
        strips[hh].append(v)
    
    strip_list = list(strips.items())
    hh, vs = strip_list[0]
    m_hubs = [h for v in vs for h in G[v] if 20 <= deg[h] < 100]
    
    covers = solve_strip_targeted(0, hh, vs, G, deg, K=4, seeds=[7, 11, 13])
    assert len(covers) >= 1, "Must generate at least one valid cover"
    for fp, runs in covers:
        assert sum(len(r) for r in runs) == len(vs), "Cover must span all strip vertices"
        for r in runs:
            assert len(r) >= 1
            for idx in range(len(r) - 1):
                assert r[idx+1] in G[r[idx]], f"Edge ({r[idx]}, {r[idx+1]}) must exist in G"
    print("test_single_strip_steering PASSED!")

def test_targeted_hub_coverage():
    # Verify that targeted steering produces endpoint candidates for adjacent M-hubs
    G = collections.defaultdict(set)
    for l in open('/home/ubuntu/HCP/FHCPCS-col/graph950.col'):
        t = l.split()
        if t and t[0] == 'e':
            u, v = int(t[1]), int(t[2])
            G[u].add(v)
            G[v].add(u)
    deg = {v: len(a) for v, a in G.items()}
    verts = sorted(G.keys())
    bulk_set = set(v for v in verts if deg[v] < 20)
    big_hub = {v for v in verts if deg[v] >= 100}
    strips = collections.defaultdict(list)
    for v in bulk_set:
        hh = tuple(sorted(u for u in G[v] if u in big_hub))
        strips[hh].append(v)
    
    hh, vs = list(strips.items())[0]
    mh = sorted(set(h for v in vs for h in G[v] if 20 <= deg[h] < 100))
    
    covers = solve_strip_targeted(0, hh, vs, G, deg, K=4, seeds=[7, 11])
    all_endpoints = set()
    for fp, runs in covers:
        for r in runs:
            all_endpoints.add(r[0])
            all_endpoints.add(r[-1])
    
    for h in mh:
        I_h = set(v for v in vs if h in G[v])
        covered = bool(I_h & all_endpoints)
        assert covered, f"M-hub {h} must have at least one candidate endpoint in generated covers"
    print("test_targeted_hub_coverage PASSED!")

if __name__ == '__main__':
    test_single_strip_steering()
    test_targeted_hub_coverage()
