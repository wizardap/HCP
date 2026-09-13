import os, sys, time, json, multiprocessing

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
from scratch.graph746.cluster_decomposer import load_and_decompose_graph746
from scratch.graph950.perfect_cluster_assembler import solve_cluster_path

def worker_solve_group(item):
    sh, u_in, u_out, v_bulk, edges = item
    print(f"[Worker] Solving Group {sh} ({len(v_bulk)}v) from {u_in} to {u_out}...")
    t0 = time.time()
    local_G = {u: set(nbrs) for u, nbrs in edges.items()}
    path = solve_cluster_path(sh, u_in, u_out, set(v_bulk), local_G, max_it=300, verbose=True)
    dt = time.time() - t0
    print(f"[Worker] Group {sh} FINISHED in {dt:.2f}s (len={len(path) if path else None})")
    return sh, path

def main():
    col_path = "FHCPCS-col/graph746.col"
    print("Loading graph746.col...")
    G, degs, group_targets, macro_nodes, bulks = load_and_decompose_graph746(col_path)

    base_dir = os.path.dirname(os.path.abspath(__file__))
    cache_file = os.path.join(base_dir, "group_paths.json")

    cached = {}
    if os.path.exists(cache_file):
        with open(cache_file, "r") as f:
            cached = {int(k): v for k, v in json.load(f).items()}

    tasks = []
    for sh, u_in, u_out, cfg in group_targets:
        if sh in cached and len(cached[sh]) == 855 and cached[sh][0] == u_in and cached[sh][-1] == u_out:
            print(f"Group {sh} already solved, skipping.")
            continue
        v_bulk = bulks[sh]
        edges = {u: list(G[u] & v_bulk) for u in v_bulk}
        tasks.append((sh, u_in, u_out, list(v_bulk), edges))

    print(f"\nDispatching {len(tasks)} tasks to multiprocessing Pool(4)...")
    if tasks:
        with multiprocessing.Pool(processes=min(4, os.cpu_count() or 4)) as pool:
            for sh, path in pool.imap_unordered(worker_solve_group, tasks):
                assert path is not None and len(path) == 855
                cached[sh] = path
                with open(cache_file, "w") as f:
                    json.dump({str(k): v for k, v in cached.items()}, f)

    print("All 5 groups solved and cached successfully!")

if __name__ == "__main__":
    main()
