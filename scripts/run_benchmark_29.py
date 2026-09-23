#!/usr/bin/env python3
"""
Scientific benchmark runner for the 29 timeout/hardest FHCP Challenge graphs.
Uses the pure-Rust unified solver binary (`src/cegar-fix/target/release/cegar-fix`).
Cutoff time: 1800.0s (SAT Competition standard).
Zero cache, zero tour injection, 100% de novo computation.
Supports concurrent workers via ThreadPoolExecutor.
"""

import os
import sys
import time
import json
import argparse
import threading
import subprocess
from datetime import datetime
from concurrent.futures import ThreadPoolExecutor, as_completed

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BINARY = os.path.join(REPO_ROOT, "src", "cegar-fix", "target", "release", "cegar-fix")
OUT_DIR = os.path.join(REPO_ROOT, "output_tours")
RESULTS_FILE = os.path.join(REPO_ROOT, "scratch", "benchmark_29_results.json")
LOG_FILE = os.path.join(REPO_ROOT, "scratch", "benchmark_29.log")
VERIFY_SCRIPT = os.path.join(REPO_ROOT, "scripts", "verify_tour.py")

os.makedirs(OUT_DIR, exist_ok=True)
os.makedirs(os.path.dirname(RESULTS_FILE), exist_ok=True)

CHALLENGE_29 = [
    710, 717, 746, 788, 832, 868, 882, 937, 944, 950,
    951, 954, 959, 960, 963, 965, 966, 971, 974, 975,
    976, 981, 982, 983, 987, 990, 993, 994, 998
]

log_lock = threading.Lock()
results_lock = threading.Lock()

def log(msg: str):
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    formatted = f"[{timestamp}] {msg}"
    with log_lock:
        print(formatted, flush=True)
        with open(LOG_FILE, "a", encoding="utf-8") as f:
            f.write(formatted + "\n")

def verify_tour(col_path: str, tour_path: str) -> bool:
    """Verifies tour independently using upstream is_hamiltonian.py via verify_tour.py."""
    if not os.path.exists(tour_path):
        return False
    try:
        proc = subprocess.run(
            [sys.executable, VERIFY_SCRIPT, col_path, tour_path],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=60.0
        )
        return "True" in proc.stdout
    except Exception:
        return False

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
            # Independent upstream certification
            if verify_tour(col_path, out_tour):
                status = "SAT_VERIFIED"
            else:
                status = "UNVERIFIED_TOUR"
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

def print_summary(results: dict, timeout: float):
    log("=" * 80)
    log("29 CHALLENGE GRAPH BENCHMARK SUMMARY:")
    log(f"{'Graph':<10} {'Status':<18} {'Time (s)':<12} {'PAR-2 (s)':<12}")
    log("-" * 55)
    solved = 0
    total_time = 0.0
    par2_penalties = []
    
    for gid in CHALLENGE_29:
        if gid in results:
            r = results[gid]
            status = r["status"]
            t = r["time"]
            penalty = t if status == "SAT_VERIFIED" else 2.0 * timeout
            par2_penalties.append(penalty)
            log(f"graph{gid:<5} {status:<18} {t:<12.2f} {penalty:<12.2f}")
            if status == "SAT_VERIFIED":
                solved += 1
            total_time += t
        else:
            par2_penalties.append(2.0 * timeout)
            log(f"graph{gid:<5} {'PENDING':<18} {'N/A':<12} {2.0*timeout:<12.2f}")
            
    par2_score = sum(par2_penalties) / len(CHALLENGE_29)
    log("-" * 55)
    log(f"Solved: {solved}/{len(CHALLENGE_29)} ({solved/len(CHALLENGE_29)*100:.1f}%)")
    log(f"Total Solve Time: {total_time:.2f}s")
    log(f"PAR-2 Score (Cutoff={timeout}s): {par2_score:.2f}s")
    log("=" * 80)

def main():
    parser = argparse.ArgumentParser(description="Run 29 Challenge Graphs Benchmark")
    parser.add_argument("--workers", type=int, default=5, help="Number of concurrent graph workers (default: 5)")
    parser.add_argument("--timeout", type=float, default=1800.0, help="Per-graph cutoff time in seconds (default: 1800.0)")
    parser.add_argument("--summary", action="store_true", help="Print summary of existing results and exit")
    parser.add_argument("--force", action="store_true", help="Force re-running all graphs even if completed")
    parser.add_argument("--graphs", nargs="+", type=int, default=CHALLENGE_29, help="Specific graph IDs to run")
    args = parser.parse_args()

    results = {}
    if os.path.exists(RESULTS_FILE) and not args.force:
        try:
            with open(RESULTS_FILE, "r", encoding="utf-8") as f:
                data = json.load(f)
                results = {item["gid"]: item for item in data}
        except Exception:
            pass

    if args.summary:
        print_summary(results, args.timeout)
        return

    # Ensure binary is built
    if not os.path.exists(BINARY):
        log("Binary not found. Building release binary...")
        subprocess.run(["cargo", "build", "--manifest-path", os.path.join(REPO_ROOT, "src", "cegar-fix", "Cargo.toml"), "--release"], check=True)

    target_graphs = [g for g in args.graphs if args.force or g not in results or results[g]["status"] not in ("SAT_VERIFIED", "TIMEOUT")]

    log("=" * 80)
    log(f"STARTING 29 CHALLENGE GRAPH BENCHMARK")
    log(f"Cutoff: {args.timeout}s | Workers: {args.workers} concurrent vCPUs")
    log(f"Total Graphs: {len(args.graphs)} | Already Completed: {len(results)} | To Run: {len(target_graphs)}")
    log("=" * 80)

    def worker_task(gid):
        res = run_single(gid, args.timeout)
        with results_lock:
            results[gid] = res
            with open(RESULTS_FILE, "w", encoding="utf-8") as f:
                json.dump(list(results.values()), f, indent=2)
        return res

    with ThreadPoolExecutor(max_workers=args.workers) as executor:
        futures = {executor.submit(worker_task, gid): gid for gid in target_graphs}
        for future in as_completed(futures):
            gid = futures[future]
            try:
                future.result()
            except Exception as e:
                log(f"Exception running graph{gid}: {e}")

    print_summary(results, args.timeout)

if __name__ == "__main__":
    main()
