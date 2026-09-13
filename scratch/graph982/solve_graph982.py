#!/usr/bin/env python3
"""
Two-Half Two-Tier Hierarchical Decomposition Solver for graph982.col (N=7,620, M=33,218)
Solves all 10 group bulk instances (760 vertices each) via CaDiCaL CEGAR,
assembles certified macro-paths for Half 1 (3,810v) and Half 2 (3,810v),
stitches cross-half bridge cuts ((5852, 5022) and (6956, 4740)),
and outputs the certified 7,620-vertex Hamiltonian cycle to scratch/graph982/found_tour_graph982.hcp.
"""

import time, os, sys, json
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
from scratch.graph982.two_half_decomposer import load_and_partition_graph982
from scratch.graph982.cluster_path_solver import solve_all_half_groups
from scratch.graph950.two_half_two_tier_solver import decompose_half
from scratch.graph950.perfect_cluster_assembler import verify_tour, write_hcp

def main():
    print("=================================================================")
    print("      TWO-TIER PURE SAT DECOMPOSITION SOLVER: GRAPH982.COL       ")
    print("=================================================================")

    col_path = "FHCPCS-col/graph982.col"
    t_start = time.time()
    G, degs, grp1, grp2, h1_targets, h2_targets, bridges = load_and_partition_graph982(col_path)
    print(f"Graph loaded & partitioned in {time.time()-t_start:.2f}s: |V|={len(G)}")

    base_dir = os.path.dirname(os.path.abspath(__file__))
    cache_h1 = os.path.join(base_dir, "half1_group_paths.json")
    cache_h2 = os.path.join(base_dir, "half2_group_paths.json")

    # 1. Solve / Load Half 1 groups
    print("\n--- STEP 1: Solving/Loading Half 1 Groups (5 x 760v = 3,800v) ---")
    all_hubs1, strips1, _, strip_adj_hubs1, _ = decompose_half(G, degs, grp1)
    solved_h1 = solve_all_half_groups(G, degs, grp1, h1_targets, strips1, strip_adj_hubs1, cache_h1)

    # Assemble Half 1: 4740 -> 5852 (3,810 vertices)
    half1_path = [4740, 5575, 80]
    half1_path.extend(solved_h1[5575])    # 3111 -> 2162
    half1_path.append(1974)
    half1_path.extend(solved_h1[5852])    # 184 -> 820
    half1_path.extend(solved_h1[1714])    # 6240 -> 1612
    half1_path.extend([6065, 1714])
    half1_path.extend(solved_h1[4740])    # 462 -> 186
    half1_path.extend([3205, 6696, 6405])
    half1_path.extend(solved_h1[6696])    # 2127 -> 3538
    half1_path.append(5852)

    print(f"Half 1 assembled: {len(half1_path)} vertices. Start={half1_path[0]}, End={half1_path[-1]}")
    assert len(half1_path) == 3810, f"Half 1 length mismatch: {len(half1_path)} != 3810"
    assert len(set(half1_path)) == 3810, "Half 1 contains duplicate vertices"
    assert set(half1_path) == grp1, "Half 1 does not cover grp1 exactly"
    for i in range(len(half1_path) - 1):
        u, v = half1_path[i], half1_path[i+1]
        assert v in G[u], f"Half 1 invalid edge at {i}: ({u}, {v})"
    print(">>> HALF 1 100% CERTIFIED! (3,810 vertices from 4740 to 5852) <<<")

    # 2. Solve / Load Half 2 groups
    print("\n--- STEP 2: Solving/Loading Half 2 Groups (5 x 760v = 3,800v) ---")
    all_hubs2, strips2, _, strip_adj_hubs2, _ = decompose_half(G, degs, grp2)
    solved_h2 = solve_all_half_groups(G, degs, grp2, h2_targets, strips2, strip_adj_hubs2, cache_h2)

    # Assemble Half 2: 5022 -> 6956 (3,810 vertices)
    half2_path = [5022, 2907, 2250]
    half2_path.extend(solved_h2[2907])    # 6633 -> 6369
    half2_path.append(4420)
    half2_path.extend(solved_h2[6956])    # 3915 -> 3566
    half2_path.extend(solved_h2[6335])    # 1686 -> 1822
    half2_path.extend([7504, 6335])
    half2_path.extend(solved_h2[5022])    # 1495 -> 6670
    half2_path.extend([7311, 5378, 917])
    half2_path.extend(solved_h2[5378])    # 4392 -> 2164
    half2_path.append(6956)

    print(f"Half 2 assembled: {len(half2_path)} vertices. Start={half2_path[0]}, End={half2_path[-1]}")
    assert len(half2_path) == 3810, f"Half 2 length mismatch: {len(half2_path)} != 3810"
    assert len(set(half2_path)) == 3810, "Half 2 contains duplicate vertices"
    assert set(half2_path) == grp2, "Half 2 does not cover grp2 exactly"
    for i in range(len(half2_path) - 1):
        u, v = half2_path[i], half2_path[i+1]
        assert v in G[u], f"Half 2 invalid edge at {i}: ({u}, {v})"
    print(">>> HALF 2 100% CERTIFIED! (3,810 vertices from 5022 to 6956) <<<")

    # 3. Stitch across bridges
    print("\n--- STEP 3: Stitching Half 1 and Half 2 across Bridge Edges ---")
    assert half2_path[0] in G[half1_path[-1]], f"Bridge edge ({half1_path[-1]}, {half2_path[0]}) missing!"
    assert half1_path[0] in G[half2_path[-1]], f"Bridge edge ({half2_path[-1]}, {half1_path[0]}) missing!"

    full_tour = half1_path + half2_path
    print(f"Stitched Tour length: {len(full_tour)} (expected 7,620)")

    # 4. Independent Verification on Raw Graph
    print("\n--- STEP 4: Independent Verification on Raw Graph ---")
    assert verify_tour(full_tour, G), "VERIFICATION FAILED!"
    print("*****************************************************************")
    print("*** 100.000% MATHEMATICALLY CERTIFIED HAMILTONIAN CYCLE PASS! ***")
    print("*****************************************************************")

    out_tour = os.path.join(base_dir, "found_tour_graph982.hcp")
    write_hcp(full_tour, out_tour)
    print(f"Tour written to {out_tour}")
    print(f"Total time elapsed: {time.time()-t_start:.2f}s")

if __name__ == "__main__":
    main()
