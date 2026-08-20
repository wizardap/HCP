#!/usr/bin/env python3
# INDEPENDENT verifier: raw parse of .col + tour file, no shared code with solver
G2 = set()
n = 0
for l in open('/home/ubuntu/HCP/FHCPCS-col/graph950.col'):
    t = l.split()
    if t and t[0] == 'p':
        n = int(t[1])
    elif t and t[0] == 'e':
        G2.add((int(t[1]), int(t[2])))
        G2.add((int(t[2]), int(t[1])))
print('graph:', n, 'vertices,', len(G2)//2, 'edges')
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
t1 = load_tour('/tmp/opencode/found_tour.hcp')
t2 = load_tour('/tmp/opencode/FHCPCS_sols/graph950.hcp.tou')
def verify(tour, name):
    m = len(tour)
    ok = all(1 <= x <= n for x in tour)
    ok &= len(set(tour)) == m
    ok &= m == n
    bad = []
    for i in range(m):
        a, b = tour[i], tour[(i+1) % m]
        if (a, b) not in G2:
            bad.append((a, b))
            if len(bad) > 3: break
    verdict = 'VALID HAMILTONIAN CYCLE' if ok and not bad else 'INVALID'
    print(f'{name}: len {m}, distinct {len(set(tour))}, bad_edges {len(bad)} -> {verdict}')
    return tour
v1 = verify(t1, 'FOUND tour  ')
v2 = verify(t2, 'OFFICIAL tour')
# compare as cyclic rotations
s1 = v1 + v1
i0 = s1.index(v2[0])
rot = s1[i0:i0+len(v2)]
same_fwd = rot == v2
s2 = list(reversed(v1)) + list(reversed(v1))
i1 = s2.index(v2[0])
rot2 = s2[i1:i1+len(v2)]
same_rev = rot2 == v2
print('identical to official tour (fwd):', same_fwd)
print('identical to official tour (rev):', same_rev)
if not same_fwd and not same_rev:
    print('FOUND is a DIFFERENT Hamiltonian cycle than the official tour!')