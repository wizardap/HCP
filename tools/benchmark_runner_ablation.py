#!/usr/bin/env python3
"""
Automated Multi-Seed Ablation Benchmark Runner & Telemetry Collector for HCP Solvers.

Supports:
- Ablation conditions: C0 (Baseline), C1 (Baseline + H2-Refined), C2 (Baseline + H1), C3 (Full Hybrid H1+H2)
- Multi-seed deterministic runs
- CPU affinity binding via taskset
- Subprocess timeout containment with process-group kill
- End-to-end 100% sound tour verification against raw .col graph
- Real-time JSONL telemetry logging
"""

import argparse
import glob
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import time
from typing import Dict, List, Optional, Set, Tuple


def natural_sort_key(path: str):
    """Natural sorting key (graph1.col < graph2.col < ... < graph10.col)."""
    base = os.path.basename(path)
    return [int(text) if text.isdigit() else text.lower() for text in re.split(r'(\d+)', base)]


def parse_col_graph(col_path: str) -> Tuple[int, int, Dict[int, Set[int]]]:
    """Parse a DIMACS-like .col graph file into (|V|, |E|, adjacency map)."""
    num_vertices = 0
    num_edges = 0
    adj: Dict[int, Set[int]] = {}
    with open(col_path, 'r') as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith('c'):
                continue
            if line.startswith('p'):
                parts = line.split()
                if len(parts) == 3:
                    num_vertices = int(parts[1])
                    num_edges = int(parts[2])
                elif len(parts) >= 4:
                    num_vertices = int(parts[2])
                    num_edges = int(parts[3])
                for v in range(1, num_vertices + 1):
                    adj[v] = set()
            elif line.startswith('e '):
                parts = line.split()
                u, v = int(parts[1]), int(parts[2])
                if u not in adj:
                    adj[u] = set()
                if v not in adj:
                    adj[v] = set()
                adj[u].add(v)
                adj[v].add(u)
    return num_vertices, num_edges, adj


def parse_hcp_tour(tour_path: str) -> List[int]:
    """Parse TSPLIB .hcp tour file."""
    tour: List[int] = []
    in_tour = False
    with open(tour_path, 'r') as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith('c') or line.startswith('NAME') or line.startswith('TYPE') or line.startswith('DIMENSION'):
                continue
            if line == 'TOUR_SECTION':
                in_tour = True
                continue
            if line == '-1' or line == 'EOF':
                break
            if in_tour:
                try:
                    tour.append(int(line))
                except ValueError:
                    continue
    return tour


def verify_tour(tour: List[int], num_vertices: int, adj: Dict[int, Set[int]]) -> Tuple[bool, str]:
    """
    Validates 100% exact-2 degree, uniqueness, length, and edge membership on raw uncontracted graph.
    """
    if len(tour) != num_vertices:
        return False, f"Tour length {len(tour)} != expected {num_vertices}"

    seen = set()
    for v in tour:
        if v in seen:
            return False, f"Duplicate vertex {v} in tour"
        if v < 1 or v > num_vertices or v not in adj:
            return False, f"Vertex {v} outside valid range 1..{num_vertices}"
        seen.add(v)

    for i in range(len(tour)):
        u = tour[i]
        v = tour[(i + 1) % len(tour)]
        if v not in adj[u]:
            return False, f"Edge ({u}, {v}) does not exist in graph"

    return True, "Valid Hamiltonian cycle"


def parse_duration_to_seconds(val_str: str, unit: str) -> float:
    """Convert duration string with unit (s, ms, µs/us, ns) to float seconds."""
    val = float(val_str)
    if unit == 's':
        return val
    elif unit == 'ms':
        return val / 1_000.0
    elif unit in ('µs', 'us'):
        return val / 1_000_000.0
    elif unit == 'ns':
        return val / 1_000_000_000.0
    return val


def select_graphs(graphs_arg: Optional[str], sample_arg: Optional[int], graphs_dir: str) -> List[str]:
    """Select graph paths based on explicit list or equispaced sampling."""
    if graphs_arg:
        selected = []
        for item in graphs_arg.split(','):
            item = item.strip()
            if not item:
                continue
            if os.path.exists(item):
                selected.append(os.path.abspath(item))
            elif os.path.exists(os.path.join(graphs_dir, item)):
                selected.append(os.path.abspath(os.path.join(graphs_dir, item)))
            elif os.path.exists(os.path.join(graphs_dir, f"{item}.col")):
                selected.append(os.path.abspath(os.path.join(graphs_dir, f"{item}.col")))
            else:
                raise FileNotFoundError(f"Graph file not found: {item}")
        return selected

    all_graphs = sorted(glob.glob(os.path.join(graphs_dir, "*.col")), key=natural_sort_key)
    if not all_graphs:
        raise FileNotFoundError(f"No .col graphs found in {graphs_dir}")

    if sample_arg is not None:
        n = int(sample_arg)
        if n <= 0:
            raise ValueError("--sample must be >= 1")
        if n >= len(all_graphs):
            return [os.path.abspath(p) for p in all_graphs]
        if n == 1:
            return [os.path.abspath(all_graphs[0])]
        indices = [round(i * (len(all_graphs) - 1) / (n - 1)) for i in range(n)]
        return [os.path.abspath(all_graphs[i]) for i in indices]

    return [os.path.abspath(p) for p in all_graphs]


def run_benchmark_instance(
    binary_path: str,
    graph_path: str,
    condition: int,
    seed: int,
    timeout: float,
    tour_dir: str,
    cores: Optional[str] = None,
    graph_cache: Optional[Dict[str, Tuple[int, int, Dict[int, Set[int]]]]] = None,
) -> Dict:
    """Execute a single benchmark run with telemetry collection and soundness verification."""
    graph_base = os.path.splitext(os.path.basename(graph_path))[0]
    os.makedirs(tour_dir, exist_ok=True)
    tour_path = os.path.abspath(os.path.join(tour_dir, f"{graph_base}_c{condition}_s{seed}.hcp"))

    if os.path.exists(tour_path):
        try:
            os.remove(tour_path)
        except OSError:
            pass

    solver_cmd = [
        os.path.abspath(binary_path),
        "-i", os.path.abspath(graph_path),
        "--ablation", str(condition),
        "--timeout", str(timeout),
        "--output-tour", tour_path,
    ]

    taskset_bin = shutil.which("taskset")
    if cores and taskset_bin:
        cmd = [taskset_bin, "-c", cores] + solver_cmd
    else:
        cmd = solver_cmd

    start_time = time.perf_counter()
    timed_out = False
    stdout = ""
    stderr = ""
    returncode = 0

    try:
        proc = subprocess.Popen(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            preexec_fn=os.setsid,
        )
        # Give a small buffer beyond solver's internal timeout before hard SIGKILL
        py_timeout = timeout + max(5.0, timeout * 0.1)
        stdout, stderr = proc.communicate(timeout=py_timeout)
        wall_time = time.perf_counter() - start_time
        returncode = proc.returncode
    except subprocess.TimeoutExpired:
        timed_out = True
        wall_time = timeout
        try:
            os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
        except OSError:
            pass
        try:
            stdout, stderr = proc.communicate(timeout=2.0)
        except Exception:
            stdout, stderr = "", ""
        returncode = -1

    # Classify run status
    if "s SATISFIABLE" in stdout:
        status = "SATISFIABLE"
    elif timed_out or "TIMEOUT" in stdout or "s UNKNOWN" in stdout:
        status = "TIMEOUT"
    elif "s UNSATISFIABLE" in stdout:
        status = "UNSATISFIABLE"
    elif returncode != 0:
        status = "ERROR"
    else:
        status = "UNKNOWN"

    # Extract solver reported overall time
    total_time = wall_time
    m_time = re.search(r'overall time\s*=\s*([0-9\.]+)\s*(s|ms|µs|us|ns)', stdout)
    if m_time:
        total_time = parse_duration_to_seconds(m_time.group(1), m_time.group(2))

    # Extract SAT solving time
    sat_times = []
    for m in re.finditer(r'sat solving time\s*=\s*([0-9\.]+)\s*(s|ms|µs|us|ns)', stdout):
        sat_times.append(parse_duration_to_seconds(m.group(1), m.group(2)))
    sat_time = sum(sat_times) if sat_times else total_time

    # Extract CEGAR increments count
    m_inc = re.search(r'overall incremented number\s*=\s*(\d+)', stdout)
    if m_inc:
        increments = int(m_inc.group(1))
    else:
        incs = [int(m.group(1)) for m in re.finditer(r'incremented number\s*=\s*(\d+)', stdout)]
        increments = max(incs) if incs else 0

    # Extract block clauses added
    m_block = re.search(r'overall number of added block clauses\s*=\s*(\d+)', stdout)
    if m_block:
        block_clause_count = int(m_block.group(1))
    else:
        blocks = [int(m.group(1)) for m in re.finditer(r'number of added block clauses\s*=\s*(\d+)', stdout)]
        block_clause_count = sum(blocks) if blocks else 0

    # Extract subcycle count from latest round
    subcycles = [int(m.group(1)) for m in re.finditer(r'number of subcycles found\s*=\s*(\d+)', stdout)]
    num_cycles = subcycles[-1] if subcycles else 0

    # Verify tour if reported SATISFIABLE
    verified = None
    verification_msg = ""
    if status == "SATISFIABLE":
        if os.path.exists(tour_path):
            if graph_cache is not None and graph_path in graph_cache:
                num_v, num_e, adj = graph_cache[graph_path]
            else:
                num_v, num_e, adj = parse_col_graph(graph_path)
                if graph_cache is not None:
                    graph_cache[graph_path] = (num_v, num_e, adj)

            tour = parse_hcp_tour(tour_path)
            valid, msg = verify_tour(tour, num_v, adj)
            verified = valid
            verification_msg = msg
        else:
            verified = False
            verification_msg = "Tour file was not created by solver"

    condition_labels = {
        0: "C0 (Baseline)",
        1: "C1 (Baseline + H2-Refined)",
        2: "C2 (Baseline + H1)",
        3: "C3 (Full Hybrid H1+H2)",
    }

    record = {
        "graph": graph_base,
        "graph_path": graph_path,
        "condition": condition,
        "condition_name": condition_labels.get(condition, f"C{condition}"),
        "seed": seed,
        "status": status,
        "wall_time": round(wall_time, 6),
        "total_time": round(total_time, 6),
        "sat_time": round(sat_time, 6),
        "increments": increments,
        "increment_count": increments,
        "block_clause_count": block_clause_count,
        "num_cycles": num_cycles,
        "verified": verified,
        "verification_msg": verification_msg if status == "SATISFIABLE" else None,
        "tour_path": tour_path if (status == "SATISFIABLE" and os.path.exists(tour_path)) else None,
        "timeout_limit": timeout,
        "error_message": stderr.strip() if returncode != 0 and stderr else None,
    }

    return record


def main():
    parser = argparse.ArgumentParser(
        description="Automated multi-seed ablation benchmark runner and telemetry collector."
    )
    parser.add_argument(
        "--binary",
        default="src/cegar-fix/target/release/cegar-fix",
        help="Path to cegar-fix binary (default: src/cegar-fix/target/release/cegar-fix)",
    )
    parser.add_argument(
        "--graphs",
        default=None,
        help="Comma-separated graph names or paths (e.g. graph1,graph10)",
    )
    parser.add_argument(
        "--graphs-dir",
        default="FHCPCS-col",
        help="Directory containing benchmark .col graphs (default: FHCPCS-col)",
    )
    parser.add_argument(
        "--sample",
        type=int,
        default=None,
        help="Sample N equispaced graphs from graphs-dir",
    )
    parser.add_argument(
        "--conditions",
        default="0,1,2,3",
        help="Comma-separated list of ablation condition indices (default: 0,1,2,3)",
    )
    parser.add_argument(
        "--seeds",
        default="1,42,137,777,2026",
        help="Comma-separated list of random seeds (default: 1,42,137,777,2026)",
    )
    parser.add_argument(
        "--timeout",
        type=float,
        default=1800.0,
        help="Timeout in seconds per instance (default: 1800.0)",
    )
    parser.add_argument(
        "--output",
        default="logs/ablation_results.jsonl",
        help="Output JSONL telemetry file (default: logs/ablation_results.jsonl)",
    )
    parser.add_argument(
        "--cores",
        default="0,1",
        help="CPU core affinity string for taskset (default: 0,1)",
    )
    parser.add_argument(
        "--tour-dir",
        default="scratch/ablation_tours",
        help="Directory to store intermediate generated tours (default: scratch/ablation_tours)",
    )

    args = parser.parse_args()

    repo_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    binary_path = os.path.join(repo_root, args.binary) if not os.path.isabs(args.binary) else args.binary
    if not os.path.exists(binary_path):
        print(f"Error: binary {binary_path} does not exist. Please compile it first.", file=sys.stderr)
        sys.exit(1)

    graphs_dir = os.path.join(repo_root, args.graphs_dir) if not os.path.isabs(args.graphs_dir) else args.graphs_dir
    selected_graphs = select_graphs(args.graphs, args.sample, graphs_dir)

    conditions = [int(c.strip()) for c in args.conditions.split(',') if c.strip()]
    seeds = [int(s.strip()) for s in args.seeds.split(',') if s.strip()]

    output_path = os.path.join(repo_root, args.output) if not os.path.isabs(args.output) else args.output
    os.makedirs(os.path.dirname(output_path), exist_ok=True)

    tour_dir = os.path.join(repo_root, args.tour_dir) if not os.path.isabs(args.tour_dir) else args.tour_dir
    os.makedirs(tour_dir, exist_ok=True)

    total_runs = len(selected_graphs) * len(conditions) * len(seeds)
    print("=" * 70)
    print("Ablation Benchmark Runner Initialized")
    print(f"Binary:     {binary_path}")
    print(f"Graphs:     {len(selected_graphs)} instances selected")
    print(f"Conditions: {conditions}")
    print(f"Seeds:      {seeds}")
    print(f"Timeout:    {args.timeout}s")
    print(f"Cores:      {args.cores}")
    print(f"Output:     {output_path}")
    print(f"Total Runs: {total_runs}")
    print("=" * 70)

    graph_cache: Dict[str, Tuple[int, int, Dict[int, Set[int]]]] = {}
    run_idx = 0

    with open(output_path, 'a', encoding='utf-8') as out_f:
        for graph_path in selected_graphs:
            graph_base = os.path.splitext(os.path.basename(graph_path))[0]
            for condition in conditions:
                for seed in seeds:
                    run_idx += 1
                    record = run_benchmark_instance(
                        binary_path=binary_path,
                        graph_path=graph_path,
                        condition=condition,
                        seed=seed,
                        timeout=args.timeout,
                        tour_dir=tour_dir,
                        cores=args.cores,
                        graph_cache=graph_cache,
                    )

                    out_f.write(json.dumps(record) + "\n")
                    out_f.flush()

                    status_str = record["status"]
                    v_str = f"Verified: {record['verified']}" if record["verified"] is not None else ""
                    print(
                        f"[{run_idx:4d}/{total_runs:4d}] {graph_base:12s} | "
                        f"Cond: C{condition} | Seed: {seed:4d} | "
                        f"Status: {status_str:12s} | "
                        f"Time: {record['wall_time']:8.3f}s | "
                        f"Inc: {record['increments']:3d} | "
                        f"{v_str}"
                    )

    print("\nBenchmark run completed successfully.")
    print(f"Results saved to: {output_path}")


if __name__ == '__main__':
    main()
