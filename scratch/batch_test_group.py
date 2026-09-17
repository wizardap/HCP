import subprocess
import time
import os
import sys

sys.path.append(os.path.abspath("scratch"))
from verify_benchmarks import parse_col_graph, parse_hcp_tour, verify_tour

target_graphs = [
    710, 717, 746, 788, 832, 868, 882, 937, 944, 950,
    951, 954, 959, 960, 963, 965, 966, 971, 974, 975,
    976, 981, 982, 983, 987, 990, 993, 994, 998
]

bin_path = "src/cegar-fix/target/release/cegar-fix"
os.makedirs("scratch/batch_tours", exist_ok=True)

results = []

print(f"{'Graph':<10} | {'Status':<12} | {'Time (s)':<10} | {'Verification':<25}")
print("-" * 65)

for g in target_graphs:
    col_path = f"FHCPCS-col/graph{g}.col"
    tour_path = f"scratch/batch_tours/graph{g}.tour"
    if not os.path.exists(col_path):
        print(f"graph{g:<5} | NOT FOUND    | -          | -")
        continue

    # Remove previous tour if any
    if os.path.exists(tour_path):
        os.remove(tour_path)

    t0 = time.time()
    try:
        cmd = [bin_path, col_path, "--output-tour", tour_path, "--timeout", "10"]
        proc = subprocess.run(cmd, capture_output=True, text=True, timeout=12)
        elapsed = time.time() - t0
        output = proc.stdout + proc.stderr

        if proc.returncode == 0 and "s SATISFIABLE" in output and os.path.exists(tour_path):
            num_v, num_e, adj = parse_col_graph(col_path)
            tour = parse_hcp_tour(tour_path)
            valid, reason = verify_tour(tour, num_v, adj)
            if valid:
                v_res = "PASS (100% Sound)"
                status = "SOLVED"
            else:
                v_res = f"FAIL ({reason})"
                status = "INVALID"
        elif "s UNKNOWN" in output or "TIMEOUT" in output:
            status = "TIMEOUT"
            v_res = "-"
        elif "s SATISFIABLE" in output:
            status = "SOLVED (No tour file)"
            v_res = "-"
        else:
            status = "IN_PROGRESS"
            v_res = "-"

        print(f"graph{g:<5} | {status:<12} | {elapsed:<10.3f} | {v_res:<25}")
        results.append((g, status, elapsed, v_res))
    except subprocess.TimeoutExpired:
        elapsed = time.time() - t0
        print(f"graph{g:<5} | TIMEOUT (>12s) | {elapsed:<10.3f} | -")
        results.append((g, "TIMEOUT", elapsed, "-"))
    except Exception as e:
        print(f"graph{g:<5} | ERROR: {e}")

print("\n" + "=" * 65)
solved = sum(1 for r in results if "SOLVED" in r[1])
print(f"TOTAL SOLVED IN GROUP: {solved} / {len(results)}")
print("=" * 65)
