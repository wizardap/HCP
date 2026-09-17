import collections, time
from pysat.solvers import Cadical195
from pysat.card import CardEnc, EncType

col_path = "FHCPCS-col/graph717.col"
G = collections.defaultdict(set)
with open(col_path) as f:
    for line in f:
        if line.startswith("e "):
            p = line.split()
            u, v = int(p[1]), int(p[2])
            G[u].add(v); G[v].add(u)

cut_nodes = {255, 540, 577, 702, 773, 1016, 1177, 1213, 1389, 1955, 2467, 2609, 2677, 2681, 3358, 3986}
rem_16 = set(G.keys()) - cut_nodes

# Extract Comp 0
vis = set()
for x in rem_16:
    if x not in vis:
        c = []
        q = [x]
        vis.add(x)
        for y in q:
            c.append(y)
            for nbr in G[y]:
                if nbr in rem_16 and nbr not in vis:
                    vis.add(nbr); q.append(nbr)
        if len(c) > 200:
            comp_0 = set(c)

ports = {255, 1955, 2609, 3358}
V_c0 = comp_0 | ports

adj_c0 = collections.defaultdict(set)
for u in V_c0:
    for v in G[u]:
        if v in V_c0:
            adj_c0[u].add(v)

# Add virtual edges
adj_c0[255].add(1955); adj_c0[1955].add(255)
adj_c0[3358].add(2609); adj_c0[2609].add(3358)

# Degree-2 contraction
rem = set(V_c0)
edge_chains = {}
while True:
    d2 = [u for u in rem if u not in ports and len(adj_c0[u]) == 2]
    if not d2:
        break
    v = d2[0]
    u, w = list(adj_c0[v])
    adj_c0[u].remove(v); adj_c0[w].remove(v)
    del adj_c0[v]; rem.remove(v)
    e_uv = tuple(sorted([u, v]))
    e_vw = tuple(sorted([v, w]))
    chain_uv = edge_chains.pop(e_uv, [u, v])
    chain_vw = edge_chains.pop(e_vw, [v, w])
    if chain_uv[-1] != v: chain_uv = list(reversed(chain_uv))
    if chain_vw[0] != v: chain_vw = list(reversed(chain_vw))
    merged_chain = chain_uv[:-1] + chain_vw
    e_uw = tuple(sorted([u, w]))
    adj_c0[u].add(w); adj_c0[w].add(u)
    edge_chains[e_uw] = merged_chain

contracted_edges = set(edge_chains.keys())
edges = set()
for u in rem:
    for v in adj_c0[u]:
        if u < v:
            edges.add((u, v))
edge_list = sorted(list(edges))
edge_to_var = {e: i + 1 for i, e in enumerate(edge_list)}
inc_edges = collections.defaultdict(list)
for e in edge_list:
    inc_edges[e[0]].append(edge_to_var[e])
    inc_edges[e[1]].append(edge_to_var[e])

solver = Cadical195()
top = len(edge_list) + 1

for u in rem:
    lits = inc_edges[u]
    clauses = CardEnc.equals(lits=lits, bound=2, top_id=top, encoding=EncType.cardnetwrk)
    for cl in clauses:
        solver.add_clause(cl)
        for lit in cl:
            top = max(top, abs(lit) + 1)

virt1 = tuple(sorted([255, 1955]))
virt2 = tuple(sorted([3358, 2609]))
solver.add_clause([edge_to_var[virt1]])
solver.add_clause([edge_to_var[virt2]])

for ce in contracted_edges:
    solver.add_clause([edge_to_var[ce]])

triangles = []
rem_list = sorted(list(rem))
for u in rem_list:
    for v in adj_c0[u]:
        if v > u:
            for w in adj_c0[v]:
                if w > v and w in adj_c0[u]:
                    triangles.append((u, v, w))
for u, v, w in triangles:
    solver.add_clause([-edge_to_var[tuple(sorted([u, v]))], -edge_to_var[tuple(sorted([v, w]))], -edge_to_var[tuple(sorted([w, u]))]])

# Solve Round 0 SAT
assert solver.solve()
model = set(solver.get_model())
active = [e for e in edge_list if edge_to_var[e] in model]
adj = collections.defaultdict(list)
for u, v in active:
    adj[u].append(v); adj[v].append(u)

visited = set()
cycles = []
for u in rem:
    if u not in visited:
        cyc = []
        curr, prev = u, None
        while curr not in visited:
            visited.add(curr); cyc.append(curr)
            nbrs = adj[curr]
            nxt = nbrs[0] if nbrs[0] != prev else nbrs[1]
            prev, curr = curr, nxt
        cycles.append(cyc)

print(f"Round 0 SAT model has {len(cycles)} cycles. Max: {max(len(c) for c in cycles)}, Min: {min(len(c) for c in cycles)}")

# Let us test greedy 2-opt cycle absorption between cycles!
def try_2opt_merge(c1, c2, adj_graph, protected_edges):
    # Try to find u1-u2 in c1 and v1-v2 in c2 such that (u1, v1) and (u2, v2) exist in adj_graph
    n1, n2 = len(c1), len(c2)
    for i in range(n1):
        u1 = c1[i]
        u2 = c1[(i + 1) % n1]
        e_u = tuple(sorted([u1, u2]))
        if e_u in protected_edges:
            continue
        for j in range(n2):
            v1 = c2[j]
            v2 = c2[(j + 1) % n2]
            e_v = tuple(sorted([v1, v2]))
            if e_v in protected_edges:
                continue
            # Check parallel 2-opt: (u1, v1) and (u2, v2)
            if v1 in adj_graph[u1] and v2 in adj_graph[u2]:
                # Merge: c1[..u1] + c2[v1..reversed..v2] + c1[u2..]
                # Reconstruct merged cycle
                # Segment 1: c1 from u2 to u1
                seg1 = c1[i+1:] + c1[:i+1] # starts at u2, ends at u1
                # Segment 2: c2 from v1 to v2 reversed
                seg2 = c2[j::-1] + c2[:j:-1] # starts at v1, ends at v2
                merged = seg1 + seg2
                return merged
            # Check cross 2-opt: (u1, v2) and (u2, v1)
            if v2 in adj_graph[u1] and v1 in adj_graph[u2]:
                seg1 = c1[i+1:] + c1[:i+1] # starts at u2, ends at u1
                seg2 = c2[j+1:] + c2[:j+1] # starts at v2, ends at v1 reversed
                merged = seg1 + seg2
                return merged
    return None

# Protected edges: virtual edges and contracted edges cannot be cut!
protected = contracted_edges | {virt1, virt2}
print(f"Number of protected edges: {len(protected)}")

# Sort cycles by length, largest first
cycles.sort(key=len, reverse=True)
current_cycles = list(cycles)

changed = True
while changed:
    changed = False
    new_cycles = []
    # Try to merge smaller cycles into the giant cycle
    giant = current_cycles[0]
    rest = current_cycles[1:]
    unmerged = []
    for c in rest:
        m = try_2opt_merge(giant, c, adj_c0, protected)
        if m is not None:
            giant = m
            changed = True
        else:
            unmerged.append(c)
    current_cycles = [giant] + unmerged
    if changed:
        print(f"2-opt absorption step: cycles reduced to {len(current_cycles)}. Giant size: {len(giant)}")

print(f"Final cycles after 2-opt absorption: {len(current_cycles)}. Sizes: {[len(c) for c in current_cycles]}")
