import sys
sys.path.insert(0, ".")
import collections, time
from scratch.engine.graph_loader import load_dimacs

col_path = "FHCPCS-col/graph960.col"
t0 = time.time()
G = load_dimacs(col_path)
nv = len(G)
deg2 = [u for u in G if len(G[u]) == 2]
print(f"[*] graph960 loaded: |V|={nv}, Deg-2 nodes={len(deg2)}")

blocks = []
v_partner = {}
for v in deg2:
    u, w = list(G[v])
    blocks.append((u, w, v))
    v_partner[u] = w
    v_partner[w] = u

non_deg2 = set(G.keys()) - set(deg2)
color = {blocks[0][0]: 0}
q = [blocks[0][0]]
bip = True
while q:
    curr = q.pop()
    c = color[curr]
    vp = v_partner[curr]
    if vp not in color:
        color[vp] = 1 - c; q.append(vp)
    elif color[vp] == c:
        bip = False; break
    for nxt in G[curr]:
        if nxt != vp and nxt in non_deg2:
            if nxt not in color:
                color[nxt] = 1 - c; q.append(nxt)
            elif color[nxt] == c:
                bip = False; break
    if not bip: break

print(f"[*] Bipartite: {bip}, Blocks: {len(blocks)}")
n_dir = len(blocks)
node_to_id = {}
id_to_pair = {}
for idx, (u, w, v) in enumerate(blocks):
    node_to_id[u] = idx; node_to_id[w] = idx
    in_v = u if color[u] == 0 else w
    out_v = w if in_v == u else u
    id_to_pair[idx] = (in_v, out_v, v)

dir_adj = collections.defaultdict(list)
arcs = set()
for idx in range(n_dir):
    in_v, out_v, _ = id_to_pair[idx]
    for nxt in sorted(G[out_v]):
        if nxt != in_v and nxt in node_to_id and nxt == id_to_pair[node_to_id[nxt]][0]:
            arc = (idx, node_to_id[nxt])
            if arc not in arcs:
                arcs.add(arc); dir_adj[idx].append(node_to_id[nxt])

print(f"[*] Directed macro-graph for graph960: {n_dir} nodes, {len(arcs)} arcs.")
