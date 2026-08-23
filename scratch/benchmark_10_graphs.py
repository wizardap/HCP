#!/usr/bin/env python3
import os
import subprocess
import time
import json
import sys

TARGET_GRAPHS = [
    ("B1", "FHCPCS-col/graph560.col"),
    ("B1", "FHCPCS-col/graph584.col"),
    ("B1", "FHCPCS-col/graph612.col"),
    ("B1", "FHCPCS-col/graph797.col"),
    ("B2a", "FHCPCS-col/graph479.col"),
    ("B2b", "FHCPCS-col/graph566.col"),
    ("B2b", "FHCPCS-col/graph677.col"),
    ("B2b", "FHCPCS-col/graph725.col"),
    ("B2b", "FHCPCS-col/graph810.col"),
    ("B2b", "FHCPCS-col/graph940.col"),
]

BINARY = "/home/ubuntu/HCP/src/cegar-fix/target/release/cegar-fix"
TIMEOUT_PER_GRAPH = 300  # 300 seconds
RESULTS_FILE = "/home/ubuntu/HCP/scratch/benchmark_10_results.json"
LOG_FILE = "/home/ubuntu/HCP/scratch/benchmark_10.log"

def main():
    os.makedirs("/home/ubuntu/HCP/scratch", exist_ok=True)
    results = []
    
    with open(LOG_FILE, "w") as log_f:
        log_f.write(f"=== Starting 10-Graph Baseline Benchmark (300s cap each) ===\n")
        log_f.write(f"Binary: {BINARY}\n")
        log_f.write(f"Time: {time.strftime('%Y-%m-%d %H:%M:%S')}\n\n")
        log_f.flush()

        for idx, (cls, graph_path) in enumerate(TARGET_GRAPHS, 1):
            graph_name = os.path.basename(graph_path)
            full_path = os.path.join("/home/ubuntu/HCP", graph_path)
            
            log_f.write(f"[{idx}/10] Testing {graph_name} (Class {cls})...\n")
            log_f.flush()
            print(f"[{idx}/10] Testing {graph_name} (Class {cls})...", flush=True)

            cmd = [
                "taskset", "-c", "0,1",
                "nice", "-n", "19",
                BINARY,
                "--input", full_path,
                "-e", "1",
                "-t", "3",
                "-l", "1",
                "--timeout", str(TIMEOUT_PER_GRAPH),
            ]

            t0 = time.time()
            try:
                proc = subprocess.run(
                    cmd,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,
                    text=True,
                    timeout=TIMEOUT_PER_GRAPH + 15,
                )
                dur = time.time() - t0
                output = proc.stdout
                
                # Check outcome
                if "SUCCESS!" in output or "CERTIFICATION PASSED" in output or "Single cycle found" in output:
                    status = "SOLVED"
                elif "TIMEOUT" in output or dur >= TIMEOUT_PER_GRAPH:
                    status = "TIMEOUT"
                elif "UNSAT" in output:
                    status = "UNSAT"
                else:
                    status = "UNKNOWN"
            except subprocess.TimeoutExpired as e:
                dur = TIMEOUT_PER_GRAPH
                status = "TIMEOUT"
                output = (e.stdout or "") if isinstance(e.stdout, str) else ""

            res_entry = {
                "graph": graph_name,
                "class": cls,
                "status": status,
                "duration_seconds": round(dur, 2),
            }
            results.append(res_entry)

            log_f.write(f"    -> Result: {status} in {dur:.2f}s\n")
            log_f.flush()
            print(f"    -> Result: {status} in {dur:.2f}s", flush=True)

            with open(RESULTS_FILE, "w") as f:
                json.dump(results, f, indent=2)

        log_f.write("\n=== Benchmark Completed ===\n")
        log_f.flush()

if __name__ == "__main__":
    main()
