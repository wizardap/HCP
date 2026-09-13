import os, sys, time, json, multiprocessing

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
from scratch.graph990.two_half_decomposer import load_and_partition_graph990
from scratch.graph950.two_half_two_tier_solver import decompose_half
from scratch.graph990.cluster_path_solver import build_group_bulk
from scratch.graph950.perfect_cluster_assembler import solve_cluster_path

def worker_solve_group(item):
    sh, u_in, u_out, v_bulk, edges, half_num = item
    print(f"[Worker Half {half_num}] Solving Group {sh} ({len(v_bulk)}v) from {u_in} to {u_out}...", flush=True)
    t0 = time.time()
    # Reconstruct local G
    local_G = {u: set(nbrs) for u, nbrs in edges.items()}
    path = solve_cluster_path(sh, u_in, u_out, set(v_bulk), local_G, max_it=300, verbose=True)
    dt = time.time() - t0
    print(f"[Worker Half {half_num}] Group {sh} FINISHED in {dt:.2f}s (len={len(path) if path else None})", flush=True)
    return half_num, sh, path

def main():
    col_path = "FHCPCS-col/graph990.col"
    print("Loading graph990.col...")
    G, degs, grp1, grp2, h1_targets, h2_targets, _ = load_and_partition_graph990(col_path)

    base_dir = os.path.dirname(os.path.abspath(__file__))
    cache_h1 = os.path.join(base_dir, "half1_group_paths.json")
    cache_h2 = os.path.join(base_dir, "half2_group_paths.json")

    cached_h1 = {}
    if os.path.exists(cache_h1):
        with open(cache_h1, "r") as f:
            cached_h1 = {int(k): v for k, v in json.load(f).items()}

    cached_h2 = {}
    if os.path.exists(cache_h2):
        with open(cache_h2, "r") as f:
            cached_h2 = {int(k): v for k, v in json.load(f).items()}

    all_hubs1, strips1, _, strip_adj_hubs1, _ = decompose_half(G, degs, grp1)
    all_hubs2, strips2, _, strip_adj_hubs2, _ = decompose_half(G, degs, grp2)

    tasks = []
    # Half 1 tasks
    for sh, u_in, u_out, cfg in h1_targets:
        if sh in cached_h1 and len(cached_h1[sh]) == 800 and cached_h1[sh][0] == u_in and cached_h1[sh][-1] == u_out:
            print(f"[Half 1] Group {sh} already solved, skipping.")
            continue
        v_bulk = build_group_bulk(cfg, strips1, strip_adj_hubs1, degs)
        edges = {u: list(G[u] & v_bulk) for u in v_bulk}
        tasks.append((sh, u_in, u_out, list(v_bulk), edges, 1))

    # Half 2 tasks
    for sh, u_in, u_out, cfg in h2_targets:
        if sh in cached_h2 and len(cached_h2[sh]) == 800 and cached_h2[sh][0] == u_in and cached_h2[sh][-1] == u_out:
            print(f"[Half 2] Group {sh} already solved, skipping.")
            continue
        v_bulk = build_group_bulk(cfg, strips2, strip_adj_hubs2, degs)
        edges = {u: list(G[u] & v_bulk) for u in v_bulk}
        tasks.append((sh, u_in, u_out, list(v_bulk), edges, 2))

    num_workers = min(4, os.cpu_count() or 4)
    print(f"\nDispatching {len(tasks)} tasks to multiprocessing Pool({num_workers})...")
    if tasks:
        with multiprocessing.Pool(processes=num_workers) as pool:
            for half_num, sh, path in pool.imap_unordered(worker_solve_group, tasks):
                assert path is not None and len(path) == 800, f"Failed Group {sh}"
                if half_num == 1:
                    cached_h1[sh] = path
                    with open(cache_h1, "w") as f:
                        json.dump({str(k): v for k, v in cached_h1.items()}, f)
                else:
                    cached_h2[sh] = path
                    with open(cache_h2, "w") as f:
                        json.dump({str(k): v for k, v in cached_h2.items()}, f)

    print("All 10 groups solved and cached successfully!")

if __name__ == "__main__":
    main()
