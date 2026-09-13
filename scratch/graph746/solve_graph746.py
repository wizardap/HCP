#!/usr/bin/env python3
"""
5-Cluster Macro-Ring Hierarchical Decomposition Solver for graph746.col (N=4,286, M=18,286)
"""

import time, os, sys, json
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
from scratch.graph746.cluster_decomposer import load_and_decompose_graph746
from scratch.graph746.cluster_path_solver import solve_all_groups
from scratch.graph950.perfect_cluster_assembler import verify_tour, write_hcp

def main():
    print("=================================================================")
    print("      5-CLUSTER PURE SAT DECOMPOSITION SOLVER: GRAPH746.COL      ")
    print("=================================================================")

    col_path = "FHCPCS-col/graph746.col"
    t_start = time.time()
    G, degs, group_targets, macro_nodes, bulks = load_and_decompose_graph746(col_path)
    print(f"Graph loaded & decomposed in {time.time()-t_start:.2f}s: |V|={len(G)}")

    base_dir = os.path.dirname(os.path.abspath(__file__))
    cache_file = os.path.join(base_dir, "group_paths.json")

    print("\n--- STEP 1: Solving/Loading 5 Groups (5 x 855v = 4,275v) ---")
    solved = solve_all_groups(G, degs, group_targets, bulks, cache_file)

    print("\n--- STEP 2: Assembling 16-Stage Macro Hamiltonian Tour ---")
    # Traversal sequence:
    # 1430 -> 3566 -> G_1430(3003 -> 2623) -> G_3790(2165 -> 1264) -> 3692 -> G_3960(1025 -> 3498)
    # -> 3106 -> 3960 -> 3735 -> 2433 -> 3790 -> G_3641(3146 -> 2397) -> 2361 -> 3641 -> 1321
    # -> G_3735(46 -> 3547) -> (closes to 1430)

    full_tour = [1430, 3566]
    full_tour.extend(solved[1430])   # 3003 -> 2623
    full_tour.extend(solved[3790])   # 2165 -> 1264
    full_tour.append(3692)
    full_tour.extend(solved[3960])   # 1025 -> 3498
    full_tour.extend([3106, 3960, 3735, 2433, 3790])
    full_tour.extend(solved[3641])   # 3146 -> 2397
    full_tour.extend([2361, 3641, 1321])
    full_tour.extend(solved[3735])   # 46 -> 3547

    print(f"Assembled Tour length: {len(full_tour)} (expected 4,286)")
    assert len(full_tour) == 4286, f"Length mismatch: {len(full_tour)} != 4286"
    assert len(set(full_tour)) == 4286, "Tour contains duplicate vertices"
    assert set(full_tour) == set(G.keys()), "Tour does not cover all vertices of G"

    # Step 3: Independent Verification on Raw Graph
    print("\n--- STEP 3: Independent Verification on Raw Graph ---")
    assert verify_tour(full_tour, G), "VERIFICATION FAILED!"
    print("*****************************************************************")
    print("*** 100.000% MATHEMATICALLY CERTIFIED HAMILTONIAN CYCLE PASS! ***")
    print("*****************************************************************")

    out_tour = os.path.join(base_dir, "found_tour_graph746.hcp")
    write_hcp(full_tour, out_tour)
    print(f"Tour written to {out_tour}")
    print(f"Total time elapsed: {time.time()-t_start:.2f}s")

if __name__ == "__main__":
    main()
