#!/usr/bin/env python3
"""
Comprehensive 100-Graph Benchmark Runner for Cegar-Fix
Benchmarks 100 representative graphs across all size tiers of FHCP dataset:
graph10, graph20, graph30, ..., graph1000
with 15s timeout per instance.
"""

import os
import sys
import time
import subprocess
import json

GRAPH_DIR = "FHCPCS-col"
BINARY = "./src/cegar-fix/target/release/cegar-fix"
TIMEOUT_SEC = 15

def get_target_graphs():
    graphs = []
    for i in range(10, 1001, 10):
        name = f"graph{i}"
        col_path = os.path.join(GRAPH_DIR, f"{name}.col")
        if os.path.exists(col_path):
            graphs.append((name, col_path))
    return graphs

def run_benchmark():
    target_graphs = get_target_graphs()
    print(f"=========================================================================")
    print(f"         FHCPCS 100-GRAPH COMPREHENSIVE BENCHMARK (15s Timeout)")
    print(f"=========================================================================")
    print(f"Total target graphs: {len(target_graphs)}")
    print(f"Binary: {BINARY}")
    print(f"Timeout per graph: {TIMEOUT_SEC}s")
    print(f"{'#':<4} | {'Graph':<10} | {'Status':<15} | {'Time (s)':<10} | {'Increments':<12} | {'Notes'}")
    print("-" * 75)

    results = []
    solved_count = 0
    timeout_count = 0
    error_count = 0
    total_time = 0.0

    for idx, (name, col_path) in enumerate(target_graphs, 1):
        cmd = [
            BINARY,
            "-i", col_path,
            "-e", "1",
            "-b", "3",
            "-y", "0",
            "-t", "3",
            "-l", "1",
            "--three-opt", "1"
        ]

        t0 = time.time()
        try:
            res = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=TIMEOUT_SEC
            )
            elapsed = time.time() - t0
            total_time += elapsed

            stdout = res.stdout
            if "s SATISFIABLE" in stdout:
                status = "SATISFIABLE"
                solved_count += 1
            elif "s UNSATISFIABLE" in stdout:
                status = "UNSATISFIABLE"
                error_count += 1
            else:
                status = f"EXIT_{res.returncode}"
                error_count += 1

            increments = "?"
            for line in stdout.splitlines():
                if "overall incremented number =" in line:
                    increments = line.split("=")[-1].strip()
                elif "incremented number =" in line and increments == "?":
                    increments = line.split("=")[-1].strip()

            notes = ""
            if "stem-and-cycle" in stdout or "StemCycle" in stdout:
                notes += "[StemCycle] "
            if "Modular macro-decomposition" in stdout or "ModularSolver" in stdout:
                notes += "[Modular] "
            if "added cluster cut" in stdout:
                notes += "[ClusterCut] "

            print(f"{idx:<4} | {name:<10} | {status:<15} | {elapsed:<10.3f} | {increments:<12} | {notes}")
            results.append({
                "index": idx,
                "graph": name,
                "status": status,
                "time": elapsed,
                "increments": increments,
                "notes": notes
            })

        except subprocess.TimeoutExpired:
            elapsed = TIMEOUT_SEC
            total_time += elapsed
            status = "TIMEOUT"
            timeout_count += 1
            increments = "N/A"
            notes = f">{TIMEOUT_SEC}s"
            print(f"{idx:<4} | {name:<10} | {status:<15} | {elapsed:<10.3f} | {increments:<12} | {notes}")
            results.append({
                "index": idx,
                "graph": name,
                "status": status,
                "time": elapsed,
                "increments": increments,
                "notes": notes
            })
        except Exception as e:
            elapsed = time.time() - t0
            total_time += elapsed
            status = "ERROR"
            error_count += 1
            print(f"{idx:<4} | {name:<10} | {status:<15} | {elapsed:<10.3f} | N/A          | {e}")
            results.append({
                "index": idx,
                "graph": name,
                "status": status,
                "time": elapsed,
                "increments": "N/A",
                "notes": str(e)
            })

    pass_rate = (solved_count / len(target_graphs)) * 100.0
    print("=" * 75)
    print("                      BENCHMARK SUMMARY")
    print("=" * 75)
    print(f"Total Graphs:    {len(target_graphs)}")
    print(f"Solved (SAT):    {solved_count} ({pass_rate:.1f}%)")
    print(f"Timeouts (>15s): {timeout_count}")
    print(f"Errors/UNSAT:    {error_count}")
    print(f"Total Time:      {total_time:.2f}s (avg {total_time/len(target_graphs):.3f}s/graph)")
    print("=" * 75)

    # Save results as JSON
    out_json = "scratch/benchmark_100_results.json"
    with open(out_json, "w") as f:
        json.dump({
            "total": len(target_graphs),
            "solved": solved_count,
            "timeout": timeout_count,
            "errors": error_count,
            "pass_rate_pct": pass_rate,
            "total_time_sec": total_time,
            "results": results
        }, f, indent=2)
    print(f"Detailed results saved to {out_json}")

if __name__ == "__main__":
    run_benchmark()
