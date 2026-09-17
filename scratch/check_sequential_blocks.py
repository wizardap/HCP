import collections

with open('FHCPCS-col/graph868.col') as f:
    edges = []
    for line in f:
        if line.startswith('e '):
            p = line.split()
            edges.append((int(p[1]), int(p[2])))

G = collections.defaultdict(set)
for u, v in edges:
    G[u].add(v)
    G[v].add(u)

# Check edges between blocks of 132 vertices:
# Block k: [132*k + 1, 132*(k+1)]
blocks = [set(range(132*k + 1, 132*(k+1) + 1)) for k in range(42)]

# Count internal vs external edges for each block:
for k in range(5):
    internal = 0
    external = collections.defaultdict(int)
    for u in blocks[k]:
        for v in G[u]:
            if v > u:
                if v in blocks[k]:
                    internal += 1
                else:
                    for other in range(42):
                        if v in blocks[other]:
                            external[other] += 1
    print(f"Block {k}: internal={internal}, external={dict(external)}")
