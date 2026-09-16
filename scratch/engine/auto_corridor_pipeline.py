#!/usr/bin/env python3
import os
import sys
import time
import subprocess

REPO_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "../.."))
if REPO_ROOT not in sys.path:
    sys.path.insert(0, REPO_ROOT)

SCRATCH_ENGINE = os.path.join(REPO_ROOT, "scratch/engine")
RESULTS_FILE = os.path.join(SCRATCH_ENGINE, "corridor_suite_results.md")

def log(msg: str):
    timestamp = time.strftime("%Y-%m-%d %H:%M:%S")
    line = f"[{timestamp}] {msg}"
    print(line, flush=True)
    with open(RESULTS_FILE, "a") as f:
        f.write(line + "\n")

def init_results_file():
    if not os.path.exists(RESULTS_FILE):
        with open(RESULTS_FILE, "w") as f:
            f.write("# Corridor Challenge Graphs Autonomous Benchmark Results\n\n")
            f.write(f"Started: {time.strftime('%Y-%m-%d %H:%M:%S')}\n")
            f.write("Time Limit per graph: 1800s (30 minutes)\n\n")
            f.write("| Graph | Dimension (N) | Corridors | Status | Runtime | Verified Sound |\n")
            f.write("| :--- | :---: | :---: | :---: | :---: | :---: |\n")

def record_entry(graph_name: str, status: str, runtime: str, verified: str):
    from scratch.engine.graph_loader import load_dimacs
    from scratch.engine.decomposer import detect_bridge_corridors
    col_path = os.path.join(REPO_ROOT, "FHCPCS-col", graph_name)
    G = load_dimacs(col_path)
    corridors, c0 = detect_bridge_corridors(G)
    c_lens = [len(c['nodes']) for c in corridors]
    entry = f"| `{graph_name}` | {len(G)} | `{c_lens}` | **{status}** | {runtime} | {verified} |\n"
    with open(RESULTS_FILE, "a") as f:
        f.write(entry)

def run_graph_benchmark(graph_name: str, time_limit_s: int = 1800):
    col_path = os.path.join(REPO_ROOT, "FHCPCS-col", graph_name)
    tour_name = f"found_tour_{graph_name.replace('.col', '')}.hcp"
    tour_path = os.path.join(SCRATCH_ENGINE, tour_name)
    
    if not os.path.exists(col_path):
        log(f"[-] {col_path} not found, skipping.")
        return

    log(f"[*] Starting benchmark for {graph_name} (limit {time_limit_s}s)...")
    cmd = [
        "timeout", str(time_limit_s),
        sys.executable,
        os.path.join(SCRATCH_ENGINE, "unified_solver.py"),
        col_path,
        tour_path
    ]
    
    t0 = time.time()
    proc = subprocess.run(cmd, capture_output=True, text=True)
    elapsed = time.time() - t0

    if proc.returncode == 0 and os.path.exists(tour_path):
        log(f"[✓] {graph_name} solved successfully in {elapsed:.2f}s! Verifying soundness...")
        verify_cmd = [
            sys.executable,
            os.path.join(REPO_ROOT, "scratch/verify_benchmarks.py"),
            "--graph", col_path,
            "--tour", tour_path
        ]
        v_proc = subprocess.run(verify_cmd, capture_output=True, text=True)
        if v_proc.returncode == 0 and "PASS - CERTIFIED SOUND" in v_proc.stdout:
            log(f"[✓✓✓] {graph_name} CERTIFIED 100% SOUND in {elapsed:.2f}s!")
            record_entry(graph_name, "SOLVED & CERTIFIED SOUND", f"{elapsed:.2f}s", "YES")
        else:
            log(f"[X] {graph_name} verification failed: {v_proc.stdout}")
            record_entry(graph_name, "VERIFICATION FAILED", f"{elapsed:.2f}s", "NO")
    elif proc.returncode == 124 or elapsed >= time_limit_s - 5:
        log(f"[!] {graph_name} TIMEOUT reached ({time_limit_s}s limit).")
        record_entry(graph_name, "TIMEOUT", f">{time_limit_s}s", "N/A")
    else:
        log(f"[X] {graph_name} exited with code {proc.returncode} in {elapsed:.2f}s.")
        if "TimeoutError" in proc.stderr or "TimeoutError" in proc.stdout:
            record_entry(graph_name, "TIMEOUT", f"{elapsed:.2f}s", "N/A")
        else:
            record_entry(graph_name, f"ERROR (code {proc.returncode})", f"{elapsed:.2f}s", "N/A")

def main():
    init_results_file()
    log("Autonomous Corridor Suite Runner resuming for remaining graphs.")

    # Sequence of remaining corridor graphs to run
    queue = [
        "graph965.col",
        "graph966.col",
        "graph971.col",
        "graph994.col",
        "graph998.col"
    ]

    for g_name in queue:
        log(f"==================================================================")
        log(f"NEXT IN QUEUE: {g_name}")
        log(f"==================================================================")
        run_graph_benchmark(g_name, time_limit_s=1800)

    log("==================================================================")
    log("[✓] Autonomous Corridor Suite Queue Execution Completed.")
    log("==================================================================")

if __name__ == "__main__":
    main()
