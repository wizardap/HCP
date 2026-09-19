#!/usr/bin/env python3
"""
Targeted 300s Timeout Runner for the 151 unresolved graphs (under graph400).
Leverages 6 parallel workers with up to 300s per instance.
Saves certified results in real time.
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
BASE_JSON = os.path.join(REPO_ROOT, "scratch/batch_1001_results.json")
OUT_JSON = os.path.join(REPO_ROOT, "scratch/batch_400_300s_results.json")
LOG_FILE = os.path.join(REPO_ROOT, "scratch/batch_400_300s.log")
TIMEOUT_SEC = 300

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

def solve_one(gid):
    col_path = os.path.join(GRAPH_DIR, f"graph{gid}.col")
    if not os.path.exists(col_path):
        return {"gid": gid, "status": "MISSING_FILE", "time": 0.0}

    t0 = time.time()
    cmd = [
        CEGAR_BIN,
        "-i", col_path,
        "--auto", "0",
        "-e", "1",
        "-b", "3",
        "-y", "0",
        "-t", "3",
        "-l", "1",
        "--timeout", str(TIMEOUT_SEC)
    ]

    try:
        proc = subprocess.run(cmd, capture_output=True, text=True, timeout=TIMEOUT_SEC + 5)
        elapsed = time.time() - t0
        out = proc.stdout

        if "s SATISFIABLE" in out:
            tour = []
            for line in out.splitlines():
                if line.startswith("solution:"):
                    continue
                parts = line.strip().split()
                if len(parts) > 1 and all(p.isdigit() for p in parts[:min(10, len(parts))]):
                    tour = [int(p) for p in parts]
                    break

            if tour:
                num_nodes, edges = parse_dimacs(col_path)
                ok, msg = verify_cycle(tour, num_nodes, edges)
                if ok:
                    return {"gid": gid, "status": "SAT_VERIFIED", "time": elapsed, "nodes": num_nodes}
                else:
                    return {"gid": gid, "status": "SAT_INVALID", "time": elapsed, "err": msg}
            else:
                return {"gid": gid, "status": "SAT_NO_TOUR", "time": elapsed}

        elif "s UNSATISFIABLE" in out:
            return {"gid": gid, "status": "PROVED_UNSAT", "time": elapsed}
        else:
            return {"gid": gid, "status": "TIMEOUT", "time": elapsed}

    except subprocess.TimeoutExpired:
        return {"gid": gid, "status": "TIMEOUT", "time": float(TIMEOUT_SEC)}
    except Exception as e:
        return {"gid": gid, "status": "ERROR", "time": time.time() - t0, "err": str(e)}

def main():
    max_workers = 1
    # Get 151 timeout graphs from base json
    with open(BASE_JSON, "r") as f:
        base_data = json.load(f)

    target_gids = [d["gid"] for d in base_data if d["gid"] <= 400 and d["status"] == "TIMEOUT"]
    target_gids.sort()

    print(f"================================================================================")
    print(f"  TARGETED RE-RUN FOR 151 TIMEOUT INSTANCES (GRAPHS 1..400) WITH TIMEOUT={TIMEOUT_SEC}s")
    print(f"================================================================================")
    print(f"Target count: {len(target_gids)} graphs")
    print(f"Workers: {max_workers} processes")
    print(f"Log: {LOG_FILE}")
    print(f"Output: {OUT_JSON}")
    print("-" * 80)

    results = {}
    if os.path.exists(OUT_JSON):
        try:
            with open(OUT_JSON, "r") as f:
                saved = json.load(f)
                for item in saved:
                    results[item["gid"]] = item
            print(f"[*] Loaded checkpoint: {len(results)}/{len(target_gids)} already processed.")
        except Exception:
            results = {}

    to_run = [gid for gid in target_gids if gid not in results]
    print(f"[*] Running {len(to_run)} instances...")

    stats = {"SAT_VERIFIED": 0, "TIMEOUT": 0, "OTHER": 0}
    for item in results.values():
        st = item.get("status", "OTHER")
        stats[st] = stats.get(st, 0) + 1

    t_start = time.time()
    count = len(results)

    with open(LOG_FILE, "a") as log_f:
        log_f.write(f"\n--- Targeted 300s Run Started at {time.ctime()} ({len(to_run)} graphs) ---\n")

        with ProcessPoolExecutor(max_workers=max_workers) as executor:
            future_to_gid = {executor.submit(solve_one, gid): gid for gid in to_run}

            for future in as_completed(future_to_gid):
                res = future.result()
                gid = res["gid"]
                results[gid] = res
                count += 1

                st = res["status"]
                stats[st] = stats.get(st, 0) + 1
                tm = res["time"]

                line = f"[{count:3d}/{len(target_gids)}] graph{gid:4d}: {st:<18} ({tm:6.2f}s)"
                log_f.write(line + "\n")
                log_f.flush()

                sat_cnt = stats.get("SAT_VERIFIED", 0)
                tout_cnt = stats.get("TIMEOUT", 0)
                elapsed_all = time.time() - t_start
                print(f"[{count:3d}/{len(target_gids)}] ({(count/len(target_gids))*100:5.1f}%) | "
                      f"SAT: {sat_cnt} | TIMEOUT: {tout_cnt} | "
                      f"Last: graph{gid} ({tm:.1f}s) | Total Elapsed: {elapsed_all:.1f}s", flush=True)

                with open(OUT_JSON, "w") as jf:
                    json.dump(list(results.values()), jf, indent=2)

    with open(OUT_JSON, "w") as jf:
        json.dump(list(results.values()), jf, indent=2)

    print("=" * 80)
    print(f"[*] COMPLETED TARGETED 300s RUN in {time.time()-t_start:.2f}s!")
    print(f"[*] Final stats for the {len(target_gids)} timeout instances: {stats}")
    print("=" * 80)

if __name__ == "__main__":
    main()
