#!/usr/bin/env python3
"""
Verify a single TSPLIB .hcp tour against a .col graph
using Takehide Soh's upstream verifier at ~/SAT-based-CEGAR/parse/is_hamiltonian.py.
"""

import os
import subprocess
import sys
from pathlib import Path

def main():
    if len(sys.argv) < 3:
        print("Usage: python3 scripts/verify_tour.py <graph.col> <tour.hcp>")
        sys.exit(1)

    graph_path = sys.argv[1]
    tour_hcp_path = sys.argv[2]
    script_path = "/home/ubuntu/SAT-based-CEGAR/parse/is_hamiltonian.py"

    if not os.path.isfile(graph_path):
        print(f"Error: Graph file not found: {graph_path}")
        sys.exit(1)

    if not os.path.isfile(tour_hcp_path):
        print(f"Error: Tour file not found: {tour_hcp_path}")
        sys.exit(1)

    if not os.path.isfile(script_path):
        print(f"Error: Upstream verifier script not found at {script_path}")
        sys.exit(1)

    # Parse tour from TSPLIB .hcp
    with open(tour_hcp_path, "r") as f:
        lines = f.readlines()

    tour = []
    in_section = False
    for line in lines:
        t = line.strip()
        if not t or t == "EOF" or t == "-1":
            continue
        if t == "TOUR_SECTION":
            in_section = True
            continue
        if in_section:
            try:
                tour.append(int(t))
            except ValueError:
                pass

    tmp_sol = f"/tmp/verify_tmp_{os.getpid()}_{Path(tour_hcp_path).stem}.txt"
    try:
        with open(tmp_sol, "w") as f:
            f.write("solution:\n")
            f.write(" ".join(map(str, tour)) + "\n")
            f.write("s SATISFIABLE\n")

        res = subprocess.run(
            ["python3", script_path, graph_path, tmp_sol],
            capture_output=True,
            text=True,
        )

        stdout = res.stdout.strip()
        print(stdout)
        if stdout.endswith("True"):
            sys.exit(0)
        else:
            if res.stderr:
                print(res.stderr.strip(), file=sys.stderr)
            sys.exit(1)
    finally:
        if os.path.exists(tmp_sol):
            os.remove(tmp_sol)

if __name__ == "__main__":
    main()
