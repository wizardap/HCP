import collections, itertools, json, os, sys, time
from typing import Dict, List, Set, Tuple
from pysat.solvers import Cadical153

repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
if repo_root not in sys.path:
    sys.path.insert(0, repo_root)

from scratch.graph882.chain_solver import solve_subgraph_path
from scratch.graph677.decomposer import load_and_decompose_graph677

def solve_module_corridor(G: Dict[int, Set[int]], mod_nodes: Set[int], port1: int, port2: int, cache_path: str = None) -> List[int]:
    """
    Solves Hamiltonian path for the 169-vertex module connecting port1 and port2.
    """
    if cache_path and os.path.exists(cache_path):
        with open(cache_path, "r") as f:
            p = json.load(f)
        if len(p) == len(mod_nodes) and p[0] == port1 and p[-1] == port2:
            return p

    t0 = time.time()
    path = solve_subgraph_path(G, mod_nodes, src=port1, dst=port2)
    assert len(path) == len(mod_nodes)
    assert path[0] == port1 and path[-1] == port2
    print(f"[*] Solved module path ({len(path)} vertices) in {time.time()-t0:.2f}s.")

    if cache_path:
        os.makedirs(os.path.dirname(os.path.abspath(cache_path)), exist_ok=True)
        with open(cache_path, "w") as f:
            json.dump(path, f)

    return path

if __name__ == "__main__":
    col_path = os.path.join(repo_root, "FHCPCS-col/graph677.col")
    G, mod_nodes, comp0_nodes, (port1, port2), _ = load_and_decompose_graph677(col_path)
    solve_module_corridor(G, mod_nodes, port1, port2)
