import re
from collections import defaultdict

# Read raw graph
raw_adj = defaultdict(set)
with open("FHCPCS-col/graph868.col") as f:
    for line in f:
        p = line.strip().split()
        if len(p) >= 3 and p[0] == "e":
            u, v = int(p[1]), int(p[2])
            raw_adj[u].add(v)
            raw_adj[v].add(u)

# Contract degree 2 vertices
deg2 = {v for v, nbrs in raw_adj.items() if len(nbrs) == 2}
chain_map = {}
visited_deg2 = set()
contracted_adj = defaultdict(set)

for v in deg2:
    if v in visited_deg2:
        continue
    nbrs = list(raw_adj[v])
    
    # Trace left towards non-deg2
    prev, curr = v, nbrs[0]
    path_left = []
    while curr in deg2:
        visited_deg2.add(curr)
        path_left.append(curr)
        c_nbrs = list(raw_adj[curr])
        nxt = c_nbrs[1] if c_nbrs[0] == prev else c_nbrs[0]
        prev, curr = curr, nxt
    end_u = curr

    # Trace right towards non-deg2
    prev, curr = v, nbrs[1]
    path_right = []
    while curr in deg2:
        visited_deg2.add(curr)
        path_right.append(curr)
        c_nbrs = list(raw_adj[curr])
        nxt = c_nbrs[1] if c_nbrs[0] == prev else c_nbrs[0]
        prev, curr = curr, nxt
    end_w = curr

    visited_deg2.add(v)
    path_left.reverse()
    full_path = path_left + [v] + path_right
    if end_u < end_w:
        chain_map[(end_u, end_w)] = full_path
        chain_map[(end_w, end_u)] = list(reversed(full_path))
    else:
        chain_map[(end_w, end_u)] = list(reversed(full_path))
        chain_map[(end_u, end_w)] = full_path

# Contracted adj
for u, nbrs in raw_adj.items():
    if u not in deg2:
        for v in nbrs:
            if v not in deg2:
                contracted_adj[u].add(v)

for (u, w) in chain_map:
    contracted_adj[u].add(w)
    contracted_adj[w].add(u)

print(f"Contracted graph vertices: {len(contracted_adj)}, chain_map pairs: {len(chain_map)}")

# Read snapshot edges
snap_edges = set()
with open("scratch/graph868_giant_1686_full_edges.txt") as f:
    for line in f:
        p = line.strip().split()
        if len(p) == 2:
            snap_edges.add(tuple(sorted([int(p[0]), int(p[1])])))

print(f"Snapshot edges: {len(snap_edges)}")

# Extract cycles in contracted graph
cycle_adj = defaultdict(set)
for u, v in snap_edges:
    cycle_adj[u].add(v)
    cycle_adj[v].add(u)

visited = set()
cycles = []
for start in list(cycle_adj.keys()):
    if start not in visited:
        c = []
        curr = start
        prev = None
        while True:
            visited.add(curr)
            c.append(curr)
            nbrs = [n for n in cycle_adj[curr] if n != prev]
            if not nbrs:
                break
            nxt = nbrs[0]
            if nxt == start:
                break
            prev = curr
            curr = nxt
        cycles.append(c)

cycles.sort(key=len)
print(f"Total cycles in contracted graph: {len(cycles)}")
print("Cycle lengths:", [len(c) for c in cycles])

giant = cycles[-1]
giant_set = set(giant)

for i, c in enumerate(cycles[:-1]):
    c_set = set(c)
    cross_to_giant = 0
    cross_to_other = 0
    for u in c:
        for v in contracted_adj[u]:
            if (min(u, v), max(u, v)) not in snap_edges:
                if v in giant_set:
                    cross_to_giant += 1
                elif v not in c_set:
                    cross_to_other += 1
    print(f"Cycle {i:2d} (len {len(c):2d}): cross edges to giant = {cross_to_giant:2d}, to other small cycles = {cross_to_other:2d}")
