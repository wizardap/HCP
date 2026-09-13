import json, os, sys, time
from typing import Dict, List, Set, Optional

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
from scratch.graph950.perfect_cluster_assembler import solve_cluster_path

def build_group_bulk(cfg: dict, strips, strip_adj_hubs, degs) -> Set[int]:
    v_bulk = set()
    for ci in cfg["clusters"]:
        v_bulk.update(strips[ci])
        for h in strip_adj_hubs[ci]:
            if degs[h] != 762:
                v_bulk.add(h)
    for ti in cfg["tiny"]:
        v_bulk.update(strips[ti])
    return v_bulk

def solve_group_bulk(sh: int, u_in: int, u_out: int, cfg: dict, strips, strip_adj_hubs, degs, G, max_it: int = 300) -> Optional[List[int]]:
    v_bulk = build_group_bulk(cfg, strips, strip_adj_hubs, degs)
    assert len(v_bulk) == 760, f"Bulk size error: {len(v_bulk)} != 760"
    return solve_cluster_path(sh, u_in, u_out, v_bulk, G, max_it=max_it, verbose=True)

def solve_all_half_groups(G, degs, grp, targets, strips, strip_adj_hubs, cache_file: str, max_it: int = 300) -> Dict[int, List[int]]:
    solved = {}
    if os.path.exists(cache_file):
        with open(cache_file, "r") as f:
            for k, v in json.load(f).items():
                solved[int(k)] = v
                print(f"Loaded cached path for Group {k}: {len(v)} vertices.")

    for sh, u_in, u_out, cfg in targets:
        if sh in solved and len(solved[sh]) == 760:
            print(f"Group {sh} already solved (cached), skipping.")
            continue
        print(f"\nSolving Group {sh} (760v) from {u_in} to {u_out}...")
        t0 = time.time()
        path = solve_group_bulk(sh, u_in, u_out, cfg, strips, strip_adj_hubs, degs, G, max_it=max_it)
        assert path is not None and len(path) == 760, f"Failed to solve Group {sh}"
        solved[sh] = path
        print(f"Group {sh} SUCCESS in {time.time()-t0:.2f}s!")
        os.makedirs(os.path.dirname(os.path.abspath(cache_file)), exist_ok=True)
        with open(cache_file, "w") as f:
            json.dump({str(k): v for k, v in solved.items()}, f)

    return solved

if __name__ == "__main__":
    import argparse
    from scratch.graph982.two_half_decomposer import load_and_partition_graph982
    from scratch.graph950.two_half_two_tier_solver import decompose_half

    parser = argparse.ArgumentParser()
    parser.add_argument("--half", type=int, choices=[1, 2, 0], default=0, help="1 for Half 1, 2 for Half 2, 0 for both")
    args = parser.parse_args()

    col_path = "FHCPCS-col/graph982.col"
    G, degs, grp1, grp2, h1_targets, h2_targets, bridges = load_and_partition_graph982(col_path)

    base_dir = os.path.dirname(os.path.abspath(__file__))
    h1_cache = os.path.join(base_dir, "half1_group_paths.json")
    h2_cache = os.path.join(base_dir, "half2_group_paths.json")

    if args.half in (1, 0):
        print("=== Solving Half 1 Groups ===")
        all_hubs1, strips1, _, strip_adj_hubs1, _ = decompose_half(G, degs, grp1)
        solve_all_half_groups(G, degs, grp1, h1_targets, strips1, strip_adj_hubs1, h1_cache)

    if args.half in (2, 0):
        print("=== Solving Half 2 Groups ===")
        all_hubs2, strips2, _, strip_adj_hubs2, _ = decompose_half(G, degs, grp2)
        solve_all_half_groups(G, degs, grp2, h2_targets, strips2, strip_adj_hubs2, h2_cache)
