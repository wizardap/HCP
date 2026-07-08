#!/usr/bin/env python3
"""
Benchmark: so sánh incremental SAT baseline vs stagnation strategies.
Strategies: none (baseline), dfj, union, both
Usage:
    python3 scripts/benchmark_strategies.py [--time-limit N] [--stagnation-k K] [--graph SUBSTR] [--out FILE]
"""
import os
import sys
import time
import subprocess
import re
import csv

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
ROOT_DIR = os.path.join(SCRIPT_DIR, "..")
SOLVER = os.path.join(ROOT_DIR, "src", "hcp-solver")
GRAPHS_DIR = os.path.join(ROOT_DIR, "graphs")

# --- Parse CLI args ---
time_limit = 600
stagnation_k = 3
graph_filter = "fhcppp"
out_csv = os.path.join(ROOT_DIR, "benchmark_results.csv")

if "--time-limit" in sys.argv:
    time_limit = int(sys.argv[sys.argv.index("--time-limit") + 1])
if "-t" in sys.argv:
    time_limit = int(sys.argv[sys.argv.index("-t") + 1])
if "--stagnation-k" in sys.argv:
    stagnation_k = int(sys.argv[sys.argv.index("--stagnation-k") + 1])
if "--graph" in sys.argv:
    graph_filter = sys.argv[sys.argv.index("--graph") + 1]
if "-g" in sys.argv:
    graph_filter = sys.argv[sys.argv.index("-g") + 1]
if "--out" in sys.argv:
    out_csv = sys.argv[sys.argv.index("--out") + 1]

# "none" = pure incremental baseline (no stagnation args at all)
# others use --stagnation-k and --stagnation-strategy
STRATEGIES = ["none", "dfj", "union", "both", "mincut"]


def find_graphs():
    graphs = []
    for root, _, filenames in os.walk(GRAPHS_DIR):
        for f in sorted(filenames):
            if f.endswith(".edge"):
                path = os.path.join(root, f)
                rel = os.path.relpath(path, GRAPHS_DIR)
                if graph_filter in rel:
                    graphs.append((rel, path))
    def sort_key(item):
        base = os.path.basename(item[0])
        m = re.search(r'\d+', base)
        return (os.path.dirname(item[0]), int(m.group()) if m else 999999)
    graphs.sort(key=sort_key)
    return graphs


def parse_output(stdout, stderr, wall_time):
    out = stdout + stderr
    result = {
        "wall_time": round(wall_time, 3),
        "status": "Timeout",
        "actions": "N/A",
        "total_solver_time": "N/A",
        "conflicts": "N/A",
    }
    if "c HAMILTONIAN found" in out:
        result["status"] = "SAT"
    elif "c UNSAT" in out:
        result["status"] = "UNSAT"

    for line in stderr.split("\n"):
        if "c incremental actions:" in line:
            try: result["actions"] = int(line.split("c incremental actions:")[1].strip())
            except: pass
        if "c total solver time:" in line:
            try: result["total_solver_time"] = float(line.split("c total solver time:")[1].strip())
            except: pass

    m = re.search(r'conflicts:\s+(\d+)', out)
    if m: result["conflicts"] = int(m.group(1))

    return result


def run_solver(graph_path, strategy):
    if strategy == "none":
        # Pure incremental baseline — no stagnation args
        cmd = [
            SOLVER, graph_path,
            "--incremental",
            "--preprocess",
            "--time-limit", str(time_limit),
        ]
    else:
        cmd = [
            SOLVER, graph_path,
            "--incremental",
            "--preprocess",
            "--time-limit", str(time_limit),
            "--stagnation-k", str(stagnation_k),
            "--stagnation-strategy", strategy,
        ]

    t0 = time.time()
    try:
        proc = subprocess.run(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=time_limit + 30,
        )
        wall = time.time() - t0
        return parse_output(proc.stdout, proc.stderr, wall)
    except subprocess.TimeoutExpired:
        return {"wall_time": time_limit, "status": "Timeout", "actions": "N/A",
                "total_solver_time": "N/A", "conflicts": "N/A"}


def fmt(r, strat):
    t = f"{r['wall_time']:.1f}s"
    st = r['status']
    acts = str(r['actions'])
    icon = "✓" if st == "SAT" else "✗"
    return f"{icon} {t:<8} act={acts:<6}"


def main():
    graphs = find_graphs()
    if not graphs:
        print(f"No graphs found matching '{graph_filter}'")
        sys.exit(1)

    print(f"\nBenchmark: incremental SAT baseline vs stagnation strategies")
    print(f"Graphs: {len(graphs)} | Time limit: {time_limit}s | Stagnation-k: {stagnation_k}")
    print(f"Strategies: {STRATEGIES}")
    print(f"Output CSV: {out_csv}\n")

    rows = []
    col_w = 26
    strat_w = 24

    header = f"{'Graph':<{col_w}}"
    for s in STRATEGIES:
        label = f"[{s}]"
        header += f"  {label:<{strat_w}}"
    print(header)
    print("-" * (col_w + len(STRATEGIES) * (strat_w + 2) + 4))

    for idx, (rel_path, abs_path) in enumerate(graphs):
        name = os.path.splitext(rel_path)[0]
        row = {"graph": name}
        line = f"{name:<{col_w}}"
        total = len(graphs)

        for strat in STRATEGIES:
            sys.stderr.write(f"  [{idx+1}/{total}] {name} [{strat}]...\n")
            sys.stderr.flush()
            r = run_solver(abs_path, strat)
            row[f"{strat}_wall"] = r["wall_time"]
            row[f"{strat}_actions"] = r["actions"]
            row[f"{strat}_status"] = r["status"]
            row[f"{strat}_solver_time"] = r["total_solver_time"]
            line += f"  {fmt(r, strat):<{strat_w}}"

        print(line)
        sys.stdout.flush()
        rows.append(row)

    # Write CSV
    fieldnames = ["graph"]
    for s in STRATEGIES:
        fieldnames += [f"{s}_wall", f"{s}_actions", f"{s}_status", f"{s}_solver_time"]

    with open(out_csv, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)

    print(f"\nCSV saved → {out_csv}")

    # ---- Summary ----
    print("\n" + "="*60)
    print("SUMMARY vs BASELINE (none)")
    print("="*60)

    wins = {s: 0 for s in STRATEGIES}
    sat_counts = {s: 0 for s in STRATEGIES}
    sat_times  = {s: [] for s in STRATEGIES}

    baseline_timeouts = [r for r in rows if r["none_status"] == "Timeout"]
    new_solve = {s: [] for s in STRATEGIES if s != "none"}

    for row in rows:
        for s in STRATEGIES:
            if row[f"{s}_status"] == "SAT":
                sat_counts[s] += 1
                sat_times[s].append(row[f"{s}_wall"])

        # count wins (fastest SAT among all)
        best_t = float("inf")
        best_s = None
        for s in STRATEGIES:
            if row[f"{s}_status"] == "SAT":
                if row[f"{s}_wall"] < best_t:
                    best_t = row[f"{s}_wall"]
                    best_s = s
        if best_s:
            wins[best_s] += 1

        # track baseline timeouts solved by new strategies
        if row["none_status"] == "Timeout":
            for s in [x for x in STRATEGIES if x != "none"]:
                if row[f"{s}_status"] == "SAT":
                    new_solve[s].append((row["graph"], row[f"{s}_wall"]))

    print(f"\n{'Strategy':<10} {'SAT solved':<14} {'Wins':<8} {'Avg time (SAT)'}")
    print("-" * 50)
    for s in STRATEGIES:
        avg = sum(sat_times[s]) / len(sat_times[s]) if sat_times[s] else float("inf")
        label = "(baseline)" if s == "none" else ""
        print(f"  {s:<10} {sat_counts[s]:<14} {wins[s]:<8} {avg:.1f}s  {label}")

    print(f"\nBaseline timeouts rescued by new strategies:")
    for s in [x for x in STRATEGIES if x != "none"]:
        if new_solve[s]:
            print(f"  {s}: {len(new_solve[s])} graphs → {', '.join(g+f'({t:.0f}s)' for g, t in new_solve[s])}")
        else:
            print(f"  {s}: 0 graphs rescued")

    print()


if __name__ == "__main__":
    main()
