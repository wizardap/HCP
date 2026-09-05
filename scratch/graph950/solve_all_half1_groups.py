#!/usr/bin/env python3
"""
Solve all 5 groups of Half 1 using exact CEGAR SAT solver on 660-vertex bulk instances,
then splice with the macro physical sequence to produce a certified 3,310-vertex Hamiltonian path from 164 to 5835.
"""

import time, collections, json, os
from scratch.graph950.perfect_cluster_assembler import solve_cluster_path
from scratch.graph950.two_half_two_tier_solver import load_graph, partition_halves, decompose_half

def main():
    print("=== STEP 1: Load Graph and Half 1 Partition ===")
    t0 = time.time()
    G, degs = load_graph("FHCPCS-col/graph950.col")
    grp1, grp2 = partition_halves(G, degs)
    all_hubs, strips, hh_edges, strip_adj_hubs, hub_adj_strips = decompose_half(G, degs, grp1)
    print(f"Graph loaded and partitioned in {time.time()-t0:.2f}s. Half 1 has {len(grp1)} vertices.")

    group_targets = [
        (164, 3944, 6285, {"clusters": [18, 2, 3, 15, 6], "tiny": [27, 31]}),
        (5787, 4655, 5666, {"clusters": [0, 21, 17, 5, 24], "tiny": [28, 35]}),
        (5835, 902, 3206, {"clusters": [12, 13, 20, 14, 22], "tiny": [26, 36]}),
        (4785, 6341, 433, {"clusters": [1, 8, 7, 9, 16], "tiny": [30, 33]}),
        (4000, 2594, 6541, {"clusters": [10, 4, 23, 19, 11], "tiny": [25, 32]}),
    ]

    solved_paths = {}
    
    # Check if any paths are already cached:
    cache_file = "scratch/graph950/half1_group_paths.json"
    if os.path.exists(cache_file):
        with open(cache_file, "r") as f:
            cached = json.load(f)
            for k, v in cached.items():
                solved_paths[int(k)] = v
                print(f"Loaded cached path for Group {k}: {len(v)} vertices.")

    for sh, u_in, u_out, cfg in group_targets:
        if sh in solved_paths and len(solved_paths[sh]) == 660:
            print(f"Group {sh} already solved (cached), skipping.")
            continue

        v_bulk = set()
        for ci in cfg["clusters"]:
            v_bulk.update(strips[ci])
            for h in strip_adj_hubs[ci]:
                if degs[h] != 662:
                    v_bulk.add(h)
        for ti in cfg["tiny"]:
            v_bulk.update(strips[ti])
            
        assert len(v_bulk) == 660, f"Group {sh} bulk size {len(v_bulk)} != 660!"
        print(f"\n--- Solving Group {sh} bulk ({len(v_bulk)} vertices) from {u_in} to {u_out} ---")
        t_g = time.time()
        path = solve_cluster_path(sh, u_in, u_out, v_bulk, G, max_it=300, verbose=True)
        if not path:
            print(f"FAILED to solve Group {sh}!")
            return
        assert len(path) == 660
        assert path[0] == u_in and path[-1] == u_out
        solved_paths[sh] = path
        print(f"SUCCESS: Group {sh} solved in {time.time()-t_g:.2f}s!")
        
        # Save cache:
        with open(cache_file, "w") as f:
            json.dump({str(k): v for k, v in solved_paths.items()}, f)

    print("\n=== STEP 2: Assemble Full Half 1 Hamiltonian Path ===")
    # Splicing according to the certified macro sequence:
    # 164 -> 5787 -> 1942 -> G5787 (4655..5666) -> 2492 -> G5835 (902..3206) ->
    # G4785 (6341..433) -> 6454 -> 4785 -> G164 (3944..6285) -> 5036 -> 4000 -> 6014 ->
    # G4000 (2594..6541) -> 5835
    
    half1_path = [164, 5787, 1942]
    half1_path.extend(solved_paths[5787])
    half1_path.append(2492)
    half1_path.extend(solved_paths[5835])
    half1_path.extend(solved_paths[4785])
    half1_path.extend([6454, 4785])
    half1_path.extend(solved_paths[164])
    half1_path.extend([5036, 4000, 6014])
    half1_path.extend(solved_paths[4000])
    half1_path.append(5835)

    print(f"\nAssembly complete! Total vertices in Half 1 path: {len(half1_path)}")
    print(f"Start vertex: {half1_path[0]} (expected 164)")
    print(f"End vertex: {half1_path[-1]} (expected 5835)")
    
    print("\n=== STEP 3: Rigorous Mathematical Verification of Half 1 ===")
    assert len(half1_path) == 3310, f"Length {len(half1_path)} != 3310"
    assert len(set(half1_path)) == 3310, f"Duplicate vertices found! ({len(set(half1_path))} unique)"
    assert set(half1_path) == grp1, "Vertex set does not exactly match Half 1 partition!"
    
    for i in range(len(half1_path) - 1):
        u, v = half1_path[i], half1_path[i+1]
        assert v in G[u], f"Edge violation at step {i}: ({u}, {v}) NOT in E(G)!"
    print("ALL 3,309 CONSECUTIVE EDGES ARE 100% VALID EDGES IN G!")
    print("HALF 1 IS 100% SOLVED AND CERTIFIED!")
    
    with open("scratch/graph950/half1_certified_path.json", "w") as f:
        json.dump(half1_path, f)
    print("Saved certified path to scratch/graph950/half1_certified_path.json")

if __name__ == "__main__":
    main()
