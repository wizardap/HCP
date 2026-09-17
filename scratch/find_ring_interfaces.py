import collections

with open('FHCPCS-col/graph868.col') as f:
    edges = []
    n = 0
    for line in f:
        if line.startswith('p '):
            n = int(line.split()[2])
        elif line.startswith('e '):
            p = line.split()
            edges.append((int(p[1]), int(p[2])))

G = collections.defaultdict(set)
for u, v in edges:
    G[u].add(v)
    G[v].add(u)

deg2 = sorted([u for u in G if len(G[u]) == 2])
v_partner = {}
for v in deg2:
    u, w = list(G[v])
    v_partner[u] = w
    v_partner[w] = u

chains = sorted(list(set([(min(u, w), max(u, w)) for u, w in v_partner.items()])))
node_to_ve = {}
for idx, (u, w) in enumerate(chains):
    node_to_ve[u] = idx
    node_to_ve[w] = idx

# 1848 virtual edges = 42 * 44.
# Notice: in raw graph, 5544 vertices = 42 * 132 vertices!
# Let's check: are raw vertices numbered consecutively per module?
# e.g. module 0: vertices 1..132, module 1: 133..264, etc.?
# Let's test this hypothesis!
mod_size_raw = 5544 // 42 # 132
print(f"Testing consecutive numbering hypothesis: mod_size_raw = {mod_size_raw}")

inter_mod_edges = 0
intra_mod_edges = 0
mod_connections = collections.defaultdict(int)

for u, v in edges:
    m_u = (u - 1) // mod_size_raw
    m_v = (v - 1) // mod_size_raw
    if m_u == m_v:
        intra_mod_edges += 1
    else:
        inter_mod_edges += 1
        m1, m2 = min(m_u, m_v), max(m_u, m_v)
        mod_connections[(m1, m2)] += 1

print(f"Consecutive hypothesis: intra_mod_edges = {intra_mod_edges}, inter_mod_edges = {inter_mod_edges}")
print(f"Number of inter-module connection pairs: {len(mod_connections)}")
for (m1, m2), cnt in sorted(mod_connections.items())[:15]:
    print(f"  Mod {m1:2d} <-> Mod {m2:2d}: {cnt} edges")

