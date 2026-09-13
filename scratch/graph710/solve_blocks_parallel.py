import json, multiprocessing, os, sys, time
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "../..")))
from typing import Dict, List
from scratch.graph710.decomposer import load_and_decompose_graph710
from scratch.graph710.block_solver import solve_block_a, solve_block_b

def _worker_a(col_path: str):
    G, V_A, _, port_u, port_v = load_and_decompose_graph710(col_path)
    print("Worker A started...")
    path_a = solve_block_a(G, V_A, port_u, port_v)
    return "A", path_a

def _worker_b(col_path: str):
    G, _, V_B, port_u, port_v = load_and_decompose_graph710(col_path)
    print("Worker B started...")
    path_b = solve_block_b(G, V_B, port_u, port_v)
    return "B", path_b

def solve_both_blocks_parallel(col_path: str, cache_path: str) -> Dict[str, List[int]]:
    t0 = time.time()
    os.makedirs(os.path.dirname(os.path.abspath(cache_path)), exist_ok=True)
    if os.path.exists(cache_path):
        with open(cache_path, "r") as f:
            data = json.load(f)
        if "A" in data and "B" in data and len(data["A"]) == 887 and len(data["B"]) == 3179:
            print("Both block paths already cached!")
            return data

    with multiprocessing.Pool(processes=2) as pool:
        res_a = pool.apply_async(_worker_a, (col_path,))
        res_b = pool.apply_async(_worker_b, (col_path,))
        k_a, path_a = res_a.get()
        k_b, path_b = res_b.get()

    result = {k_a: path_a, k_b: path_b}
    with open(cache_path, "w") as f:
        json.dump(result, f)
    print(f"Both blocks solved and cached to {cache_path} in {time.time()-t0:.2f}s.")
    return result

if __name__ == "__main__":
    base_dir = os.path.dirname(os.path.abspath(__file__))
    col_path = "FHCPCS-col/graph710.col"
    cache_path = os.path.join(base_dir, "block_paths.json")
    solve_both_blocks_parallel(col_path, cache_path)
