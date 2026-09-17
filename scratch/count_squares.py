import collections

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
    e_uw = tuple(sorted([u, w]))
    adj_c0[u].add(w); adj_c0[w].add(u)

# Find 4-cycles (chordless squares)
squares = set()
rem_list = sorted(list(rem))
for u in rem_list:
    for v in adj_c0[u]:
        if v > u:
            # common neighbors between u and v
            # to form a 4-cycle u-a-v-b-u
            common = [w for w in adj_c0[u] if w != v and w in adj_c0[v]]
            # that is triangles
            # For 4-cycles: pick a in adj_c0[u], b in adj_c0[v] such that b in adj_c0[a]
            pass

# Better 4-cycle enumeration:
for a in rem_list:
    nbrs_a = sorted(list(adj_c0[a]))
    for i in range(len(nbrs_a)):
        u = nbrs_a[i]
        for j in range(i+1, len(nbrs_a)):
            v = nbrs_a[j]
            # u and v are both neighbors of a
            # Find common neighbors of u and v other than a
            common = [w for w in adj_c0[u] if w != a and w in adj_c0[v]]
            for w in common:
                sq = tuple(sorted([a, u, w, v]))
                # Check chordless: no edge (a, w) and no edge (u, v)
                if w not in adj_c0[a] and v not in adj_c0[u]:
                    squares.add(sq)

print(f"Chordless 4-cycles (squares) in contracted Comp 0: {len(squares)}")
