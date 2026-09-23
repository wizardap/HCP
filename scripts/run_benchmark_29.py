#!/usr/bin/env python3
"""
Scientific benchmark runner for the 29 timeout/hardest FHCP Challenge graphs.
Uses the pure-Rust unified solver binary (`src/cegar-fix/target/release/cegar-fix`).
Cutoff time: 1800.0s (SAT Competition standard).
Zero cache, zero tour injection, 100% de novo computation.
"""

import os
import sys
import time
import json
import subprocess
from datetime import datetime

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BINARY = os.path.join(REPO_ROOT, "src", "cegar-fix", "target", "release", "cegar-fix")
OUT_DIR = os.path.join(REPO_ROOT, "output_tours")
RESULTS_FILE = os.path.join(REPO_ROOT, "scratch", "benchmark_29_results.json")
LOG_FILE = os.path.join(REPO_ROOT, "scratch", "benchmark_29.log")

os.makedirs(OUT_DIR, exist_ok=True)
os.makedirs(os.path.dirname(RESULTS_FILE), exist_ok=True)

CHALLENGE_29 = [
    710, 717, 746, 788, 832, 868, 882, 937, 944, 950,
    951, 954, 959, 960, 963, 965, 966, 971, 974, 975,
    976, 981, 982, 983, 987, 990, 993, 994, 998
]

def log(msg: str):
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    formatted = f"[{timestamp}] {msg}"
    print(formatted, flush=True)
    with open(LOG_FILE, "a", encoding="utf-8") as f:
        f.write(formatted + "\n")

def run_single(gid: int, timeout_sec: float = 1800.0) -> dict:
    col_path = os.path.join(REPO_ROOT, "FHCPCS-col", f"graph{gid}.col")
    out_tour = os.path.join(OUT_DIR, f"tour_graph{gid}.hcp")
    
    if not os.path.exists(col_path):
        return {"gid": gid, "status": "MISSING_FILE", "time": 0.0, "details": "File not found"}
    
    cmd = [BINARY, "-i", col_path, "-t", str(timeout_sec), "-o", out_tour]
    log(f"Starting graph{gid} (timeout={timeout_sec}s)...")
    
    t0 = time.time()
    try:
        proc = subprocess.run(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            timeout=timeout_sec + 30.0 # Grace margin for process shutdown
        )
        elapsed = time.time() - t0
        output = proc.stdout
        
        status = "UNKNOWN"
        if "s SATISFIABLE" in output or "tour certified" in output:
            status = "SAT_VERIFIED"
        elif "TIMEOUT" in output or elapsed >= timeout_sec:
            status = "TIMEOUT"
        elif "s UNSATISFIABLE" in output:
            status = "UNSAT"
        else:
            status = "ERROR"
            
        log(f"graph{gid}: {status} in {elapsed:.2f}s")
        return {
            "gid": gid,
            "status": status,
            "time": elapsed,
            "details": output[-500:] if len(output) > 500 else output
        }
    except subprocess.TimeoutExpired:
        elapsed = time.time() - t0
        log(f"graph{gid}: TIMEOUT (process hard killed after {elapsed:.2f}s)")
        return {"gid": gid, "status": "TIMEOUT", "time": elapsed, "details": "Subprocess TimeoutExpired"}
    except Exception as e:
        elapsed = time.time() - t0
        log(f"graph{gid}: ERROR ({str(e)}) in {elapsed:.2f}s")
        return {"gid": gid, "status": "ERROR", "time": elapsed, "details": str(e)}

def main():
    timeout = 1800.0
    if len(sys.argv) > 1:
        try:
            timeout = float(sys.argv[1])
        except ValueError:
            pass

    log("=" * 80)
    log(f"STARTING 29 CHALLENGE GRAPH BENCHMARK (Cutoff = {timeout}s)")
    log(f"Binary: {BINARY}")
    log(f"Graphs: {CHALLENGE_29}")
    log("=" * 80)

    # Load existing results if resuming
    results = {}
    if os.path.exists(RESULTS_FILE):
        try:
            with open(RESULTS_FILE, "r", encoding="utf-8") as f:
                data = json.load(f)
                results = {item["gid"]: item for item in data}
                log(f"Loaded {len(results)} prior results from {RESULTS_FILE}")
        except Exception:
            pass

    for idx, gid in enumerate(CHALLENGE_29, 1):
        if gid in results and results[gid]["status"] in ("SAT_VERIFIED", "TIMEOUT"):
            log(f"[{idx}/{len(CHALLENGE_29)}] graph{gid} already completed with {results[gid]['status']} ({results[gid]['time']:.2f}s). Skipping.")
            continue
        
        log(f"[{idx}/{len(CHALLENGE_29)}] Processing graph{gid}...")
        res = run_single(gid, timeout)
        results[gid] = res
        
        # Save atomically
        with open(RESULTS_FILE, "w", encoding="utf-8") as f:
            json.dump(list(results.values()), f, indent=2)

    # Summary
    log("=" * 80)
    log("BENCHMARK EXECUTION COMPLETE. SUMMARY:")
    log(f"{'Graph':<10} {'Status':<15} {'Time (s)':<12}")
    log("-" * 40)
    solved = 0
    total_time = 0.0
    for gid in CHALLENGE_29:
        if gid in results:
            r = results[gid]
            log(f"graph{gid:<5} {r['status']:<15} {r['time']:<12.2f}")
            if r['status'] == 'SAT_VERIFIED':
                solved += 1
            total_time += r['time']
    log("-" * 40)
    log(f"Total Solved: {solved}/{len(CHALLENGE_29)} ({solved/len(CHALLENGE_29)*100:.1f}%)")
    log(f"Total Wall Time: {total_time:.2f}s")
    log("=" * 80)

if __name__ == "__main__":
    main()
