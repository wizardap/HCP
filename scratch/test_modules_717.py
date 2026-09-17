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

# Extract modules
vis = set()
comps = []
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
        comps.append(set(c))

mod_map = {}
for c in comps:
    if len(c) == 167:
        adj_cuts = tuple(sorted(list(set(nbr for u in c for nbr in G[u] if nbr in cut_nodes))))
        mod_map[adj_cuts] = c

print(f"Testing CaDiCaL CEGAR on all 6 modules of 167 vertices:")

for (u_port, v_port), mod in sorted(mod_map.items()):
    t0 = time.time()
    # Vertices in this module plus the two ports: 167 + 2 = 169 vertices!
    V_mod = mod | {u_port, v_port}
    # Virtual edge between u_port and v_port
    edges = set()
    for u in V_mod:
        for v in G[u]:
            if v in V_mod and u < v:
                edges.add((u, v))
    virt = tuple(sorted([u_port, v_port]))
    edges.add(virt)
    edge_list = sorted(list(edges))
    edge_to_var = {e: i + 1 for i, e in enumerate(edge_list)}
    inc_edges = collections.defaultdict(list)
    for e in edge_list:
        inc_edges[e[0]].append(edge_to_var[e])
        inc_edges[e[1]].append(edge_to_var[e])

    solver = Cadical195()
    top = len(edge_list) + 1

    # Exactly-2
    for u in V_mod:
        lits = inc_edges[u]
        clauses = CardEnc.equals(lits=lits, bound=2, top_id=top, encoding=EncType.cardnetwrk)
        for cl in clauses:
            solver.add_clause(cl)
            for lit in cl:
                top = max(top, abs(lit) + 1)

    # Force virtual edge
    solver.add_clause([edge_to_var[virt]])

    it = 0
    while True:
        it += 1
        if not solver.solve():
            print(f"Module {u_port}-{v_port}: UNSAT!")
            break
        model = set(solver.get_model())
        active = [e for e in edge_list if edge_to_var[e] in model]
        adj = collections.defaultdict(list)
        for u, v in active:
            adj[u].append(v); adj[v].append(u)

        visited = set()
        cycles = []
        for u in V_mod:
            if u not in visited:
                cyc = []
                curr, prev = u, None
                while curr not in visited:
                    visited.add(curr); cyc.append(curr)
                    nbrs = adj[curr]
                    nxt = nbrs[0] if nbrs[0] != prev else nbrs[1]
                    prev, curr = curr, nxt
                cycles.append(cyc)

        if len(cycles) == 1:
            solver.delete()
            print(f"Module {u_port} <-> {v_port} (169v): SOLVED in {time.time()-t0:.3f}s (iter {it})!")
            break

        for cyc in cycles:
            neg_clause = [-edge_to_var[tuple(sorted([cyc[i], cyc[(i+1)%len(cyc)]]))] for i in range(len(cyc))]
            solver.add_clause(neg_clause)
