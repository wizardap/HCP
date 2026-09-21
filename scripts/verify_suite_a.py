#!/usr/bin/env python3
"""
Cross-verification script for Suite A (graph1..graph50) tours
against Takehide Soh's upstream verifier at ~/SAT-based-CEGAR/parse/is_hamiltonian.py.
"""

import os
import subprocess
import sys
from pathlib import Path

def main():
    success_count = 0
    fail_count = 0
    total = 50

    tour_dir = Path("scratch/suite_a_tours")
    script_path = "/home/ubuntu/SAT-based-CEGAR/parse/is_hamiltonian.py"

    if not os.path.isfile(script_path):
        print(f"Error: Upstream verifier script not found at {script_path}")
        sys.exit(1)

    for gid in range(1, total + 1):
        graph_path = f"FHCPCS-col/graph{gid}.col"
        tour_hcp_path = tour_dir / f"tour_graph{gid}.hcp"

        if not Path(graph_path).is_file():
            print(f"graph{gid:02d}: Graph file missing: {graph_path}")
            fail_count += 1
            continue
        if not tour_hcp_path.is_file():
            print(f"graph{gid:02d}: Tour file missing: {tour_hcp_path}")
            fail_count += 1
            continue

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

        # Create temporary solution file adhering to is_hamiltonian.py expectation
        tmp_sol = f"/tmp/verify_suite_a_g{gid}_{os.getpid()}.txt"
        with open(tmp_sol, "w") as f:
            f.write("solution:\n")
            f.write(" ".join(map(str, tour)) + "\n")
            f.write("s SATISFIABLE\n")

        res = subprocess.run(
            ["python3", script_path, graph_path, tmp_sol],
            capture_output=True,
            text=True
        )

        Path(tmp_sol).unlink(missing_ok=True)

        output = res.stdout.strip()
        if output.endswith("True"):
            success_count += 1
            print(f"graph{gid:02d}: PASS (upstream is_hamiltonian.py verified True, |V|={len(tour)})")
        else:
            fail_count += 1
            print(f"graph{gid:02d}: FAIL! output: {output}")

    print("\n================ UPSTREAM CROSS-VERIFICATION SUMMARY ================")
    print(f"Total verified: {success_count}/{total} ({success_count / total * 100:.1f}%)")
    print(f"Failures:       {fail_count}")
    print("====================================================================")

    if fail_count > 0:
        sys.exit(1)

if __name__ == "__main__":
    main()
