#!/usr/bin/env python3
"""
Independent 100% Tour Verifier for FHCPCS-col/graph788.col
Validates:
1. Exact vertex count: 4,620
2. Exact vertex uniqueness (permutation of 1..4620)
3. Every single edge (all 4,620 edges) exists in the original benchmark graph788.col
4. No heuristic input, pure independent ground-truth verification
"""

import sys
import os

def verify_tour(col_path, tour_path):
    print("=================================================================")
    print("   INDEPENDENT TOUR VERIFICATION: GRAPH788.COL                  ")
    print("=================================================================")
    
    if not os.path.exists(col_path):
        print(f"Error: Graph file {col_path} not found!")
        sys.exit(1)
        
    if not os.path.exists(tour_path):
        print(f"Error: Tour file {tour_path} not found!")
        sys.exit(1)

    print(f"1. Loading benchmark graph {col_path}...")
    adj = {}
    total_edges = 0
    with open(col_path, 'r') as f:
        for line in f:
            line = line.strip()
            if line.startswith('p '):
                parts = line.split()
                n = int(parts[2]) if len(parts) >= 4 else int(parts[1])
                for v in range(1, n + 1):
                    adj[v] = set()
            elif line.startswith('e '):
                parts = line.split()
                u, v = int(parts[1]), int(parts[2])
                if u not in adj: adj[u] = set()
                if v not in adj: adj[v] = set()
                adj[u].add(v)
                adj[v].add(u)
                total_edges += 1

    n_vertices = len(adj)
    print(f"   Graph loaded: N={n_vertices}, M={total_edges}")
    assert n_vertices == 4620, f"Expected 4620 vertices, got {n_vertices}"

    print(f"2. Parsing TSPLIB tour from {tour_path}...")
    tour = []
    in_tour_section = False
    with open(tour_path, 'r') as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            if line == 'TOUR_SECTION':
                in_tour_section = True
                continue
            if in_tour_section:
                if line == '-1' or line == 'EOF':
                    break
                try:
                    v = int(line)
                    if v > 0:
                        tour.append(v)
                except ValueError:
                    continue

    print(f"   Parsed tour length: {len(tour)} vertices")
    assert len(tour) == n_vertices, f"Tour length mismatch: expected {n_vertices}, got {len(tour)}"

    print("3. Validating vertex uniqueness...")
    tour_set = set(tour)
    assert len(tour_set) == n_vertices, f"Duplicate vertices found! Unique count: {len(tour_set)}"
    expected_set = set(range(1, n_vertices + 1))
    assert tour_set == expected_set, "Tour does not contain exact set of vertices 1..N"
    print("   All 4,620 vertices visited exactly once!")

    print("4. Validating all 4,620 edges in the benchmark graph...")
    invalid_edges = []
    for i in range(len(tour)):
        u = tour[i]
        v = tour[(i + 1) % len(tour)]
        if v not in adj[u]:
            invalid_edges.append((i, u, v))

    if invalid_edges:
        print(f"FAIL: {len(invalid_edges)} invalid edges found in tour!")
        for idx, u, v in invalid_edges[:10]:
            print(f"   Edge #{idx}: ({u}, {v}) NOT in graph!")
        sys.exit(1)

    print(f"   ALL {len(tour)} EDGES INDEPENDENTLY VALIDATED IN G!")
    print("=================================================================")
    print(">>> [PASS] 100% INDEPENDENT TOUR VERIFICATION CERTIFIED! <<<")
    print("=================================================================")

if __name__ == '__main__':
    col_file = 'FHCPCS-col/graph788.col' if len(sys.argv) < 2 else sys.argv[1]
    tour_file = 'scratch/graph788/found_tour_graph788.hcp' if len(sys.argv) < 3 else sys.argv[2]
    verify_tour(col_file, tour_file)
