import os
import sys
import time
import subprocess
import json

VBS_IDS = [
    668, 710, 717, 746, 761, 788, 809, 832, 868, 882,
    937, 944, 950, 951, 954, 959, 960, 963, 965, 966,
    971, 974, 975, 976, 981, 982, 983, 986, 987, 990,
    993, 994, 998
]

GLOBAL_BUDGET_SECS = 1800
PER_GRAPH_TIMEOUT = 300

binary_path = "./src/cegar-fix/target/release/cegar-fix"
os.makedirs("scratch", exist_ok=True)
results_log = "scratch/vbs_batch_results.json"

start_global = time.time()
end_global = start_global + GLOBAL_BUDGET_SECS

summary_results = []

print(f"=== Starting 1800s Batch Benchmark on 33 VBS Timeout Graphs ===")
print(f"Global Time Limit: {GLOBAL_BUDGET_SECS}s | Max Per-Graph Timeout: {PER_GRAPH_TIMEOUT}s")
print(f"Core Reservation: Cores 0,1,2 (Core 3 100% reserved for user)")
print("=" * 65, flush=True)

for idx, q_id in enumerate(VBS_IDS):
    now = time.time()
    remaining_global = end_global - now
    if remaining_global <= 5:
        print(f"\n[!] Global budget of {GLOBAL_BUDGET_SECS}s exhausted. Stopping batch.", flush=True)
        break

    graph_path = f"FHCPCS-col/graph{q_id}.col"
    if not os.path.exists(graph_path):
        print(f"Skipping {graph_path} (not found)", flush=True)
        continue

    current_timeout = min(PER_GRAPH_TIMEOUT, int(remaining_global))
    out_tour = f"scratch/found_tour_{q_id}.hcp"

    cmd = [
        "taskset", "-c", "0,1,2", "nice", "-n", "19",
        "timeout", str(current_timeout),
        binary_path,
        "--input", graph_path,
        "--auto", "1",
        "--timeout", str(current_timeout),
        "--output-tour", out_tour
    ]

    print(f"\n>>> [{idx+1}/{len(VBS_IDS)}] Running graph{q_id}.col (Timeout: {current_timeout}s | Remaining global: {int(remaining_global)}s)...", flush=True)
    t0 = time.time()
    try:
        proc = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        elapsed = time.time() - t0
        stdout = proc.stdout
        stderr = proc.stderr
        ret = proc.returncode

        is_sat = "s SATISFIABLE" in stdout
        is_unsat = "s UNSATISFIABLE" in stdout
        is_to = ret == 124 or ("timeout" in stdout.lower() and not is_sat)

        status_str = "SATISFIABLE" if is_sat else ("UNSATISFIABLE" if is_unsat else ("TIMEOUT" if is_to else f"EXIT_{ret}"))
        print(f"    -> Result: {status_str} in {elapsed:.2f}s", flush=True)

        res_entry = {
            "graph_id": q_id,
            "graph_file": graph_path,
            "status": status_str,
            "elapsed_seconds": round(elapsed, 2),
            "is_satisfiable": is_sat,
            "return_code": ret,
            "stdout_tail": stdout.strip().splitlines()[-10:] if stdout else []
        }
        summary_results.append(res_entry)

        with open(results_log, "w") as fp:
            json.dump(summary_results, fp, indent=2)

    except Exception as e:
        print(f"    -> Error executing graph{q_id}: {e}", flush=True)

total_elapsed = time.time() - start_global
print("\n" + "=" * 65, flush=True)
print(f"=== Batch Benchmark Completed in {total_elapsed:.2f}s ===", flush=True)
print(f"Total Graphs Evaluated: {len(summary_results)}", flush=True)
sat_count = sum(1 for r in summary_results if r["status"] == "SATISFIABLE")
to_count = sum(1 for r in summary_results if r["status"] == "TIMEOUT")
print(f"SATISFIABLE: {sat_count} | TIMEOUT: {to_count}", flush=True)
print(f"Detailed logs written to: {results_log}", flush=True)
