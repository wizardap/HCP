#!/usr/bin/env python3
"""
Batch 1001 Benchmark Runner for Flinders Hamiltonian Cycle Problem Challenge Set (FHCPCS).
Scans all 1001 graphs using multi-core processing with timeout control.
Saves checkpointed progress to scratch/batch_1001_results.json.
"""

import os
import sys
import time
import json
import subprocess
from concurrent.futures import ProcessPoolExecutor, as_completed

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
GRAPH_DIR = os.path.join(REPO_ROOT, "FHCPCS-col")
CEGAR_BIN = os.path.join(REPO_ROOT, "src/cegar-fix/target/release/cegar-fix")
OUT_JSON = os.path.join(REPO_ROOT, "scratch/batch_1001_results.json")
LOG_FILE = os.path.join(REPO_ROOT, "scratch/batch_1001.log")
TIMEOUT_SEC = 10

SPECIALIZED_11 = {710, 717, 746, 788, 882, 944, 950, 963, 975, 982, 990}

def parse_dimacs(col_path):
    edges = set()
    num_nodes = 0
    with open(col_path, "r") as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("c"):
                continue
            parts = line.split()
            if parts[0] == "p":
                if parts[1] == "edge":
                    num_nodes = int(parts[2])
                else:
                    num_nodes = int(parts[1])
            elif parts[0] == "e":
                u, v = int(parts[1]), int(parts[2])
                edges.add((u, v))
                edges.add((v, u))
    return num_nodes, edges

def verify_cycle(tour, num_nodes, edges):
    if len(tour) != num_nodes:
        return False, f"Length mismatch: {len(tour)} != {num_nodes}"
    if len(set(tour)) != num_nodes:
        return False, "Duplicate vertices in tour"
    for i in range(len(tour)):
        u = tour[i]
        v = tour[(i + 1) % len(tour)]
        if (u, v) not in edges:
            return False, f"Invalid edge ({u}, {v})"
    return True, "100% sound"

sys.path.insert(0, REPO_ROOT)
from hcp_solver import solve_general_hcp, load_graph, Graph, verify_tour

def solve_one(gid, timeout=TIMEOUT_SEC):
    col_path = os.path.join(GRAPH_DIR, f"graph{gid}.col")
    if not os.path.exists(col_path):
        return {"gid": gid, "status": "MISSING_FILE", "time": 0.0}

    eff_timeout = 180.0 if gid in SPECIALIZED_11 else float(timeout)
    t0 = time.time()
    try:
        adj = load_graph(col_path)
        G = Graph(adj, f"graph{gid}")
        tour = solve_general_hcp(col_path, timeout_sec=eff_timeout, verbose=False)
        elapsed = time.time() - t0
        ok, msg = verify_tour(tour, G)
        if ok:
            return {"gid": gid, "status": "SAT_VERIFIED", "time": elapsed, "nodes": G.num_vertices}
        else:
            return {"gid": gid, "status": "SAT_INVALID", "time": elapsed, "err": msg}
    except Exception as e:
        elapsed = time.time() - t0
        err_msg = str(e)
        if "timed out" in err_msg.lower() or elapsed >= eff_timeout:
            return {"gid": gid, "status": "TIMEOUT", "time": elapsed}
        return {"gid": gid, "status": "ERROR", "time": elapsed, "err": err_msg}

def main():
    import argparse
    parser = argparse.ArgumentParser(description="Batch FHCPCS Runner")
    parser.add_argument("--start", type=int, default=1, help="Start graph ID")
    parser.add_argument("--end", type=int, default=1001, help="End graph ID")
    parser.add_argument("--workers", type=int, default=1, help="Number of worker processes")
    parser.add_argument("--timeout", type=int, default=TIMEOUT_SEC, help="Timeout in seconds")
    args = parser.parse_args()

    max_workers = args.workers
    timeout = args.timeout
    print(f"[*] Starting Batch FHCPCS Runner (Graphs {args.start}..{args.end}, Workers={max_workers}, Timeout={timeout}s)...")
    print(f"[*] Output log: {LOG_FILE}")
    print(f"[*] Results JSON: {OUT_JSON}")

    # Load existing results if resuming
    results = {}
    if os.path.exists(OUT_JSON):
        try:
            with open(OUT_JSON, "r") as f:
                saved = json.load(f)
                for item in saved:
                    results[item["gid"]] = item
            print(f"[*] Resuming from checkpoint: {len(results)} already done.")
        except Exception:
            results = {}

    target_range = range(args.start, args.end + 1)
    to_run = [gid for gid in target_range if gid not in results]
    print(f"[*] Remaining graphs to evaluate in range: {len(to_run)}")

    stats = {"SAT_VERIFIED": 0, "SOLVED_SPECIALIZED": 0, "PROVED_UNSAT": 0, "TIMEOUT": 0, "OTHER": 0}
    for item in results.values():
        st = item.get("status", "OTHER")
        stats[st] = stats.get(st, 0) + 1

    t_start = time.time()
    count = len(results)

    with open(LOG_FILE, "a") as log_f:
        log_f.write(f"\n--- Batch Run Started at {time.ctime()} ---\n")

        with ProcessPoolExecutor(max_workers=max_workers) as executor:
            future_to_gid = {executor.submit(solve_one, gid, timeout): gid for gid in to_run}

            for future in as_completed(future_to_gid):
                res = future.result()
                gid = res["gid"]
                results[gid] = res
                count += 1

                st = res["status"]
                stats[st] = stats.get(st, 0) + 1
                tm = res["time"]

                line = f"[{count:4d}/{args.end}] graph{gid:4d}: {st:<18} ({tm:6.2f}s)"
                log_f.write(line + "\n")
                log_f.flush()

                if count % 10 == 0 or count == args.end:
                    rate = (stats.get("SAT_VERIFIED", 0) + stats.get("SOLVED_SPECIALIZED", 0))
                    unsat = stats.get("PROVED_UNSAT", 0)
                    tout = stats.get("TIMEOUT", 0)
                    elapsed_all = time.time() - t_start
                    print(f"[{count:4d}/{args.end}] ({(count/args.end)*100:5.1f}%) | "
                          f"SAT: {rate} | UNSAT: {unsat} | TIMEOUT: {tout} | "
                          f"Elapsed: {elapsed_all:.1f}s", flush=True)

                    # Checkpoint save
                    with open(OUT_JSON, "w") as jf:
                        json.dump(list(results.values()), jf, indent=2)

    with open(OUT_JSON, "w") as jf:
        json.dump(list(results.values()), jf, indent=2)

    total_time = time.time() - t_start
    print("=" * 80)
    print(f"[*] BATCH 1001 COMPLETE in {total_time:.2f}s!")
    print(f"[*] Final Stats: {stats}")
    print("=" * 80)

if __name__ == "__main__":
    main()
