import sys, os, collections
repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from scratch.engine.graph_loader import load_dimacs
from scratch.engine.decomposer import detect_bridge_corridors

unsolved = [832, 868, 937, 951, 954, 959, 960, 965, 966, 971, 974, 976, 981, 983, 987, 993, 994, 998]

print(f"{'Graph':10} | {'|V|':5} | {'Comp0':5} | {'Corridors':10} | {'Corridor Nodes':15}")
print("-" * 60)

for g_id in unsolved:
    path = f"FHCPCS-col/graph{g_id}.col"
    G = load_dimacs(path)
    corridors, comp0 = detect_bridge_corridors(G)
    total_corr_nodes = sum(len(c['nodes']) for c in corridors)
    print(f"graph{g_id:<5} | {len(G):<5} | {len(comp0):<5} | {len(corridors):<10} | {total_corr_nodes:<15}")
