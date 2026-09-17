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
chains = []
v_partner = {}
for v in deg2:
    u, w = list(G[v])
    chains.append((min(u, w), max(u, w)))
    v_partner[u] = w
    v_partner[w] = u

chains = sorted(list(set(chains)))
node_to_ve = {}
for idx, (u, w) in enumerate(chains):
    node_to_ve[u] = idx
    node_to_ve[w] = idx

num_ve = len(chains)

# ve_adj with weights
ve_adj = collections.defaultdict(lambda: collections.defaultdict(int))
for idx, (u, w) in enumerate(chains):
    for end in [u, w]:
        for nxt in G[end]:
            if nxt in node_to_ve:
                other = node_to_ve[nxt]
                if other != idx:
                    ve_adj[idx][other] += 1

# Replicate BipartiteModuleDetector greedy expansion
expected_k = 84
target_ve = num_ve // expected_k # 22
unassigned = set(range(num_ve))
clusters = []

while unassigned:
    if len(clusters) + 1 == expected_k:
        clusters.append(sorted(list(unassigned)))
        break
    start = min(unassigned)
    chunk = [start]
    unassigned.remove(start)
    
    cand_conn = collections.defaultdict(int)
    for nbr, w in ve_adj[start].items():
        if nbr in unassigned:
            cand_conn[nbr] += w
            
    while len(chunk) < target_ve and unassigned:
        if cand_conn:
            best_cand = max(cand_conn.keys(), key=lambda c: (cand_conn[c], -c))
        else:
            best_cand = min(unassigned)
        chunk.append(best_cand)
        unassigned.remove(best_cand)
        if best_cand in cand_conn: del cand_conn[best_cand]
        for nbr, w in ve_adj[best_cand].items():
            if nbr in unassigned:
                cand_conn[nbr] += w
                
    clusters.append(sorted(chunk))

print(f"Generated {len(clusters)} clusters of target size {target_ve}")

# Quotient graph between the 84 clusters:
ve_to_cluster = {}
for c_idx, cl in enumerate(clusters):
    for ve in cl:
        ve_to_cluster[ve] = c_idx

cluster_adj = collections.defaultdict(lambda: collections.defaultdict(int))
for c_idx, cl in enumerate(clusters):
    for ve in cl:
        for nbr, w in ve_adj[ve].items():
            other_c = ve_to_cluster[nbr]
            if other_c != c_idx:
                cluster_adj[c_idx][other_c] += w

print("\nConnections between 84 clusters (first 10 clusters):")
for i in range(10):
    sorted_nbrs = sorted(cluster_adj[i].items(), key=lambda x: -x[1])
    print(f"Cluster {i:2d} (size {len(clusters[i])}): top connections = {sorted_nbrs[:5]}")

