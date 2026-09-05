#!/usr/bin/env python3
"""
Solve all 5 groups of Half 2 using exact CEGAR SAT solver on 660-vertex bulk instances,
then splice with the macro physical sequence to produce a certified 3,310-vertex Hamiltonian path from 5540 to 6080.
Finally, stitch Half 1 and Half 2 to produce the certified 6,620-vertex Hamiltonian Tour for graph950.col!
"""

import time, collections, json, os
from scratch.graph950.perfect_cluster_assembler import solve_cluster_path, verify_tour, write_hcp
from scratch.graph950.two_half_two_tier_solver import load_graph, partition_halves, decompose_half

def main():
    print("=== STEP 1: Load Graph and Half 2 Partition ===")
    t0 = time.time()
    G, degs = load_graph("FHCPCS-col/graph950.col")
    grp1, grp2 = partition_halves(G, degs)
    all_hubs, strips, hh_edges, strip_adj_hubs, hub_adj_strips = decompose_half(G, degs, grp2)
    print(f"Graph loaded and partitioned in {time.time()-t0:.2f}s. Half 2 has {len(grp2)} vertices.")

    group_targets = [
        (2803, 2317, 388, {"clusters": [17, 18, 20, 21, 22], "tiny": [26, 33]}),
        (6080, 4808, 4713, {"clusters": [1, 3, 8, 16, 19], "tiny": [27, 34]}),
        (1171, 3407, 2189, {"clusters": [0, 5, 10, 15, 24], "tiny": [29, 32]}),
        (5540, 4613, 5710, {"clusters": [2, 4, 6, 7, 9], "tiny": [25, 36]}),
        (4540, 1648, 4833, {"clusters": [11, 12, 13, 14, 23], "tiny": [28, 35]}),
    ]

    solved_paths = {}
    cache_file = "scratch/graph950/half2_group_paths.json"
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
        
        with open(cache_file, "w") as f:
            json.dump({str(k): v for k, v in solved_paths.items()}, f)

    print("\n=== STEP 2: Assemble Full Half 2 Hamiltonian Path ===")
    # 5540 -> 2803 -> 5013 -> G2803 (2317..388) -> 764 -> G6080 (4808..4713) ->
    # G1171 (3407..2189) -> 4319 -> 1171 -> G5540 (4613..5710) -> 5749 -> 4540 ->
    # 3679 -> G4540 (1648..4833) -> 6080
    
    half2_path = [5540, 2803, 5013]
    half2_path.extend(solved_paths[2803])
    half2_path.append(764)
    half2_path.extend(solved_paths[6080])
    half2_path.extend(solved_paths[1171])
    half2_path.extend([4319, 1171])
    half2_path.extend(solved_paths[5540])
    half2_path.extend([5749, 4540, 3679])
    half2_path.extend(solved_paths[4540])
    half2_path.append(6080)

    print(f"\nAssembly complete! Total vertices in Half 2 path: {len(half2_path)}")
    print(f"Start vertex: {half2_path[0]} (expected 5540)")
    print(f"End vertex: {half2_path[-1]} (expected 6080)")
    
    print("\n=== STEP 3: Rigorous Mathematical Verification of Half 2 ===")
    assert len(half2_path) == 3310, f"Length {len(half2_path)} != 3310"
    assert len(set(half2_path)) == 3310, f"Duplicate vertices found! ({len(set(half2_path))} unique)"
    assert set(half2_path) == grp2, "Vertex set does not exactly match Half 2 partition!"
    
    for i in range(len(half2_path) - 1):
        u, v = half2_path[i], half2_path[i+1]
        assert v in G[u], f"Edge violation at step {i}: ({u}, {v}) NOT in E(G)!"
    print("ALL 3,309 CONSECUTIVE EDGES ARE 100% VALID EDGES IN G!")
    print("HALF 2 IS 100% SOLVED AND CERTIFIED!")
    
    with open("scratch/graph950/half2_certified_path.json", "w") as f:
        json.dump(half2_path, f)

    print("\n=== STEP 4: Stitch Half 1 and Half 2 into Complete Tour ===")
    with open("scratch/graph950/half1_certified_path.json", "r") as f:
        half1_path = json.load(f)

    # Bridge edges: (5835, 5540) and (6080, 164)
    assert 5540 in G[half1_path[-1]], f"Bridge edge ({half1_path[-1]}, 5540) not in G!"
    assert 164 in G[half2_path[-1]], f"Bridge edge ({half2_path[-1]}, 164) not in G!"
    
    full_tour = half1_path + half2_path
    print(f"Stitched tour length: {len(full_tour)} (expected 6620)")
    
    print("\n=== STEP 5: Final Independent Verification of Complete Tour ===")
    assert verify_tour(full_tour, G), "VERIFICATION FAILED!"
    print("*************************************************************")
    print("*** MATHEMATICAL PROOF & VERIFICATION PASSED 100.000% !! ***")
    print("*************************************************************")
    
    out_hcp = "scratch/graph950/found_tour_puresat.hcp"
    write_hcp(full_tour, out_hcp)
    print(f"Certified tour written to {out_hcp}")

if __name__ == "__main__":
    main()
