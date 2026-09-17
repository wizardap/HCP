#!/usr/bin/env python3
"""
Full Two-Tier SAT Decomposition Solver for graph975.col (N=7,420, M=32,318)
Same parametric family as graph950.col and graph963.col.
Solves all 10 group bulk instances (740 vertices each) with CaDiCaL CEGAR,
assembles Half 1 (3,710v) and Half 2 (3,710v), stitches cross-half bridges,
and validates the complete 7,420-vertex Hamiltonian tour independently.
"""

import time, collections, json, os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
from scratch.graph950.perfect_cluster_assembler import solve_cluster_path, verify_tour, write_hcp
from scratch.graph950.two_half_two_tier_solver import load_graph, decompose_half

def main():
    print("=================================================================")
    print("      TWO-TIER PURE SAT DECOMPOSITION SOLVER: GRAPH975.COL       ")
    print("=================================================================")

    col_path = "FHCPCS-col/graph975.col"
    t0 = time.time()
    G, degs = load_graph(col_path)
    print(f"Loaded {col_path} in {time.time()-t0:.2f}s: |V|={len(G)}, |E|={sum(len(adj) for adj in G.values())//2}")

    h1_roots = {5256, 4754, 947, 5309, 3038}
    h2_roots = {4227, 3333, 4934, 5244, 4957}
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
    assert len(grp1) == 3710 and len(grp2) == 3710, f"Partition sizes mismatch: {len(grp1)}, {len(grp2)}"
    print(f"Partitioned into two halves: Half 1 = {len(grp1)} vertices, Half 2 = {len(grp2)} vertices.")

    # -------------------------------------------------------------
    # STEP 1: SOLVE HALF 1 GROUPS (5 groups * 740 vertices = 3,700)
    # -------------------------------------------------------------
    print("\n--- STEP 1: Solving Half 1 Groups (740 vertices each) ---")
    all_hubs1, strips1, hh1, strip_adj_hubs1, _ = decompose_half(G, degs, grp1)

    # h1_targets: (super_hub, u_in, u_out, cfg)
    h1_targets = [
        (5256, 2199, 2085, {"clusters": [5, 12, 14, 21, 23], "tiny": [29, 34]}),
        (947, 3691, 2440, {"clusters": [1, 7, 11, 19, 22], "tiny": [30, 31]}),
        (5309, 3873, 2873, {"clusters": [3, 4, 15, 16, 24], "tiny": [25, 35]}),
        (3038, 7306, 1522, {"clusters": [2, 6, 9, 13, 20], "tiny": [27, 32]}),
        (4754, 5755, 5108, {"clusters": [0, 8, 10, 17, 18], "tiny": [28, 33]}),
    ]

    cache_h1 = "scratch/graph975/half1_group_paths.json"
    solved_h1 = {}
    if os.path.exists(cache_h1):
        with open(cache_h1, "r") as f:
            for k, v in json.load(f).items():
                solved_h1[int(k)] = v
                print(f"Loaded cached path for Group {k}: {len(v)} vertices.")

    for sh, u_in, u_out, cfg in h1_targets:
        if sh in solved_h1 and len(solved_h1[sh]) == 740:
            print(f"Group {sh} already solved (cached), skipping.")
            continue
        v_bulk = set()
        for ci in cfg["clusters"]:
            v_bulk.update(strips1[ci])
            for h in strip_adj_hubs1[ci]:
                if degs[h] != 742: v_bulk.add(h)
        for ti in cfg["tiny"]:
            v_bulk.update(strips1[ti])
        assert len(v_bulk) == 740
        print(f"\n[H1] Solving Group {sh} ({len(v_bulk)}v) from {u_in} to {u_out}...")
        t_g = time.time()
        path = solve_cluster_path(sh, u_in, u_out, v_bulk, G, max_it=300, verbose=True)
        assert path is not None and len(path) == 740
        solved_h1[sh] = path
        print(f"[H1] Group {sh} SUCCESS in {time.time()-t_g:.2f}s!")
        with open(cache_h1, "w") as f:
            json.dump({str(k): v for k, v in solved_h1.items()}, f)

    # Assemble Half 1:
    half1_path = [3038]
    half1_path.extend(solved_h1[5256])
    half1_path.extend([1992, 5256, 7001])
    half1_path.extend(solved_h1[947])
    half1_path.extend([5309, 1157])
    half1_path.extend(solved_h1[5309])
    half1_path.extend(solved_h1[3038])
    half1_path.append(6022)
    half1_path.extend(solved_h1[4754])
    half1_path.extend([5715, 4754, 947])

    print(f"\nHalf 1 assembled: {len(half1_path)} vertices. Start={half1_path[0]}, End={half1_path[-1]}")
    assert len(half1_path) == 3710 and len(set(half1_path)) == 3710
    assert set(half1_path) == grp1
    for i in range(len(half1_path) - 1):
        assert half1_path[i+1] in G[half1_path[i]], f"Half 1 edge error at {i}: ({half1_path[i]}, {half1_path[i+1]})"
    print(">>> HALF 1 100% CERTIFIED! (3,710 vertices from 3038 to 947) <<<")

    # -------------------------------------------------------------
    # STEP 2: SOLVE HALF 2 GROUPS (5 groups * 740 vertices = 3,700)
    # -------------------------------------------------------------
    print("\n--- STEP 2: Solving Half 2 Groups (740 vertices each) ---")
    all_hubs2, strips2, hh2, strip_adj_hubs2, _ = decompose_half(G, degs, grp2)

    # h2_targets: (super_hub, u_in, u_out, cfg)
    h2_targets = [
        (4934, 5661, 4140, {"clusters": [0, 3, 7, 8, 11], "tiny": [26, 32]}),
        (4227, 6768, 1700, {"clusters": [2, 5, 9, 13, 18], "tiny": [27, 33]}),
        (5244, 798, 3616, {"clusters": [6, 14, 17, 19, 21], "tiny": [25, 35]}),
        (4957, 1378, 6171, {"clusters": [1, 15, 16, 20, 22], "tiny": [28, 34]}),
        (3333, 402, 540, {"clusters": [4, 10, 12, 23, 24], "tiny": [30, 31]}),
    ]

    cache_h2 = "scratch/graph975/half2_group_paths.json"
    solved_h2 = {}
    if os.path.exists(cache_h2):
        with open(cache_h2, "r") as f:
            for k, v in json.load(f).items():
                solved_h2[int(k)] = v
                print(f"Loaded cached path for Group {k}: {len(v)} vertices.")

    for sh, u_in, u_out, cfg in h2_targets:
        if sh in solved_h2 and len(solved_h2[sh]) == 740:
            print(f"Group {sh} already solved (cached), skipping.")
            continue
        v_bulk = set()
        for ci in cfg["clusters"]:
            v_bulk.update(strips2[ci])
            for h in strip_adj_hubs2[ci]:
                if degs[h] != 742: v_bulk.add(h)
        for ti in cfg["tiny"]:
            v_bulk.update(strips2[ti])
        assert len(v_bulk) == 740
        print(f"\n[H2] Solving Group {sh} ({len(v_bulk)}v) from {u_in} to {u_out}...")
        t_g = time.time()
        path = solve_cluster_path(sh, u_in, u_out, v_bulk, G, max_it=300, verbose=True)
        assert path is not None and len(path) == 740
        solved_h2[sh] = path
        print(f"[H2] Group {sh} SUCCESS in {time.time()-t_g:.2f}s!")
        with open(cache_h2, "w") as f:
            json.dump({str(k): v for k, v in solved_h2.items()}, f)

    # Assemble Half 2:
    half2_path = [4957]
    half2_path.extend(solved_h2[4934])
    half2_path.extend([2797, 4934, 4651])
    half2_path.extend(solved_h2[4227])
    half2_path.extend([5244, 6779])
    half2_path.extend(solved_h2[5244])
    half2_path.extend(solved_h2[4957])
    half2_path.append(5668)
    half2_path.extend(solved_h2[3333])
    half2_path.extend([6444, 3333, 4227])

    print(f"\nHalf 2 assembled: {len(half2_path)} vertices. Start={half2_path[0]}, End={half2_path[-1]}")
    assert len(half2_path) == 3710 and len(set(half2_path)) == 3710
    assert set(half2_path) == grp2
    for i in range(len(half2_path) - 1):
        assert half2_path[i+1] in G[half2_path[i]], f"Half 2 edge error at {i}: ({half2_path[i]}, {half2_path[i+1]})"
    print(">>> HALF 2 100% CERTIFIED! (3,710 vertices from 4957 to 4227) <<<")

    # -------------------------------------------------------------
    # STEP 3: STITCHING AND FULL 7,420-VERTEX TOUR CERTIFICATION
    # -------------------------------------------------------------
    print("\n--- STEP 3: Stitching Half 1 and Half 2 across Bridge Edges ---")
    assert 4957 in G[half1_path[-1]], f"Bridge 1 (947, 4957) error!"
    assert 3038 in G[half2_path[-1]], f"Bridge 2 (4227, 3038) error!"

    full_tour = half1_path + half2_path
    print(f"Stitched Tour length: {len(full_tour)} (expected 7,420)")

    print("\n--- STEP 4: Independent Verification on Raw Graph ---")
    assert verify_tour(full_tour, G), "VERIFICATION FAILED!"
    print("*****************************************************************")
    print("*** 100.000% MATHEMATICALLY CERTIFIED HAMILTONIAN CYCLE PASS! ***")
    print("*****************************************************************")

    out_tour = "scratch/graph975/found_tour_graph975.hcp"
    write_hcp(full_tour, out_tour)
    print(f"Tour written to {out_tour}")
    print(f"Total time elapsed: {time.time()-t0:.2f}s")

if __name__ == "__main__":
    main()
