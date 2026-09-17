import collections, glob, os

graphs = ["graph717", "graph882", "graph937", "graph944", "graph954", "graph959", "graph965", "graph966", "graph971", "graph994", "graph998"]

print(f"{'Graph':<10} {'N':<6} {'M':<7} {'Deg Min-Max':<12} {'#Deg2':<7} {'#Deg3':<7} {'#Deg>=10':<9}")
print("-" * 65)

for g_name in graphs:
    col_path = f"FHCPCS-col/{g_name}.col"
    if not os.path.exists(col_path):
        print(f"{g_name}: NOT FOUND")
        continue
    G = collections.defaultdict(set)
    with open(col_path) as f:
        for line in f:
            if line.startswith("e "):
                p = line.split()
                u, v = int(p[1]), int(p[2])
                G[u].add(v)
                G[v].add(u)
    N = len(G)
    M = sum(len(v) for v in G.values()) // 2
    degs = [len(v) for v in G.values()]
    d_count = collections.Counter(degs)
    deg_min = min(degs)
    deg_max = max(degs)
    deg2 = d_count[2]
    deg3 = d_count[3]
    deg10plus = sum(c for d, c in d_count.items() if d >= 10)
    print(f"{g_name:<10} {N:<6} {M:<7} {f'{deg_min}-{deg_max}':<12} {deg2:<7} {deg3:<7} {deg10plus:<9}")
