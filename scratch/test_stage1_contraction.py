import collections
import sys
import os

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))
from scratch.solve_graph788_dp_bitmask import load_graph, setup_stage1

def test_contraction():
    G = collections.defaultdict(set)
    with open('FHCPCS-col/graph788.col', 'r') as f:
        for line in f:
            if line.startswith('e '):
                p = line.split()
                u, v = int(p[1]), int(p[2])
                G[u].add(v); G[v].add(u)
    assert len(G) == 4620
    deg2 = [u for u in G if len(G[u]) == 2]
    assert len(deg2) == 1540
    print("Stage 1 test PASSED: 1540 degree-2 vertices verified.")

def test_setup_stage1():
    G, d = load_graph('FHCPCS-col/graph788.col')
    b, n2b, e2v, v2e, vcnt, adj, badj, m2, m3 = setup_stage1(G, d)
    assert len(b) == 1540, f"Expected 1540 blocks, got {len(b)}"
    assert len(m2) == 840, f"Expected 840 2-cycle mutexes, got {len(m2)}"
    assert len(m3) == 49, f"Expected 49 3-cycle mutexes, got {len(m3)}"
    print("Stage 1 Implementation Verified: 1540 blocks, 840 2-mutex, 49 3-mutex.")

if __name__ == '__main__':
    test_contraction()
    test_setup_stage1()
