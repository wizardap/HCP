import time, multiprocessing, os, sys
sys.path.insert(0, ".")
from scratch.graph950.two_half_two_tier_solver import load_graph, partition_halves, decompose_half
from scratch.graph950.perfect_cluster_assembler import solve_cluster_path
from hcp_solver.core.verifier import certify_tour
from hcp_solver.core.graph import Graph

def _worker(task):
    sh, u_in, u_out, v_bulk, edges = task
    local_G = {u: set(nbrs) for u, nbrs in edges.items()}
    p = solve_cluster_path(sh, u_in, u_out, set(v_bulk), local_G, max_it=300, verbose=False)
    return sh, p

def main():
    t0 = time.time()
    col_path = "FHCPCS-col/graph950.col"
    print(f"[*] Loading and partitioning {col_path}...")
    G, degs = load_graph(col_path)
    grp1, grp2 = partition_halves(G, degs)

    all_hubs1, strips1, _, strip_adj_hubs1, _ = decompose_half(G, degs, grp1)
    all_hubs2, strips2, _, strip_adj_hubs2, _ = decompose_half(G, degs, grp2)

    h1_targets = [
        (164, 3944, 6285, {"clusters": [18, 2, 3, 15, 6], "tiny": [27, 31]}),
        (5787, 4655, 5666, {"clusters": [0, 21, 17, 5, 24], "tiny": [28, 35]}),
        (5835, 902, 3206, {"clusters": [12, 13, 20, 14, 22], "tiny": [26, 36]}),
        (4785, 6341, 433, {"clusters": [1, 8, 7, 9, 16], "tiny": [30, 33]}),
        (4000, 2594, 6541, {"clusters": [10, 4, 23, 19, 11], "tiny": [25, 32]}),
    ]

    h2_targets = [
        (2803, 2317, 388, {"clusters": [17, 18, 20, 21, 22], "tiny": [26, 33]}),
        (6080, 4808, 4713, {"clusters": [1, 3, 8, 16, 19], "tiny": [27, 34]}),
        (1171, 3407, 2189, {"clusters": [0, 5, 10, 15, 24], "tiny": [29, 32]}),
        (5540, 4613, 5710, {"clusters": [2, 4, 6, 7, 9], "tiny": [25, 36]}),
        (4540, 1648, 4833, {"clusters": [11, 12, 13, 14, 23], "tiny": [28, 35]}),
    ]

    tasks = []
    for sh, u_in, u_out, cfg in h1_targets:
        v_bulk = set()
        for ci in cfg["clusters"]:
            v_bulk.update(strips1[ci])
            for h in strip_adj_hubs1[ci]:
                if degs[h] != 662: v_bulk.add(h)
        for ti in cfg["tiny"]: v_bulk.update(strips1[ti])
        edges = {u: list(G[u] & v_bulk) for u in v_bulk}
        tasks.append((sh, u_in, u_out, list(v_bulk), edges))

    for sh, u_in, u_out, cfg in h2_targets:
        v_bulk = set()
        for ci in cfg["clusters"]:
            v_bulk.update(strips2[ci])
            for h in strip_adj_hubs2[ci]:
                if degs[h] != 662: v_bulk.add(h)
        for ti in cfg["tiny"]: v_bulk.update(strips2[ti])
        edges = {u: list(G[u] & v_bulk) for u in v_bulk}
        tasks.append((sh, u_in, u_out, list(v_bulk), edges))

    print(f"[*] Solving all 10 clusters in parallel on 8 CPU cores (pure SAT CEGAR, zero cache)...")
    t_sat = time.time()
    solved = {}
    with multiprocessing.Pool(processes=8) as pool:
        for sh, path in pool.imap_unordered(_worker, tasks):
            assert path is not None and len(path) == 660, f"Failed on group {sh}"
            solved[sh] = path
            print(f"    Group {sh} solved ({len(path)}v) in {time.time()-t_sat:.2f}s elapsed")

    h1 = solved
    h2 = solved

    half1_path = [164, 5787, 1942]
    half1_path.extend(h1[5787])
    half1_path.append(2492)
    half1_path.extend(h1[5835])
    half1_path.extend(h1[4785])
    half1_path.extend([6454, 4785])
    half1_path.extend(h1[164])
    half1_path.extend([5036, 4000, 6014])
    half1_path.extend(h1[4000])
    half1_path.append(5835)

    half2_path = [5540, 2803, 5013]
    half2_path.extend(h2[2803])
    half2_path.append(764)
    half2_path.extend(h2[6080])
    half2_path.extend(h2[1171])
    half2_path.extend([4319, 1171])
    half2_path.extend(h2[5540])
    half2_path.extend([5749, 4540, 3679])
    half2_path.extend(h2[4540])
    half2_path.append(6080)

    tour = half1_path + half2_path
    assert len(tour) == 6620
    assert len(set(tour)) == 6620

    certify_tour(tour, Graph(G, "graph950.col"), "graph950")
    print(f"[*] SUCCESS! graph950 solved from scratch in {time.time()-t0:.2f}s total!")

if __name__ == "__main__":
    main()
