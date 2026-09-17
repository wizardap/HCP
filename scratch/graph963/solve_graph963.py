#!/usr/bin/env python3
"""
Full Two-Tier SAT Decomposition Solver for graph963.col (N=7,020, M=30,518)
Same parametric family as graph950.col.
Solves all 10 group bulk instances (700 vertices each) with CaDiCaL CEGAR,
assembles Half 1 (3,510v) and Half 2 (3,510v), stitches cross-half bridges,
and validates the complete 7,020-vertex Hamiltonian tour independently.
"""

import time, collections, json, os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
from scratch.graph950.perfect_cluster_assembler import solve_cluster_path, verify_tour, write_hcp
from scratch.graph950.two_half_two_tier_solver import load_graph, decompose_half

def main():
    print("=================================================================")
    print("      TWO-TIER PURE SAT DECOMPOSITION SOLVER: GRAPH963.COL       ")
    print("=================================================================")

    col_path = "FHCPCS-col/graph963.col"
    t0 = time.time()
    G, degs = load_graph(col_path)
    print(f"Loaded {col_path} in {time.time()-t0:.2f}s: |V|={len(G)}, |E|={sum(len(adj) for adj in G.values())//2}")

    h1_roots = {2115, 2170, 3048, 5125, 6936}
    h2_roots = {1759, 3707, 5198, 5386, 6447}
    s_hubs = h1_roots | h2_roots

    dist, owner = {}, {}
    q = collections.deque()
    for s in s_hubs:
        owner[s] = s; dist[s] = 0; q.append(s)
    while q:
        u = q.popleft()
        for v in G[u]:
            if v not in dist:
                dist[v] = dist[u] + 1; owner[v] = owner[u]; q.append(v)

    grp1 = set(u for u in G if owner[u] in h1_roots)
    grp2 = set(u for u in G if owner[u] in h2_roots)
    assert len(grp1) == 3510 and len(grp2) == 3510, f"Partition sizes mismatch: {len(grp1)}, {len(grp2)}"
    print(f"Partitioned into two halves: Half 1 = {len(grp1)} vertices, Half 2 = {len(grp2)} vertices.")

    # -------------------------------------------------------------
    # STEP 1: SOLVE HALF 1 GROUPS (5 groups * 700 vertices = 3,500)
    # -------------------------------------------------------------
    print("\n--- STEP 1: Solving Half 1 Groups (700 vertices each) ---")
    all_hubs1, strips1, hh1, strip_adj_hubs1, _ = decompose_half(G, degs, grp1)

    h1_targets = [
        (2115, 2122, 6045, {"clusters": [1, 2, 6, 8, 17], "tiny": [27, 36]}),
        (6936, 6702, 1321, {"clusters": [4, 7, 9, 13, 18], "tiny": [29, 34]}),
        (2170, 5144, 3745, {"clusters": [3, 5, 10, 12, 20], "tiny": [30, 32]}),
        (3048, 2873, 5960, {"clusters": [11, 14, 16, 22, 23], "tiny": [26, 31]}),
        (5125, 3172, 6667, {"clusters": [0, 15, 19, 21, 24], "tiny": [25, 33]}),
    ]

    cache_h1 = "scratch/graph963/half1_group_paths.json"
    solved_h1 = {}
    if os.path.exists(cache_h1):
        with open(cache_h1, "r") as f:
            for k, v in json.load(f).items():
                solved_h1[int(k)] = v
                print(f"Loaded cached path for Group {k}: {len(v)} vertices.")

    for sh, u_in, u_out, cfg in h1_targets:
        if sh in solved_h1 and len(solved_h1[sh]) == 700:
            print(f"Group {sh} already solved (cached), skipping.")
            continue
        v_bulk = set()
        for ci in cfg["clusters"]:
            v_bulk.update(strips1[ci])
            for h in strip_adj_hubs1[ci]:
                if degs[h] != 702: v_bulk.add(h)
        for ti in cfg["tiny"]:
            v_bulk.update(strips1[ti])
        assert len(v_bulk) == 700
        print(f"\n[H1] Solving Group {sh} ({len(v_bulk)}v) from {u_in} to {u_out}...")
        t_g = time.time()
        path = solve_cluster_path(sh, u_in, u_out, v_bulk, G, max_it=300, verbose=True)
        assert path is not None and len(path) == 700
        solved_h1[sh] = path
        print(f"[H1] Group {sh} SUCCESS in {time.time()-t_g:.2f}s!")
        with open(cache_h1, "w") as f:
            json.dump({str(k): v for k, v in solved_h1.items()}, f)

    # Assemble Half 1:
    half1_path = [3048, 2115, 4037]
    half1_path.extend(solved_h1[2115])
    half1_path.append(5742)
    half1_path.extend(solved_h1[6936])
    half1_path.extend(solved_h1[2170])
    half1_path.extend([1390, 2170])
    half1_path.extend(solved_h1[3048])
    half1_path.extend([4072, 5125, 4444])
    half1_path.extend(solved_h1[5125])
    half1_path.append(6936)

    print(f"\nHalf 1 assembled: {len(half1_path)} vertices. Start={half1_path[0]}, End={half1_path[-1]}")
    assert len(half1_path) == 3510 and len(set(half1_path)) == 3510
    assert set(half1_path) == grp1
    for i in range(len(half1_path) - 1):
        assert half1_path[i+1] in G[half1_path[i]], f"Half 1 edge error at {i}"
    print(">>> HALF 1 100% CERTIFIED! (3,510 vertices from 3048 to 6936) <<<")

    # -------------------------------------------------------------
    # STEP 2: SOLVE HALF 2 GROUPS (5 groups * 700 vertices = 3,500)
    # -------------------------------------------------------------
    print("\n--- STEP 2: Solving Half 2 Groups (700 vertices each) ---")
    all_hubs2, strips2, hh2, strip_adj_hubs2, _ = decompose_half(G, degs, grp2)

    h2_targets = [
        (1759, 993, 5536, {"clusters": [3, 8, 14, 18, 19], "tiny": [30, 36]}),
        (3707, 7013, 6052, {"clusters": [0, 1, 4, 6, 7], "tiny": [28, 35]}),
        (5198, 4961, 1179, {"clusters": [2, 10, 13, 15, 22], "tiny": [27, 32]}),
        (5386, 4917, 1086, {"clusters": [5, 9, 11, 20, 24], "tiny": [29, 31]}),
        (6447, 1071, 4337, {"clusters": [12, 16, 17, 21, 23], "tiny": [25, 33]}),
    ]

    cache_h2 = "scratch/graph963/half2_group_paths.json"
    solved_h2 = {}
    if os.path.exists(cache_h2):
        with open(cache_h2, "r") as f:
            for k, v in json.load(f).items():
                solved_h2[int(k)] = v
                print(f"Loaded cached path for Group {k}: {len(v)} vertices.")

    for sh, u_in, u_out, cfg in h2_targets:
        if sh in solved_h2 and len(solved_h2[sh]) == 700:
            print(f"Group {sh} already solved (cached), skipping.")
            continue
        v_bulk = set()
        for ci in cfg["clusters"]:
            v_bulk.update(strips2[ci])
            for h in strip_adj_hubs2[ci]:
                if degs[h] != 702: v_bulk.add(h)
        for ti in cfg["tiny"]:
            v_bulk.update(strips2[ti])
        assert len(v_bulk) == 700
        print(f"\n[H2] Solving Group {sh} ({len(v_bulk)}v) from {u_in} to {u_out}...")
        t_g = time.time()
        path = solve_cluster_path(sh, u_in, u_out, v_bulk, G, max_it=300, verbose=True)
        assert path is not None and len(path) == 700
        solved_h2[sh] = path
        print(f"[H2] Group {sh} SUCCESS in {time.time()-t_g:.2f}s!")
        with open(cache_h2, "w") as f:
            json.dump({str(k): v for k, v in solved_h2.items()}, f)

    # Assemble Half 2:
    half2_path = [5386, 1759, 5775]
    half2_path.extend(solved_h2[1759])
    half2_path.append(1424)
    half2_path.extend(solved_h2[3707])
    half2_path.extend(solved_h2[5198])
    half2_path.extend([565, 5198])
    half2_path.extend(solved_h2[5386])
    half2_path.extend([632, 6447, 5903])
    half2_path.extend(solved_h2[6447])
    half2_path.append(3707)

    print(f"\nHalf 2 assembled: {len(half2_path)} vertices. Start={half2_path[0]}, End={half2_path[-1]}")
    assert len(half2_path) == 3510 and len(set(half2_path)) == 3510
    assert set(half2_path) == grp2
    for i in range(len(half2_path) - 1):
        assert half2_path[i+1] in G[half2_path[i]], f"Half 2 edge error at {i}"
    print(">>> HALF 2 100% CERTIFIED! (3,510 vertices from 5386 to 3707) <<<")

    # -------------------------------------------------------------
    # STEP 3: STITCHING AND FULL 7,020-VERTEX TOUR CERTIFICATION
    # -------------------------------------------------------------
    print("\n--- STEP 3: Stitching Half 1 and Half 2 across Bridge Edges ---")
    assert 5386 in G[half1_path[-1]], f"Bridge 1 (6936, 5386) error!"
    assert 3048 in G[half2_path[-1]], f"Bridge 2 (3707, 3048) error!"

    full_tour = half1_path + half2_path
    print(f"Stitched Tour length: {len(full_tour)} (expected 7,020)")

    print("\n--- STEP 4: Independent Verification on Raw Graph ---")
    assert verify_tour(full_tour, G), "VERIFICATION FAILED!"
    print("*****************************************************************")
    print("*** 100.000% MATHEMATICALLY CERTIFIED HAMILTONIAN CYCLE PASS! ***")
    print("*****************************************************************")

    out_tour = "scratch/graph963/found_tour_graph963.hcp"
    write_hcp(full_tour, out_tour)
    print(f"Tour written to {out_tour}")
    print(f"Total time elapsed: {time.time()-t0:.2f}s")

if __name__ == "__main__":
    main()
