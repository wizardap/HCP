#!/usr/bin/env python3
import json, os, sys, time
from multiprocessing import Pool

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
from scratch.graph982.two_half_decomposer import load_and_partition_graph982
from scratch.graph950.two_half_two_tier_solver import decompose_half
from scratch.graph982.cluster_path_solver import solve_group_bulk

def worker_solve_group(item):
    half, sh, u_in, u_out, cfg, strips, strip_adj_hubs, degs, G = item
    print(f"[Worker H{half}] Starting Group {sh} ({u_in} -> {u_out})...", flush=True)
    t0 = time.time()
    path = solve_group_bulk(sh, u_in, u_out, cfg, strips, strip_adj_hubs, degs, G, max_it=300)
    elapsed = time.time() - t0
    assert path is not None and len(path) == 760, f"Failed Group {sh}"
    assert path[0] == u_in and path[-1] == u_out
    print(f"[Worker H{half}] Group {sh} COMPLETED in {elapsed:.2f}s!", flush=True)
    return (half, sh, path)

def main():
    col_path = "FHCPCS-col/graph982.col"
    t0 = time.time()
    G, degs, grp1, grp2, h1_targets, h2_targets, bridges = load_and_partition_graph982(col_path)

    base_dir = os.path.dirname(os.path.abspath(__file__))
    h1_cache = os.path.join(base_dir, "half1_group_paths.json")
    h2_cache = os.path.join(base_dir, "half2_group_paths.json")

    solved_h1 = {}
    if os.path.exists(h1_cache):
        with open(h1_cache, "r") as f:
            for k, v in json.load(f).items():
                solved_h1[int(k)] = v

    solved_h2 = {}
    if os.path.exists(h2_cache):
        with open(h2_cache, "r") as f:
            for k, v in json.load(f).items():
                solved_h2[int(k)] = v

    all_hubs1, strips1, _, strip_adj_hubs1, _ = decompose_half(G, degs, grp1)
    all_hubs2, strips2, _, strip_adj_hubs2, _ = decompose_half(G, degs, grp2)

    tasks = []
    for sh, u_in, u_out, cfg in h1_targets:
        if sh in solved_h1 and len(solved_h1[sh]) == 760 and solved_h1[sh][0] == u_in and solved_h1[sh][-1] == u_out:
            print(f"[Cache Hit] H1 Group {sh} already solved ({u_in} -> {u_out})", flush=True)
        else:
            tasks.append((1, sh, u_in, u_out, cfg, strips1, strip_adj_hubs1, degs, G))

    for sh, u_in, u_out, cfg in h2_targets:
        if sh in solved_h2 and len(solved_h2[sh]) == 760 and solved_h2[sh][0] == u_in and solved_h2[sh][-1] == u_out:
            print(f"[Cache Hit] H2 Group {sh} already solved ({u_in} -> {u_out})", flush=True)
        else:
            tasks.append((2, sh, u_in, u_out, cfg, strips2, strip_adj_hubs2, degs, G))

    print(f"\nTotal tasks to solve in parallel: {len(tasks)}", flush=True)

    if tasks:
        with Pool(processes=min(4, len(tasks))) as pool:
            results = pool.map(worker_solve_group, tasks)

        for half, sh, path in results:
            if half == 1:
                solved_h1[sh] = path
            else:
                solved_h2[sh] = path

        with open(h1_cache, "w") as f:
            json.dump({str(k): v for k, v in solved_h1.items()}, f)
        with open(h2_cache, "w") as f:
            json.dump({str(k): v for k, v in solved_h2.items()}, f)

    print(f"\nAll groups solved and saved in {time.time()-t0:.2f}s total!", flush=True)

if __name__ == "__main__":
    main()
