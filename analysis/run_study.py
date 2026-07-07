#!/usr/bin/env python3
"""Run the full subtour trajectory experimental study."""

import os
import sys
import json
import subprocess
import argparse
from pathlib import Path


def discover_graphs(graphs_dir):
    """Find all .edge files recursively."""
    files = []
    for root, _, filenames in os.walk(graphs_dir):
        for f in filenames:
            if f.endswith(".edge"):
                rel = os.path.relpath(os.path.join(root, f), graphs_dir)
                files.append(rel)
    return sorted(files)


def run_solver(hcp_solver, graph_path, time_limit, trajectory_path, seed=None):
    """Run hcp-solver with --trajectory. Returns (returncode, stdout, stderr)."""
    cmd = [hcp_solver, graph_path, "--incremental",
           "--time-limit", str(time_limit),
           "--trajectory", trajectory_path]
    if seed is not None:
        cmd.extend(["--random", str(seed)])

    result = subprocess.run(
        cmd,
        capture_output=True, text=True,
        timeout=time_limit + 30,
        cwd=os.path.dirname(hcp_solver),
    )
    return result.returncode, result.stdout, result.stderr


def main():
    parser = argparse.ArgumentParser(description="Run subtour trajectory study")
    parser.add_argument("--graphs-dir", default=None,
                        help="Graph directory (default: ../graphs from script location)")
    parser.add_argument("--solver", default=None,
                        help="Path to hcp-solver binary (default: ../src/hcp-solver)")
    parser.add_argument("--output", default="experiments",
                        help="Output directory for traces and results")
    parser.add_argument("--time-limit", type=int, default=600,
                        help="Solver time limit per graph in seconds")
    parser.add_argument("--families", nargs="+",
                        default=["fhcpcs", "fhcppp", "fhcpsl", "tsphcp", "vset"],
                        help="Graph families to include")
    parser.add_argument("--g", "--graph", dest="graph_filter", default=None,
                        help="Run only graphs matching substring")
    parser.add_argument("--seeds", type=int, default=1,
                        help="Number of random seeds per graph (default: 1)")
    args = parser.parse_args()

    script_dir = os.path.dirname(os.path.abspath(__file__))
    graphs_dir = args.graphs_dir or os.path.join(script_dir, "..", "graphs")
    solver = args.solver or os.path.join(script_dir, "..", "src", "hcp-solver")
    output_dir = args.output
    os.makedirs(output_dir, exist_ok=True)

    all_files = discover_graphs(graphs_dir)

    # Filter by family
    files = [f for f in all_files if any(fam in f for fam in args.families)]
    if args.graph_filter:
        files = [f for f in files if args.graph_filter in f]

    if not files:
        print(f"No graphs found matching families {args.families}")
        return

    print(f"Running study on {len(files)} graphs with {args.seeds} seed(s) each")
    print(f"Output: {output_dir}")

    for graph_rel in files:
        graph_path = os.path.join(graphs_dir, graph_rel)
        graph_name = graph_rel.replace("/", "_").replace(".edge", "")

        for seed in range(args.seeds):
            trace_name = f"{graph_name}_seed{seed}.ndjson"
            trace_path = os.path.join(output_dir, trace_name)

            if os.path.exists(trace_path):
                print(f"  SKIP (exists): {trace_name}")
                continue

            print(f"  RUN: {trace_name}")
            try:
                rc, stdout, stderr = run_solver(solver, graph_path, args.time_limit,
                                                trace_path, seed=seed if args.seeds > 1 else None)
                # Save solver output alongside trace
                log_path = trace_path.replace(".ndjson", ".log")
                with open(log_path, "w") as f:
                    f.write(stdout)
                    f.write(stderr)
                status = "SAT" if "HAMILTONIAN found" in stderr else \
                         "UNSAT" if "UNSAT" in stderr else \
                         "TIMEOUT" if "TIMEOUT" in stderr else "UNKNOWN"
                print(f"    -> {status}")
            except subprocess.TimeoutExpired:
                print(f"    -> TIMEOUT (wall clock)")
            except Exception as e:
                print(f"    -> ERROR: {e}")

    print(f"\nDone. Traces in {output_dir}")


if __name__ == "__main__":
    main()
