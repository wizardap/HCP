"""
Comprehensive Verification Script for Universal Router-Free HCP Solver.
Evaluates:
1. Suite A: 50 General Graphs (graph1 .. graph50)
2. Suite B: 11 Massive Challenge Graphs (graph710, 717, 746, 788, 882, 944, 950, 963, 975, 982, 990)
Outputs benchmark table, PAR-2 statistics, and 100% soundness verification log.
"""

import os
import sys
import time
from typing import Dict, List, Tuple

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, REPO_ROOT)

from hcp_solver import solve_general_hcp, load_graph, Graph, verify_tour

def run_suite(name: str, graph_ids: List[int], timeout_sec: float = 15.0) -> Dict:
    print("=" * 80)
    print(f"BENCHMARK SUITE: {name} ({len(graph_ids)} instances, timeout={timeout_sec}s)")
    print("=" * 80)
    print(f"{'Graph':<10} {'|V|':<8} {'|E|':<8} {'Time (s)':<12} {'Status':<15}")
    print("-" * 60)

    results = []
    solved_count = 0
    total_time = 0.0

    for gid in graph_ids:
        col_path = os.path.join(REPO_ROOT, "FHCPCS-col", f"graph{gid}.col")
        if not os.path.exists(col_path):
            continue

        adj = load_graph(col_path)
        G = Graph(adj, f"graph{gid}")
        N = G.num_vertices
        M = G.num_edges

        t0 = time.time()
        status = "UNKNOWN"
        try:
            tour = solve_general_hcp(col_path, timeout_sec=timeout_sec, verbose=False)
            dt = time.time() - t0
            ok, msg = verify_tour(tour, G)
            if ok:
                status = "100% SOUND"
                solved_count += 1
                total_time += dt
            else:
                status = f"INVALID ({msg})"
        except Exception as e:
            dt = time.time() - t0
            status = "TIMEOUT"
            total_time += 2 * timeout_sec  # PAR-2 penalty

        print(f"graph{gid:<5} {N:<8} {M:<8} {dt:<12.3f} {status:<15}")
        results.append({
            "id": gid,
            "N": N,
            "M": M,
            "time": dt,
            "status": status
        })

    print("-" * 60)
    par2 = total_time / len(results) if results else 0.0
    print(f"[*] Summary for {name}: {solved_count}/{len(results)} solved (Solve Rate: {solved_count/len(results)*100:.1f}%)")
    print(f"[*] PAR-2 Average: {par2:.3f}s")
    print()

    return {
        "name": name,
        "total": len(results),
        "solved": solved_count,
        "par2": par2,
        "results": results
    }

def main():
    # 1. Suite A: Graphs 1 to 50
    suite_a_ids = list(range(1, 51))
    suite_a_res = run_suite("General Benchmark Suite (graph1 .. graph50)", suite_a_ids, timeout_sec=10.0)

    # 2. Suite B: 11 Challenge Graphs
    suite_b_ids = [710, 717, 746, 788, 882, 944, 950, 963, 975, 982, 990]
    suite_b_res = run_suite("11 Massive Challenge Graphs (|V| >= 4000)", suite_b_ids, timeout_sec=30.0)

if __name__ == "__main__":
    main()
