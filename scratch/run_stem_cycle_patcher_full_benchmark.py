#!/usr/bin/env python3
import os
import sys
import time
import subprocess
import json
import re

DATASET_DIR = "/home/ubuntu/HCP/FHCPCS-col"
BINARY_PATH = "/home/ubuntu/HCP/src/cegar-fix/target/release/cegar-fix"
LOG_FILE = "/home/ubuntu/HCP/results_stem_cycle_patcher_full.log"
JSON_FILE = "/home/ubuntu/HCP/scratch/stem_cycle_patcher_results.json"
BASELINE_LOG = "/home/ubuntu/HCP/results_no_sym_official.log"
TIMEOUT_SECONDS = 1800

def load_baseline():
    baseline = {}
    if not os.path.exists(BASELINE_LOG):
        return baseline
    current_g = None
    with open(BASELINE_LOG, 'r', errors='ignore') as f:
        for line in f:
            m = re.search(r'Processing FHCPCS-col/(graph\d+)\.col', line)
            if m:
                current_g = m.group(1)
            elif current_g and 's SATISFIABLE' in line:
                baseline[current_g] = {'sat': True, 'time': None}
            elif current_g and 'overall time = ' in line:
                t_str = line.split('overall time = ')[1].strip()
                secs = 0.0
                if 'ms' in t_str:
                    secs = float(t_str.replace('ms','').replace('µs','').replace('ns','')) / 1000.0
                elif 'µs' in t_str:
                    secs = float(t_str.replace('µs','')) / 1e6
                elif 's' in t_str:
                    secs = float(t_str.replace('s',''))
                if current_g in baseline:
                    baseline[current_g]['time'] = secs
    return baseline

def main():
    baseline_data = load_baseline()
    print(f"Loaded {len(baseline_data)} baseline results.")

    # Get sorted list of graphs
    graph_files = []
    for i in range(1, 1002):
        g_name = f"graph{i}.col"
        g_path = os.path.join(DATASET_DIR, g_name)
        if os.path.exists(g_path):
            graph_files.append((i, g_name, g_path))

    print(f"Found {len(graph_files)} graph files to evaluate.")

    # Load existing checkpoint if any
    results = {}
    total_solved = 0
    total_timeout = 0
    total_error = 0
    total_time = 0.0

    if os.path.exists(JSON_FILE):
        try:
            with open(JSON_FILE, 'r') as f:
                checkpoint = json.load(f)
                results = checkpoint.get("results", {})
                total_solved = checkpoint.get("total_solved", 0)
                total_timeout = checkpoint.get("total_timeout", 0)
                total_error = checkpoint.get("total_error", 0)
                total_time = checkpoint.get("total_runtime_seconds", 0.0)
                print(f"Resuming from checkpoint: {len(results)} graphs already completed.")
        except Exception as e:
            print(f"Warning: could not read checkpoint: {e}")

    with open(LOG_FILE, "a", buffering=1) as log_out:
        log_out.write(f"=== FULL BENCHMARK RUN: STEM-CYCLE PATCHER (Commit 4d094b7) ===\n")
        log_out.write(f"Timestamp: {time.strftime('%Y-%m-%d %H:%M:%S')}\n")
        log_out.write(f"Timeout: {TIMEOUT_SECONDS}s per instance\n")
        log_out.write(f"Command: {BINARY_PATH} -i <graph> -e 1 -b 3 -y 0 -t 3 -l 1 --three-opt 1\n\n")

        for idx, g_name, g_path in graph_files:
            g_key = f"graph{idx}"
            if g_key in results:
                continue

            cmd = [
                BINARY_PATH,
                "-i", g_path,
                "-e", "1",
                "-b", "3",
                "-y", "0",
                "-t", "3",
                "-l", "1",
                "--three-opt", "1"
            ]

            start_t = time.time()
            status = "UNKNOWN"
            out_text = ""

            try:
                proc = subprocess.Popen(
                    cmd,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,
                    text=True
                )
                try:
                    stdout, _ = proc.communicate(timeout=TIMEOUT_SECONDS)
                    elapsed = time.time() - start_t
                    out_text = stdout
                    if "s SATISFIABLE" in stdout:
                        status = "SATISFIABLE"
                        total_solved += 1
                    elif "s UNSATISFIABLE" in stdout:
                        status = "UNSATISFIABLE"
                        total_error += 1
                    else:
                        status = f"ERROR(exit={proc.returncode})"
                        total_error += 1
                except subprocess.TimeoutExpired:
                    proc.kill()
                    stdout, _ = proc.communicate()
                    elapsed = TIMEOUT_SECONDS
                    status = "TIMEOUT"
                    total_timeout += 1
            except Exception as e:
                elapsed = time.time() - start_t
                status = f"EXCEPTION({e})"
                total_error += 1

            total_time += elapsed

            base_info = baseline_data.get(g_key, {})
            base_t = base_info.get("time", None)
            speedup_str = "N/A"
            if base_t and status == "SATISFIABLE" and elapsed > 0:
                speedup = base_t / elapsed
                speedup_str = f"{speedup:.2f}x"

            res_entry = {
                "index": idx,
                "graph": g_key,
                "status": status,
                "time_sec": round(elapsed, 3),
                "baseline_time_sec": base_t,
                "speedup": speedup_str
            }
            results[g_key] = res_entry

            log_line = f"[{idx:4d}/1001] {g_key:<10} -> {status:<12} in {elapsed:10.3f}s | Baseline: {str(base_t):<8}s | Speedup: {speedup_str}\n"
            log_out.write(log_line)
            print(log_line, end="", flush=True)

            # Save checkpoint every graph
            checkpoint_data = {
                "total_graphs": len(graph_files),
                "completed": len(results),
                "total_solved": total_solved,
                "total_timeout": total_timeout,
                "total_error": total_error,
                "total_runtime_seconds": round(total_time, 2),
                "results": results
            }
            with open(JSON_FILE, "w") as jf:
                json.dump(checkpoint_data, jf, indent=2)

    print("\nBenchmark completed!")
    print(f"Total Solved: {total_solved}/{len(graph_files)} ({total_solved/len(graph_files)*100:.1f}%)")
    print(f"Total Time: {total_time:.2f}s ({total_time/3600:.2f} hours)")

if __name__ == "__main__":
    main()
