import time
from pysat.solvers import Cadical195

# Let's test solving the 27-cycle checkpoint CNF with different CaDiCaL configurations
import collections

G = collections.defaultdict(set)
with open('FHCPCS-col/graph868.col') as f:
    for line in f:
        if line.startswith('e '):
            parts = line.split()
            u, v = int(parts[1]), int(parts[2])
            G[u].add(v); G[v].add(u)

deg2 = sorted([u for u, d in G.items() if len(d) == 2])
v_partner = {}
node_to_block = {}
for b_id, v in enumerate(deg2):
    u, w = list(G[v])
    v_partner[u] = w; v_partner[w] = u
    node_to_block[u] = b_id; node_to_block[w] = b_id

color = {}
color[1] = 0
q = [1]
while q:
    curr = q.pop()
    c = color.get(curr, 0)
    color[curr] = c
    vp = v_partner[curr]
    if vp not in color:
        color[vp] = 1 - c; q.append(vp)
    for nxt in G[curr]:
        if nxt != vp and nxt in node_to_block and nxt not in color:
            color[nxt] = 1 - c; q.append(nxt)

dir_adj = collections.defaultdict(list)
in_adj = collections.defaultdict(list)
var_map = {}
rev_map = {}
v_cnt = 0

for b_id, v in enumerate(deg2):
    u, w = list(G[v])
    out_v = u if color[u] == 1 else w
    for nxt in G[out_v]:
        if nxt != v:
            b2 = node_to_block[nxt]
            dir_adj[b_id].append(b2)
            in_adj[b2].append(b_id)
            v_cnt += 1
            var_map[(b_id, b2)] = v_cnt
            rev_map[v_cnt] = (b_id, b2)

base_clauses = []
# Degree 1
for u in range(len(deg2)):
    lits = [var_map[(u, v)] for v in dir_adj[u]]
    base_clauses.append(lits)
    for i in range(len(lits)):
        for j in range(i + 1, len(lits)):
            base_clauses.append([-lits[i], -lits[j]])

for v in range(len(deg2)):
    lits = [var_map[(u, v)] for u in in_adj[v]]
    base_clauses.append(lits)
    for i in range(len(lits)):
        for j in range(i + 1, len(lits)):
            base_clauses.append([-lits[i], -lits[j]])

# 2-cycle mutexes
for (u, v), lit in var_map.items():
    if (v, u) in var_map and u < v:
        base_clauses.append([-lit, -var_map[(v, u)]])

# Static 3-cycles
for u in range(len(deg2)):
    for v in dir_adj[u]:
        for w in dir_adj[v]:
            if w != u and u in dir_adj[w]:
                if u < v and u < w:
                    base_clauses.append([-var_map[(u, v)], -var_map[(v, w)], -var_map[(w, u)]])

# Read 27-cycle checkpoint
edges = []
with open('scratch/graph868/checkpoint_best.txt') as f:
    for line in f:
        line = line.strip()
        if line:
            u, v = map(int, line.split())
            edges.append((u, v))

u_ext = {}
for u, v in edges: u_ext[u] = v; u_ext[v] = u
block_next = {}
for b_id, v in enumerate(deg2):
    u, w = list(G[v])
    out_v = u if color[u] == 1 else w
    block_next[b_id] = node_to_block[u_ext[out_v]]

visited = set(); cycles = []
for b in range(len(deg2)):
    if b not in visited:
        c = []
        curr = b
        while curr not in visited:
            visited.add(curr); c.append(curr); curr = block_next[curr]
        cycles.append(c)

cycles.sort(key=len, reverse=True)
print(f'Base clauses: {len(base_clauses)}, Checkpoint cycles: {len(cycles)}')

# Cut the 23 eight-block cycles!
subtour_cuts = []
for c in cycles[4:]:
    lits = [-var_map[(c[i], c[(i+1)%len(c)])] for i in range(len(c))]
    subtour_cuts.append(lits)

print(f'Subtour cuts to add: {len(subtour_cuts)}')

# Test 1: Default Cadical195
t0 = time.time()
s_def = Cadical195(bootstrap_with=base_clauses + subtour_cuts)
res1 = s_def.solve()
t_def = time.time() - t0
print(f'Default CaDiCaL solved in {t_def:.3f}s, result: {res1}')

if res1:
    m = set(s_def.get_model())
    next_map = {}
    for (u, v), lit in var_map.items():
        if lit in m: next_map[u] = v
    vis = set(); new_cycs = []
    for b in range(len(deg2)):
        if b not in vis:
            c = []; curr = b
            while curr not in vis:
                vis.add(curr); c.append(curr); curr = next_map[curr]
            new_cycs.append(c)
    new_cycs.sort(key=len, reverse=True)
    print(f'New solution has {len(new_cycs)} cycles! Top 5 lengths: {[len(c) for c in new_cycs[:5]]}')
