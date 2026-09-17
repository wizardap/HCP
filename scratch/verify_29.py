#!/usr/bin/env python3
"""
Official 29-Graph Benchmark Verifier & Status Tracker for HCP Solver.

Target Set:
  710, 717, 746, 788, 832, 868, 882, 937, 944, 950,
  951, 954, 959, 960, 963, 965, 966, 971, 974, 975,
  976, 981, 982, 983, 987, 990, 993, 994, 998
"""

import os
import sys
import glob

REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))

TARGET_29 = [
    710, 717, 746, 788, 832, 868, 882, 937, 944, 950,
    951, 954, 959, 960, 963, 965, 966, 971, 974, 975,
    976, 981, 982, 983, 987, 990, 993, 994, 998
]

# Canonical tour paths for known solved graphs
CANONICAL_TOURS = {
    710: "scratch/graph710/found_tour_graph710.hcp",
    717: "scratch/graph717/found_tour_graph717.hcp",
    746: "scratch/graph746/found_tour_graph746.hcp",
    788: "scratch/graph788/found_tour_graph788.hcp",
    882: "scratch/graph882/found_tour_graph882.hcp",
    944: "scratch/engine/found_tour_graph944.hcp",
    950: "scratch/graph950/found_tour_puresat.hcp",
    963: "scratch/graph963/found_tour_graph963.hcp",
    975: "scratch/graph975/found_tour_graph975.hcp",
    982: "scratch/graph982/found_tour_graph982.hcp",
    990: "scratch/graph990/found_tour_graph990.hcp",
}

def parse_col(col_path):
    adj = {}
    with open(col_path, "r") as f:
        for line in f:
            if line.startswith("e "):
                p = line.split()
                u, v = int(p[1]), int(p[2])
                if u not in adj: adj[u] = set()
                if v not in adj: adj[v] = set()
                adj[u].add(v)
                adj[v].add(u)
    return len(adj), sum(len(v) for v in adj.values()) // 2, adj

def parse_tour(tour_path):
    tour = []
    in_tour = False
    with open(tour_path, "r") as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("c") or line.startswith("NAME") or line.startswith("TYPE") or line.startswith("DIMENSION"):
                continue
            if line == "TOUR_SECTION":
                in_tour = True
                continue
            if line == "-1" or line == "EOF":
                break
            if in_tour:
                tour.append(int(line))
    return tour

def verify_tour(tour, n_verts, adj):
    if len(tour) != n_verts:
        return False, f"Length {len(tour)} != {n_verts}"
    seen = set()
    for v in tour:
        if v in seen:
            return False, f"Duplicate vertex {v}"
        if v not in adj:
            return False, f"Invalid vertex {v}"
        seen.add(v)
    for i in range(len(tour)):
        u = tour[i]
        v = tour[(i + 1) % len(tour)]
        if v not in adj[u]:
            return False, f"Missing edge ({u}, {v})"
    return True, "SOUND"

def main():
    print("=" * 80)
    print("OFFICIAL 29-GRAPH BENCHMARK VERIFICATION & CERTIFICATION")
    print("=" * 80)
    print(f"{'Graph':<10} {'|V|':<7} {'|E|':<8} {'Status':<18} {'Details / Tour Path'}")
    print("-" * 80)

    solved_count = 0
    unsolved_count = 0

    for gid in sorted(TARGET_29):
        col_rel = f"FHCPCS-col/graph{gid}.col"
        col_path = os.path.join(REPO_ROOT, col_rel)
        if not os.path.exists(col_path):
            print(f"graph{gid:<5} {'N/A':<7} {'N/A':<8} {'NOT FOUND':<18} {col_rel}")
            continue

        n_v, n_e, adj = parse_col(col_path)

        # Check for tour
        tour_rel = CANONICAL_TOURS.get(gid)
        tour_path = os.path.join(REPO_ROOT, tour_rel) if tour_rel else None

        # Also search dynamically if canonical not set or doesn't exist
        if not tour_path or not os.path.exists(tour_path):
            cand = glob.glob(os.path.join(REPO_ROOT, f"scratch/**/found_tour*graph{gid}*.hcp"), recursive=True)
            cand += glob.glob(os.path.join(REPO_ROOT, f"scratch/**/found_tour_{gid}*.hcp"), recursive=True)
            if cand:
                tour_path = cand[0]
                tour_rel = os.path.relpath(tour_path, REPO_ROOT)

        if tour_path and os.path.exists(tour_path):
            tour = parse_tour(tour_path)
            ok, msg = verify_tour(tour, n_v, adj)
            if ok:
                solved_count += 1
                status = "PASS (SOUND)"
                print(f"graph{gid:<5} {n_v:<7} {n_e:<8} {status:<18} {tour_rel}")
            else:
                unsolved_count += 1
                status = f"FAIL ({msg})"
                print(f"graph{gid:<5} {n_v:<7} {n_e:<8} {status:<18} {tour_rel}")
        else:
            unsolved_count += 1
            status = "UNSOLVED"
            print(f"graph{gid:<5} {n_v:<7} {n_e:<8} {status:<18} (No tour generated)")

    print("-" * 80)
    pct = (solved_count / len(TARGET_29)) * 100
    print(f"Summary: {solved_count}/{len(TARGET_29)} Solved ({pct:.1f}%), {unsolved_count} Remaining.")
    print("=" * 80)

if __name__ == "__main__":
    main()
