from collections import defaultdict

# Load graph868 and understand window splicing
# Contracted graph edges
with open("scratch/best_edges_graph868.txt") as f:
    edges = [tuple(map(int, line.split())) for line in f]

print(f"Edges count: {len(edges)}")
