import json, os, sys, time
from typing import Dict, List, Set, Optional

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
from scratch.graph950.perfect_cluster_assembler import solve_cluster_path
from scratch.graph746.cluster_decomposer import load_and_decompose_graph746

def solve_all_groups(G, degs, group_targets, bulks, cache_file: str, max_it: int = 300) -> Dict[int, List[int]]:
    solved = {}
    if os.path.exists(cache_file):
        with open(cache_file, "r") as f:
            for k, v in json.load(f).items():
                solved[int(k)] = v
                print(f"Loaded cached path for Group {k}: {len(v)} vertices.")

    for sh, u_in, u_out, cfg in group_targets:
        if sh in solved and len(solved[sh]) == 855 and solved[sh][0] == u_in and solved[sh][-1] == u_out:
            print(f"Group {sh} already solved with matching endpoints (cached), skipping.")
            continue
        v_bulk = bulks[sh]
        assert len(v_bulk) == 855
        print(f"\nSolving Group {sh} (855v) from {u_in} to {u_out}...")
        t0 = time.time()
        path = solve_cluster_path(sh, u_in, u_out, v_bulk, G, max_it=max_it, verbose=True)
        assert path is not None and len(path) == 855, f"Failed to solve Group {sh}"
        solved[sh] = path
        print(f"Group {sh} SUCCESS in {time.time()-t0:.2f}s!")
        os.makedirs(os.path.dirname(os.path.abspath(cache_file)), exist_ok=True)
        with open(cache_file, "w") as f:
            json.dump({str(k): v for k, v in solved.items()}, f)

    return solved
