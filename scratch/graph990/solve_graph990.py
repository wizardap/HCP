#!/usr/bin/env python3
"""
Two-Half Two-Tier Hierarchical Decomposition Solver for graph990.col (N=8,020, M=35,018)
"""

import time, os, sys, json
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
from scratch.graph990.two_half_decomposer import load_and_partition_graph990
from scratch.graph990.cluster_path_solver import solve_all_half_groups
from scratch.graph950.two_half_two_tier_solver import decompose_half
from scratch.graph950.perfect_cluster_assembler import verify_tour, write_hcp

def main():
    print("=================================================================")
    print("      TWO-TIER PURE SAT DECOMPOSITION SOLVER: GRAPH990.COL       ")
    print("=================================================================")

    col_path = "FHCPCS-col/graph990.col"
    t_start = time.time()
    G, degs, grp1, grp2, h1_targets, h2_targets, bridges = load_and_partition_graph990(col_path)
    print(f"Graph loaded & partitioned in {time.time()-t_start:.2f}s: |V|={len(G)}")

    base_dir = os.path.dirname(os.path.abspath(__file__))
    cache_h1 = os.path.join(base_dir, "half1_group_paths.json")
    cache_h2 = os.path.join(base_dir, "half2_group_paths.json")

    # 1. Load / Solve Half 1 groups
    print("\n--- STEP 1: Solving/Loading Half 1 Groups (5 x 800v = 4,000v) ---")
    all_hubs1, strips1, _, strip_adj_hubs1, _ = decompose_half(G, degs, grp1)
    solved_h1 = solve_all_half_groups(G, degs, grp1, h1_targets, strips1, strip_adj_hubs1, cache_h1)

    # Assemble Half 1: 3517 -> 3728 (4,010 vertices)
    half1_path = [3517]
    half1_path.extend(solved_h1[5293])    # 7644 -> 5381
    half1_path.extend([4974, 5293, 5263])
    half1_path.extend(solved_h1[3728])    # 3862 -> 3071
    half1_path.extend([3076, 78])
    half1_path.extend(solved_h1[3076])    # 3494 -> 4316
    half1_path.extend(solved_h1[3517])    # 3455 -> 3729
    half1_path.append(1225)
    half1_path.extend(solved_h1[6726])    # 3391 -> 6248
    half1_path.extend([2062, 6726, 3728])

    print(f"Half 1 assembled: {len(half1_path)} vertices. Start={half1_path[0]}, End={half1_path[-1]}")
    assert len(half1_path) == 4010, f"Half 1 length mismatch: {len(half1_path)} != 4010"
    assert len(set(half1_path)) == 4010, "Half 1 contains duplicate vertices"
    assert set(half1_path) == grp1, "Half 1 does not cover grp1 exactly"
    for i in range(len(half1_path) - 1):
        u, v = half1_path[i], half1_path[i+1]
        assert v in G[u], f"Half 1 invalid edge at {i}: ({u}, {v})"
    print(">>> HALF 1 100% CERTIFIED! (4,010 vertices from 3517 to 3728) <<<")

    # 2. Load / Solve Half 2 groups
    print("\n--- STEP 2: Solving/Loading Half 2 Groups (5 x 800v = 4,000v) ---")
    all_hubs2, strips2, _, strip_adj_hubs2, _ = decompose_half(G, degs, grp2)
    solved_h2 = solve_all_half_groups(G, degs, grp2, h2_targets, strips2, strip_adj_hubs2, cache_h2)

    # Assemble Half 2: 4178 -> 7858 (4,010 vertices)
    half2_path = [4178]
    half2_path.extend(solved_h2[2205])    # 304 -> 1029
    half2_path.extend([6262, 2205, 3950])
    half2_path.extend(solved_h2[7858])    # 1272 -> 6331
    half2_path.extend([3905, 6068])
    half2_path.extend(solved_h2[3905])    # 4011 -> 5747
    half2_path.extend(solved_h2[4178])    # 340 -> 4340
    half2_path.append(1040)
    half2_path.extend(solved_h2[7717])    # 1121 -> 7436
    half2_path.extend([567, 7717, 7858])

    print(f"Half 2 assembled: {len(half2_path)} vertices. Start={half2_path[0]}, End={half2_path[-1]}")
    assert len(half2_path) == 4010, f"Half 2 length mismatch: {len(half2_path)} != 4010"
    assert len(set(half2_path)) == 4010, "Half 2 contains duplicate vertices"
    assert set(half2_path) == grp2, "Half 2 does not cover grp2 exactly"
    for i in range(len(half2_path) - 1):
        u, v = half2_path[i], half2_path[i+1]
        assert v in G[u], f"Half 2 invalid edge at {i}: ({u}, {v})"
    print(">>> HALF 2 100% CERTIFIED! (4,010 vertices from 4178 to 7858) <<<")

    # 3. Stitch across bridges
    print("\n--- STEP 3: Stitching Half 1 and Half 2 across Bridge Edges ---")
    assert half2_path[0] in G[half1_path[-1]], f"Bridge edge ({half1_path[-1]}, {half2_path[0]}) missing!"
    assert half1_path[0] in G[half2_path[-1]], f"Bridge edge ({half2_path[-1]}, {half1_path[0]}) missing!"

    full_tour = half1_path + half2_path
    print(f"Stitched Tour length: {len(full_tour)} (expected 8,020)")

    # 4. Independent Verification on Raw Graph
    print("\n--- STEP 4: Independent Verification on Raw Graph ---")
    assert verify_tour(full_tour, G), "VERIFICATION FAILED!"
    print("*****************************************************************")
    print("*** 100.000% MATHEMATICALLY CERTIFIED HAMILTONIAN CYCLE PASS! ***")
    print("*****************************************************************")

    out_tour = os.path.join(base_dir, "found_tour_graph990.hcp")
    write_hcp(full_tour, out_tour)
    print(f"Tour written to {out_tour}")
    print(f"Total time elapsed: {time.time()-t_start:.2f}s")

if __name__ == "__main__":
    main()
